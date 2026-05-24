use std::cell::Cell;

use agent_sdk_core::{
    AgentErrorKind, AgentId, DestinationKind, DestinationRef, EffectId, EffectIntent, EffectKind,
    EffectResult, EntityKind, EntityRef, JournalRecord, JournalRecordBase, JournalRecordPayload,
    PendingSideEffect, PrivacyClass, RecoveryMarker, RunCheckpoint, RunId, RunJournal, SourceKind,
    SourceRef, append_before_effect, append_result_or_recovery,
    testing::{FakeJournalStore, read_fixture},
};
use serde_json::Value;

#[test]
fn effect_intent_record_matches_golden_fixture() {
    let record = intent_record(1);
    let actual = normalized(record);
    let expected =
        read_fixture("tests/fixtures/journal/effect-intent.json").expect("intent fixture");

    assert_eq!(actual, expected);
    assert_eq!(actual["journal_schema_version"], 1);
    assert_eq!(actual["record_kind"], "effect_intent");
    assert_eq!(actual["payload"]["type"], "effect_intent");
}

#[test]
fn checkpoint_record_is_accelerator_not_truth_and_matches_fixture() {
    let checkpoint = RunCheckpoint {
        checkpoint_id: "checkpoint.run.1".to_string(),
        run_id: RunId::new("run.journal.1"),
        checkpoint_seq: 1,
        covers_journal_seq: 1,
        loop_state: "awaiting_model".to_string(),
        turn_id: None,
        attempt_id: None,
        runtime_package_fingerprint: "runtime.package.fingerprint.test".to_string(),
        pending_side_effects: vec![pending_effect()],
        pending_approvals: Vec::new(),
        content_ref_manifest: Vec::new(),
        state_hash: "state.hash.replayed.1".to_string(),
        created_at_millis: 40,
        writer_id: "writer.fake".to_string(),
    };
    checkpoint
        .validate_against_latest_seq(1)
        .expect("checkpoint covers committed records");
    assert!(checkpoint.validate_against_latest_seq(0).is_err());

    let record = JournalRecord::checkpoint(base(2), checkpoint);
    let actual = normalized(record);
    let expected =
        read_fixture("tests/fixtures/journal/checkpoint.json").expect("checkpoint fixture");

    assert_eq!(actual, expected);
    assert_eq!(actual["payload"]["type"], "checkpoint");
}

#[test]
fn recovery_record_marks_unsafe_pending_effects() {
    let record = recovery_record(2);
    let actual = normalized(record);
    let expected = read_fixture("tests/fixtures/journal/recovery-unsafe-pending.json")
        .expect("recovery fixture");

    assert_eq!(actual, expected);
    assert_eq!(actual["record_kind"], "recovery");
    assert_eq!(
        actual["payload"]["unsafe_pending"][0]["unsafe_pending_reason"],
        "terminal result append failed after external operation"
    );
}

#[test]
fn append_before_effect_failure_prevents_execution() {
    let journal = FakeJournalStore::default();
    let executed = Cell::new(false);
    journal.fail_next_append("disk full before effect intent");

    let error = append_before_effect(&journal, intent_record(1), || executed.set(true))
        .expect_err("intent append must fail closed");

    assert_eq!(error.kind(), AgentErrorKind::JournalFailure);
    assert!(
        !executed.get(),
        "effect must not execute after intent append failure"
    );
    assert!(journal.records().is_empty());
}

#[test]
fn result_append_failure_appends_recovery_before_next_effect() {
    let journal = FakeJournalStore::default();
    append_before_effect(&journal, intent_record(1), || ()).expect("intent append");
    journal.fail_next_append("disk full after external operation");

    let cursor = append_result_or_recovery(&journal, result_record(2), recovery_record(2))
        .expect("recovery marker append");

    assert_eq!(cursor.as_str(), "journal.2");
    let records = journal.records();
    assert_eq!(records.len(), 2);
    assert_eq!(
        records[1].record_kind,
        agent_sdk_core::JournalRecordKind::Recovery
    );
    assert!(matches!(
        records[1].payload,
        JournalRecordPayload::Recovery(RecoveryMarker { .. })
    ));
}

#[test]
fn fake_journal_enforces_monotonic_sequences_for_conformance() {
    let journal = FakeJournalStore::default();
    journal.append(intent_record(1)).expect("seq 1");

    let error = journal
        .append(result_record(3))
        .expect_err("seq gap must fail contract");

    assert_eq!(error.kind(), AgentErrorKind::InvalidStateTransition);
}

fn intent_record(journal_seq: u64) -> JournalRecord {
    let mut intent = EffectIntent::new(
        EffectId::new("effect.tool.1"),
        EffectKind::ToolExecution,
        EntityRef::new(EntityKind::ToolCall, "tool.call.1"),
        source(),
        "execute workspace read tool",
    );
    intent.destination = Some(destination());
    intent.idempotency_key = Some(agent_sdk_core::IdempotencyKey::new("idem.tool.1"));

    JournalRecord::effect_intent(base(journal_seq), intent)
}

fn result_record(journal_seq: u64) -> JournalRecord {
    let mut result = EffectResult::completed(EffectId::new("effect.tool.1"), "tool completed");
    result.external_operation_id = Some("external.tool.operation.1".to_string());

    JournalRecord::effect_result(base(journal_seq), result)
}

fn recovery_record(journal_seq: u64) -> JournalRecord {
    JournalRecord::recovery(
        base(journal_seq),
        RecoveryMarker {
            unsafe_pending: vec![pending_effect()],
            recovery_reason: "terminal result append failed".to_string(),
            policy_refs: Vec::new(),
        },
    )
}

fn pending_effect() -> PendingSideEffect {
    PendingSideEffect {
        effect_id: EffectId::new("effect.tool.1"),
        intent_record_id: "journal.record.intent.1".to_string(),
        idempotency_key: Some(agent_sdk_core::IdempotencyKey::new("idem.tool.1")),
        dedupe_key: None,
        unsafe_pending_reason: "terminal result append failed after external operation".to_string(),
    }
}

fn base(journal_seq: u64) -> JournalRecordBase {
    let mut base = JournalRecordBase::new(
        journal_seq,
        format!("journal.record.{journal_seq}"),
        RunId::new("run.journal.1"),
        AgentId::new("agent.journal.1"),
        source(),
    );
    base.destination = Some(destination());
    base.timestamp_millis = 40 + journal_seq;
    base.privacy = PrivacyClass::ContentRefsOnly;
    base
}

fn source() -> SourceRef {
    SourceRef::with_kind(SourceKind::Sdk, "source.sdk.run_loop")
}

fn destination() -> DestinationRef {
    DestinationRef::with_kind(DestinationKind::Tool, "destination.tool.fake")
}

fn normalized(record: JournalRecord) -> Value {
    agent_sdk_core::testing::normalize_json_value(
        serde_json::to_value(record).expect("record JSON"),
    )
}
