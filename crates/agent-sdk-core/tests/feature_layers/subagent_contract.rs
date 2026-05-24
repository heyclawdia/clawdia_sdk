use serde_json::{Value, json};

use agent_sdk_core::{
    Agent, AgentError, AgentId, AgentPool, AgentPoolId, CapabilityKind, CapabilitySpec,
    ContextHandoffPolicy, DepthBudget, DestinationKind, DestinationRef, EventKind, ExecutorRef,
    IdempotencyKey, InMemoryAgentEventBus, MessageId, MessageResponseContract, MessageStatus,
    PackageSidecarRef, PolicyDecision, PolicyKind, PolicyOutcome, PolicyRef, PolicyStage,
    PrivacyClass, ProviderRouteSnapshot, RetentionClass, RunAddress, RunId, RunMessage,
    RuntimePackage, RuntimePackageId, RuntimePolicyPort, SourceKind, SourceRef, SubagentRecord,
    SubagentRequest, SubagentRequestId, SubagentRoutePolicy, SubagentSupervisor,
    SubagentTerminalStatus, SubagentToolPolicy, WakeCondition, WakeConditionId,
    WakeRegistrationStatus,
    domain::ContentRef as ContentRefId,
    event::{EventFamily, EventFilter, EventFilterSet},
    testing::{
        FakeContentResolver, FakeJournalStore, FakeProvider, normalize_json_value, read_fixture,
    },
};

#[derive(Clone, Debug)]
struct AllowPolicy;

impl RuntimePolicyPort for AllowPolicy {
    fn evaluate_run_start(
        &self,
        request: &agent_sdk_core::RunRequest,
        _package: &RuntimePackage,
    ) -> Result<PolicyOutcome, AgentError> {
        Ok(PolicyOutcome {
            stage: PolicyStage::Input,
            decision: PolicyDecision::allow("policy.subagent.runtime_allow"),
            subject: None,
            source: Some(request.source.clone()),
            destination: None,
            policy_refs: vec![PolicyRef::with_kind(
                PolicyKind::RuntimePackage,
                "policy.subagent.runtime",
            )],
            privacy: PrivacyClass::Internal,
            retention: RetentionClass::RunScoped,
        })
    }
}

#[test]
fn default_context_handoff_is_none_and_passes_no_parent_context() {
    let handoff = ContextHandoffPolicy::default();
    handoff.validate().expect("none handoff validates");

    let summary = normalize_json_value(json!({
        "schema_version": 1,
        "policy": handoff,
        "selected_content_refs": handoff.selected_content_refs(),
        "policy_refs": handoff.policy_refs(),
    }));

    assert_eq!(
        summary,
        read_fixture("tests/fixtures/subagents/context-handoff-none.json")
            .expect("none handoff fixture")
    );
}

#[test]
fn child_package_strips_subagent_tools_and_records_manifest() {
    let agent = test_agent();
    let parent = parent_package(&agent);
    let parent_fingerprint = parent.fingerprint().expect("parent fingerprint");
    let child_policy =
        agent_sdk_core::ChildRuntimePackagePolicy::strip_recursive_defaults(parent_fingerprint);

    let child_package = agent_sdk_core::build_child_runtime_package(
        &parent,
        agent.id().clone(),
        &SubagentRoutePolicy::InheritParent,
        &ContextHandoffPolicy::None,
        &child_policy,
        &SubagentToolPolicy::ReadOnly,
    )
    .expect("child package builds");

    assert!(
        child_package
            .package
            .capabilities
            .iter()
            .all(|capability| capability.kind != CapabilityKind::AgentAsTool)
    );

    let summary = child_package_summary(&child_package.strip_manifest);
    assert_eq!(
        summary,
        read_fixture("tests/fixtures/subagents/child-package-strip.json")
            .expect("child package strip fixture")
    );
}

#[test]
fn subagent_request_lowers_to_child_run_agent_pool_message_and_wake() {
    let Harness {
        supervisor,
        runtime,
        journal,
        event_bus: _event_bus,
        request,
    } = harness("one");

    let handle = supervisor
        .start_child(request.clone())
        .expect("child starts");
    assert_eq!(handle.child_run_id, request.child_run_id);
    assert_eq!(handle.run_handle.run_id(), &request.child_run_id);
    assert_eq!(
        runtime
            .run_snapshot(&request.child_run_id)
            .expect("child registered")
            .agent_id,
        request.child_agent_id
    );
    assert!(
        handle
            .wrapped_event_filter
            .indexed_fields
            .contains(&agent_sdk_core::event::EventIndexField::RunId)
    );

    let condition = wake_condition(
        "wake.subagent.child.done",
        request.parent_run_id.as_str(),
        EventFilter {
            families: EventFilterSet::Include(vec![EventFamily::AgentPool]),
            kinds: EventFilterSet::Include(vec![EventKind::RunMessageDelivered]),
            ..EventFilter::default()
        },
    )
    .policy_ref(request.wake_policy_ref.clone());
    let wake = supervisor
        .suspend_until(request.parent_run_id.clone(), condition)
        .expect("wake registered or triggered");
    assert_eq!(wake.status, WakeRegistrationStatus::Triggered);

    let journal_records = journal.records();
    let subagent_started_position = journal_records
        .iter()
        .position(|record| {
            matches!(
                record.payload,
                agent_sdk_core::JournalRecordPayload::Subagent(SubagentRecord::Started(_))
            )
        })
        .expect("subagent started record is durable");
    let first_run_message_position = journal_records
        .iter()
        .position(|record| {
            matches!(
                record.payload,
                agent_sdk_core::JournalRecordPayload::RunMessage(_)
            )
        })
        .expect("initial run message is durable");
    assert!(
        subagent_started_position < first_run_message_position,
        "subagent start/handoff records must be journaled before child message effects"
    );

    let summary = normalize_json_value(json!({
        "schema_version": 1,
        "subagent_records": supervisor.records().expect("records").iter().map(SubagentRecord::kind).collect::<Vec<_>>(),
        "journal": journal_summary(&journal.records()),
    }));
    assert_eq!(
        summary,
        read_fixture("tests/fixtures/subagents/supervision-records.json")
            .expect("supervision records fixture")
    );
}

#[test]
fn clarification_round_trip_uses_run_message_and_wake_without_user_chat_promotion() {
    let Harness {
        supervisor,
        request,
        ..
    } = harness("clarify");
    supervisor
        .start_child(request.clone())
        .expect("child starts");

    let question = RunMessage::new(
        MessageId::new("message.subagent.question"),
        request.child_run_id.clone(),
        RunAddress::run(request.parent_run_id.clone()),
        ContentRefId::new("content.subagent.question"),
        IdempotencyKey::new("idem.subagent.question"),
    )
    .policy_ref(request.message_policy_ref.clone());
    let mut question = question;
    question.response_contract = Some(MessageResponseContract::one_response(300_000));
    let question_receipt = supervisor
        .send_message(question.clone())
        .expect("question sent");
    assert_eq!(question_receipt.status, MessageStatus::Delivered);

    let mut reply = RunMessage::new(
        MessageId::new("message.subagent.reply"),
        request.parent_run_id.clone(),
        RunAddress::run(request.child_run_id.clone()),
        ContentRefId::new("content.subagent.reply"),
        IdempotencyKey::new("idem.subagent.reply"),
    )
    .policy_ref(request.message_policy_ref.clone());
    reply.reply_to = Some(question.message_id.clone());
    let reply_receipt = supervisor.send_message(reply).expect("reply sent");
    assert_eq!(reply_receipt.status, MessageStatus::Delivered);

    let wake = supervisor
        .suspend_until(
            request.child_run_id.clone(),
            wake_condition(
                "wake.subagent.reply",
                request.child_run_id.as_str(),
                EventFilter {
                    run_ids: EventFilterSet::Include(vec![request.parent_run_id.clone()]),
                    families: EventFilterSet::Include(vec![EventFamily::AgentPool]),
                    kinds: EventFilterSet::Include(vec![EventKind::RunMessageDelivered]),
                    ..EventFilter::default()
                },
            )
            .policy_ref(request.wake_policy_ref.clone()),
        )
        .expect("reply wake");
    assert_eq!(wake.status, WakeRegistrationStatus::Triggered);
    assert!(
        !supervisor
            .child_can_be_addressed_as_user_chat(&request.child_run_id)
            .expect("child known"),
        "subagent messages do not promote the child into a user chat"
    );
}

#[test]
fn wrapped_events_child_journal_refs_and_usage_rollup_are_idempotent() {
    let Harness {
        supervisor,
        request,
        ..
    } = harness("wrap");
    supervisor
        .start_child(request.clone())
        .expect("child starts");

    let frame = agent_sdk_core::subagent_runtime_event_frame(
        request.parent_run_id.clone(),
        request.child_run_id.clone(),
        request.child_agent_id.clone(),
        9,
        EventKind::RunCompleted,
        Some(agent_sdk_core::JournalCursor::new("journal.child.9")),
    );
    let wrapped = supervisor
        .wrap_child_event(frame.event)
        .expect("child event wraps");
    assert_eq!(
        wrapped.child_journal_ref.run_id,
        RunId::new("run.subagent.wrap.child")
    );

    let first = supervisor
        .rollup_usage(
            request.child_run_id.clone(),
            "usage.child.1",
            3,
            5,
            Some(12),
            Some("USD".to_string()),
            SubagentTerminalStatus::Completed,
        )
        .expect("usage rolled up");
    let second = supervisor
        .rollup_usage(
            request.child_run_id.clone(),
            "usage.child.1",
            3,
            5,
            Some(12),
            Some("USD".to_string()),
            SubagentTerminalStatus::Completed,
        )
        .expect("usage deduped");
    assert_eq!(first, second);

    let summary = normalize_json_value(json!({
        "schema_version": 1,
        "wrapped": {
            "child_run_id": wrapped.child_run_id,
            "original_kind": wrapped.original_child_event_kind,
            "journal_ref": wrapped.child_journal_ref,
            "cursor": wrapped.child_journal_cursor,
            "privacy": wrapped.privacy,
        },
        "usage_records": supervisor.records().expect("records").iter().filter_map(|record| match record {
            SubagentRecord::UsageRolledUp(record) => Some(json!({
                "child_run_id": record.child_run_id,
                "parent_run_id": record.parent_run_id,
                "child_usage_ref": record.child_usage_ref,
                "total_tokens": record.total_tokens,
                "cost_micros": record.cost_micros,
                "terminal_status": record.terminal_status,
            })),
            _ => None,
        }).collect::<Vec<_>>(),
    }));
    assert_eq!(
        summary,
        read_fixture("tests/fixtures/subagents/wrapped-event-usage.json")
            .expect("wrapped event usage fixture")
    );
}

#[test]
fn parent_cancel_and_detach_are_parent_owned_lifecycle_records() {
    let Harness {
        supervisor,
        runtime,
        journal,
        request,
        ..
    } = harness("life");
    supervisor
        .start_child(request.clone())
        .expect("child starts");
    let cancel_records = supervisor
        .cancel_child(request.child_run_id.clone())
        .expect("child cancelled");
    assert_eq!(cancel_records.len(), 2);
    assert!(
        runtime
            .run_snapshot(&request.child_run_id)
            .expect("child snapshot")
            .cancellation_requested
    );
    supervisor
        .complete_child(
            request.child_run_id.clone(),
            SubagentTerminalStatus::Cancelled,
            None,
            None,
        )
        .expect("child terminal");
    assert!(
        !supervisor
            .child_requires_terminal_rollup_or_detach(&request.child_run_id)
            .expect("child known")
    );

    let detach_request = request_with_suffix("life_detach", &parent_package(&test_agent()));
    supervisor
        .start_child(detach_request.clone())
        .expect("detached child starts");
    let detach_records = supervisor
        .detach_child(
            detach_request.child_run_id.clone(),
            "host_ack.subagent.detach.1",
            PolicyRef::with_kind(PolicyKind::RuntimePackage, "policy.reclaim.host"),
        )
        .expect("child detached");
    assert_eq!(detach_records.len(), 2);
    assert!(
        supervisor
            .cancel_child(detach_request.child_run_id.clone())
            .is_err(),
        "detached child supervision transfers to host-owned lifecycle"
    );
    let child_lifecycle_records = journal
        .records()
        .into_iter()
        .filter(|record| {
            matches!(
                record.payload,
                agent_sdk_core::JournalRecordPayload::ChildLifecycle(_)
            )
        })
        .count();
    assert_eq!(
        child_lifecycle_records, 4,
        "cancel and detach lifecycle facts must be durable on the parent journal"
    );

    let summary = normalize_json_value(json!({
        "schema_version": 1,
        "lifecycle": supervisor.records().expect("records").iter().filter_map(|record| match record {
            SubagentRecord::ChildLifecycle(record) => Some(json!({
                "child_run_id": record.child_run_id,
                "action": record.action,
                "status": record.status,
                "effect_intent_kind": record.effect_intent.as_ref().map(|intent| &intent.kind),
                "effect_result_status": record.effect_result.as_ref().map(|result| &result.terminal_status),
                "host_ack_ref": record.host_ack_ref,
            })),
            _ => None,
        }).collect::<Vec<_>>(),
    }));
    assert_eq!(
        summary,
        read_fixture("tests/fixtures/subagents/lifecycle-records.json").expect("lifecycle fixture")
    );
}

#[test]
fn depth_recursion_and_unknown_routes_fail_closed_before_child_start() {
    let Harness {
        supervisor,
        journal,
        request,
        ..
    } = harness("deny");

    let mut exhausted = request.clone();
    exhausted.depth_budget = DepthBudget {
        current_depth: 1,
        max_depth: 1,
        max_children: 1,
    };
    assert!(supervisor.start_child(exhausted).is_err());

    let mut recursive = request.clone();
    recursive.child_package_policy.strip_recursive_subagents = false;
    assert!(supervisor.start_child(recursive).is_err());

    let mut unknown_route = request;
    unknown_route.route_policy = SubagentRoutePolicy::UseAllowedOverride {
        route_id: "provider.missing".to_string(),
        model_id: "model.missing".to_string(),
    };
    unknown_route
        .child_package_policy
        .allowed_route_overrides
        .push("provider.missing".to_string());
    assert!(supervisor.start_child(unknown_route).is_err());
    assert!(
        journal.records().is_empty(),
        "fail-closed validation happens before child-start journal intent"
    );
}

struct Harness {
    supervisor: SubagentSupervisor,
    runtime: agent_sdk_core::AgentRuntime,
    journal: FakeJournalStore,
    event_bus: InMemoryAgentEventBus,
    request: SubagentRequest,
}

fn harness(suffix: &str) -> Harness {
    let agent = test_agent();
    let package = parent_package(&agent);
    let journal = FakeJournalStore::default();
    let event_bus = InMemoryAgentEventBus::default();
    let runtime = agent_sdk_core::AgentRuntime::builder()
        .journal(journal.clone())
        .event_bus(event_bus.clone())
        .default_package(package.clone())
        .provider("provider.fake", FakeProvider::default())
        .expect("provider")
        .content(FakeContentResolver::default())
        .policy(AllowPolicy)
        .build()
        .expect("runtime");
    let pool = AgentPool::builder(AgentPoolId::new(format!("pool.subagent.{suffix}")))
        .runtime(runtime.clone())
        .build()
        .expect("pool");
    let supervisor = SubagentSupervisor::new(runtime.clone(), pool, package.clone());
    Harness {
        supervisor,
        runtime,
        journal,
        event_bus,
        request: request_with_suffix(suffix, &package),
    }
}

fn test_agent() -> Agent {
    Agent::builder()
        .id(AgentId::new("agent.subagent.worker"))
        .name("subagent-worker")
        .build()
        .expect("agent")
}

fn parent_package(agent: &Agent) -> RuntimePackage {
    RuntimePackage::builder(RuntimePackageId::new("package.subagent.parent"))
        .agent(agent.snapshot())
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake"))
        .capability(fake_tool("tool.workspace_read", "workspace_read"))
        .capability(fake_tool(
            "tool.subagent_send_message",
            "subagent_send_message",
        ))
        .capability(CapabilitySpec::reserved_inactive(
            "agent_as_tool.reviewer",
            CapabilityKind::AgentAsTool,
            PolicyRef::with_kind(PolicyKind::RuntimePackage, "policy.subagent.agent_as_tool"),
            SourceRef::with_kind(SourceKind::Sdk, "source.test.subagent"),
        ))
        .policy(PolicyRef::with_kind(
            PolicyKind::RuntimePackage,
            "policy.subagent.package",
        ))
        .build()
        .expect("package")
}

fn fake_tool(capability_id: &str, name: &str) -> CapabilitySpec {
    CapabilitySpec::fake_tool(
        capability_id,
        name,
        PackageSidecarRef::new(format!("schema.{capability_id}"), "schema", "v1"),
        ExecutorRef::new(format!("executor.{capability_id}")),
        PolicyRef::with_kind(
            PolicyKind::RuntimePackage,
            format!("policy.{capability_id}"),
        ),
        SourceRef::with_kind(SourceKind::Sdk, "source.test.tool"),
    )
}

fn request_with_suffix(suffix: &str, package: &RuntimePackage) -> SubagentRequest {
    let parent_fingerprint = package.fingerprint().expect("parent fingerprint");
    SubagentRequest {
        request_id: SubagentRequestId::new(format!("subagent.request.{suffix}")),
        parent_run_id: RunId::new(format!("run.subagent.{suffix}.parent")),
        parent_agent_id: package.agent.agent_id.clone(),
        parent_tool_call_id: agent_sdk_core::ToolCallId::new(format!("toolcall.subagent.{suffix}")),
        child_run_id: RunId::new(format!("run.subagent.{suffix}.child")),
        child_agent_id: package.agent.agent_id.clone(),
        child_source: SourceRef::with_kind(
            SourceKind::Subagent,
            format!("source.subagent.{suffix}"),
        ),
        child_destination: DestinationRef::with_kind(
            DestinationKind::Subagent,
            format!("destination.subagent.{suffix}"),
        ),
        route_policy: SubagentRoutePolicy::InheritParent,
        context_handoff: ContextHandoffPolicy::None,
        child_package_policy: agent_sdk_core::ChildRuntimePackagePolicy::strip_recursive_defaults(
            parent_fingerprint,
        ),
        child_tool_policy: SubagentToolPolicy::ReadOnly,
        message_policy_ref: PolicyRef::with_kind(PolicyKind::Permission, "policy.subagent.message"),
        wake_policy_ref: PolicyRef::with_kind(PolicyKind::Permission, "policy.subagent.wake"),
        lifecycle_policy_ref: Some(PolicyRef::with_kind(
            PolicyKind::RuntimePackage,
            "policy.subagent.lifecycle",
        )),
        depth_budget: DepthBudget::max_depth(1),
        idempotency_key: IdempotencyKey::new(format!("idem.subagent.start.{suffix}")),
        initial_message_ref: Some(ContentRefId::new(format!(
            "content.subagent.initial.{suffix}"
        ))),
    }
}

fn wake_condition(condition_id: &str, run_id: &str, filter: EventFilter) -> WakeCondition {
    WakeCondition::new(
        WakeConditionId::new(condition_id),
        RunId::new(run_id),
        filter,
        IdempotencyKey::new(format!("idem.{condition_id}")),
    )
}

fn child_package_summary(manifest: &agent_sdk_core::ChildPackageStripManifest) -> Value {
    normalize_json_value(json!({
        "schema_version": 1,
        "child_agent_id": manifest.child_agent_id,
        "selected_provider_route_id": manifest.selected_provider_route_id,
        "handoff_policy_variant": manifest.handoff_policy_variant,
        "recursive_subagent_strip": manifest.recursive_subagent_strip,
        "stripped_capability_ids": manifest.stripped_capability_ids,
        "retained_capability_ids": manifest.retained_capability_ids,
        "lifecycle_policy_ref": manifest.lifecycle_policy_ref,
        "redaction_policy_ref": manifest.redaction_policy_ref,
    }))
}

fn journal_summary(records: &[agent_sdk_core::JournalRecord]) -> Value {
    normalize_json_value(json!({
        "effect_intents": records.iter().filter_map(|record| match &record.payload {
            agent_sdk_core::JournalRecordPayload::EffectIntent(intent) => Some(json!({
                "record_kind": record.record_kind,
                "effect_kind": intent.kind,
                "subject_kind": intent.subject_ref.kind,
                "destination_kind": intent.destination.as_ref().map(|destination| &destination.kind),
                "content_refs": intent.content_refs,
            })),
            _ => None,
        }).collect::<Vec<_>>(),
        "run_messages": records.iter().filter_map(|record| match &record.payload {
            agent_sdk_core::JournalRecordPayload::RunMessage(message) => Some(json!({
                "event_kind": record.event_index.event_kind,
                "message_id": message.message_id,
                "delivery_status": message.delivery_status,
                "delivered_to": message.delivered_to,
                "content_ref": message.content_ref,
                "effect_intent_kind": message.effect_intent.as_ref().map(|intent| &intent.kind),
                "effect_result_status": message.effect_result.as_ref().map(|result| &result.terminal_status),
            })),
            _ => None,
        }).collect::<Vec<_>>(),
        "wake_records": records.iter().filter_map(|record| match &record.payload {
            agent_sdk_core::JournalRecordPayload::Wake(wake) => Some(json!({
                "event_kind": record.event_index.event_kind,
                "condition_id": wake.condition_id,
                "trigger_status": wake.trigger_status,
            })),
            _ => None,
        }).collect::<Vec<_>>(),
    }))
}
