use std::sync::{Arc, Mutex};

use agent_sdk_core::{
    AgentError, AgentErrorKind, AgentId, DestinationKind, DestinationRef, JournalCursor,
    JournalRecord, JournalRecordPayload, PolicyKind, PolicyRef, PrivacyClass, RetryClassification,
    RunId, RunJournal, SourceKind, SourceRef,
    domain::ContentRef as ContentRefId,
    output_delivery::{
        OutputContentMode, OutputDeliveryEventKind, OutputDeliveryEventRecord, OutputDeliveryKind,
        OutputDeliveryPolicy, OutputDeliveryRequirement, OutputDispatchStatus, OutputSinkRef,
        RawOutputContentPolicy, ReplayRepairDecision, TerminalAppendStatus,
    },
    output_delivery_port::{OutputSinkCapabilities, OutputSinkRegistry},
    output_delivery_service::{
        OutputDedupeProof, OutputDeliveryCandidate, OutputDeliveryContext,
        OutputDeliveryDedupeIndex, OutputDeliveryService,
    },
    testing::FakeJournalStore,
    testing::ScriptedOutputSink,
};
use serde::Serialize;
use serde_json::Value;

#[test]
fn final_delivery_appends_intent_before_sink_and_result_after_sink() {
    let sink_ref = OutputSinkRef::new("sink.output.final");
    let sink = ScriptedOutputSink::new(
        sink_ref.clone(),
        OutputSinkCapabilities::refs_and_summaries(sink_ref.clone()),
    );
    let mut registry = OutputSinkRegistry::new();
    registry.register(sink.clone()).expect("sink registers");
    let journal = FakeJournalStore::default();
    let service = OutputDeliveryService::new(Arc::new(journal.clone()), registry);

    let outcome = service
        .dispatch(context("run.output.final"), final_candidate(sink_ref))
        .expect("delivery dispatch");

    assert_eq!(outcome.status, OutputDispatchStatus::Completed);
    assert_eq!(sink.calls().len(), 1);
    let records = journal.records();
    assert_eq!(records.len(), 2, "intent and result records are journaled");
    assert!(
        matches!(
            records[0].payload,
            JournalRecordPayload::OutputDelivery(agent_sdk_core::OutputDeliveryRecord::Intent(_))
        ),
        "intent must be durable before the sink call"
    );
    assert!(matches!(
        records[1].payload,
        JournalRecordPayload::OutputDelivery(agent_sdk_core::OutputDeliveryRecord::Result(_))
    ));
    assert_eq!(records[0].event_index.event_family, "output_delivery");
    assert_eq!(
        records[0].event_index.event_kind,
        "output_dispatch_requested"
    );
    assert_eq!(
        records[1].event_index.event_kind,
        "output_dispatch_completed"
    );
    assert_eq!(
        outcome.request.as_ref().expect("request").content_mode,
        OutputContentMode::ContentRefsOnly
    );
}

#[test]
fn chunk_and_final_delivery_share_policy_dedupe_and_sink_port() {
    let sink_ref = OutputSinkRef::new("sink.output.chunk");
    let sink = ScriptedOutputSink::new(
        sink_ref.clone(),
        OutputSinkCapabilities::refs_and_summaries(sink_ref.clone()),
    );
    let mut registry = OutputSinkRegistry::new();
    registry.register(sink.clone()).expect("sink registers");
    let journal = FakeJournalStore::default();
    let service = OutputDeliveryService::new(Arc::new(journal.clone()), registry);

    let final_outcome = service
        .dispatch(
            context("run.output.final-shared"),
            final_candidate(sink_ref.clone()),
        )
        .expect("final dispatch");
    let mut chunk = final_candidate(sink_ref);
    chunk.delivery_kind = OutputDeliveryKind::StreamChunk {
        stream_cursor: "stream.cursor.1".to_string(),
        chunk_index: 7,
    };
    chunk.redacted_summary = "stream chunk ready for delivery".to_string();
    let chunk_outcome = service
        .dispatch(context("run.output.chunk-shared"), chunk)
        .expect("chunk dispatch");

    assert_eq!(sink.calls().len(), 2);
    assert_ne!(
        final_outcome.request.as_ref().unwrap().dedupe_key,
        chunk_outcome.request.as_ref().unwrap().dedupe_key,
        "final and chunk dedupe keys must be distinct but built by the same path"
    );
}

#[test]
fn required_missing_sink_records_host_configuration_needed_without_send() {
    let sink_ref = OutputSinkRef::new("sink.output.missing");
    let journal = FakeJournalStore::default();
    let service = OutputDeliveryService::new(Arc::new(journal.clone()), OutputSinkRegistry::new());

    let outcome = service
        .dispatch(context("run.output.missing"), final_candidate(sink_ref))
        .expect("missing required sink is recorded");

    let error = outcome
        .terminal_error
        .as_ref()
        .expect("host configuration error");
    assert_eq!(error.kind(), AgentErrorKind::HostConfigurationNeeded);
    assert_eq!(error.retry(), RetryClassification::HostConfigurationNeeded);
    assert_eq!(
        outcome.status,
        OutputDispatchStatus::HostConfigurationNeeded
    );
    assert_eq!(journal.records().len(), 2);
    assert_eq!(
        outcome.result_record.unwrap().dispatch_status,
        OutputDispatchStatus::HostConfigurationNeeded
    );
}

#[test]
fn optional_missing_sink_skips_without_intent_or_failure() {
    let mut candidate = final_candidate(OutputSinkRef::new("sink.output.optional-missing"));
    candidate.policy = OutputDeliveryPolicy::optional(PolicyRef::with_kind(
        PolicyKind::Host,
        "policy.output.optional",
    ));
    candidate.preferred_sink_ref = Some(OutputSinkRef::new("sink.output.absent"));
    let journal = FakeJournalStore::default();
    let service = OutputDeliveryService::new(Arc::new(journal.clone()), OutputSinkRegistry::new());

    let outcome = service
        .dispatch(context("run.output.optional-missing"), candidate)
        .expect("optional missing sink skips");

    assert_eq!(outcome.status, OutputDispatchStatus::SkippedOptional);
    assert!(outcome.terminal_error.is_none());
    assert!(journal.records().is_empty());
}

#[test]
fn sink_capability_mismatch_for_required_delivery_is_host_configuration_needed() {
    let sink_ref = OutputSinkRef::new("sink.output.no-refs");
    let mut capabilities = OutputSinkCapabilities::refs_and_summaries(sink_ref.clone());
    capabilities.can_resolve_content_refs = false;
    let sink = ScriptedOutputSink::new(sink_ref.clone(), capabilities);
    let mut registry = OutputSinkRegistry::new();
    registry.register(sink.clone()).expect("sink registers");
    let journal = FakeJournalStore::default();
    let service = OutputDeliveryService::new(Arc::new(journal.clone()), registry);
    let mut candidate = final_candidate(sink_ref);
    candidate.policy.allowed_content_modes = vec![OutputContentMode::ContentRefsOnly];

    let outcome = service
        .dispatch(context("run.output.no-refs"), candidate)
        .expect("capability mismatch records host configuration");

    assert_eq!(sink.calls().len(), 0);
    assert_eq!(
        outcome.status,
        OutputDispatchStatus::HostConfigurationNeeded
    );
    assert_eq!(
        outcome.terminal_error.as_ref().unwrap().kind(),
        AgentErrorKind::HostConfigurationNeeded
    );
    assert_eq!(journal.records().len(), 2);
}

#[test]
fn journal_append_failure_blocks_sink_call() {
    let sink_ref = OutputSinkRef::new("sink.output.journal-fail");
    let sink = ScriptedOutputSink::new(
        sink_ref.clone(),
        OutputSinkCapabilities::refs_and_summaries(sink_ref.clone()),
    );
    let mut registry = OutputSinkRegistry::new();
    registry.register(sink.clone()).expect("sink registers");
    let journal = FakeJournalStore::default();
    journal.fail_next_append("journal unavailable before output send");
    let service = OutputDeliveryService::new(Arc::new(journal), registry);

    let error = service
        .dispatch(
            context("run.output.journal-fail"),
            final_candidate(sink_ref),
        )
        .expect_err("journal append failure fails closed");

    assert_eq!(error.kind(), AgentErrorKind::JournalFailure);
    assert_eq!(sink.calls().len(), 0);
}

#[test]
fn terminal_result_append_failure_after_sink_enters_reconciliation() {
    let sink_ref = OutputSinkRef::new("sink.output.result-fail");
    let sink = ScriptedOutputSink::new(
        sink_ref.clone(),
        OutputSinkCapabilities::refs_and_summaries(sink_ref.clone()),
    );
    let mut registry = OutputSinkRegistry::new();
    registry.register(sink.clone()).expect("sink registers");
    let journal = Arc::new(FailOutputResultJournal::default());
    let service = OutputDeliveryService::new(journal.clone(), registry);

    let outcome = service
        .dispatch(context("run.output.result-fail"), final_candidate(sink_ref))
        .expect("result append failure records reconciliation");

    assert_eq!(sink.calls().len(), 1);
    assert_eq!(outcome.status, OutputDispatchStatus::ReconciliationNeeded);
    assert_eq!(
        outcome
            .terminal_error
            .as_ref()
            .expect("recovery error")
            .kind(),
        AgentErrorKind::RecoveryRepairNeeded
    );
    let reconciliation = outcome
        .reconciliation_record
        .as_ref()
        .expect("reconciliation record");
    assert_eq!(
        reconciliation.terminal_append_status,
        TerminalAppendStatus::AppendFailed
    );
    assert!(!reconciliation.resend_allowed);
    assert_eq!(journal.records().len(), 2);
    assert!(matches!(
        journal.records()[1].payload,
        JournalRecordPayload::OutputDelivery(agent_sdk_core::OutputDeliveryRecord::Reconciliation(
            _
        ))
    ));
}

#[test]
fn dedupe_completed_delivery_does_not_call_sink_again() {
    let sink_ref = OutputSinkRef::new("sink.output.dedupe");
    let sink = ScriptedOutputSink::new(
        sink_ref.clone(),
        OutputSinkCapabilities::refs_and_summaries(sink_ref.clone()),
    );
    let mut registry = OutputSinkRegistry::new();
    registry.register(sink.clone()).expect("sink registers");
    let journal = FakeJournalStore::default();
    let dedupe_index = OutputDeliveryDedupeIndex::default();
    let service = OutputDeliveryService::new(Arc::new(journal.clone()), registry)
        .with_dedupe_index(dedupe_index.clone());
    let candidate = final_candidate(sink_ref);

    let first = service
        .dispatch(context("run.output.dedupe"), candidate.clone())
        .expect("first dispatch");
    let proof = OutputDedupeProof {
        dedupe_key: first.request.as_ref().unwrap().dedupe_key.clone(),
        delivery_id: first.request.as_ref().unwrap().delivery_id.clone(),
        external_operation_id: Some("external.output.1".to_string()),
        status: OutputDispatchStatus::Completed,
    };
    dedupe_index
        .insert_completed(proof)
        .expect("dedupe proof records");
    let second = service
        .dispatch(context("run.output.dedupe"), candidate)
        .expect("second dispatch dedupes");

    assert_eq!(sink.calls().len(), 1);
    assert_eq!(second.status, OutputDispatchStatus::Deduped);
    assert!(second.dedupe_record.is_some());
    let records = journal.records();
    assert_eq!(
        records.len(),
        3,
        "first dispatch journals intent/result and dedupe journals skipped-send fact"
    );
    assert!(matches!(
        records[2].payload,
        JournalRecordPayload::OutputDelivery(agent_sdk_core::OutputDeliveryRecord::Dedupe(_))
    ));
    assert_eq!(records[2].event_index.event_kind, "output_dispatch_deduped");
}

#[test]
fn repair_replay_never_resends_without_completed_dedupe_proof() {
    let sink_ref = OutputSinkRef::new("sink.output.replay");
    let sink = ScriptedOutputSink::new(
        sink_ref.clone(),
        OutputSinkCapabilities::refs_and_summaries(sink_ref.clone()),
    );
    let mut registry = OutputSinkRegistry::new();
    registry.register(sink.clone()).expect("sink registers");
    let journal = FakeJournalStore::default();
    let service = OutputDeliveryService::new(Arc::new(journal.clone()), registry);
    let first = service
        .dispatch(context("run.output.replay"), final_candidate(sink_ref))
        .expect("dispatch");
    let intent = first.intent_record.as_ref().expect("intent record");
    let empty_repair_service = OutputDeliveryService::new(
        Arc::new(FakeJournalStore::default()),
        OutputSinkRegistry::new(),
    );

    let repair = empty_repair_service
        .repair_replay(intent, None)
        .expect("repair replay classifies pending delivery");

    assert_eq!(sink.calls().len(), 1, "repair replay must not send again");
    assert!(!repair.resend_allowed);
    assert_eq!(repair.replay_decision, ReplayRepairDecision::UnsafePending);
    assert_eq!(
        repair.unsafe_pending_reason,
        "repair replay cannot resend output delivery without completed dedupe proof"
    );
}

#[test]
fn repair_replay_uses_dedupe_proof_without_resending() {
    let sink_ref = OutputSinkRef::new("sink.output.replay-proof");
    let sink = ScriptedOutputSink::new(
        sink_ref.clone(),
        OutputSinkCapabilities::refs_and_summaries(sink_ref.clone()),
    );
    let mut registry = OutputSinkRegistry::new();
    registry.register(sink.clone()).expect("sink registers");
    let journal = FakeJournalStore::default();
    let dedupe_index = OutputDeliveryDedupeIndex::default();
    let service = OutputDeliveryService::new(Arc::new(journal), registry)
        .with_dedupe_index(dedupe_index.clone());
    let first = service
        .dispatch(
            context("run.output.replay-proof"),
            final_candidate(sink_ref),
        )
        .expect("dispatch");
    let request = first.request.as_ref().expect("request");
    dedupe_index
        .insert_completed(OutputDedupeProof {
            dedupe_key: request.dedupe_key.clone(),
            delivery_id: request.delivery_id.clone(),
            external_operation_id: Some("external.output.replay-proof".to_string()),
            status: OutputDispatchStatus::Completed,
        })
        .expect("proof inserted");

    let repair = service
        .repair_replay(first.intent_record.as_ref().unwrap(), None)
        .expect("repair replay");

    assert_eq!(sink.calls().len(), 1);
    assert!(!repair.resend_allowed);
    assert_eq!(
        repair.replay_decision,
        ReplayRepairDecision::CompletedByDedupeProof
    );
}

#[test]
fn raw_content_denied_by_policy_is_downgraded_to_redacted_summary() {
    let sink_ref = OutputSinkRef::new("sink.output.raw-denied");
    let sink = ScriptedOutputSink::new(
        sink_ref.clone(),
        OutputSinkCapabilities::refs_and_summaries(sink_ref.clone()).with_raw_content(),
    );
    let mut registry = OutputSinkRegistry::new();
    registry.register(sink.clone()).expect("sink registers");
    let journal = FakeJournalStore::default();
    let service = OutputDeliveryService::new(Arc::new(journal.clone()), registry);
    let mut candidate = final_candidate(sink_ref);
    candidate.requested_content_mode = Some(OutputContentMode::RawContentIfPolicyAllows);
    candidate.raw_content = Some("secret raw model output".to_string());
    candidate.policy.allowed_content_modes = vec![
        OutputContentMode::RawContentIfPolicyAllows,
        OutputContentMode::RedactedSummary,
    ];

    let outcome = service
        .dispatch(context("run.output.raw-denied"), candidate)
        .expect("raw denied downgrades");

    let request = outcome.request.as_ref().expect("request");
    assert_eq!(request.content_mode, OutputContentMode::RedactedSummary);
    assert!(!request.carries_raw_content());
    assert!(!sink.calls()[0].carries_raw_content());
}

#[test]
fn raw_content_requires_policy_and_sink_capability() {
    let sink_ref = OutputSinkRef::new("sink.output.raw-allowed");
    let sink = ScriptedOutputSink::new(
        sink_ref.clone(),
        OutputSinkCapabilities::refs_and_summaries(sink_ref.clone()).with_raw_content(),
    );
    let mut registry = OutputSinkRegistry::new();
    registry.register(sink.clone()).expect("sink registers");
    let journal = FakeJournalStore::default();
    let service = OutputDeliveryService::new(Arc::new(journal.clone()), registry);
    let mut candidate = final_candidate(sink_ref.clone());
    candidate.requested_content_mode = Some(OutputContentMode::RawContentIfPolicyAllows);
    candidate.raw_content = Some("explicitly allowed raw output".to_string());
    candidate.policy.allowed_content_modes = vec![OutputContentMode::RawContentIfPolicyAllows];
    candidate.policy.raw_content_policy = RawOutputContentPolicy::allow_for_sink(
        PolicyRef::with_kind(PolicyKind::Privacy, "policy.raw.output.allowed"),
        sink_ref,
        128,
    );

    let outcome = service
        .dispatch(context("run.output.raw-allowed"), candidate)
        .expect("raw allowed dispatch");

    assert_eq!(
        outcome.request.as_ref().unwrap().content_mode,
        OutputContentMode::RawContentIfPolicyAllows
    );
    assert!(sink.calls()[0].carries_raw_content());
    assert!(journal_payloads_have_no_raw_content(
        &journal.normalized_records()
    ));
}

#[test]
fn output_delivery_golden_fixtures_match_records_and_event_payloads() {
    let sink_ref = OutputSinkRef::new("sink.output.fixture");
    let sink = ScriptedOutputSink::new(
        sink_ref.clone(),
        OutputSinkCapabilities::refs_and_summaries(sink_ref.clone()),
    );
    let mut registry = OutputSinkRegistry::new();
    registry.register(sink).expect("sink registers");
    let journal = FakeJournalStore::default();
    let service = OutputDeliveryService::new(Arc::new(journal), registry);
    let outcome = service
        .dispatch(context("run.output.fixture"), final_candidate(sink_ref))
        .expect("fixture dispatch");
    let intent = outcome.intent_record.as_ref().expect("intent");
    let result = outcome.result_record.as_ref().expect("result");
    let dedupe = agent_sdk_core::output_delivery::OutputDeliveryDedupeRecord {
        delivery_id: intent.delivery_id.clone(),
        dedupe_key: intent.dedupe_key.clone(),
        prior_delivery_id: Some(intent.delivery_id.clone()),
        prior_external_operation_id: Some("external.fixture".to_string()),
        prior_terminal_status: OutputDispatchStatus::Completed,
        current_status: OutputDispatchStatus::Deduped,
        redacted_summary: "output delivery skipped by completed dedupe proof".to_string(),
        policy_refs: intent.policy_refs.clone(),
    };
    let reconciliation = service
        .with_dedupe_index(OutputDeliveryDedupeIndex::default())
        .repair_replay(intent, None)
        .expect("reconciliation fixture");

    assert_fixture(
        "tests/fixtures/output_delivery/journal/output_delivery_intent_v1.json",
        intent,
    );
    assert_fixture(
        "tests/fixtures/output_delivery/journal/output_delivery_result_v1.json",
        result,
    );
    assert_fixture(
        "tests/fixtures/output_delivery/journal/output_delivery_dedupe_v1.json",
        &dedupe,
    );
    assert_fixture(
        "tests/fixtures/output_delivery/journal/output_delivery_reconciliation_v1.json",
        &reconciliation,
    );

    assert_fixture(
        "tests/fixtures/output_delivery/events/output_delivery_requested_v1.json",
        &event_payload(
            OutputDeliveryEventKind::OutputDispatchRequested,
            intent,
            OutputDispatchStatus::Requested,
            None,
            None,
        ),
    );
    assert_fixture(
        "tests/fixtures/output_delivery/events/output_delivery_completed_v1.json",
        &event_payload(
            OutputDeliveryEventKind::OutputDispatchCompleted,
            intent,
            OutputDispatchStatus::Completed,
            result.ack_ref.clone(),
            None,
        ),
    );
    assert_fixture(
        "tests/fixtures/output_delivery/events/output_delivery_failed_v1.json",
        &event_payload(
            OutputDeliveryEventKind::OutputDispatchFailed,
            intent,
            OutputDispatchStatus::HostConfigurationNeeded,
            None,
            Some(ReplayRepairDecision::RequiresHostReconciliation),
        ),
    );
    assert_fixture(
        "tests/fixtures/output_delivery/events/output_delivery_deduped_v1.json",
        &event_payload(
            OutputDeliveryEventKind::OutputDispatchDeduped,
            intent,
            OutputDispatchStatus::Deduped,
            None,
            Some(ReplayRepairDecision::CompletedByDedupeProof),
        ),
    );
}

fn final_candidate(sink_ref: OutputSinkRef) -> OutputDeliveryCandidate {
    OutputDeliveryCandidate {
        destination: DestinationRef::with_kind(
            DestinationKind::OutputSink,
            "destination.output.chat",
        ),
        preferred_sink_ref: Some(sink_ref.clone()),
        delivery_kind: OutputDeliveryKind::FinalMessage,
        source_message_id: Some(agent_sdk_core::MessageId::new("message.output.final")),
        validated_output_id: None,
        content_refs: vec![ContentRefId::new("content.ref.output.final.v1")],
        redacted_summary: "final assistant message ready for output delivery".to_string(),
        raw_content: None,
        requested_content_mode: Some(OutputContentMode::ContentRefsOnly),
        privacy: PrivacyClass::ContentRefsOnly,
        retention: agent_sdk_core::RetentionClass::RunScoped,
        policy: OutputDeliveryPolicy {
            policy_ref: PolicyRef::with_kind(PolicyKind::Host, "policy.output.delivery.required"),
            requirement: OutputDeliveryRequirement::Required,
            default_content_mode: OutputContentMode::ContentRefsOnly,
            allowed_content_modes: vec![
                OutputContentMode::ContentRefsOnly,
                OutputContentMode::RedactedSummary,
            ],
            required_sink_ref: Some(sink_ref),
            retry_policy_ref: Some(PolicyRef::with_kind(
                PolicyKind::Host,
                "policy.output.delivery.retry.host-owned",
            )),
            reconciliation_policy_ref: Some(PolicyRef::with_kind(
                PolicyKind::Host,
                "policy.output.delivery.reconcile.host-owned",
            )),
            raw_content_policy: RawOutputContentPolicy::deny(),
        },
    }
}

fn context(run_id: &str) -> OutputDeliveryContext {
    let mut context = OutputDeliveryContext::new(
        RunId::new(run_id),
        AgentId::new("agent.output.delivery"),
        agent_sdk_core::RuntimePackageFingerprint("runtime.package.output.delivery.v1".to_string()),
    );
    context.turn_id = Some(agent_sdk_core::TurnId::new("turn.output.delivery"));
    context.attempt_id = Some(agent_sdk_core::AttemptId::new("attempt.output.delivery"));
    context.source = SourceRef::with_kind(SourceKind::Sdk, "source.sdk.output_delivery");
    context
}

fn event_payload(
    event_kind: OutputDeliveryEventKind,
    intent: &agent_sdk_core::output_delivery::OutputDeliveryIntentRecord,
    dispatch_status: OutputDispatchStatus,
    ack_ref: Option<String>,
    reconciliation_status: Option<ReplayRepairDecision>,
) -> OutputDeliveryEventRecord {
    OutputDeliveryEventRecord {
        event_kind,
        delivery_id: intent.delivery_id.clone(),
        destination: intent.destination.clone(),
        sink_ref: intent.sink_ref.clone(),
        dedupe_key: intent.dedupe_key.clone(),
        source_message_id: Some(agent_sdk_core::MessageId::new("message.output.final")),
        dispatch_status,
        ack_ref,
        reconciliation_status,
        redacted_summary: "output delivery event contains refs and bounded summary only"
            .to_string(),
    }
}

fn assert_fixture<T>(path: &str, value: &T)
where
    T: Serialize,
{
    let normalized = normalize(serde_json::to_value(value).expect("record JSON"));
    assert_eq!(normalized, fixture(path), "fixture mismatch at {path}");
}

fn fixture(path: &str) -> Value {
    agent_sdk_core::testing::read_fixture(path).expect("fixture reads")
}

fn normalize(value: Value) -> Value {
    agent_sdk_core::testing::normalize_json_value(value)
}

fn journal_payloads_have_no_raw_content(records: &[Value]) -> bool {
    !records
        .iter()
        .any(|record| record.to_string().contains("explicitly allowed raw output"))
}

#[derive(Default)]
struct FailOutputResultJournal {
    records: Mutex<Vec<JournalRecord>>,
}

impl FailOutputResultJournal {
    fn records(&self) -> Vec<JournalRecord> {
        self.records.lock().expect("output journal lock").clone()
    }
}

impl RunJournal for FailOutputResultJournal {
    fn append(&self, record: JournalRecord) -> Result<JournalCursor, AgentError> {
        if record.record_id.ends_with(".result") {
            return Err(AgentError::new(
                AgentErrorKind::JournalFailure,
                RetryClassification::RepairNeeded,
                "injected output result append failure",
            ));
        }
        let mut records = self.records.lock().expect("output journal lock");
        records.push(record);
        Ok(JournalCursor::new(format!("journal.{}", records.len())))
    }
}
