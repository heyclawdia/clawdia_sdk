use std::sync::{Arc, Mutex};

use agent_sdk_core::{
    AgentError, AgentErrorKind, AgentId, AllowToolPolicy, CapabilityId, CapabilitySpec,
    DestinationKind, DestinationRef, ExecutorRef, IdempotencyKey, JournalRecord,
    JournalRecordPayload, PolicyDecision, PolicyKind, PolicyOutcome, PolicyRef, PolicyStage,
    PrivacyClass, ProviderRouteSnapshot, RetentionClass, RetryClassification, RunId,
    RuntimePackage, RuntimePackageId, SourceKind, SourceRef, ToolCallId, ToolExecutionContext,
    ToolExecutionCoordinator,
    domain::ContentRef as ContentRefId,
    hook_ports::InMemoryHookExecutorRegistry,
    package_hooks::{
        DenyReason, HookMutationRight, HookMutationRights, HookPoint, HookResponse,
        HookResponseClass, HookSource, HookSpec, ToolRequestPatch, ToolResultPatch,
    },
    testing::{ScriptedHookExecutor, ScriptedToolExecutor, read_fixture},
    tool_ports::{
        ToolCallRequest, ToolExecutionOutput, ToolExecutionStrategy, ToolExecutorRegistry,
        ToolPolicyPort, ToolRegistrySnapshot, ToolRoute, ToolRouter,
    },
    tool_records::{CanonicalToolName, ToolCallRecordStatus, ToolResultRef},
};
use serde_json::{Value, json};

#[test]
fn tool_registry_snapshot_lowers_runtime_package_routes_to_fixture() {
    let package = package_with_tools();
    let snapshot = registry_snapshot(&package);

    assert_eq!(snapshot.routes.len(), 2);
    assert_eq!(
        registry_summary(&snapshot),
        read_fixture("tests/fixtures/tools/tool-registry-snapshot.json").expect("registry fixture")
    );
}

#[test]
fn read_tool_records_intent_and_result_before_content_refs_are_exposed() {
    let package = package_with_tools();
    let snapshot = registry_snapshot(&package);
    let executor = Arc::new(ScriptedToolExecutor::new(
        ExecutorRef::new("executor.workspace_read.v1"),
        {
            let mut output = ToolExecutionOutput::completed("workspace read returned content refs");
            output
                .content_refs
                .push(ContentRefId::new("content.result.read.1"));
            output
        },
    ));
    let mut executors = ToolExecutorRegistry::new();
    executors
        .register(executor.clone())
        .expect("executor registers");
    let journal = agent_sdk_core::testing::FakeJournalStore::default();
    let coordinator = ToolExecutionCoordinator::new(ToolRouter::new(snapshot), executors)
        .with_policy(Arc::new(AllowToolPolicy));

    let outcome = coordinator
        .execute(
            &journal,
            ToolCallRequest {
                tool_call_id: ToolCallId::new("tool.call.read.1"),
                canonical_tool_name: CanonicalToolName::new("workspace_read"),
                source: source("source.model.tool_call"),
                requested_args_refs: vec![ContentRefId::new("content.args.read.1")],
                redacted_args_summary: "read docs/start-here.md".to_string(),
                idempotency_key: Some(IdempotencyKey::new("idem.read.1")),
                dedupe_key: None,
            },
            execution_context(&package),
        )
        .expect("tool executes");

    assert_eq!(executor.call_count(), 1);
    assert_eq!(
        executor.calls()[0].effect_intent.redacted_summary,
        "execute tool workspace_read with redacted arguments"
    );
    assert_eq!(outcome.record.status, ToolCallRecordStatus::Completed);
    assert_eq!(
        outcome.intent_cursor.as_ref().unwrap().as_str(),
        "journal.1"
    );
    assert_eq!(
        outcome.terminal_cursor.as_ref().unwrap().as_str(),
        "journal.2"
    );
    assert!(outcome.post_tool_policy.as_ref().unwrap().is_allowed());
    assert_eq!(
        ToolResultRef::from_record(&outcome.record)
            .expect("tool result ref")
            .content_refs,
        vec![ContentRefId::new("content.result.read.1")]
    );
    assert_eq!(
        journal_summary(&journal.records()),
        read_fixture("tests/fixtures/tools/read-tool-intent-result.json")
            .expect("read journal fixture")
    );
}

#[test]
fn missing_policy_denies_before_executor_and_journal_intent() {
    let package = package_with_tools();
    let snapshot = registry_snapshot(&package);
    let executor = Arc::new(ScriptedToolExecutor::new(
        ExecutorRef::new("executor.workspace_read.v1"),
        ToolExecutionOutput::completed("must not execute"),
    ));
    let mut executors = ToolExecutorRegistry::new();
    executors
        .register(executor.clone())
        .expect("executor registers");
    let journal = agent_sdk_core::testing::FakeJournalStore::default();
    let coordinator = ToolExecutionCoordinator::new(ToolRouter::new(snapshot), executors);

    let outcome = coordinator
        .execute(
            &journal,
            read_request("tool.call.missing_policy"),
            execution_context(&package),
        )
        .expect("missing policy returns denied outcome");

    assert_eq!(
        outcome.record.status,
        ToolCallRecordStatus::DeniedBeforeExecution
    );
    assert_eq!(
        outcome.record.policy_outcome.as_ref().unwrap().policy_refs,
        Vec::<PolicyRef>::new()
    );
    assert_eq!(executor.call_count(), 0);
    assert!(journal.records().is_empty());
}

#[test]
fn missing_executor_denies_before_journal_intent() {
    let package = package_with_tools();
    let snapshot = registry_snapshot(&package);
    let executors = ToolExecutorRegistry::new();
    let journal = agent_sdk_core::testing::FakeJournalStore::default();
    let coordinator = ToolExecutionCoordinator::new(ToolRouter::new(snapshot), executors)
        .with_policy(Arc::new(AllowToolPolicy));

    let outcome = coordinator
        .execute(
            &journal,
            read_request("tool.call.missing_executor"),
            execution_context(&package),
        )
        .expect("missing executor returns denied outcome");

    assert_eq!(
        outcome.record.status,
        ToolCallRecordStatus::DeniedBeforeExecution
    );
    assert!(journal.records().is_empty());
}

#[test]
fn tool_intent_append_failure_prevents_executor_start() {
    let package = package_with_tools();
    let snapshot = registry_snapshot(&package);
    let executor = Arc::new(ScriptedToolExecutor::new(
        ExecutorRef::new("executor.workspace_read.v1"),
        ToolExecutionOutput::completed("must not execute"),
    ));
    let mut executors = ToolExecutorRegistry::new();
    executors
        .register(executor.clone())
        .expect("executor registers");
    let journal = agent_sdk_core::testing::FakeJournalStore::default();
    journal.fail_next_append("disk full before tool intent");
    let coordinator = ToolExecutionCoordinator::new(ToolRouter::new(snapshot), executors)
        .with_policy(Arc::new(AllowToolPolicy));

    let error = coordinator
        .execute(
            &journal,
            read_request("tool.call.intent_fail"),
            execution_context(&package),
        )
        .expect_err("intent append failure must fail closed");

    assert_eq!(error.kind(), AgentErrorKind::JournalFailure);
    assert_eq!(executor.call_count(), 0);
    assert!(journal.records().is_empty());
}

#[test]
fn non_idempotent_write_result_append_failure_enters_recovery() {
    let package = package_with_tools();
    let snapshot = registry_snapshot(&package);
    let executor = Arc::new(ScriptedToolExecutor::new(
        ExecutorRef::new("executor.workspace_write.v1"),
        ToolExecutionOutput::completed("workspace write applied externally"),
    ));
    let mut executors = ToolExecutorRegistry::new();
    executors
        .register(executor.clone())
        .expect("executor registers");
    let journal = SequencedJournal::fail_on_seq(2);
    let coordinator = ToolExecutionCoordinator::new(ToolRouter::new(snapshot), executors)
        .with_policy(Arc::new(AllowToolPolicy))
        .with_strategy(ToolExecutionStrategy::BoundedConcurrent { max_in_flight: 1 });

    let outcome = coordinator
        .execute(
            &journal,
            ToolCallRequest {
                tool_call_id: ToolCallId::new("tool.call.write.1"),
                canonical_tool_name: CanonicalToolName::new("workspace_write"),
                source: source("source.model.tool_call"),
                requested_args_refs: vec![ContentRefId::new("content.args.write.1")],
                redacted_args_summary: "write docs/notes.md".to_string(),
                idempotency_key: None,
                dedupe_key: None,
            },
            execution_context(&package),
        )
        .expect("recovery marker is journaled");

    assert!(outcome.recovery_required);
    assert_eq!(executor.call_count(), 1);
    assert!(matches!(
        executor.calls()[0].strategy,
        ToolExecutionStrategy::BoundedConcurrent { max_in_flight: 1 }
    ));
    assert_eq!(
        outcome.record.status,
        ToolCallRecordStatus::RecoveryRequired
    );
    assert_eq!(
        journal_summary(&journal.records()),
        read_fixture("tests/fixtures/tools/non-idempotent-write-recovery.json")
            .expect("write recovery fixture")
    );
}

#[test]
fn before_tool_hook_deny_records_terminal_denial_without_executor_start() {
    let hook = before_tool_hook(
        "deny.before_tool",
        "executor.hook.before_tool.deny",
        HookMutationRight::Deny,
    );
    let package = package_with_hooks([hook.clone()]);
    let snapshot = registry_snapshot(&package);
    let executor = Arc::new(ScriptedToolExecutor::new(
        ExecutorRef::new("executor.workspace_read.v1"),
        ToolExecutionOutput::completed("must not execute"),
    ));
    let mut executors = ToolExecutorRegistry::new();
    executors
        .register(executor.clone())
        .expect("executor registers");
    let hook_registry = hook_registry_with(ScriptedHookExecutor::once(
        "executor.hook.before_tool.deny",
        HookResponse::Deny(DenyReason {
            code: "tool.hook.denied".to_string(),
            redacted_summary: "hook denied this tool call".to_string(),
        }),
        1,
    ));
    let journal = agent_sdk_core::testing::FakeJournalStore::default();
    let coordinator = ToolExecutionCoordinator::new(ToolRouter::new(snapshot), executors)
        .with_policy(Arc::new(AllowToolPolicy))
        .with_hooks([hook], hook_registry);

    let outcome = coordinator
        .execute(
            &journal,
            read_request("tool.call.hook_deny"),
            execution_context(&package),
        )
        .expect("hook denial returns a terminal denied outcome");

    assert_eq!(executor.call_count(), 0);
    assert_eq!(
        outcome.record.status,
        ToolCallRecordStatus::DeniedBeforeExecution
    );
    assert!(outcome.intent_cursor.is_none());
    assert_eq!(
        outcome
            .record
            .effect_result
            .as_ref()
            .expect("hook denial effect result")
            .terminal_status,
        agent_sdk_core::EffectTerminalStatus::DeniedBeforeExecution
    );
    let records = journal.records();
    assert_eq!(accepted_hook_records(&records, HookResponseClass::Deny), 1);
    let denied_record = records
        .iter()
        .find_map(tool_record)
        .expect("tool denied record");
    assert_eq!(
        denied_record.status,
        ToolCallRecordStatus::DeniedBeforeExecution
    );
    assert_eq!(
        denied_record
            .hook_id
            .as_ref()
            .map(|hook_id| hook_id.as_str()),
        Some("deny.before_tool")
    );
    assert_eq!(
        tool_hook_record_summary(&records),
        read_fixture("tests/fixtures/tools/hook-deny-tool-record.json")
            .expect("hook deny tool fixture")
    );
}

#[test]
fn before_tool_hook_modifies_redacted_request_summary_before_policy_and_executor() {
    let hook = before_tool_hook(
        "modify.before_tool",
        "executor.hook.before_tool.modify",
        HookMutationRight::ModifyToolRequest,
    );
    let package = package_with_hooks([hook.clone()]);
    let snapshot = registry_snapshot(&package);
    let executor = Arc::new(ScriptedToolExecutor::new(
        ExecutorRef::new("executor.workspace_read.v1"),
        ToolExecutionOutput::completed("read completed"),
    ));
    let mut executors = ToolExecutorRegistry::new();
    executors
        .register(executor.clone())
        .expect("executor registers");
    let hook_registry = hook_registry_with(ScriptedHookExecutor::once(
        "executor.hook.before_tool.modify",
        HookResponse::ModifyToolRequest(ToolRequestPatch {
            redacted_summary: "read docs/start-here.md with narrowed scope".to_string(),
        }),
        1,
    ));
    let journal = agent_sdk_core::testing::FakeJournalStore::default();
    let coordinator = ToolExecutionCoordinator::new(ToolRouter::new(snapshot), executors)
        .with_policy(Arc::new(AllowToolPolicy))
        .with_hooks([hook], hook_registry);

    let outcome = coordinator
        .execute(
            &journal,
            read_request("tool.call.hook_modify"),
            execution_context(&package),
        )
        .expect("hook request modification succeeds");

    assert_eq!(outcome.record.status, ToolCallRecordStatus::Completed);
    assert_eq!(executor.call_count(), 1);
    assert_eq!(
        executor.calls()[0]
            .resolved_call
            .request
            .redacted_args_summary,
        "read docs/start-here.md with narrowed scope"
    );
    let records = journal.records();
    let modified_record = records
        .iter()
        .find_map(|record| match tool_record(record) {
            Some(tool) if tool.status == ToolCallRecordStatus::RequestModified => Some(tool),
            _ => None,
        })
        .expect("tool request modification record");
    assert_eq!(
        modified_record.original_redacted_args_summary.as_deref(),
        Some("read docs/start-here.md")
    );
    assert_eq!(
        modified_record.patched_redacted_args_summary.as_deref(),
        Some("read docs/start-here.md with narrowed scope")
    );
    assert_eq!(
        accepted_hook_records(&records, HookResponseClass::ModifyToolRequest),
        1
    );
    assert_eq!(
        tool_hook_record_summary(&records),
        read_fixture("tests/fixtures/tools/hook-request-modified-tool-record.json")
            .expect("hook request modified tool fixture")
    );
}

#[test]
fn after_tool_observe_hook_runs_with_result_view_after_original_result_record() {
    let hook = HookSpec::observe(
        "observe.after_tool",
        HookPoint::AfterToolCall,
        HookSource::InProcess,
        "executor.hook.after_tool.observe",
        hook_policy("policy.hooks.after_tool.observe"),
    );
    let package = package_with_hooks([hook.clone()]);
    let snapshot = registry_snapshot(&package);
    let executor = Arc::new(ScriptedToolExecutor::new(
        ExecutorRef::new("executor.workspace_read.v1"),
        ToolExecutionOutput::completed("read completed"),
    ));
    let mut executors = ToolExecutorRegistry::new();
    executors.register(executor).expect("executor registers");
    let hook_executor = ScriptedHookExecutor::once(
        "executor.hook.after_tool.observe",
        HookResponse::ObserveOnly,
        1,
    );
    let hook_executor_view = hook_executor.clone();
    let hook_registry = hook_registry_with(hook_executor);
    let journal = agent_sdk_core::testing::FakeJournalStore::default();
    let coordinator = ToolExecutionCoordinator::new(ToolRouter::new(snapshot), executors)
        .with_policy(Arc::new(AllowToolPolicy))
        .with_hooks([hook], hook_registry);

    coordinator
        .execute(
            &journal,
            read_request("tool.call.after_observe"),
            execution_context(&package),
        )
        .expect("after-tool observe hook succeeds");

    assert_eq!(hook_executor_view.invocations().len(), 1);
    assert_eq!(
        hook_executor_view.invocations()[0].view.redacted_summary,
        "after tool call: read completed"
    );
    assert!(journal.records().iter().any(|record| {
        matches!(
            tool_record(record),
            Some(tool) if tool.status == ToolCallRecordStatus::Completed
        )
    }));
}

#[test]
fn before_tool_hook_view_is_bounded_for_oversized_request_summary() {
    let hook = before_tool_hook(
        "observe.before_tool",
        "executor.hook.before_tool.observe",
        HookMutationRight::Observe,
    );
    let package = package_with_hooks([hook.clone()]);
    let snapshot = registry_snapshot(&package);
    let executor = Arc::new(ScriptedToolExecutor::new(
        ExecutorRef::new("executor.workspace_read.v1"),
        ToolExecutionOutput::completed("read completed"),
    ));
    let mut executors = ToolExecutorRegistry::new();
    executors.register(executor).expect("executor registers");
    let hook_executor = ScriptedHookExecutor::once(
        "executor.hook.before_tool.observe",
        HookResponse::ObserveOnly,
        1,
    );
    let hook_executor_view = hook_executor.clone();
    let hook_registry = hook_registry_with(hook_executor);
    let coordinator = ToolExecutionCoordinator::new(ToolRouter::new(snapshot), executors)
        .with_policy(Arc::new(AllowToolPolicy))
        .with_hooks([hook], hook_registry);
    let mut request = read_request("tool.call.before_view_bounded");
    request.redacted_args_summary = "x".repeat(3_000);

    coordinator
        .execute(
            &agent_sdk_core::testing::FakeJournalStore::default(),
            request,
            execution_context(&package),
        )
        .expect("bounded before-tool view succeeds");

    let view_summary = &hook_executor_view.invocations()[0].view.redacted_summary;
    assert!(view_summary.chars().count() <= 2_048);
    assert!(view_summary.contains("[truncated; original_chars="));
}

#[test]
fn after_tool_hook_view_is_bounded_for_oversized_result_summary() {
    let hook = HookSpec::observe(
        "observe.after_tool",
        HookPoint::AfterToolCall,
        HookSource::InProcess,
        "executor.hook.after_tool.observe",
        hook_policy("policy.hooks.after_tool.observe"),
    );
    let package = package_with_hooks([hook.clone()]);
    let snapshot = registry_snapshot(&package);
    let executor = Arc::new(ScriptedToolExecutor::new(
        ExecutorRef::new("executor.workspace_read.v1"),
        ToolExecutionOutput::completed("x".repeat(3_000)),
    ));
    let mut executors = ToolExecutorRegistry::new();
    executors.register(executor).expect("executor registers");
    let hook_executor = ScriptedHookExecutor::once(
        "executor.hook.after_tool.observe",
        HookResponse::ObserveOnly,
        1,
    );
    let hook_executor_view = hook_executor.clone();
    let hook_registry = hook_registry_with(hook_executor);
    let coordinator = ToolExecutionCoordinator::new(ToolRouter::new(snapshot), executors)
        .with_policy(Arc::new(AllowToolPolicy))
        .with_hooks([hook], hook_registry);

    coordinator
        .execute(
            &agent_sdk_core::testing::FakeJournalStore::default(),
            read_request("tool.call.after_view_bounded"),
            execution_context(&package),
        )
        .expect("bounded after-tool view succeeds");

    let view_summary = &hook_executor_view.invocations()[0].view.redacted_summary;
    assert!(view_summary.chars().count() <= 2_048);
    assert!(view_summary.contains("[truncated; original_chars="));
}

#[test]
fn after_tool_hook_rewrite_preserves_original_result_and_updates_final_summary() {
    let hook = after_tool_hook(
        "rewrite.after_tool",
        "executor.hook.after_tool.rewrite",
        HookMutationRight::RewriteToolResult,
    );
    let package = package_with_hooks([hook.clone()]);
    let snapshot = registry_snapshot(&package);
    let executor = Arc::new(ScriptedToolExecutor::new(
        ExecutorRef::new("executor.workspace_read.v1"),
        ToolExecutionOutput::completed("original read summary"),
    ));
    let mut executors = ToolExecutorRegistry::new();
    executors.register(executor).expect("executor registers");
    let hook_registry = hook_registry_with(ScriptedHookExecutor::once(
        "executor.hook.after_tool.rewrite",
        HookResponse::RewriteToolResult(ToolResultPatch {
            redacted_summary: "rewritten read summary".to_string(),
        }),
        1,
    ));
    let journal = agent_sdk_core::testing::FakeJournalStore::default();
    let policy = Arc::new(RecordingToolPolicy::default());
    let coordinator = ToolExecutionCoordinator::new(ToolRouter::new(snapshot), executors)
        .with_policy(policy.clone())
        .with_hooks([hook], hook_registry);

    let outcome = coordinator
        .execute(
            &journal,
            read_request("tool.call.after_rewrite"),
            execution_context(&package),
        )
        .expect("after-tool rewrite succeeds");

    assert_eq!(
        outcome.record.redacted_result_summary.as_deref(),
        Some("rewritten read summary")
    );
    assert_eq!(
        outcome.terminal_cursor.as_ref().unwrap().as_str(),
        "journal.6"
    );
    assert_eq!(policy.post_summaries(), vec!["rewritten read summary"]);
    let records = journal.records();
    let completed = records
        .iter()
        .filter_map(tool_record)
        .find(|tool| tool.status == ToolCallRecordStatus::Completed)
        .expect("original result record");
    assert_eq!(
        completed.redacted_result_summary.as_deref(),
        Some("original read summary")
    );
    let rewritten = records
        .iter()
        .filter_map(tool_record)
        .find(|tool| tool.status == ToolCallRecordStatus::ResultRewritten)
        .expect("rewrite record");
    assert_eq!(
        rewritten.original_redacted_result_summary.as_deref(),
        Some("original read summary")
    );
    assert_eq!(
        rewritten.patched_redacted_result_summary.as_deref(),
        Some("rewritten read summary")
    );
    assert_eq!(
        accepted_hook_records(&records, HookResponseClass::RewriteToolResult),
        1
    );
    assert_eq!(
        tool_hook_record_summary(&records),
        read_fixture("tests/fixtures/tools/hook-result-rewritten-tool-record.json")
            .expect("hook result rewritten tool fixture")
    );
}

#[test]
fn missing_tool_hook_executor_fails_before_tool_executor_start() {
    let hook = before_tool_hook(
        "deny.before_tool",
        "executor.hook.before_tool.missing",
        HookMutationRight::Deny,
    );
    let package = package_with_hooks([hook.clone()]);
    let snapshot = registry_snapshot(&package);
    let executor = Arc::new(ScriptedToolExecutor::new(
        ExecutorRef::new("executor.workspace_read.v1"),
        ToolExecutionOutput::completed("must not execute"),
    ));
    let mut executors = ToolExecutorRegistry::new();
    executors
        .register(executor.clone())
        .expect("executor registers");
    let coordinator = ToolExecutionCoordinator::new(ToolRouter::new(snapshot), executors)
        .with_policy(Arc::new(AllowToolPolicy))
        .with_hooks([hook], InMemoryHookExecutorRegistry::default());

    let error = coordinator
        .execute(
            &agent_sdk_core::testing::FakeJournalStore::default(),
            read_request("tool.call.missing_hook_executor"),
            execution_context(&package),
        )
        .expect_err("missing hook executor fails before tool executor");

    assert_eq!(error.kind(), AgentErrorKind::InvalidPackage);
    assert_eq!(executor.call_count(), 0);
}

#[test]
fn missing_after_tool_hook_executor_fails_before_tool_executor_start() {
    let hook = HookSpec::observe(
        "observe.after_tool",
        HookPoint::AfterToolCall,
        HookSource::InProcess,
        "executor.hook.after_tool.missing",
        hook_policy("policy.hooks.after_tool.missing"),
    );
    let package = package_with_hooks([hook.clone()]);
    let snapshot = registry_snapshot(&package);
    let executor = Arc::new(ScriptedToolExecutor::new(
        ExecutorRef::new("executor.workspace_read.v1"),
        ToolExecutionOutput::completed("must not execute"),
    ));
    let mut executors = ToolExecutorRegistry::new();
    executors
        .register(executor.clone())
        .expect("executor registers");
    let journal = agent_sdk_core::testing::FakeJournalStore::default();
    let coordinator = ToolExecutionCoordinator::new(ToolRouter::new(snapshot), executors)
        .with_policy(Arc::new(AllowToolPolicy))
        .with_hooks([hook], InMemoryHookExecutorRegistry::default());

    let error = coordinator
        .execute(
            &journal,
            read_request("tool.call.missing_after_hook_executor"),
            execution_context(&package),
        )
        .expect_err("missing after-tool hook executor fails before tool executor");

    assert_eq!(error.kind(), AgentErrorKind::InvalidPackage);
    assert_eq!(executor.call_count(), 0);
    assert!(journal.records().is_empty());
}

#[test]
fn unsupported_active_tool_hook_rights_fail_before_tool_executor_start() {
    let before_approval = before_tool_hook(
        "approval.before_tool",
        "executor.hook.before_tool.approval",
        HookMutationRight::RequestApproval,
    );
    let after_retry = after_tool_hook(
        "retry.after_tool",
        "executor.hook.after_tool.retry",
        HookMutationRight::RequestRetry,
    );
    for hook in [before_approval, after_retry] {
        let package = package_with_hooks([hook.clone()]);
        let snapshot = registry_snapshot(&package);
        let executor = Arc::new(ScriptedToolExecutor::new(
            ExecutorRef::new("executor.workspace_read.v1"),
            ToolExecutionOutput::completed("must not execute"),
        ));
        let mut executors = ToolExecutorRegistry::new();
        executors
            .register(executor.clone())
            .expect("executor registers");
        let coordinator = ToolExecutionCoordinator::new(ToolRouter::new(snapshot), executors)
            .with_policy(Arc::new(AllowToolPolicy))
            .with_hooks([hook], InMemoryHookExecutorRegistry::default());

        let error = coordinator
            .execute(
                &agent_sdk_core::testing::FakeJournalStore::default(),
                read_request("tool.call.unsupported_hook_right"),
                execution_context(&package),
            )
            .expect_err("unsupported hook right fails before tool executor");

        assert_eq!(error.kind(), AgentErrorKind::InvalidPackage);
        assert_eq!(executor.call_count(), 0);
    }
}

#[test]
fn oversized_before_tool_hook_payloads_fail_closed_with_rejected_hook_records() {
    for (hook, response, response_class) in [
        (
            before_tool_hook(
                "deny.before_tool",
                "executor.hook.before_tool.oversized_deny",
                HookMutationRight::Deny,
            ),
            HookResponse::Deny(DenyReason {
                code: "tool.hook.denied".to_string(),
                redacted_summary: "x".repeat(3_000),
            }),
            HookResponseClass::Deny,
        ),
        (
            before_tool_hook(
                "modify.before_tool",
                "executor.hook.before_tool.oversized_modify",
                HookMutationRight::ModifyToolRequest,
            ),
            HookResponse::ModifyToolRequest(ToolRequestPatch {
                redacted_summary: "x".repeat(3_000),
            }),
            HookResponseClass::ModifyToolRequest,
        ),
    ] {
        let package = package_with_hooks([hook.clone()]);
        let snapshot = registry_snapshot(&package);
        let executor = Arc::new(ScriptedToolExecutor::new(
            ExecutorRef::new("executor.workspace_read.v1"),
            ToolExecutionOutput::completed("must not execute"),
        ));
        let mut executors = ToolExecutorRegistry::new();
        executors
            .register(executor.clone())
            .expect("executor registers");
        let hook_registry = hook_registry_with(ScriptedHookExecutor::once(
            hook.executor_ref.as_str(),
            response,
            1,
        ));
        let journal = agent_sdk_core::testing::FakeJournalStore::default();
        let coordinator = ToolExecutionCoordinator::new(ToolRouter::new(snapshot), executors)
            .with_policy(Arc::new(AllowToolPolicy))
            .with_hooks([hook], hook_registry);

        let error = coordinator
            .execute(
                &journal,
                read_request("tool.call.oversized_before_hook"),
                execution_context(&package),
            )
            .expect_err("oversized before-tool hook payload fails closed");

        assert_eq!(error.kind(), AgentErrorKind::PolicyDenial);
        assert_eq!(executor.call_count(), 0);
        assert_eq!(rejected_hook_records(&journal.records(), response_class), 1);
        assert!(
            !journal
                .records()
                .iter()
                .any(|record| tool_record(record).is_some())
        );
    }
}

#[test]
fn oversized_after_tool_rewrite_preserves_original_result_and_skips_post_policy() {
    let hook = after_tool_hook(
        "rewrite.after_tool",
        "executor.hook.after_tool.oversized_rewrite",
        HookMutationRight::RewriteToolResult,
    );
    let package = package_with_hooks([hook.clone()]);
    let snapshot = registry_snapshot(&package);
    let executor = Arc::new(ScriptedToolExecutor::new(
        ExecutorRef::new("executor.workspace_read.v1"),
        ToolExecutionOutput::completed("original read summary"),
    ));
    let mut executors = ToolExecutorRegistry::new();
    executors
        .register(executor.clone())
        .expect("executor registers");
    let hook_registry = hook_registry_with(ScriptedHookExecutor::once(
        "executor.hook.after_tool.oversized_rewrite",
        HookResponse::RewriteToolResult(ToolResultPatch {
            redacted_summary: "x".repeat(3_000),
        }),
        1,
    ));
    let journal = agent_sdk_core::testing::FakeJournalStore::default();
    let policy = Arc::new(RecordingToolPolicy::default());
    let coordinator = ToolExecutionCoordinator::new(ToolRouter::new(snapshot), executors)
        .with_policy(policy.clone())
        .with_hooks([hook], hook_registry);

    let error = coordinator
        .execute(
            &journal,
            read_request("tool.call.oversized_after_rewrite"),
            execution_context(&package),
        )
        .expect_err("oversized rewrite fails closed");

    assert_eq!(error.kind(), AgentErrorKind::PolicyDenial);
    assert_eq!(executor.call_count(), 1);
    assert!(policy.post_summaries().is_empty());
    let records = journal.records();
    assert!(records.iter().any(|record| {
        matches!(
            tool_record(record),
            Some(tool)
                if tool.status == ToolCallRecordStatus::Completed
                    && tool.redacted_result_summary.as_deref() == Some("original read summary")
        )
    }));
    assert!(!records.iter().any(|record| {
        matches!(
            tool_record(record),
            Some(tool) if tool.status == ToolCallRecordStatus::ResultRewritten
        )
    }));
    assert_eq!(
        rejected_hook_records(&records, HookResponseClass::RewriteToolResult),
        1
    );
}

#[test]
fn core_tool_execution_surface_has_no_builtin_tool_pack_behavior() {
    let application_source = include_str!("../../src/application/tool.rs");
    let ports_source = include_str!("../../src/ports/tool.rs");

    for forbidden in [
        "std::fs",
        "std::process",
        "Command::new",
        "mcp",
        "workspace_readonly",
        "workspace_edit",
        "shell",
    ] {
        assert!(
            !application_source.contains(forbidden),
            "{forbidden} belongs to optional tool packs or host adapters, not core application/tool.rs"
        );
        assert!(
            !ports_source.contains(forbidden),
            "{forbidden} belongs to optional tool packs or host adapters, not core ports/tool.rs"
        );
    }
}

#[derive(Clone, Debug)]
struct SequencedJournal {
    records: Arc<Mutex<Vec<JournalRecord>>>,
    fail_on_seq: u64,
}

impl SequencedJournal {
    fn fail_on_seq(fail_on_seq: u64) -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
            fail_on_seq,
        }
    }

    fn records(&self) -> Vec<JournalRecord> {
        self.records.lock().expect("journal lock").clone()
    }
}

impl agent_sdk_core::RunJournal for SequencedJournal {
    fn append(&self, record: JournalRecord) -> Result<agent_sdk_core::JournalCursor, AgentError> {
        if record.journal_seq == self.fail_on_seq && record.record_id.ends_with("tool.result") {
            return Err(AgentError::new(
                AgentErrorKind::JournalFailure,
                RetryClassification::RepairNeeded,
                "injected result append failure",
            ));
        }
        let mut records = self.records.lock().expect("journal lock");
        records.push(record);
        Ok(agent_sdk_core::JournalCursor::new(format!(
            "journal.{}",
            records.len()
        )))
    }
}

#[derive(Clone, Debug, Default)]
struct RecordingToolPolicy {
    post_summaries: Arc<Mutex<Vec<String>>>,
}

impl RecordingToolPolicy {
    fn post_summaries(&self) -> Vec<String> {
        self.post_summaries
            .lock()
            .expect("post summaries lock")
            .clone()
    }
}

impl ToolPolicyPort for RecordingToolPolicy {
    fn evaluate_pre_tool(
        &self,
        call: &agent_sdk_core::tool_ports::ResolvedToolCall,
    ) -> Result<PolicyOutcome, AgentError> {
        Ok(agent_sdk_core::tool_ports::allowed_tool_policy_outcome(
            call.request.source.clone(),
            call.route.destination.clone(),
            call.route.policy_refs.clone(),
        ))
    }

    fn evaluate_post_tool(
        &self,
        call: &agent_sdk_core::tool_ports::ResolvedToolCall,
        output: &ToolExecutionOutput,
    ) -> Result<PolicyOutcome, AgentError> {
        self.post_summaries
            .lock()
            .expect("post summaries lock")
            .push(output.redacted_summary.clone());
        Ok(PolicyOutcome {
            stage: PolicyStage::PostTool,
            decision: PolicyDecision::allow("tool.policy.allowed"),
            subject: None,
            source: Some(call.request.source.clone()),
            destination: Some(call.route.destination.clone()),
            policy_refs: call.route.policy_refs.clone(),
            privacy: PrivacyClass::Internal,
            retention: RetentionClass::RunScoped,
        })
    }
}

fn package_with_tools() -> RuntimePackage {
    RuntimePackage::builder(RuntimePackageId::new("package.tool.execution"))
        .agent(agent_sdk_core::AgentSnapshot {
            agent_id: AgentId::new("agent.tool.execution"),
            name: "tool execution".to_string(),
            default_behavior_refs: Vec::new(),
        })
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake"))
        .capability(CapabilitySpec::fake_tool(
            "cap.tool.read",
            "workspace_read",
            schema_ref("sidecar.schema.read"),
            ExecutorRef::new("executor.workspace_read.v1"),
            approval_policy("policy.approval.read"),
            source("source.sdk.toolpack"),
        ))
        .capability(CapabilitySpec::fake_tool(
            "cap.tool.write",
            "workspace_write",
            schema_ref("sidecar.schema.write"),
            ExecutorRef::new("executor.workspace_write.v1"),
            approval_policy("policy.approval.write"),
            source("source.sdk.toolpack"),
        ))
        .build()
        .expect("package builds")
}

fn package_with_hooks(hooks: impl IntoIterator<Item = HookSpec>) -> RuntimePackage {
    let mut builder = RuntimePackage::builder(RuntimePackageId::new("package.tool.execution"))
        .agent(agent_sdk_core::AgentSnapshot {
            agent_id: AgentId::new("agent.tool.execution"),
            name: "tool execution".to_string(),
            default_behavior_refs: Vec::new(),
        })
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake"))
        .capability(CapabilitySpec::fake_tool(
            "cap.tool.read",
            "workspace_read",
            schema_ref("sidecar.schema.read"),
            ExecutorRef::new("executor.workspace_read.v1"),
            approval_policy("policy.approval.read"),
            source("source.sdk.toolpack"),
        ))
        .capability(CapabilitySpec::fake_tool(
            "cap.tool.write",
            "workspace_write",
            schema_ref("sidecar.schema.write"),
            ExecutorRef::new("executor.workspace_write.v1"),
            approval_policy("policy.approval.write"),
            source("source.sdk.toolpack"),
        ));
    for hook in hooks {
        builder = builder.hook(hook);
    }
    builder.build().expect("package builds")
}

fn registry_snapshot(package: &RuntimePackage) -> ToolRegistrySnapshot {
    ToolRegistrySnapshot::from_runtime_package(package, [read_route(), write_route()])
        .expect("registry snapshot")
}

fn read_route() -> ToolRoute {
    ToolRoute {
        capability_id: CapabilityId::new("cap.tool.read"),
        canonical_tool_name: CanonicalToolName::new("workspace_read"),
        namespace: agent_sdk_core::CapabilityNamespace::new("tool.workspace_read"),
        source: source("source.sdk.toolpack"),
        destination: DestinationRef::with_kind(DestinationKind::Tool, "destination.tool.read"),
        executor_ref: Some(ExecutorRef::new("executor.workspace_read.v1")),
        policy_refs: vec![approval_policy("policy.approval.read")],
        sidecar_refs: vec![schema_ref("sidecar.schema.read")],
        effect_class: agent_sdk_core::policy::EffectClass::Read,
        risk_class: agent_sdk_core::policy::RiskClass::Low,
        privacy: PrivacyClass::ContentRefsOnly,
        retention: RetentionClass::RunScoped,
    }
}

fn write_route() -> ToolRoute {
    ToolRoute {
        capability_id: CapabilityId::new("cap.tool.write"),
        canonical_tool_name: CanonicalToolName::new("workspace_write"),
        namespace: agent_sdk_core::CapabilityNamespace::new("tool.workspace_write"),
        source: source("source.sdk.toolpack"),
        destination: DestinationRef::with_kind(DestinationKind::Tool, "destination.tool.write"),
        executor_ref: Some(ExecutorRef::new("executor.workspace_write.v1")),
        policy_refs: vec![approval_policy("policy.approval.write")],
        sidecar_refs: vec![schema_ref("sidecar.schema.write")],
        effect_class: agent_sdk_core::policy::EffectClass::Write,
        risk_class: agent_sdk_core::policy::RiskClass::High,
        privacy: PrivacyClass::ContentRefsOnly,
        retention: RetentionClass::RunScoped,
    }
}

fn execution_context(package: &RuntimePackage) -> ToolExecutionContext {
    ToolExecutionContext {
        timestamp_millis: 100,
        ..ToolExecutionContext::new(
            RunId::new("run.tool.execution"),
            AgentId::new("agent.tool.execution"),
            source("source.sdk.run_loop"),
            package.fingerprint().expect("package fingerprint").as_str(),
        )
    }
}

fn read_request(tool_call_id: &str) -> ToolCallRequest {
    ToolCallRequest {
        tool_call_id: ToolCallId::new(tool_call_id),
        canonical_tool_name: CanonicalToolName::new("workspace_read"),
        source: source("source.model.tool_call"),
        requested_args_refs: vec![ContentRefId::new("content.args.read.1")],
        redacted_args_summary: "read docs/start-here.md".to_string(),
        idempotency_key: Some(IdempotencyKey::new("idem.read.1")),
        dedupe_key: None,
    }
}

fn source(id: &str) -> SourceRef {
    SourceRef::with_kind(SourceKind::Sdk, id)
}

fn approval_policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::Approval, id)
}

fn schema_ref(id: &str) -> agent_sdk_core::PackageSidecarRef {
    agent_sdk_core::PackageSidecarRef::new(id, "tool_schema", "v1")
}

fn hook_policy(id: &str) -> PolicyRef {
    agent_sdk_core::hook_policy_ref(id)
}

fn before_tool_hook(id: &str, executor_ref: &str, right: HookMutationRight) -> HookSpec {
    HookSpec::blocking(
        id,
        HookPoint::BeforeToolCall,
        HookSource::InProcess,
        executor_ref,
        hook_policy(&format!("policy.hooks.{id}")),
        HookMutationRights::from_rights([right]),
    )
}

fn after_tool_hook(id: &str, executor_ref: &str, right: HookMutationRight) -> HookSpec {
    HookSpec::blocking(
        id,
        HookPoint::AfterToolCall,
        HookSource::InProcess,
        executor_ref,
        hook_policy(&format!("policy.hooks.{id}")),
        HookMutationRights::from_rights([right]),
    )
}

fn hook_registry_with(executor: ScriptedHookExecutor) -> InMemoryHookExecutorRegistry {
    let registry = InMemoryHookExecutorRegistry::default();
    registry
        .register(executor)
        .expect("hook executor registers");
    registry
}

fn tool_record(record: &JournalRecord) -> Option<&agent_sdk_core::ToolCallRecord> {
    match &record.payload {
        JournalRecordPayload::Tool(tool) => Some(tool),
        _ => None,
    }
}

fn accepted_hook_records(records: &[JournalRecord], response_class: HookResponseClass) -> usize {
    hook_response_decision_records(
        records,
        agent_sdk_core::HookResponseDecision::AcceptedJournaledBeforeApply,
        response_class,
    )
}

fn rejected_hook_records(records: &[JournalRecord], response_class: HookResponseClass) -> usize {
    hook_response_decision_records(
        records,
        agent_sdk_core::HookResponseDecision::RejectedPolicy,
        response_class,
    )
}

fn hook_response_decision_records(
    records: &[JournalRecord],
    decision: agent_sdk_core::HookResponseDecision,
    response_class: HookResponseClass,
) -> usize {
    records
        .iter()
        .filter(|record| match &record.payload {
            JournalRecordPayload::Hook(agent_sdk_core::HookRecord {
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

fn registry_summary(snapshot: &ToolRegistrySnapshot) -> Value {
    let routes = snapshot
        .routes
        .iter()
        .map(|route| {
            json!({
                "capability_id": route.capability_id.as_str(),
                "canonical_tool_name": route.canonical_tool_name.as_str(),
                "namespace": route.namespace.as_str(),
                "executor_ref": route.executor_ref.as_ref().map(|executor| executor.as_str()),
                "policy_refs": route.policy_refs.iter().map(|policy| policy.as_str()).collect::<Vec<_>>(),
                "effect_class": format!("{:?}", route.effect_class),
                "risk_class": format!("{:?}", route.risk_class),
            })
        })
        .collect::<Vec<_>>();
    agent_sdk_core::testing::normalize_json_value(json!({ "routes": routes }))
}

fn journal_summary(records: &[JournalRecord]) -> Value {
    let records = records
        .iter()
        .map(|record| match &record.payload {
            JournalRecordPayload::Tool(tool_record) if tool_record.effect_result.is_none() => {
                let intent = tool_record.effect_intent.as_ref().expect("tool intent");
                json!({
                "journal_seq": record.journal_seq,
                "record_kind": "tool_intent",
                "effect_id": intent.effect_id.as_str(),
                "effect_kind": format!("{:?}", intent.kind),
                "subject_kind": format!("{:?}", intent.subject_ref.kind),
                "content_refs": intent.content_refs.iter().map(|content| content.as_str()).collect::<Vec<_>>(),
                "redacted_summary": intent.redacted_summary,
                })
            }
            JournalRecordPayload::Tool(tool_record) => {
                let result = tool_record.effect_result.as_ref().expect("tool result");
                json!({
                "journal_seq": record.journal_seq,
                "record_kind": "tool_result",
                "effect_id": result.effect_id.as_str(),
                "terminal_status": format!("{:?}", result.terminal_status),
                "content_refs": result.content_refs.iter().map(|content| content.as_str()).collect::<Vec<_>>(),
                "redacted_summary": result.redacted_summary,
                })
            }
            JournalRecordPayload::Recovery(recovery) => json!({
                "journal_seq": record.journal_seq,
                "record_kind": "recovery",
                "recovery_reason": recovery.recovery_reason,
                "unsafe_pending": recovery.unsafe_pending.iter().map(|pending| {
                    json!({
                        "effect_id": pending.effect_id.as_str(),
                        "intent_record_id": pending.intent_record_id,
                        "idempotency_key": pending.idempotency_key.as_ref().map(|key| key.as_str()),
                        "dedupe_key": pending.dedupe_key.as_ref().map(|key| key.as_str()),
                        "unsafe_pending_reason": pending.unsafe_pending_reason,
                    })
                }).collect::<Vec<_>>(),
            }),
            other => json!({
                "journal_seq": record.journal_seq,
                "record_kind": format!("{:?}", other),
            }),
        })
        .collect::<Vec<_>>();
    agent_sdk_core::testing::normalize_json_value(json!({ "records": records }))
}

fn tool_hook_record_summary(records: &[JournalRecord]) -> Value {
    let records = records
        .iter()
        .filter_map(|record| {
            let tool_record = tool_record(record)?;
            tool_record.hook_id.as_ref()?;
            Some(json!({
                "journal_seq": record.journal_seq,
                "record_id": record.record_id.as_str(),
                "event_kind": record.event_index.event_kind.as_str(),
                "status": serde_json::to_value(&tool_record.status).expect("tool status json"),
                "hook_id": tool_record.hook_id.as_ref().map(|hook_id| hook_id.as_str()),
                "redacted_args_summary": tool_record.redacted_args_summary.as_str(),
                "original_redacted_args_summary": tool_record.original_redacted_args_summary.as_deref(),
                "patched_redacted_args_summary": tool_record.patched_redacted_args_summary.as_deref(),
                "redacted_result_summary": tool_record.redacted_result_summary.as_deref(),
                "original_redacted_result_summary": tool_record.original_redacted_result_summary.as_deref(),
                "patched_redacted_result_summary": tool_record.patched_redacted_result_summary.as_deref(),
                "effect_result": tool_record.effect_result.as_ref().map(|result| json!({
                    "effect_id": result.effect_id.as_str(),
                    "terminal_status": serde_json::to_value(&result.terminal_status).expect("terminal status json"),
                    "error_ref": result.error_ref.as_deref(),
                    "content_refs": result.content_refs.iter().map(|content| content.as_str()).collect::<Vec<_>>(),
                    "redacted_summary": result.redacted_summary.as_str(),
                })),
            }))
        })
        .collect::<Vec<_>>();
    agent_sdk_core::testing::normalize_json_value(json!({ "records": records }))
}
