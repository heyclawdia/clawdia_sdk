use agent_sdk_core::event::{
    AgentEvent, ContentCaptureMode, EVENT_SCHEMA_VERSION, EventCorrelation, EventDeliverySemantics,
    EventEnvelope, EventFamily, EventFilter, EventFrame, EventKind, EventStreamScope,
};
use agent_sdk_core::{
    AgentId, AntiEntropyScanner, CheckpointPrunePolicy, CursorCompatibility, DestinationKind,
    DestinationRef, DurableReplaySupport, EffectId, EntityKind, EntityRef, EventId,
    InMemoryCheckpointStore, InMemorySubscriptionHub, JournalRecord, JournalRecordBase,
    JournalRecordKind, JournalRecordPayload, MissingContentPolicy, OutputContentMode,
    OutputDeliveryDedupeRecord, OutputDeliveryId, OutputDeliveryIntentRecord,
    OutputDeliveryJournalBase, OutputDeliveryKind, OutputDeliveryRequest, OutputDispatchStatus,
    OutputSinkRef, PolicyKind, PolicyRef, PrivacyClass, RecoveryMarker, ReplayMode, ReplayReducer,
    ReplayRepairKind, ReplayStatus, RetentionClass, RunCheckpoint, RunId, RunJournal,
    RunLifecycleRecord, RunSubscriptionSource, SourceKind, SourceRef, TraceId,
    check_cursor_compatibility,
    domain::ContentRef as ContentRefId,
    durable_replay_support,
    ids::SpanId,
    output_delivery::OutputDeliveryPolicy,
    package::RuntimePackageFingerprint,
    testing::{FakeJournalStore, read_fixture},
};
use serde_json::{Value, json};

#[test]
fn duplicate_subscribers_are_read_only_replay_views() {
    let run_id = RunId::new("run.replay.duplicate-subscribers");
    let agent_id = AgentId::new("agent.replay");
    let journal = FakeJournalStore::default();
    journal
        .append(run_lifecycle_record(
            1,
            run_id.clone(),
            agent_id.clone(),
            "completed",
        ))
        .expect("append terminal");
    let hub = InMemorySubscriptionHub::default();
    hub.publish_all([
        run_frame(
            1,
            run_id.clone(),
            agent_id.clone(),
            EventKind::RunStarted,
            1,
        ),
        run_frame(
            2,
            run_id.clone(),
            agent_id.clone(),
            EventKind::RunCompleted,
            1,
        ),
    ])
    .expect("publish frames");

    let first = hub
        .subscribe_run(run_id.clone(), None)
        .expect("first subscriber")
        .collect::<Vec<_>>();
    let second = hub
        .subscribe_run(run_id, None)
        .expect("second subscriber")
        .collect::<Vec<_>>();

    assert_eq!(first, second);
    assert_eq!(
        journal.records().len(),
        1,
        "subscribers must not append or replay side effects"
    );
}

#[test]
fn output_dedupe_repair_completes_without_resend() {
    let run_id = RunId::new("run.replay.output-dedupe");
    let agent_id = AgentId::new("agent.replay");
    let request = output_request(run_id.clone(), agent_id.clone());
    let intent = OutputDeliveryIntentRecord::from_request(&request);
    let dedupe = OutputDeliveryDedupeRecord {
        delivery_id: intent.delivery_id.clone(),
        dedupe_key: intent.dedupe_key.clone(),
        prior_delivery_id: Some(OutputDeliveryId::new("output.delivery.prior")),
        prior_external_operation_id: Some("external.output.prior".to_string()),
        prior_terminal_status: OutputDispatchStatus::Completed,
        current_status: OutputDispatchStatus::Deduped,
        redacted_summary: "output delivery repaired from completed dedupe proof".to_string(),
        policy_refs: intent.policy_refs.clone(),
    };
    let mut reducer = ReplayReducer::new(ReplayMode::RepairReplay);
    reducer
        .apply(intent.to_journal_record(output_base(
            1,
            "journal.replay.output.intent",
            run_id.clone(),
            agent_id.clone(),
        )))
        .expect("apply intent");
    reducer
        .apply(dedupe.to_journal_record(
            output_base(2, "journal.replay.output.dedupe", run_id, agent_id),
            request.destination.clone(),
        ))
        .expect("apply dedupe");

    let result = reducer.finish().expect("repair result");

    assert_eq!(result.status, ReplayStatus::Complete);
    assert!(result.unsafe_pending_side_effects.is_empty());
    assert_eq!(result.output_delivery_repairs.len(), 1);
    assert_eq!(
        result.output_delivery_repairs[0].replay_decision,
        agent_sdk_core::ReplayRepairDecision::CompletedByDedupeProof
    );
    assert!(
        !result.output_delivery_repairs[0].resend_allowed,
        "core repair replay must not resend sink output"
    );
}

#[test]
fn non_idempotent_pending_tool_refuses_resume() {
    let pending = agent_sdk_core::PendingSideEffect {
        effect_id: EffectId::new("effect.tool.non_idempotent"),
        intent_record_id: "journal.replay.tool.intent".to_string(),
        idempotency_key: None,
        dedupe_key: None,
        unsafe_pending_reason: "terminal result append failed after non-idempotent tool execution"
            .to_string(),
    };
    let recovery = JournalRecord::recovery(
        base(
            1,
            "journal.replay.tool.recovery",
            RunId::new("run.replay.tool"),
            AgentId::new("agent.replay"),
        ),
        RecoveryMarker {
            unsafe_pending: vec![pending],
            recovery_reason: "tool result append failed".to_string(),
            policy_refs: vec![PolicyRef::with_kind(
                PolicyKind::Host,
                "policy.tool.host_repair",
            )],
        },
    );
    let mut reducer = ReplayReducer::new(ReplayMode::ResumeReplay);
    reducer.apply(recovery).expect("apply recovery");

    let result = reducer.finish().expect("resume result");

    assert_eq!(result.status, ReplayStatus::RepairNeeded);
    assert!(!result.resume_allowed);
    assert_eq!(result.unsafe_pending_side_effects.len(), 1);
    assert!(!result.unsafe_pending_side_effects[0].retry_allowed);
    assert_eq!(
        result.repair_needed[0].kind,
        ReplayRepairKind::NonIdempotentPendingSideEffect
    );
    assert_eq!(
        normalize(json!({
            "repair_kind": result.repair_needed[0].kind,
            "resume_allowed": result.resume_allowed,
            "retry_allowed": result.unsafe_pending_side_effects[0].retry_allowed,
            "status": result.status,
            "unsafe_pending_count": result.unsafe_pending_side_effects.len(),
        })),
        fixture("tests/fixtures/replay/non-idempotent-pending-tool.json")
    );
}

#[test]
fn missing_content_refs_surface_repair_needed() {
    let run_id = RunId::new("run.replay.missing-content");
    let agent_id = AgentId::new("agent.replay");
    let missing = ContentRefId::new("content.replay.missing");
    let mut reducer = ReplayReducer::new(ReplayMode::ResumeReplay)
        .with_available_content_refs([ContentRefId::new("content.replay.present")])
        .with_missing_content_policy(MissingContentPolicy::RequestHostRepair);
    reducer
        .apply(message_record(1, run_id, agent_id, vec![missing.clone()]))
        .expect("apply message");

    let result = reducer.finish().expect("resume result");

    assert_eq!(result.status, ReplayStatus::RepairNeeded);
    assert!(!result.resume_allowed);
    assert_eq!(result.missing_content_refs, vec![missing]);
    assert_eq!(
        result.repair_needed[0].kind,
        ReplayRepairKind::MissingContentRef
    );
}

#[test]
fn anti_entropy_reports_sink_repair_cursors_and_updates_internal_views_only() {
    let run_id = RunId::new("run.replay.anti-entropy");
    let agent_id = AgentId::new("agent.replay");
    let request = output_request(run_id.clone(), agent_id.clone());
    let intent = OutputDeliveryIntentRecord::from_request(&request);
    let reconciliation = agent_sdk_core::OutputDeliveryReconciliationRecord {
        delivery_id: intent.delivery_id.clone(),
        intent_record_id: "journal.replay.output.intent".to_string(),
        side_effect_kind: agent_sdk_core::EffectKind::OutputDelivery,
        idempotency_key: intent.idempotency_key.clone(),
        dedupe_key: intent.dedupe_key.clone(),
        external_operation_id: Some("external.output.unknown".to_string()),
        terminal_status: OutputDispatchStatus::ReconciliationNeeded,
        terminal_append_status: agent_sdk_core::TerminalAppendStatus::AppendFailed,
        reconciliation_adapter: Some(intent.sink_ref.clone()),
        unsafe_pending_reason: "terminal append failed after sink contact".to_string(),
        replay_decision: agent_sdk_core::ReplayRepairDecision::RequiresHostReconciliation,
        resend_allowed: false,
    };
    let records = vec![
        intent.to_journal_record(output_base(
            1,
            "journal.replay.output.intent",
            run_id.clone(),
            agent_id.clone(),
        )),
        reconciliation.to_journal_record(
            output_base(2, "journal.replay.output.reconciliation", run_id, agent_id),
            request.destination.clone(),
        ),
    ];
    let scanner = AntiEntropyScanner::default();
    let mut view = scanner.derived_view("view.output_sink_repair", None);

    let report = scanner
        .scan(&records, std::slice::from_ref(&view))
        .expect("scan");
    let normalized = normalize(serde_json::to_value(&report).expect("report JSON"));

    assert_eq!(
        normalized,
        fixture("tests/fixtures/replay/anti-entropy-sink-repair.json")
    );
    assert_eq!(report.repairs.len(), 1);
    assert_eq!(report.repairs[0].repair_from.as_str(), "journal.1");
    assert_eq!(report.repairs[0].repair_to.as_str(), "journal.2");
    assert!(report.repairs[0].host_action_required);
    assert!(!report.repairs[0].external_side_effect_compensation);

    scanner
        .repair_internal_view(&mut view, &report.repairs[0])
        .expect("repair internal cursor");
    assert_eq!(
        view.last_repaired_cursor.as_ref().unwrap().as_str(),
        "journal.2"
    );
}

#[test]
fn terminal_checkpoint_is_preserved_during_prune() {
    let store = InMemoryCheckpointStore::default();
    let run_id = RunId::new("run.replay.checkpoint");
    let agent_id = AgentId::new("agent.replay");
    let turn_checkpoint = checkpoint(
        "checkpoint.replay.turn",
        run_id.clone(),
        1,
        1,
        "awaiting_model",
    );
    let terminal = checkpoint(
        "checkpoint.replay.terminal",
        run_id.clone(),
        2,
        2,
        "terminal:completed",
    );
    store
        .save(turn_checkpoint, 2)
        .expect("save turn checkpoint");
    store
        .save(terminal.clone(), 2)
        .expect("save terminal checkpoint");
    store
        .prune(
            &run_id,
            CheckpointPrunePolicy {
                prune_covered_before: 3,
                preserve_latest_terminal: true,
            },
        )
        .expect("prune");

    let latest = store
        .load_latest(&run_id)
        .expect("load latest")
        .expect("terminal checkpoint preserved");

    assert_eq!(latest.checkpoint_id, terminal.checkpoint_id);
    assert_eq!(latest.run_id, run_id);
    assert_eq!(latest.loop_state, "terminal:completed");

    let mut reducer = ReplayReducer::new(ReplayMode::AuditReplay);
    reducer
        .apply(JournalRecord::checkpoint(
            base(3, "journal.replay.checkpoint", run_id, agent_id),
            latest,
        ))
        .expect("apply checkpoint");
    let result = reducer.finish().expect("audit replay");
    assert_eq!(
        result.latest_checkpoint.unwrap().loop_state,
        "terminal:completed"
    );
}

#[test]
fn cursor_compatibility_is_exact_and_non_run_durable_replay_requires_archive() {
    let run_id = RunId::new("run.replay.cursor");
    let agent_id = AgentId::new("agent.replay.cursor");
    let run_cursor = run_cursor(run_id.clone(), 7);
    let agent_cursor = agent_cursor(agent_id.clone(), 7);
    let terminal_filter = EventFilter::terminal_run_events()
        .compile()
        .expect("terminal filter");
    let default_filter = EventFilter::default().compile().expect("default filter");
    let filter_cursor = agent_sdk_core::EventCursor {
        scope: terminal_filter.cursor_scope(),
        event_seq: 7,
        event_id: EventId::new("event.replay.cursor.filter"),
        journal_cursor: None,
    };

    let cases = [
        (
            EventStreamScope::Run(run_id.clone()),
            Some(run_cursor.clone()),
            CursorCompatibility::Compatible,
        ),
        (
            EventStreamScope::Run(RunId::new("run.replay.other")),
            Some(run_cursor),
            CursorCompatibility::ScopeMismatch,
        ),
        (
            EventStreamScope::Agent(agent_id.clone()),
            Some(agent_cursor),
            CursorCompatibility::Compatible,
        ),
        (
            default_filter.cursor_scope(),
            Some(filter_cursor),
            CursorCompatibility::ScopeMismatch,
        ),
    ];

    for (scope, cursor, expected) in cases {
        assert_eq!(
            check_cursor_compatibility(&scope, cursor.as_ref()),
            expected,
            "scope {scope:?}"
        );
    }

    assert_eq!(
        durable_replay_support(&EventStreamScope::Run(run_id)),
        DurableReplaySupport::RunJournal
    );
    assert_eq!(
        durable_replay_support(&EventStreamScope::All),
        DurableReplaySupport::HostArchiveRequired
    );
    assert_eq!(
        durable_replay_support(&EventStreamScope::Agent(agent_id)),
        DurableReplaySupport::HostArchiveRequired
    );
}

fn output_request(run_id: RunId, agent_id: AgentId) -> OutputDeliveryRequest {
    let destination =
        DestinationRef::with_kind(DestinationKind::OutputSink, "destination.replay.output");
    let sink_ref = OutputSinkRef::new("sink.replay.output");
    let policy = OutputDeliveryPolicy::required(
        PolicyRef::with_kind(PolicyKind::Host, "policy.replay.output"),
        sink_ref.clone(),
    );
    let mut request = OutputDeliveryRequest {
        delivery_id: OutputDeliveryId::new("output.delivery.replay"),
        effect_id: EffectId::new("effect.output.delivery.replay"),
        run_id,
        agent_id,
        turn_id: None,
        attempt_id: None,
        source_message_id: Some(agent_sdk_core::MessageId::new("message.replay.output")),
        validated_output_id: None,
        destination,
        sink_ref,
        delivery_kind: OutputDeliveryKind::FinalMessage,
        content_mode: OutputContentMode::ContentRefsOnly,
        content_refs: vec![ContentRefId::new("content.replay.output")],
        redacted_summary: "replay output delivery".to_string(),
        raw_content: None,
        privacy: PrivacyClass::ContentRefsOnly,
        retention: RetentionClass::RunScoped,
        policy_refs: policy.policy_refs(),
        idempotency_key: Some(agent_sdk_core::IdempotencyKey::new(
            "idempotency.replay.output",
        )),
        dedupe_key: agent_sdk_core::DedupeKey::new("dedupe.output_delivery.pending"),
        runtime_package_fingerprint: RuntimePackageFingerprint(
            "runtime.package.replay".to_string(),
        ),
    };
    request.dedupe_key = agent_sdk_core::build_output_delivery_dedupe_key(&request);
    request
}

fn output_base(
    journal_seq: u64,
    record_id: &str,
    run_id: RunId,
    agent_id: AgentId,
) -> OutputDeliveryJournalBase {
    OutputDeliveryJournalBase {
        journal_seq,
        record_id: record_id.to_string(),
        run_id,
        agent_id,
        turn_id: None,
        attempt_id: None,
        source: SourceRef::with_kind(SourceKind::Sdk, "source.sdk.replay"),
        destination: DestinationRef::with_kind(
            DestinationKind::OutputSink,
            "destination.replay.output",
        ),
        timestamp_millis: journal_seq,
        runtime_package_fingerprint: RuntimePackageFingerprint(
            "runtime.package.replay".to_string(),
        ),
        redaction_policy_id: "policy.redaction.replay".to_string(),
    }
}

fn run_lifecycle_record(
    journal_seq: u64,
    run_id: RunId,
    agent_id: AgentId,
    status: &str,
) -> JournalRecord {
    JournalRecord::feature_record(
        base(
            journal_seq,
            &format!("journal.replay.run.{journal_seq}"),
            run_id,
            agent_id,
        ),
        JournalRecordKind::Run,
        "run",
        status,
        EntityRef::new(EntityKind::Run, "run.replay.subject"),
        Vec::new(),
        Vec::new(),
        JournalRecordPayload::RunLifecycle(RunLifecycleRecord {
            status: status.to_string(),
            reason: "replay test lifecycle".to_string(),
        }),
    )
}

fn message_record(
    journal_seq: u64,
    run_id: RunId,
    agent_id: AgentId,
    content_refs: Vec<ContentRefId>,
) -> JournalRecord {
    JournalRecord::feature_record(
        base(journal_seq, "journal.replay.message", run_id, agent_id),
        JournalRecordKind::Message,
        "message",
        "message_committed",
        EntityRef::new(EntityKind::Message, "message.replay"),
        Vec::new(),
        content_refs,
        JournalRecordPayload::Message(agent_sdk_core::MessageRecord {
            message_id: agent_sdk_core::MessageId::new("message.replay"),
            role: "assistant".to_string(),
            redacted_summary: "message replay content refs".to_string(),
        }),
    )
}

fn checkpoint(
    checkpoint_id: &str,
    run_id: RunId,
    checkpoint_seq: u64,
    covers_journal_seq: u64,
    loop_state: &str,
) -> RunCheckpoint {
    RunCheckpoint {
        checkpoint_id: checkpoint_id.to_string(),
        run_id,
        checkpoint_seq,
        covers_journal_seq,
        loop_state: loop_state.to_string(),
        turn_id: None,
        attempt_id: None,
        runtime_package_fingerprint: "runtime.package.replay".to_string(),
        pending_side_effects: Vec::new(),
        pending_approvals: Vec::new(),
        content_ref_manifest: Vec::new(),
        state_hash: format!("state.hash.{checkpoint_seq}"),
        created_at_millis: checkpoint_seq,
        writer_id: "writer.replay.test".to_string(),
    }
}

fn base(journal_seq: u64, record_id: &str, run_id: RunId, agent_id: AgentId) -> JournalRecordBase {
    let mut base = JournalRecordBase::new(
        journal_seq,
        record_id,
        run_id,
        agent_id,
        SourceRef::with_kind(SourceKind::Sdk, "source.sdk.replay"),
    );
    base.timestamp_millis = journal_seq;
    base.privacy = PrivacyClass::ContentRefsOnly;
    base
}

fn run_frame(
    event_seq: u64,
    run_id: RunId,
    agent_id: AgentId,
    kind: EventKind,
    journal_seq: u64,
) -> EventFrame {
    let event = AgentEvent::envelope_only(EventEnvelope {
        schema_version: EVENT_SCHEMA_VERSION,
        event_id: EventId::new(format!("event.replay.{event_seq}")),
        event_seq,
        event_family: EventFamily::Run,
        event_kind: kind,
        payload_schema_version: 1,
        timestamp: "2026-05-24T00:00:00Z".to_string(),
        recorded_at: "2026-05-24T00:00:00Z".to_string(),
        run_id: run_id.clone(),
        agent_id,
        turn_id: None,
        attempt_id: None,
        message_id: None,
        context_item_id: None,
        trace_id: TraceId::new(format!("trace.replay.{event_seq}")),
        span_id: SpanId::new(format!("span.replay.{event_seq}")),
        parent_event_id: None,
        caused_by: None,
        subject_ref: EntityRef::run(run_id.clone()),
        related_refs: Vec::new(),
        causal_refs: Vec::new(),
        correlation: EventCorrelation::default(),
        tags: Vec::new(),
        source: SourceRef::with_kind(SourceKind::Sdk, "source.sdk.replay"),
        destination: Some(DestinationRef::with_kind(
            DestinationKind::EventStream,
            "destination.event_stream.replay",
        )),
        policy_refs: Vec::new(),
        journal_cursor: Some(agent_sdk_core::JournalCursor::new(format!(
            "journal.{journal_seq}"
        ))),
        state_before: None,
        state_after: None,
        delivery_semantics: EventDeliverySemantics::JournalBacked,
        privacy: PrivacyClass::ContentRefsOnly,
        content_capture: ContentCaptureMode::Off,
        redaction_policy_id: "policy.redaction.replay".to_string(),
        runtime_package_fingerprint: "runtime.package.replay".to_string(),
    });
    let cursor = event.envelope.cursor(EventStreamScope::Run(run_id));
    EventFrame {
        event,
        cursor,
        archive_cursor: None,
        overflow: None,
    }
}

fn run_cursor(run_id: RunId, event_seq: u64) -> agent_sdk_core::EventCursor {
    agent_sdk_core::EventCursor {
        scope: EventStreamScope::Run(run_id),
        event_seq,
        event_id: EventId::new(format!("event.replay.cursor.run.{event_seq}")),
        journal_cursor: Some(agent_sdk_core::JournalCursor::new(format!(
            "journal.{event_seq}"
        ))),
    }
}

fn agent_cursor(agent_id: AgentId, event_seq: u64) -> agent_sdk_core::EventCursor {
    agent_sdk_core::EventCursor {
        scope: EventStreamScope::Agent(agent_id),
        event_seq,
        event_id: EventId::new(format!("event.replay.cursor.agent.{event_seq}")),
        journal_cursor: None,
    }
}

fn fixture(path: &str) -> Value {
    read_fixture(path).expect("fixture reads")
}

fn normalize(value: Value) -> Value {
    agent_sdk_core::testing::normalize_json_value(value)
}
