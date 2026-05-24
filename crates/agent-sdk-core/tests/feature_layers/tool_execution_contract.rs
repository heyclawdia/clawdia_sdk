use std::sync::{Arc, Mutex};

use agent_sdk_core::{
    AgentError, AgentErrorKind, AgentId, AllowToolPolicy, CapabilityId, CapabilitySpec,
    DestinationKind, DestinationRef, ExecutorRef, IdempotencyKey, JournalRecord,
    JournalRecordPayload, PolicyKind, PolicyRef, PrivacyClass, ProviderRouteSnapshot,
    RetentionClass, RetryClassification, RunId, RuntimePackage, RuntimePackageId, SourceKind,
    SourceRef, ToolCallId, ToolExecutionContext, ToolExecutionCoordinator,
    domain::ContentRef as ContentRefId,
    testing::{ScriptedToolExecutor, read_fixture},
    tool_ports::{
        ToolCallRequest, ToolExecutionOutput, ToolExecutionStrategy, ToolExecutorRegistry,
        ToolRegistrySnapshot, ToolRoute, ToolRouter,
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
