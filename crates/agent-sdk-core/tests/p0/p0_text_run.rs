use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

use agent_sdk_core::{
    Agent, AgentError, AgentEventBus, AgentId, AgentRuntime, AllowToolPolicy, CapabilityId,
    CapabilityNamespace, CapabilitySpec, ContextProjection, DestinationKind, DestinationRef,
    EffectClass, ExecutorRef, HookExecutionOutcome, InMemoryAgentEventBus, PackageSidecarRef,
    PolicyDecision, PolicyKind, PolicyOutcome, PolicyRef, PolicyStage, PrivacyClass,
    ProviderAdapter, ProviderCapabilities, ProviderMessageRole, ProviderProjectionPolicy,
    ProviderRequest, ProviderResponse, ProviderRouteSnapshot, ProviderStopReason, ProviderToolCall,
    ProviderUsage, RetentionClass, RiskClass, RunId, RunRequest, RunStatus, RuntimePackage,
    RuntimePackageId, RuntimePolicyPort, SessionId, SessionTimeline, SourceKind, SourceRef,
    ToolCallId, ToolExecutionOutput, ToolRoute, TurnId, TurnTrace,
    domain::ContentRef as ContentRefId,
    event::{EventFamily, EventFilterSet, EventIndexField, EventKind},
    hook_ports::InMemoryHookExecutorRegistry,
    package_hooks::{
        ContextInjectionRequest, HookFailurePolicy, HookMutationRight, HookMutationRights,
        HookPoint, HookResponse, HookSource, HookSpec, RepairNeededReason, RetryRequest,
        StopReason,
    },
    project_context_projection,
    testing::{FakeContentResolver, FakeJournalStore, FakeProvider},
    testing::{ScriptedHookExecutor, ScriptedToolExecutor},
    tool_records::{CanonicalToolName, ToolCallRecordStatus},
};

#[derive(Clone, Debug)]
struct AllowP0Policy;

impl RuntimePolicyPort for AllowP0Policy {
    fn evaluate_run_start(
        &self,
        request: &RunRequest,
        _package: &RuntimePackage,
    ) -> Result<PolicyOutcome, agent_sdk_core::AgentError> {
        Ok(PolicyOutcome {
            stage: PolicyStage::Input,
            decision: PolicyDecision::allow("policy.p0.allow"),
            subject: None,
            source: Some(request.source.clone()),
            destination: None,
            policy_refs: vec![PolicyRef::with_kind(
                PolicyKind::RuntimePackage,
                "policy.p0",
            )],
            privacy: PrivacyClass::Internal,
            retention: RetentionClass::RunScoped,
        })
    }
}

#[test]
fn p0_fake_provider_text_run_emits_journal_events_and_result() {
    let agent = p0_agent();
    let provider = FakeProvider::with_responses(["hello from fake provider"]);
    let journal = FakeJournalStore::default();
    let event_bus = InMemoryAgentEventBus::default();
    let runtime = p0_runtime(&agent, provider.clone(), journal.clone(), event_bus.clone());

    let result = agent
        .run_text(
            &runtime,
            RunId::new("run.p0.text"),
            SourceRef::with_kind(SourceKind::Host, "source.p0.test"),
            "hello sdk",
        )
        .expect("P0 run succeeds");

    assert_eq!(result.status, RunStatus::Completed);
    assert_eq!(result.output, "hello from fake provider");

    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].projection_item_count, 1);
    assert_eq!(requests[0].messages[0].content, "hello sdk");

    let journal_summary = journal_summary(&journal.records());
    assert_eq!(
        journal_summary,
        fixture("tests/fixtures/p0/text-run-journal.json")
    );

    let frames = event_bus
        .subscribe_run(RunId::new("run.p0.text"), None)
        .expect("event stream")
        .collect::<Vec<_>>();
    assert_eq!(
        event_summary(&frames),
        fixture("tests/fixtures/p0/text-run-events.json")
    );
    assert_eq!(
        frames.last().unwrap().event.envelope.event_kind,
        EventKind::TurnCompleted
    );
    assert!(
        frames
            .last()
            .unwrap()
            .event
            .envelope
            .journal_cursor
            .is_some()
    );
}

#[test]
fn agent_and_runtime_text_paths_use_the_same_run_request_shape() {
    let agent = p0_agent();
    let source = SourceRef::with_kind(SourceKind::Host, "source.p0.request-shape");
    let expected = RunRequest::text(
        RunId::new("run.p0.request-shape"),
        agent.id().clone(),
        source.clone(),
        "same input",
    );

    let provider = FakeProvider::with_responses(["same output"]);
    let runtime = p0_runtime(
        &agent,
        provider.clone(),
        FakeJournalStore::default(),
        InMemoryAgentEventBus::default(),
    );

    let result = runtime
        .run_text(expected.clone())
        .expect("explicit runtime text run succeeds");

    assert_eq!(result.output, "same output");
    assert_eq!(provider.requests()[0].messages[0].content, expected.input);
    assert_eq!(expected.agent_id, agent.id().clone());
    assert_eq!(expected.source, source);
}

#[test]
fn provider_tool_use_executes_tool_and_continues_with_tool_result() {
    let agent = p0_agent();
    let provider = ToolLoopProvider::with_responses([
        ProviderResponse::tool_use([ProviderToolCall::new(
            ToolCallId::new("tool.call.p0.read"),
            CanonicalToolName::new("workspace_read"),
            "read docs/start-here.md",
        )
        .with_args_ref(ContentRefId::new("content.args.p0.read"))]),
        ProviderResponse::text("final answer after reading"),
    ]);
    let journal = FakeJournalStore::default();
    let event_bus = InMemoryAgentEventBus::default();
    let executor = Arc::new(ScriptedToolExecutor::new(
        ExecutorRef::new("executor.workspace_read.v1"),
        ToolExecutionOutput::completed("workspace read returned content refs"),
    ));
    let package = p0_package_with_tool(&agent);
    let runtime = AgentRuntime::builder()
        .default_package(package)
        .provider("provider.fake", provider.clone())
        .expect("provider route registers")
        .journal(journal.clone())
        .event_bus(event_bus.clone())
        .content(FakeContentResolver::default())
        .policy(AllowP0Policy)
        .tool_route(p0_read_tool_route())
        .tool_executor(executor.clone())
        .expect("tool executor registers")
        .tool_policy(AllowToolPolicy)
        .build()
        .expect("runtime builds");

    let result = runtime
        .run_text(RunRequest::text(
            RunId::new("run.p0.tool-loop"),
            agent.id().clone(),
            SourceRef::with_kind(SourceKind::Host, "source.p0.tool-loop"),
            "use the workspace tool",
        ))
        .expect("provider tool loop succeeds");

    assert_eq!(result.status, RunStatus::Completed);
    assert_eq!(result.output, "final answer after reading");
    assert_eq!(executor.call_count(), 1);
    assert_eq!(
        executor.calls()[0]
            .resolved_call
            .request
            .requested_args_refs,
        vec![ContentRefId::new("content.args.p0.read")]
    );

    let requests = provider.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].tools.len(), 1);
    assert_eq!(requests[0].tools[0].name, "workspace_read");
    assert_eq!(
        requests[0].tools[0].schema_ref.sidecar_id,
        "sidecar.schema.read"
    );
    assert_eq!(requests[1].tools.len(), 1);
    let tool_message = requests[1]
        .messages
        .last()
        .expect("tool result continuation message");
    assert_eq!(tool_message.role, ProviderMessageRole::Tool);
    assert_eq!(
        tool_message.content,
        "workspace_read: workspace read returned content refs"
    );
    assert_eq!(
        requests[1].projection_item_count,
        requests[1].messages.len()
    );

    let tool_records = journal
        .records()
        .into_iter()
        .filter_map(|record| match record.payload {
            agent_sdk_core::JournalRecordPayload::Tool(tool) => Some(tool),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(tool_records.len(), 2);
    assert_eq!(
        tool_records.last().unwrap().status,
        ToolCallRecordStatus::Completed
    );

    let frames = event_bus
        .subscribe_run(RunId::new("run.p0.tool-loop"), None)
        .expect("event stream")
        .collect::<Vec<_>>();
    let model_completed_events = frames
        .iter()
        .filter(|frame| frame.event.envelope.event_kind == EventKind::ModelMessageCompleted)
        .count();
    assert_eq!(model_completed_events, 2);
    let tool_event_kinds = frames
        .iter()
        .filter(|frame| frame.event.envelope.event_family == EventFamily::Tool)
        .map(|frame| frame.event.envelope.event_kind.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        tool_event_kinds,
        vec![EventKind::ToolStarted, EventKind::ToolCompleted]
    );
}

#[test]
fn session_turn_trace_groups_only_records_for_one_user_turn() {
    let agent = p0_agent();
    let provider = CapturingProvider::with_responses(["first output", "second output"]);
    let journal = FakeJournalStore::default();
    let event_bus = InMemoryAgentEventBus::default();
    let hook = HookSpec::blocking(
        "inject.trace_context",
        HookPoint::BeforeContextAssembly,
        HookSource::InProcess,
        "executor.trace_context",
        hook_policy("policy.hooks.trace"),
        HookMutationRights::from_rights([HookMutationRight::InjectContext]),
    );
    let registry = registry_with(ScriptedHookExecutor::new(
        "executor.trace_context",
        [
            Ok(HookExecutionOutcome::new(
                HookResponse::InjectContext(vec![ContextInjectionRequest {
                    redacted_summary: "turn one context".to_string(),
                    policy_refs: vec![hook_policy("policy.hooks.trace.context")],
                }]),
                1,
            )),
            Ok(HookExecutionOutcome::new(HookResponse::ObserveOnly, 1)),
        ],
    ));
    let runtime = p0_runtime_with_package(
        p0_package_with_hook(&agent, hook),
        provider.clone(),
        journal.clone(),
        event_bus.clone(),
        registry,
    );
    let session_id = SessionId::new("session.p0.trace");
    let turn_one = TurnId::new("turn.p0.trace.one");
    let turn_two = TurnId::new("turn.p0.trace.two");

    runtime
        .run_text(
            RunRequest::text(
                RunId::new("run.p0.trace.one"),
                agent.id().clone(),
                SourceRef::with_kind(SourceKind::Host, "source.p0.trace"),
                "first question",
            )
            .with_session_turn(session_id.clone(), turn_one.clone()),
        )
        .expect("first traced turn succeeds");
    runtime
        .run_text(
            RunRequest::text(
                RunId::new("run.p0.trace.two"),
                agent.id().clone(),
                SourceRef::with_kind(SourceKind::Host, "source.p0.trace"),
                "second question",
            )
            .with_session_turn(session_id.clone(), turn_two.clone()),
        )
        .expect("second traced turn succeeds");

    assert_eq!(provider.projections()[0].items.len(), 2);
    assert_eq!(provider.projections()[1].items.len(), 1);

    let records = journal.records();
    let turn_one_trace = TurnTrace::from_records(&turn_one, records.iter());
    let turn_two_trace = TurnTrace::from_records(&turn_two, records.iter());
    assert_eq!(turn_one_trace.session_id, Some(session_id.clone()));
    assert_eq!(turn_one_trace.turn_id, Some(turn_one.clone()));
    assert_eq!(turn_one_trace.run_ids, vec![RunId::new("run.p0.trace.one")]);
    assert!(turn_one_trace.records.iter().all(|record| {
        record.session_id.as_ref() == Some(&session_id)
            && record.turn_id.as_ref() == Some(&turn_one)
            && record.run_id == RunId::new("run.p0.trace.one")
    }));
    assert!(turn_two_trace.records.iter().all(|record| {
        record.session_id.as_ref() == Some(&session_id)
            && record.turn_id.as_ref() == Some(&turn_two)
            && record.run_id == RunId::new("run.p0.trace.two")
    }));
    assert!(
        turn_one_trace
            .records
            .iter()
            .any(|record| payload_type(&record.payload) == "hook")
    );
    assert_eq!(
        turn_one_trace
            .records
            .iter()
            .filter(|record| payload_type(&record.payload) == "turn_lifecycle")
            .count(),
        2
    );
    assert_eq!(turn_one_trace.context_projection_ids.len(), 1);
    assert_eq!(turn_one_trace.message_ids.len(), 2);
    assert!(!turn_one_trace.effect_ids.is_empty());

    let timeline = SessionTimeline::from_records(&session_id, records.iter());
    assert_eq!(
        timeline
            .turns
            .iter()
            .map(|trace| trace.turn_id.clone())
            .collect::<Vec<_>>(),
        vec![Some(turn_one.clone()), Some(turn_two.clone())]
    );

    let session_filter = agent_sdk_core::EventFilter {
        session_ids: EventFilterSet::Include(vec![session_id.clone()]),
        ..agent_sdk_core::EventFilter::default()
    }
    .compile()
    .expect("session filter compiles");
    assert!(
        session_filter
            .indexed_fields
            .contains(&EventIndexField::SessionId)
    );
    let frames = event_bus
        .subscribe_all(None)
        .expect("event stream")
        .collect::<Vec<_>>();
    assert!(
        frames
            .iter()
            .all(|frame| { frame.event.envelope.session_id.as_ref() == Some(&session_id) })
    );
}

#[test]
fn p0_event_bus_assigns_monotonic_sequences_across_runs() {
    let agent = p0_agent();
    let provider = FakeProvider::with_responses(["first output", "second output"]);
    let event_bus = InMemoryAgentEventBus::default();
    let runtime = p0_runtime(
        &agent,
        provider,
        FakeJournalStore::default(),
        event_bus.clone(),
    );

    runtime
        .run_text(RunRequest::text(
            RunId::new("run.p0.cursor.first"),
            agent.id().clone(),
            SourceRef::with_kind(SourceKind::Host, "source.p0.cursor"),
            "first",
        ))
        .expect("first run");
    runtime
        .run_text(RunRequest::text(
            RunId::new("run.p0.cursor.second"),
            agent.id().clone(),
            SourceRef::with_kind(SourceKind::Host, "source.p0.cursor"),
            "second",
        ))
        .expect("second run");

    let all_frames = event_bus
        .subscribe_all(None)
        .expect("all stream")
        .collect::<Vec<_>>();
    assert_eq!(
        all_frames
            .iter()
            .map(|frame| frame.event.envelope.event_seq)
            .collect::<Vec<_>>(),
        (1..=16).collect::<Vec<_>>()
    );

    let first_terminal_cursor = all_frames[7].cursor.clone();
    let resumed_all = event_bus
        .subscribe_all(Some(first_terminal_cursor.clone()))
        .expect("resume all")
        .collect::<Vec<_>>();
    assert_eq!(resumed_all.len(), 8);
    assert!(
        resumed_all
            .iter()
            .all(|frame| { frame.event.envelope.run_id == RunId::new("run.p0.cursor.second") })
    );

    let agent_frames = event_bus
        .subscribe_agent(agent.id().clone(), None)
        .expect("agent stream")
        .collect::<Vec<_>>();
    let first_agent_terminal_cursor = agent_frames[7].cursor.clone();
    let resumed_agent = event_bus
        .subscribe_agent(agent.id().clone(), Some(first_agent_terminal_cursor))
        .expect("resume agent")
        .collect::<Vec<_>>();
    assert_eq!(resumed_agent.len(), 8);
    assert_eq!(
        resumed_agent[0].event.envelope.event_seq,
        resumed_all[0].event.envelope.event_seq
    );
}

#[test]
fn before_context_hook_injects_admitted_context_before_provider_projection() {
    let agent = p0_agent();
    let provider = CapturingProvider::with_responses(["hook-aware output"]);
    let journal = FakeJournalStore::default();
    let hook = HookSpec::blocking(
        "inject.before_context",
        HookPoint::BeforeContextAssembly,
        HookSource::InProcess,
        "executor.inject.before_context",
        hook_policy("policy.hooks.inject"),
        HookMutationRights::from_rights([HookMutationRight::InjectContext]),
    );
    let registry = registry_with(ScriptedHookExecutor::once(
        "executor.inject.before_context",
        HookResponse::InjectContext(vec![ContextInjectionRequest {
            redacted_summary: "hook supplied context".to_string(),
            policy_refs: vec![hook_policy("policy.hooks.inject.context")],
        }]),
        1,
    ));
    let runtime = p0_runtime_with_package(
        p0_package_with_hook(&agent, hook),
        provider.clone(),
        journal.clone(),
        InMemoryAgentEventBus::default(),
        registry,
    );

    let result = agent
        .run_text(
            &runtime,
            RunId::new("run.p0.hook.inject"),
            SourceRef::with_kind(SourceKind::Host, "source.p0.hook"),
            "hello sdk",
        )
        .expect("hook-injected P0 run succeeds");

    assert_eq!(result.output, "hook-aware output");
    let projection = provider.projections()[0].clone();
    assert_eq!(projection.items.len(), 2);
    let injected = &projection.items[1];
    assert_eq!(
        injected.inline_redacted_summary.as_deref(),
        Some("hook supplied context")
    );
    assert_eq!(format!("{:?}", injected.source_ref.kind), "Hook");
    assert_eq!(injected.source_ref.as_str(), "inject.before_context");
    assert_eq!(
        injected.policy_refs,
        vec![hook_policy("policy.hooks.inject.context")]
    );
    let context_record = journal
        .records()
        .into_iter()
        .find_map(|record| match record.payload {
            agent_sdk_core::JournalRecordPayload::ContextProjection(context) => Some(context),
            _ => None,
        })
        .expect("context projection record");
    assert_eq!(context_record.selected_item_count, 2);
    assert_eq!(provider.requests()[0].projection_item_count, 2);
}

#[test]
fn before_run_complete_retry_hook_gets_one_more_provider_attempt() {
    let agent = p0_agent();
    let provider = FakeProvider::with_responses(["missing final detail", "complete final answer"]);
    let hook = before_complete_retry_hook();
    let registry = registry_with(ScriptedHookExecutor::new(
        "executor.before_complete.retry",
        [
            Ok(HookExecutionOutcome::new(
                HookResponse::RequestRetry(RetryRequest {
                    redacted_summary: "retry with the missing final detail".to_string(),
                }),
                1,
            )),
            Ok(HookExecutionOutcome::new(HookResponse::ObserveOnly, 1)),
        ],
    ));
    let runtime = p0_runtime_with_package(
        p0_package_with_hook(&agent, hook),
        provider.clone(),
        FakeJournalStore::default(),
        InMemoryAgentEventBus::default(),
        registry,
    );

    let result = runtime
        .run_text(RunRequest::text(
            RunId::new("run.p0.hook.retry"),
            agent.id().clone(),
            SourceRef::with_kind(SourceKind::Host, "source.p0.hook"),
            "draft",
        ))
        .expect("hook retry succeeds");

    assert_eq!(result.output, "complete final answer");
    let requests = provider.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[1].messages.last().expect("retry nudge").content,
        "retry with the missing final detail"
    );
}

#[test]
fn before_run_complete_retry_budget_exhaustion_fails_closed() {
    let agent = p0_agent();
    let provider = FakeProvider::with_responses(["first", "second", "third"]);
    let journal = FakeJournalStore::default();
    let hook = before_complete_retry_hook();
    let registry = registry_with(ScriptedHookExecutor::new(
        "executor.before_complete.retry",
        [
            Ok(HookExecutionOutcome::new(
                HookResponse::RequestRetry(RetryRequest {
                    redacted_summary: "retry once".to_string(),
                }),
                1,
            )),
            Ok(HookExecutionOutcome::new(
                HookResponse::RequestRetry(RetryRequest {
                    redacted_summary: "retry twice".to_string(),
                }),
                1,
            )),
        ],
    ));
    let runtime = p0_runtime_with_package(
        p0_package_with_hook(&agent, hook),
        provider.clone(),
        journal.clone(),
        InMemoryAgentEventBus::default(),
        registry,
    );

    let error = runtime
        .run_text(RunRequest::text(
            RunId::new("run.p0.hook.retry-exhausted"),
            agent.id().clone(),
            SourceRef::with_kind(SourceKind::Host, "source.p0.hook"),
            "draft",
        ))
        .expect_err("retry budget exhaustion fails closed");

    assert_eq!(
        error.kind(),
        agent_sdk_core::AgentErrorKind::RecoveryRepairNeeded
    );
    assert_eq!(provider.requests().len(), 2);
    assert_eq!(accepted_retry_hook_records(&journal), 1);
    assert_eq!(rejected_retry_hook_records(&journal), 1);
}

#[test]
fn before_run_complete_repair_needed_hook_stops_normal_completion() {
    let agent = p0_agent();
    let provider = FakeProvider::with_responses(["not ready"]);
    let mut hook = HookSpec::blocking(
        "repair.before_complete",
        HookPoint::BeforeRunComplete,
        HookSource::InProcess,
        "executor.before_complete.repair",
        hook_policy("policy.hooks.repair"),
        HookMutationRights::from_rights([HookMutationRight::StopCompletionWithRepairNeeded]),
    );
    hook.failure = HookFailurePolicy::InterruptRun;
    let registry = registry_with(ScriptedHookExecutor::once(
        "executor.before_complete.repair",
        HookResponse::StopCompletionWithRepairNeeded(RepairNeededReason {
            code: "missing_condition".to_string(),
            redacted_summary: "completion condition was not met".to_string(),
        }),
        1,
    ));
    let journal = FakeJournalStore::default();
    let runtime = p0_runtime_with_package(
        p0_package_with_hook(&agent, hook),
        provider.clone(),
        journal.clone(),
        InMemoryAgentEventBus::default(),
        registry,
    );

    let error = runtime
        .run_text(RunRequest::text(
            RunId::new("run.p0.hook.repair-needed"),
            agent.id().clone(),
            SourceRef::with_kind(SourceKind::Host, "source.p0.hook"),
            "draft",
        ))
        .expect_err("repair-needed hook stops completion");

    assert_eq!(
        error.kind(),
        agent_sdk_core::AgentErrorKind::RecoveryRepairNeeded
    );
    assert_eq!(provider.requests().len(), 1);
    assert!(journal.records().iter().any(|record| {
        matches!(
            &record.payload,
            agent_sdk_core::JournalRecordPayload::TerminalResult(marker)
                if marker.terminal_status == "failed"
        )
    }));
}

#[test]
fn after_run_terminal_observe_hook_runs_after_result_is_sealed() {
    let agent = p0_agent();
    let provider = FakeProvider::with_responses(["done"]);
    let hook = HookSpec::observe(
        "observe.after_terminal",
        HookPoint::AfterRunTerminal,
        HookSource::InProcess,
        "executor.after_terminal.observe",
        hook_policy("policy.hooks.after_terminal"),
    );
    let executor = ScriptedHookExecutor::once(
        "executor.after_terminal.observe",
        HookResponse::ObserveOnly,
        1,
    );
    let executor_view = executor.clone();
    let registry = registry_with(executor);
    let runtime = p0_runtime_with_package(
        p0_package_with_hook(&agent, hook),
        provider,
        FakeJournalStore::default(),
        InMemoryAgentEventBus::default(),
        registry,
    );

    let result = runtime
        .run_text(RunRequest::text(
            RunId::new("run.p0.hook.after-terminal"),
            agent.id().clone(),
            SourceRef::with_kind(SourceKind::Host, "source.p0.hook"),
            "draft",
        ))
        .expect("terminal observe does not change result");

    assert_eq!(result.status, RunStatus::Completed);
    assert_eq!(executor_view.invocations().len(), 1);
}

#[test]
fn missing_hook_executor_fails_before_provider_call() {
    let agent = p0_agent();
    let provider = FakeProvider::with_responses(["should not be used"]);
    let runtime = p0_runtime_with_package(
        p0_package_with_hook(&agent, before_complete_retry_hook()),
        provider.clone(),
        FakeJournalStore::default(),
        InMemoryAgentEventBus::default(),
        InMemoryHookExecutorRegistry::default(),
    );

    let error = runtime
        .run_text(RunRequest::text(
            RunId::new("run.p0.hook.missing-executor"),
            agent.id().clone(),
            SourceRef::with_kind(SourceKind::Host, "source.p0.hook"),
            "draft",
        ))
        .expect_err("missing executor fails before provider");

    assert_eq!(error.kind(), agent_sdk_core::AgentErrorKind::InvalidPackage);
    assert!(provider.requests().is_empty());
}

#[test]
fn unsupported_before_run_complete_mutation_right_fails_before_provider_call() {
    let agent = p0_agent();
    let provider = FakeProvider::with_responses(["should not be used"]);
    let hook = HookSpec::blocking(
        "detach.before_complete",
        HookPoint::BeforeRunComplete,
        HookSource::InProcess,
        "executor.before_complete.detach",
        hook_policy("policy.hooks.detach"),
        HookMutationRights::from_rights([HookMutationRight::ValidateDetach]),
    );
    let registry = registry_with(ScriptedHookExecutor::once(
        "executor.before_complete.detach",
        HookResponse::ObserveOnly,
        1,
    ));
    let runtime = p0_runtime_with_package(
        p0_package_with_hook(&agent, hook),
        provider.clone(),
        FakeJournalStore::default(),
        InMemoryAgentEventBus::default(),
        registry,
    );

    let error = runtime
        .run_text(RunRequest::text(
            RunId::new("run.p0.hook.unsupported-right"),
            agent.id().clone(),
            SourceRef::with_kind(SourceKind::Host, "source.p0.hook"),
            "draft",
        ))
        .expect_err("unsupported P0 hook right fails before provider");

    assert_eq!(error.kind(), agent_sdk_core::AgentErrorKind::InvalidPackage);
    assert!(provider.requests().is_empty());
}

#[test]
fn multiple_before_run_complete_mutation_hooks_fail_before_provider_call() {
    let agent = p0_agent();
    let provider = FakeProvider::with_responses(["should not be used"]);
    let retry_hook = before_complete_retry_hook();
    let repair_hook = HookSpec::blocking(
        "repair.before_complete",
        HookPoint::BeforeRunComplete,
        HookSource::InProcess,
        "executor.before_complete.repair",
        hook_policy("policy.hooks.repair"),
        HookMutationRights::from_rights([HookMutationRight::StopCompletionWithRepairNeeded]),
    );
    let registry = registry_with_all([
        ScriptedHookExecutor::once(
            "executor.before_complete.retry",
            HookResponse::RequestRetry(RetryRequest {
                redacted_summary: "retry".to_string(),
            }),
            1,
        ),
        ScriptedHookExecutor::once(
            "executor.before_complete.repair",
            HookResponse::StopCompletionWithRepairNeeded(RepairNeededReason {
                code: "not_ready".to_string(),
                redacted_summary: "repair".to_string(),
            }),
            1,
        ),
    ]);
    let package = RuntimePackage::builder(RuntimePackageId::new("package.p0.contract"))
        .agent(agent.snapshot())
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake.p0"))
        .policy(PolicyRef::with_kind(
            PolicyKind::RuntimePackage,
            "policy.p0.package",
        ))
        .hook(retry_hook)
        .hook(repair_hook)
        .build()
        .expect("package builds");
    let runtime = p0_runtime_with_package(
        package,
        provider.clone(),
        FakeJournalStore::default(),
        InMemoryAgentEventBus::default(),
        registry,
    );

    let error = runtime
        .run_text(RunRequest::text(
            RunId::new("run.p0.hook.multi-mutating"),
            agent.id().clone(),
            SourceRef::with_kind(SourceKind::Host, "source.p0.hook"),
            "draft",
        ))
        .expect_err("multiple mutating completion hooks fail before provider");

    assert_eq!(error.kind(), agent_sdk_core::AgentErrorKind::InvalidPackage);
    assert!(provider.requests().is_empty());
}

#[test]
fn before_context_mutation_with_another_hook_fails_before_provider_call() {
    let agent = p0_agent();
    let provider = FakeProvider::with_responses(["should not be used"]);
    let inject_hook = HookSpec::blocking(
        "inject.before_context",
        HookPoint::BeforeContextAssembly,
        HookSource::InProcess,
        "executor.inject.before_context",
        hook_policy("policy.hooks.inject"),
        HookMutationRights::from_rights([HookMutationRight::InjectContext]),
    );
    let observe_hook = HookSpec::observe(
        "observe.before_context",
        HookPoint::BeforeContextAssembly,
        HookSource::InProcess,
        "executor.observe.before_context",
        hook_policy("policy.hooks.observe"),
    );
    let registry = registry_with_all([
        ScriptedHookExecutor::once(
            "executor.inject.before_context",
            HookResponse::InjectContext(vec![ContextInjectionRequest {
                redacted_summary: "context".to_string(),
                policy_refs: vec![hook_policy("policy.hooks.inject.context")],
            }]),
            1,
        ),
        ScriptedHookExecutor::once(
            "executor.observe.before_context",
            HookResponse::ObserveOnly,
            1,
        ),
    ]);
    let package = RuntimePackage::builder(RuntimePackageId::new("package.p0.contract"))
        .agent(agent.snapshot())
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake.p0"))
        .policy(PolicyRef::with_kind(
            PolicyKind::RuntimePackage,
            "policy.p0.package",
        ))
        .hook(inject_hook)
        .hook(observe_hook)
        .build()
        .expect("package builds");
    let runtime = p0_runtime_with_package(
        package,
        provider.clone(),
        FakeJournalStore::default(),
        InMemoryAgentEventBus::default(),
        registry,
    );

    let error = runtime
        .run_text(RunRequest::text(
            RunId::new("run.p0.hook.before-context-mixed"),
            agent.id().clone(),
            SourceRef::with_kind(SourceKind::Host, "source.p0.hook"),
            "draft",
        ))
        .expect_err("mixed before-context mutation hooks fail before provider");

    assert_eq!(error.kind(), agent_sdk_core::AgentErrorKind::InvalidPackage);
    assert!(provider.requests().is_empty());
}

#[test]
fn non_invoked_behavior_changing_hook_fails_before_provider_call() {
    let agent = p0_agent();
    let provider = FakeProvider::with_responses(["should not be used"]);
    let hook = HookSpec::blocking(
        "stop.before_model",
        HookPoint::BeforeModelCall,
        HookSource::InProcess,
        "executor.before_model.stop",
        hook_policy("policy.hooks.before_model"),
        HookMutationRights::from_rights([HookMutationRight::StopRun]),
    );
    let registry = registry_with(ScriptedHookExecutor::once(
        "executor.before_model.stop",
        HookResponse::StopRun(StopReason {
            code: "stop".to_string(),
            redacted_summary: "stop before model".to_string(),
        }),
        1,
    ));
    let runtime = p0_runtime_with_package(
        p0_package_with_hook(&agent, hook),
        provider.clone(),
        FakeJournalStore::default(),
        InMemoryAgentEventBus::default(),
        registry,
    );

    let error = runtime
        .run_text(RunRequest::text(
            RunId::new("run.p0.hook.non-invoked-mutating"),
            agent.id().clone(),
            SourceRef::with_kind(SourceKind::Host, "source.p0.hook"),
            "draft",
        ))
        .expect_err("non-invoked mutating hook fails before provider");

    assert_eq!(error.kind(), agent_sdk_core::AgentErrorKind::InvalidPackage);
    assert!(provider.requests().is_empty());
}

#[test]
fn oversized_context_injection_fails_before_provider_call() {
    let agent = p0_agent();
    let provider = FakeProvider::with_responses(["should not be used"]);
    let journal = FakeJournalStore::default();
    let hook = HookSpec::blocking(
        "inject.before_context",
        HookPoint::BeforeContextAssembly,
        HookSource::InProcess,
        "executor.inject.before_context",
        hook_policy("policy.hooks.inject"),
        HookMutationRights::from_rights([HookMutationRight::InjectContext]),
    );
    let registry = registry_with(ScriptedHookExecutor::once(
        "executor.inject.before_context",
        HookResponse::InjectContext(vec![ContextInjectionRequest {
            redacted_summary: "x".repeat(3_000),
            policy_refs: vec![hook_policy("policy.hooks.inject.context")],
        }]),
        1,
    ));
    let runtime = p0_runtime_with_package(
        p0_package_with_hook(&agent, hook),
        provider.clone(),
        journal.clone(),
        InMemoryAgentEventBus::default(),
        registry,
    );

    let error = runtime
        .run_text(RunRequest::text(
            RunId::new("run.p0.hook.oversized-injection"),
            agent.id().clone(),
            SourceRef::with_kind(SourceKind::Host, "source.p0.hook"),
            "draft",
        ))
        .expect_err("oversized context injection fails closed");

    assert_eq!(error.kind(), agent_sdk_core::AgentErrorKind::PolicyDenial);
    assert!(provider.requests().is_empty());
    assert_eq!(
        rejected_hook_records(&journal, agent_sdk_core::HookResponseClass::InjectContext),
        1
    );
}

#[test]
fn oversized_retry_nudge_fails_after_first_provider_call_before_acceptance() {
    let agent = p0_agent();
    let provider = FakeProvider::with_responses(["draft", "should not retry"]);
    let journal = FakeJournalStore::default();
    let hook = before_complete_retry_hook();
    let registry = registry_with(ScriptedHookExecutor::once(
        "executor.before_complete.retry",
        HookResponse::RequestRetry(RetryRequest {
            redacted_summary: "x".repeat(3_000),
        }),
        1,
    ));
    let runtime = p0_runtime_with_package(
        p0_package_with_hook(&agent, hook),
        provider.clone(),
        journal.clone(),
        InMemoryAgentEventBus::default(),
        registry,
    );

    let error = runtime
        .run_text(RunRequest::text(
            RunId::new("run.p0.hook.oversized-retry"),
            agent.id().clone(),
            SourceRef::with_kind(SourceKind::Host, "source.p0.hook"),
            "draft",
        ))
        .expect_err("oversized retry nudge fails closed");

    assert_eq!(error.kind(), agent_sdk_core::AgentErrorKind::PolicyDenial);
    assert_eq!(provider.requests().len(), 1);
    assert_eq!(accepted_retry_hook_records(&journal), 0);
    assert_eq!(rejected_retry_hook_records(&journal), 1);
}

#[derive(Clone, Debug)]
struct CapturingProvider {
    responses: Arc<Mutex<Vec<String>>>,
    requests: Arc<Mutex<Vec<ProviderRequest>>>,
    projections: Arc<Mutex<Vec<ContextProjection>>>,
}

impl CapturingProvider {
    fn with_responses(responses: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut responses = responses
            .into_iter()
            .map(Into::into)
            .collect::<Vec<String>>();
        responses.reverse();
        Self {
            responses: Arc::new(Mutex::new(responses)),
            requests: Arc::new(Mutex::new(Vec::new())),
            projections: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn requests(&self) -> Vec<ProviderRequest> {
        self.requests
            .lock()
            .expect("capturing provider requests")
            .clone()
    }

    fn projections(&self) -> Vec<ContextProjection> {
        self.projections
            .lock()
            .expect("capturing provider projections")
            .clone()
    }
}

impl ProviderAdapter for CapturingProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::text_only("provider.fake")
    }

    fn project_request(
        &self,
        projection: &ContextProjection,
        policy: &ProviderProjectionPolicy,
    ) -> Result<ProviderRequest, AgentError> {
        self.projections
            .lock()
            .expect("capturing provider projections")
            .push(projection.clone());
        project_context_projection(projection, policy)
    }

    fn complete(&self, request: &ProviderRequest) -> Result<ProviderResponse, AgentError> {
        self.requests
            .lock()
            .expect("capturing provider requests")
            .push(request.clone());
        let output_text = self
            .responses
            .lock()
            .expect("capturing provider responses")
            .pop()
            .ok_or_else(|| AgentError::contract_violation("capturing provider exhausted"))?;
        Ok(ProviderResponse {
            schema_version: ProviderResponse::SCHEMA_VERSION,
            output_text,
            stop_reason: ProviderStopReason::EndTurn,
            tool_calls: Vec::new(),
            usage: Some(ProviderUsage::default()),
        })
    }
}

#[derive(Clone, Debug)]
struct ToolLoopProvider {
    responses: Arc<Mutex<Vec<ProviderResponse>>>,
    requests: Arc<Mutex<Vec<ProviderRequest>>>,
}

impl ToolLoopProvider {
    fn with_responses(responses: impl IntoIterator<Item = ProviderResponse>) -> Self {
        let mut responses = responses.into_iter().collect::<Vec<_>>();
        responses.reverse();
        Self {
            responses: Arc::new(Mutex::new(responses)),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn requests(&self) -> Vec<ProviderRequest> {
        self.requests
            .lock()
            .expect("tool loop provider requests")
            .clone()
    }
}

impl ProviderAdapter for ToolLoopProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::text_only("provider.fake")
    }

    fn complete(&self, request: &ProviderRequest) -> Result<ProviderResponse, AgentError> {
        self.requests
            .lock()
            .expect("tool loop provider requests")
            .push(request.clone());
        self.responses
            .lock()
            .expect("tool loop provider responses")
            .pop()
            .ok_or_else(|| AgentError::contract_violation("tool loop provider exhausted"))
    }
}

fn p0_runtime_with_package<P>(
    package: RuntimePackage,
    provider: P,
    journal: FakeJournalStore,
    event_bus: InMemoryAgentEventBus,
    hook_registry: InMemoryHookExecutorRegistry,
) -> AgentRuntime
where
    P: ProviderAdapter + 'static,
{
    AgentRuntime::builder()
        .default_package(package)
        .provider("provider.fake", provider)
        .expect("provider route registers")
        .journal(journal)
        .event_bus(event_bus)
        .content(FakeContentResolver::default())
        .policy(AllowP0Policy)
        .hook_executor_registry(hook_registry)
        .build()
        .expect("runtime builds")
}

fn p0_package_with_hook(agent: &Agent, hook: HookSpec) -> RuntimePackage {
    RuntimePackage::builder(RuntimePackageId::new("package.p0.contract"))
        .agent(agent.snapshot())
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake.p0"))
        .policy(PolicyRef::with_kind(
            PolicyKind::RuntimePackage,
            "policy.p0.package",
        ))
        .hook(hook)
        .build()
        .expect("hook package builds")
}

fn before_complete_retry_hook() -> HookSpec {
    HookSpec::blocking(
        "retry.before_complete",
        HookPoint::BeforeRunComplete,
        HookSource::InProcess,
        "executor.before_complete.retry",
        hook_policy("policy.hooks.retry"),
        HookMutationRights::from_rights([HookMutationRight::RequestRetry]),
    )
}

fn hook_policy(id: &str) -> PolicyRef {
    agent_sdk_core::hook_policy_ref(id)
}

fn registry_with(executor: ScriptedHookExecutor) -> InMemoryHookExecutorRegistry {
    let registry = InMemoryHookExecutorRegistry::default();
    registry
        .register(executor)
        .expect("hook executor registers");
    registry
}

fn registry_with_all(
    executors: impl IntoIterator<Item = ScriptedHookExecutor>,
) -> InMemoryHookExecutorRegistry {
    let registry = InMemoryHookExecutorRegistry::default();
    for executor in executors {
        registry
            .register(executor)
            .expect("hook executor registers");
    }
    registry
}

fn accepted_retry_hook_records(journal: &FakeJournalStore) -> usize {
    hook_response_decision_records(
        journal,
        agent_sdk_core::HookResponseDecision::AcceptedJournaledBeforeApply,
        agent_sdk_core::HookResponseClass::RequestRetry,
    )
}

fn rejected_retry_hook_records(journal: &FakeJournalStore) -> usize {
    rejected_hook_records(journal, agent_sdk_core::HookResponseClass::RequestRetry)
}

fn rejected_hook_records(
    journal: &FakeJournalStore,
    response_class: agent_sdk_core::HookResponseClass,
) -> usize {
    hook_response_decision_records(
        journal,
        agent_sdk_core::HookResponseDecision::RejectedPolicy,
        response_class,
    )
}

fn hook_response_decision_records(
    journal: &FakeJournalStore,
    decision: agent_sdk_core::HookResponseDecision,
    response_class: agent_sdk_core::HookResponseClass,
) -> usize {
    journal
        .records()
        .into_iter()
        .filter(|record| match &record.payload {
            agent_sdk_core::JournalRecordPayload::Hook(agent_sdk_core::HookRecord {
                payload:
                    agent_sdk_core::HookRecordPayload::ResponseDecision {
                        decision: record_decision,
                        response_class: record_response_class,
                        ..
                    },
                ..
            }) => record_decision == &decision && record_response_class == &response_class,
            _ => false,
        })
        .count()
}

fn p0_runtime(
    agent: &Agent,
    provider: FakeProvider,
    journal: FakeJournalStore,
    event_bus: InMemoryAgentEventBus,
) -> AgentRuntime {
    AgentRuntime::builder()
        .default_package(p0_package(agent))
        .provider("provider.fake", provider)
        .expect("provider route registers")
        .journal(journal)
        .event_bus(event_bus)
        .content(FakeContentResolver::default())
        .policy(AllowP0Policy)
        .build()
        .expect("runtime builds")
}

fn p0_agent() -> Agent {
    Agent::builder()
        .id(AgentId::new("agent.p0.contract"))
        .name("p0 contract")
        .build()
        .expect("agent builds")
}

fn p0_package(agent: &Agent) -> RuntimePackage {
    RuntimePackage::builder(RuntimePackageId::new("package.p0.contract"))
        .agent(agent.snapshot())
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake.p0"))
        .policy(PolicyRef::with_kind(
            PolicyKind::RuntimePackage,
            "policy.p0.package",
        ))
        .build()
        .expect("package builds")
}

fn p0_package_with_tool(agent: &Agent) -> RuntimePackage {
    RuntimePackage::builder(RuntimePackageId::new("package.p0.contract"))
        .agent(agent.snapshot())
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake.p0"))
        .policy(PolicyRef::with_kind(
            PolicyKind::RuntimePackage,
            "policy.p0.package",
        ))
        .capability(CapabilitySpec::fake_tool(
            CapabilityId::new("cap.tool.read"),
            "workspace_read",
            PackageSidecarRef::new("sidecar.schema.read", "json_schema", "v1"),
            ExecutorRef::new("executor.workspace_read.v1"),
            PolicyRef::with_kind(PolicyKind::Approval, "policy.approval.read"),
            SourceRef::with_kind(SourceKind::Sdk, "source.sdk.toolpack"),
        ))
        .build()
        .expect("tool package builds")
}

fn p0_read_tool_route() -> ToolRoute {
    ToolRoute {
        capability_id: CapabilityId::new("cap.tool.read"),
        canonical_tool_name: CanonicalToolName::new("workspace_read"),
        namespace: CapabilityNamespace::new("tool.workspace_read"),
        description: None,
        source: SourceRef::with_kind(SourceKind::Sdk, "source.sdk.toolpack"),
        destination: DestinationRef::with_kind(DestinationKind::Tool, "destination.tool.read"),
        executor_ref: Some(ExecutorRef::new("executor.workspace_read.v1")),
        policy_refs: vec![PolicyRef::with_kind(
            PolicyKind::Approval,
            "policy.approval.read",
        )],
        requires_approval: false,
        sidecar_refs: vec![PackageSidecarRef::new(
            "sidecar.schema.read",
            "json_schema",
            "v1",
        )],
        effect_class: EffectClass::Read,
        risk_class: RiskClass::Low,
        privacy: PrivacyClass::ContentRefsOnly,
        retention: RetentionClass::RunScoped,
    }
}

fn event_summary(frames: &[agent_sdk_core::EventFrame]) -> Value {
    json!({
        "schema_version": 1,
        "events": frames.iter().map(|frame| {
            json!({
                "event_seq": frame.event.envelope.event_seq,
                "event_family": format!("{:?}", frame.event.envelope.event_family),
                "event_kind": format!("{:?}", frame.event.envelope.event_kind),
                "delivery_semantics": format!("{:?}", frame.event.envelope.delivery_semantics),
                "journal_cursor": frame.event.envelope.journal_cursor.as_ref().map(|cursor| cursor.as_str().to_string()),
            })
        }).collect::<Vec<_>>()
    })
}

fn journal_summary(records: &[agent_sdk_core::JournalRecord]) -> Value {
    json!({
        "schema_version": 1,
        "records": records.iter().map(|record| {
            json!({
                "journal_seq": record.journal_seq,
                "record_kind": format!("{:?}", record.record_kind),
                "payload_type": payload_type(&record.payload),
                "delivery_semantics": record.delivery_semantics,
            })
        }).collect::<Vec<_>>()
    })
}

fn payload_type(payload: &agent_sdk_core::JournalRecordPayload) -> &'static str {
    match payload {
        agent_sdk_core::JournalRecordPayload::RunLifecycle(_) => "run_lifecycle",
        agent_sdk_core::JournalRecordPayload::TurnLifecycle(_) => "turn_lifecycle",
        agent_sdk_core::JournalRecordPayload::ContextProjection(_) => "context_projection",
        agent_sdk_core::JournalRecordPayload::ModelAttempt(_) => "model_attempt",
        agent_sdk_core::JournalRecordPayload::Message(_) => "message",
        agent_sdk_core::JournalRecordPayload::StructuredOutput(_) => "structured_output",
        agent_sdk_core::JournalRecordPayload::Approval(_) => "approval",
        agent_sdk_core::JournalRecordPayload::Tool(_) => "tool",
        agent_sdk_core::JournalRecordPayload::OutputDelivery(_) => "output_delivery",
        agent_sdk_core::JournalRecordPayload::Hook(_) => "hook",
        agent_sdk_core::JournalRecordPayload::StreamRule(_) => "stream_rule",
        agent_sdk_core::JournalRecordPayload::RealtimeSession(_) => "realtime_session",
        agent_sdk_core::JournalRecordPayload::Isolation(_) => "isolation",
        agent_sdk_core::JournalRecordPayload::ChildLifecycle(_) => "child_lifecycle",
        agent_sdk_core::JournalRecordPayload::AgentPool(_) => "agent_pool",
        agent_sdk_core::JournalRecordPayload::RunMessage(_) => "run_message",
        agent_sdk_core::JournalRecordPayload::Wake(_) => "wake",
        agent_sdk_core::JournalRecordPayload::Subagent(_) => "subagent",
        agent_sdk_core::JournalRecordPayload::ExtensionAction(_) => "extension_action",
        agent_sdk_core::JournalRecordPayload::EffectIntent(_) => "effect_intent",
        agent_sdk_core::JournalRecordPayload::EffectResult(_) => "effect_result",
        agent_sdk_core::JournalRecordPayload::Checkpoint(_) => "checkpoint",
        agent_sdk_core::JournalRecordPayload::Recovery(_) => "recovery",
        agent_sdk_core::JournalRecordPayload::TerminalResult(_) => "terminal_result",
    }
}

fn fixture(path: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture readable"))
        .expect("fixture JSON")
}
