use std::sync::Mutex;

use agent_sdk_core::{
    AgentError, AgentErrorKind, AgentId, ApprovalBroker, ApprovalDecision,
    ApprovalDispatchResponse, ApprovalLifecycleStatus, ApprovalRecord, ApprovalRequest,
    ApprovalTerminalStatus, DestinationKind, DestinationRef, EffectKind, EffectTerminalStatus,
    EntityKind, JournalCursor, JournalRecord, JournalRecordPayload, PolicyKind, PolicyRef, RunId,
    RunJournal, SourceKind, SourceRef, ToolCallId, TurnId,
    testing::{FakeJournalStore, ScriptedApprovalDispatcher, normalize_json_value, read_fixture},
};
use serde_json::{Value, json};

#[test]
fn approval_dispatch_records_intent_and_result_before_tool_release() {
    let request = source_scoped_request();
    let journal = FakeJournalStore::default();
    let dispatcher = ScriptedApprovalDispatcher::new(ApprovalDispatchResponse::decision(
        ApprovalDecision::approved("actor.host.user"),
    ));
    let broker = ApprovalBroker::default();

    let outcome = broker
        .request_approval(request.clone(), Some(&dispatcher), &journal)
        .expect("approval request completes");

    assert!(outcome.releases_tool_execution());
    assert_eq!(outcome.status, ApprovalTerminalStatus::Approved);
    assert_eq!(dispatcher.requests().len(), 1);
    assert_eq!(
        dispatcher.requests()[0].dispatcher_scope,
        agent_sdk_core::policy::DispatcherScope::SourceScoped
    );
    assert_eq!(dispatcher.requests()[0].source, request.source);

    let records = journal.records();
    assert_approval_effect_sequence(&records, EffectTerminalStatus::Completed);

    let actual = approval_journal_summary(&records);
    let expected = read_fixture("tests/fixtures/approval/dispatch-intent-result.json")
        .expect("approval dispatch fixture");
    assert_eq!(actual, expected);
}

#[test]
fn missing_dispatcher_denies_fail_closed_without_tool_release() {
    let request = source_scoped_request();
    let journal = FakeJournalStore::default();
    let broker = ApprovalBroker::default();

    let outcome = broker
        .request_approval(request, None, &journal)
        .expect("missing dispatcher is recorded as a denial");

    assert!(!outcome.releases_tool_execution());
    assert_eq!(outcome.status, ApprovalTerminalStatus::Denied);
    assert_eq!(outcome.reason_code, "missing.approval_dispatcher");
    assert_approval_effect_sequence(&journal.records(), EffectTerminalStatus::Failed);
}

#[test]
fn dispatcher_timeout_records_timeout_then_denied() {
    let request = source_scoped_request();
    let journal = FakeJournalStore::default();
    let dispatcher = ScriptedApprovalDispatcher::new(ApprovalDispatchResponse::TimedOut);
    let broker = ApprovalBroker::default();

    let outcome = broker
        .request_approval(request, Some(&dispatcher), &journal)
        .expect("timeout records terminal denial");

    assert!(!outcome.releases_tool_execution());
    assert_eq!(outcome.status, ApprovalTerminalStatus::TimedOut);
    assert_eq!(outcome.reason_code, "approval.timeout");
    assert_approval_effect_sequence(&journal.records(), EffectTerminalStatus::TimedOut);
}

#[test]
fn dispatcher_cancelled_response_records_cancelled_then_denied() {
    let request = source_scoped_request();
    let journal = FakeJournalStore::default();
    let dispatcher = ScriptedApprovalDispatcher::new(ApprovalDispatchResponse::Cancelled);
    let broker = ApprovalBroker::default();

    let outcome = broker
        .request_approval(request, Some(&dispatcher), &journal)
        .expect("dispatcher cancellation records terminal denial");

    assert!(!outcome.releases_tool_execution());
    assert_eq!(outcome.status, ApprovalTerminalStatus::Cancelled);
    assert_eq!(outcome.reason_code, "approval.cancelled");
    assert_approval_effect_sequence(&journal.records(), EffectTerminalStatus::Cancelled);
}

#[test]
fn dispatcher_unavailable_response_records_unavailable_then_denied() {
    let request = source_scoped_request();
    let journal = FakeJournalStore::default();
    let dispatcher =
        ScriptedApprovalDispatcher::new(ApprovalDispatchResponse::unavailable("dispatcher.down"));
    let broker = ApprovalBroker::default();

    let outcome = broker
        .request_approval(request, Some(&dispatcher), &journal)
        .expect("dispatcher unavailable records terminal denial");

    assert!(!outcome.releases_tool_execution());
    assert_eq!(
        outcome.status,
        ApprovalTerminalStatus::DispatcherUnavailable
    );
    assert_eq!(outcome.reason_code, "dispatcher.down");
    assert_approval_effect_sequence(&journal.records(), EffectTerminalStatus::Failed);
}

#[test]
fn approval_cancel_prevents_dispatch_and_tool_release() {
    let request = source_scoped_request();
    let journal = FakeJournalStore::default();
    let dispatcher = ScriptedApprovalDispatcher::new(ApprovalDispatchResponse::decision(
        ApprovalDecision::approved("actor.host.user"),
    ));
    let broker = ApprovalBroker::default();
    broker.cancel_before_dispatch(request.approval_request_id.clone(), "host cancelled run");

    let outcome = broker
        .request_approval(request, Some(&dispatcher), &journal)
        .expect("cancelled approval records terminal denial");

    assert!(!outcome.releases_tool_execution());
    assert_eq!(outcome.status, ApprovalTerminalStatus::Cancelled);
    assert_eq!(outcome.reason_code, "approval.cancelled");
    assert!(dispatcher.requests().is_empty());
    assert_approval_effect_sequence(&journal.records(), EffectTerminalStatus::Cancelled);
}

#[test]
fn approval_finite_token_parser_accepts_only_exact_tokens() {
    let actor = SourceRef::with_kind(SourceKind::Host, "actor.host.user");

    assert!(matches!(
        ApprovalDecision::from_finite_token("approved", actor.clone()),
        Some(ApprovalDecision::Approved { .. })
    ));
    assert!(matches!(
        ApprovalDecision::from_finite_token("approved_for_session", actor.clone()),
        Some(ApprovalDecision::ApprovedForSession { .. })
    ));
    assert!(matches!(
        ApprovalDecision::approved_for_session("actor.host.user"),
        ApprovalDecision::ApprovedForSession { .. }
    ));
    assert!(
        ApprovalDecision::from_finite_token("approve", actor).is_none(),
        "approval tokens are exact and finite; no synonym guessing"
    );
}

#[test]
fn approval_record_lifecycle_shape_serializes_deterministically() {
    let record = ApprovalRecord::Requested {
        request: source_scoped_request(),
    };
    let encoded = serde_json::to_string(&record).expect("approval record serializes");
    let decoded: ApprovalRecord = serde_json::from_str(&encoded).expect("approval record decodes");

    assert_eq!(decoded, record);
    assert_eq!(
        ApprovalLifecycleStatus::Requested,
        ApprovalLifecycleStatus::Requested
    );
}

#[test]
fn dispatch_result_append_failure_blocks_tool_release_until_reconciled() {
    let request = source_scoped_request();
    let journal = FailOnAppendJournal::new(2);
    let dispatcher = ScriptedApprovalDispatcher::new(ApprovalDispatchResponse::decision(
        ApprovalDecision::approved("actor.host.user"),
    ));
    let broker = ApprovalBroker::default();

    let error = broker
        .request_approval(request, Some(&dispatcher), &journal)
        .expect_err("missing terminal dispatch result must block release");

    assert_eq!(error.kind(), AgentErrorKind::RecoveryRepairNeeded);
    assert_eq!(dispatcher.requests().len(), 1);
    assert_eq!(journal.records().len(), 2);
    assert!(matches!(
        journal.records()[0].payload,
        JournalRecordPayload::Approval(_)
    ));
    assert!(matches!(
        journal.records()[1].payload,
        JournalRecordPayload::Recovery(_)
    ));
}

#[test]
fn dispatcher_response_must_be_one_of_the_request_finite_decisions() {
    let mut request = source_scoped_request();
    request.allowed_decisions = vec![agent_sdk_core::policy::ApprovalDecisionKind::Denied];
    let journal = FakeJournalStore::default();
    let dispatcher = ScriptedApprovalDispatcher::new(ApprovalDispatchResponse::decision(
        ApprovalDecision::approved("actor.host.user"),
    ));
    let broker = ApprovalBroker::default();

    let outcome = broker
        .request_approval(request, Some(&dispatcher), &journal)
        .expect("invalid finite response records denial");

    assert!(!outcome.releases_tool_execution());
    assert_eq!(outcome.status, ApprovalTerminalStatus::Denied);
    assert_eq!(outcome.reason_code, "approval.decision_not_allowed");
}

#[test]
fn extension_cannot_answer_its_own_approval() {
    let mut request = source_scoped_request();
    request.source = SourceRef::with_kind(SourceKind::Extension, "source.extension.writer");
    let journal = FakeJournalStore::default();
    let dispatcher = ScriptedApprovalDispatcher::new(ApprovalDispatchResponse::decision(
        ApprovalDecision::Approved {
            actor_ref: request.source.clone(),
        },
    ));
    let broker = ApprovalBroker::default();

    let outcome = broker
        .request_approval(request, Some(&dispatcher), &journal)
        .expect("extension self-response records denial");

    assert!(!outcome.releases_tool_execution());
    assert_eq!(outcome.status, ApprovalTerminalStatus::Denied);
    assert_eq!(outcome.reason_code, "approval.extension_self_response");
    assert_approval_effect_sequence(
        &journal.records(),
        EffectTerminalStatus::DeniedBeforeExecution,
    );
}

fn source_scoped_request() -> ApprovalRequest {
    ApprovalRequest {
        approval_request_id: agent_sdk_core::domain::ApprovalRequestId::new("approval.request.1"),
        approval_dispatch_effect_id: agent_sdk_core::EffectId::new("effect.approval.1"),
        run_id: RunId::new("run.approval.1"),
        agent_id: AgentId::new("agent.approval.1"),
        turn_id: TurnId::new("turn.approval.1"),
        tool_call_id: ToolCallId::new("tool.call.approval.1"),
        source: SourceRef::with_kind(SourceKind::RemoteChannel, "source.remote.sms"),
        destination: DestinationRef::with_kind(DestinationKind::Tool, "destination.tool.write"),
        canonical_tool_name: "workspace.write".to_string(),
        tool_source: SourceRef::with_kind(SourceKind::Tool, "source.tool.workspace"),
        effect_class: agent_sdk_core::policy::EffectClass::Write,
        risk_class: agent_sdk_core::policy::RiskClass::High,
        requested_args_ref: agent_sdk_core::domain::ContentRef::new("content.args.redacted.1"),
        redacted_args_summary: "write docs/notes.md".to_string(),
        policy_refs: vec![PolicyRef::with_kind(
            PolicyKind::Approval,
            "policy.approval.workspace_write",
        )],
        dispatcher_scope: agent_sdk_core::policy::DispatcherScope::SourceScoped,
        timeout_ms: 120_000,
        allowed_decisions: vec![
            agent_sdk_core::policy::ApprovalDecisionKind::Approved,
            agent_sdk_core::policy::ApprovalDecisionKind::Denied,
        ],
        created_at_millis: 100,
        runtime_package_fingerprint: agent_sdk_core::RuntimePackageFingerprint(
            "runtime.package.fingerprint.approval".to_string(),
        ),
    }
}

fn assert_approval_effect_sequence(
    records: &[JournalRecord],
    expected_terminal_status: EffectTerminalStatus,
) {
    assert_eq!(records.len(), 2);
    match &records[0].payload {
        JournalRecordPayload::Approval(ApprovalRecord::DispatchIntent {
            effect_intent: intent,
            ..
        }) => {
            assert_eq!(intent.kind, EffectKind::ApprovalDispatch);
            assert_eq!(intent.effect_id.as_str(), "effect.approval.1");
            assert_eq!(intent.subject_ref.kind, EntityKind::ApprovalRequest);
        }
        payload => panic!("expected approval dispatch intent, got {payload:?}"),
    }
    match &records[1].payload {
        JournalRecordPayload::Approval(ApprovalRecord::DispatchResult {
            effect_result: result,
            ..
        }) => {
            assert_eq!(result.effect_id.as_str(), "effect.approval.1");
            assert_eq!(result.terminal_status, expected_terminal_status);
        }
        payload => panic!("expected approval dispatch result, got {payload:?}"),
    }
}

fn approval_journal_summary(records: &[JournalRecord]) -> Value {
    let entries = records
        .iter()
        .map(|record| match &record.payload {
            JournalRecordPayload::Approval(ApprovalRecord::DispatchIntent {
                effect_intent: intent,
                ..
            }) => json!({
                "record_kind": "approval_dispatch_intent",
                "effect_kind": intent.kind,
                "effect_id": intent.effect_id.as_str(),
                "subject_kind": intent.subject_ref.kind,
                "source_kind": intent.source.kind,
                "destination_kind": intent.destination.as_ref().map(|destination| &destination.kind),
                "policy_refs": intent.policy_refs.iter().map(|policy| policy.as_str()).collect::<Vec<_>>(),
            }),
            JournalRecordPayload::Approval(ApprovalRecord::DispatchResult {
                effect_result: result,
                lifecycle_status,
                ..
            }) => json!({
                "record_kind": "approval_dispatch_result",
                "lifecycle_status": lifecycle_status,
                "effect_id": result.effect_id.as_str(),
                "terminal_status": result.terminal_status,
                "summary": result.redacted_summary,
            }),
            payload => json!({
                "record_kind": format!("{payload:?}"),
            }),
        })
        .collect::<Vec<_>>();

    normalize_json_value(json!({
        "schema_version": 1,
        "records": entries,
    }))
}

#[derive(Default)]
struct FailOnAppendJournal {
    fail_on_result_seq: u64,
    records: Mutex<Vec<JournalRecord>>,
}

impl FailOnAppendJournal {
    fn new(fail_on_result_seq: u64) -> Self {
        Self {
            fail_on_result_seq,
            records: Mutex::new(Vec::new()),
        }
    }

    fn records(&self) -> Vec<JournalRecord> {
        self.records.lock().expect("journal records").clone()
    }
}

impl RunJournal for FailOnAppendJournal {
    fn append(&self, record: JournalRecord) -> Result<JournalCursor, AgentError> {
        if record.journal_seq == self.fail_on_result_seq && record.record_id.ends_with(".result") {
            return Err(AgentError::new(
                AgentErrorKind::JournalFailure,
                agent_sdk_core::RetryClassification::RepairNeeded,
                "forced approval dispatch result append failure",
            ));
        }
        let mut records = self.records.lock().expect("journal records");
        records.push(record);
        Ok(JournalCursor::new(format!("journal.{}", records.len())))
    }
}
