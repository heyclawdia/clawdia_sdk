use std::{collections::BTreeSet, sync::Arc, time::Duration};

use agent_sdk_core::event::{
    AgentEvent, ContentCaptureMode, EVENT_SCHEMA_VERSION, EventCorrelation, EventDeliverySemantics,
    EventEnvelope, EventFamily, EventFilter, EventFrame, EventKind, EventStreamScope, EventTag,
};
use agent_sdk_core::{
    AgentErrorKind, AgentId, DestinationKind, DestinationRef, EffectId, EntityRef,
    EventIndexProjection, InMemoryRunControlStore, InMemorySubscriptionHub, JournalRecord,
    JournalRecordKind, JournalRecordPayload, PrivacyClass, RunHandle, RunId, RunJournal, RunResult,
    RunStatus, RunSubscriptionSource, SourceKind, SourceRef, TraceId,
    ids::SpanId,
    journal::{JOURNAL_SCHEMA_VERSION, TerminalResultMarker},
    testing::FakeJournalStore,
};
use agent_sdk_core::{EventCursor, EventId, JournalCursor};

struct Fixture {
    run_id: RunId,
    agent_id: AgentId,
    control: InMemoryRunControlStore,
    hub: InMemorySubscriptionHub,
    journal: FakeJournalStore,
    handle: RunHandle,
}

impl Fixture {
    fn new() -> Self {
        let run_id = RunId::new("run.handle.contract");
        let agent_id = AgentId::new("agent.handle.contract");
        let control = InMemoryRunControlStore::default();
        control
            .register_run(run_id.clone(), agent_id.clone())
            .expect("register run");
        let hub = InMemorySubscriptionHub::default();
        let handle = RunHandle::new(
            run_id.clone(),
            Arc::new(control.clone()),
            Arc::new(hub.clone()),
        );

        Self {
            run_id,
            agent_id,
            control,
            hub,
            journal: FakeJournalStore::default(),
            handle,
        }
    }

    fn seal_completed(&self, journal_seq: u64, output: &str) -> RunResult {
        let record = terminal_record(
            journal_seq,
            self.run_id.clone(),
            self.agent_id.clone(),
            RunStatus::Completed,
        );
        self.journal
            .append(record.clone())
            .expect("append terminal");
        self.control
            .seal_terminal_result_from_journal(&record, output)
            .expect("seal terminal")
    }
}

#[test]
fn run_handle_wait_is_idempotent_and_matches_terminal_event_and_journal() {
    let fixture = Fixture::new();
    fixture.seal_completed(1, "sealed output");
    fixture
        .hub
        .publish(terminal_frame(
            2,
            fixture.run_id.clone(),
            fixture.agent_id.clone(),
            EventKind::RunCompleted,
            1,
            EventStreamScope::Run(fixture.run_id.clone()),
        ))
        .expect("publish terminal");

    let first = fixture.handle.wait().expect("wait terminal");
    let second = fixture.handle.wait().expect("wait terminal again");

    assert_eq!(first, second);
    assert_eq!(first.status, RunStatus::Completed);
    assert_eq!(first.output, "sealed output");
    assert_eq!(fixture.handle.status().unwrap(), RunStatus::Completed);
}

#[test]
fn run_handle_cancel_is_idempotent() {
    let fixture = Fixture::new();

    fixture.handle.cancel().expect("first cancel");
    fixture
        .handle
        .cancel()
        .expect("second cancel is idempotent");

    assert_eq!(
        fixture
            .control
            .cancel_request_count(&fixture.run_id)
            .unwrap(),
        1
    );
    assert_eq!(fixture.handle.status().unwrap(), RunStatus::Cancelling);
}

#[test]
fn subscriber_drop_and_resubscribe_from_cursor_catches_up_without_duplicates() {
    let fixture = Fixture::new();
    let start = lifecycle_frame(
        1,
        fixture.run_id.clone(),
        fixture.agent_id.clone(),
        EventKind::RunStarted,
        Some(1),
        EventStreamScope::Run(fixture.run_id.clone()),
    );
    let delta = lifecycle_frame(
        2,
        fixture.run_id.clone(),
        fixture.agent_id.clone(),
        EventKind::ModelStreamDelta,
        Some(2),
        EventStreamScope::Run(fixture.run_id.clone()),
    );
    let terminal = terminal_frame(
        3,
        fixture.run_id.clone(),
        fixture.agent_id.clone(),
        EventKind::RunCompleted,
        3,
        EventStreamScope::Run(fixture.run_id.clone()),
    );
    fixture
        .hub
        .publish_all([start.clone(), delta.clone(), terminal.clone()])
        .unwrap();
    fixture.hub.expire_live_before(3).unwrap();

    let replayed = fixture
        .handle
        .stream_from(Some(start.cursor))
        .expect("resume from cursor")
        .collect::<Vec<_>>();

    assert_eq!(replayed.len(), 2);
    assert_eq!(
        replayed[0].event.envelope.delivery_semantics,
        EventDeliverySemantics::DerivedReplay
    );
    assert_eq!(
        replayed[0].event.envelope.event_id,
        delta.event.envelope.event_id
    );
    assert_eq!(
        replayed[1].event.envelope.delivery_semantics,
        EventDeliverySemantics::JournalBacked
    );
    assert_eq!(
        replayed[1].event.envelope.event_id,
        terminal.event.envelope.event_id
    );
    assert_eq!(
        replayed
            .iter()
            .map(|frame| frame.event.envelope.event_id.as_str().to_string())
            .collect::<BTreeSet<_>>()
            .len(),
        replayed.len()
    );
}

#[test]
fn cursor_scope_mismatch_is_rejected_without_widening_or_narrowing() {
    let fixture = Fixture::new();
    let run_cursor = EventCursor {
        scope: EventStreamScope::Run(fixture.run_id.clone()),
        event_seq: 1,
        event_id: EventId::new("event.cursor.run"),
        journal_cursor: None,
    };
    let all_cursor = EventCursor {
        scope: EventStreamScope::All,
        event_seq: 1,
        event_id: EventId::new("event.cursor.all"),
        journal_cursor: None,
    };

    assert!(fixture.hub.subscribe_all(Some(run_cursor.clone())).is_err());
    assert!(fixture.handle.stream_from(Some(all_cursor)).is_err());
    assert!(
        fixture
            .hub
            .subscribe_agent(fixture.agent_id.clone(), Some(run_cursor.clone()))
            .is_err()
    );

    let first_filter = EventFilter::terminal_run_events()
        .compile()
        .expect("filter compiles");
    let second_filter = EventFilter::default().compile().expect("filter compiles");
    let filter_cursor = EventCursor {
        scope: first_filter.cursor_scope(),
        event_seq: 1,
        event_id: EventId::new("event.cursor.filter"),
        journal_cursor: None,
    };
    assert!(
        fixture
            .hub
            .subscribe_events(second_filter, Some(filter_cursor))
            .is_err()
    );
}

#[test]
fn expired_event_cursor_without_journal_cursor_returns_gap_diagnostic() {
    let fixture = Fixture::new();
    let start = lifecycle_frame(
        1,
        fixture.run_id.clone(),
        fixture.agent_id.clone(),
        EventKind::RunStarted,
        None,
        EventStreamScope::Run(fixture.run_id.clone()),
    );
    fixture.hub.publish(start.clone()).unwrap();
    fixture.hub.expire_live_before(2).unwrap();

    let frames = fixture
        .handle
        .stream_from(Some(start.cursor))
        .expect("diagnostic stream")
        .collect::<Vec<_>>();

    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].event.envelope.event_kind, EventKind::ReplayFailed);
    assert_eq!(
        frames[0].event.envelope.delivery_semantics,
        EventDeliverySemantics::DiagnosticOnly
    );
}

#[test]
fn stream_from_journal_uses_derived_replay_until_live_tail() {
    let fixture = Fixture::new();
    fixture
        .hub
        .publish_all([
            lifecycle_frame(
                1,
                fixture.run_id.clone(),
                fixture.agent_id.clone(),
                EventKind::RunStarted,
                Some(1),
                EventStreamScope::Run(fixture.run_id.clone()),
            ),
            lifecycle_frame(
                2,
                fixture.run_id.clone(),
                fixture.agent_id.clone(),
                EventKind::ModelStreamDelta,
                Some(2),
                EventStreamScope::Run(fixture.run_id.clone()),
            ),
            terminal_frame(
                3,
                fixture.run_id.clone(),
                fixture.agent_id.clone(),
                EventKind::RunCompleted,
                3,
                EventStreamScope::Run(fixture.run_id.clone()),
            ),
        ])
        .unwrap();

    let replayed = fixture
        .handle
        .stream_from_journal(JournalCursor::new("journal.1"))
        .expect("journal replay")
        .collect::<Vec<_>>();

    assert_eq!(replayed.len(), 2);
    assert!(replayed.iter().all(|frame| {
        frame.event.envelope.delivery_semantics == EventDeliverySemantics::DerivedReplay
    }));
}

#[test]
fn wait_with_timeout_does_not_cancel_run_or_visible_output_shell() {
    let fixture = Fixture::new();
    fixture
        .control
        .mark_visible_output_complete(&fixture.run_id, "visible text")
        .expect("visible output");

    let result = fixture
        .handle
        .wait_with_timeout(Duration::from_millis(1))
        .expect("timeout wait");

    assert_eq!(result, None);
    assert_eq!(fixture.handle.status().unwrap(), RunStatus::Running);
    assert_eq!(
        fixture.control.visible_output(&fixture.run_id).unwrap(),
        Some("visible text".to_string())
    );
    assert_eq!(
        fixture
            .control
            .cancel_request_count(&fixture.run_id)
            .unwrap(),
        0
    );
}

#[test]
fn status_is_idempotent_across_duplicate_calls() {
    let fixture = Fixture::new();

    let first = fixture.handle.status().expect("status");
    let second = fixture.handle.status().expect("status again");

    assert_eq!(first, second);
    assert_eq!(first, RunStatus::Running);
}

#[test]
fn duplicate_subscribers_do_not_duplicate_side_effects_or_journal_records() {
    let fixture = Fixture::new();
    let terminal = terminal_record(
        1,
        fixture.run_id.clone(),
        fixture.agent_id.clone(),
        RunStatus::Completed,
    );
    fixture.journal.append(terminal).expect("append once");
    fixture
        .hub
        .publish_all([
            lifecycle_frame(
                1,
                fixture.run_id.clone(),
                fixture.agent_id.clone(),
                EventKind::RunStarted,
                Some(1),
                EventStreamScope::Run(fixture.run_id.clone()),
            ),
            terminal_frame(
                2,
                fixture.run_id.clone(),
                fixture.agent_id.clone(),
                EventKind::RunCompleted,
                1,
                EventStreamScope::Run(fixture.run_id.clone()),
            ),
        ])
        .unwrap();

    let first = fixture
        .handle
        .stream_from(None)
        .unwrap()
        .collect::<Vec<_>>();
    let second = fixture
        .handle
        .stream_from(None)
        .unwrap()
        .collect::<Vec<_>>();

    assert_eq!(first, second);
    assert_eq!(fixture.journal.records().len(), 1);
    assert_eq!(fixture.hub.frames().unwrap().len(), 2);
}

#[test]
fn terminal_result_mismatch_between_journal_and_event_is_rejected() {
    let fixture = Fixture::new();
    fixture.seal_completed(1, "sealed output");
    fixture
        .hub
        .publish(terminal_frame(
            2,
            fixture.run_id.clone(),
            fixture.agent_id.clone(),
            EventKind::RunFailed,
            1,
            EventStreamScope::Run(fixture.run_id.clone()),
        ))
        .unwrap();

    let error = fixture
        .handle
        .wait()
        .expect_err("mismatch must be rejected");

    assert_eq!(error.kind(), AgentErrorKind::InvalidStateTransition);
}

#[test]
fn subscribe_agent_returns_all_matching_live_runs() {
    let hub = InMemorySubscriptionHub::default();
    let agent_id = AgentId::new("agent.handle.contract");
    hub.publish_all([
        lifecycle_frame(
            1,
            RunId::new("run.agent.one"),
            agent_id.clone(),
            EventKind::RunStarted,
            Some(1),
            EventStreamScope::Agent(agent_id.clone()),
        ),
        lifecycle_frame(
            2,
            RunId::new("run.agent.two"),
            agent_id.clone(),
            EventKind::RunStarted,
            Some(2),
            EventStreamScope::Agent(agent_id.clone()),
        ),
        lifecycle_frame(
            3,
            RunId::new("run.agent.other"),
            AgentId::new("agent.other"),
            EventKind::RunStarted,
            Some(3),
            EventStreamScope::All,
        ),
    ])
    .unwrap();

    let frames = hub
        .subscribe_agent(agent_id, None)
        .expect("agent subscription")
        .collect::<Vec<_>>();

    assert_eq!(frames.len(), 2);
}

#[test]
fn subscribe_filtered_terminal_events_can_resume_from_cursor() {
    let hub = InMemorySubscriptionHub::default();
    let filter = EventFilter::terminal_run_events()
        .compile()
        .expect("filter compiles");
    hub.publish_all([
        terminal_frame(
            1,
            RunId::new("run.filter.one"),
            AgentId::new("agent.filter"),
            EventKind::RunCompleted,
            1,
            filter.cursor_scope(),
        ),
        terminal_frame(
            2,
            RunId::new("run.filter.two"),
            AgentId::new("agent.filter"),
            EventKind::RunCancelled,
            2,
            filter.cursor_scope(),
        ),
    ])
    .unwrap();

    let first_pass = hub
        .subscribe_events(filter.clone(), None)
        .expect("filtered subscription")
        .collect::<Vec<_>>();
    let resumed = hub
        .subscribe_events(filter, Some(first_pass[0].cursor.clone()))
        .expect("filtered resume")
        .collect::<Vec<_>>();

    assert_eq!(resumed.len(), 1);
    assert_eq!(
        resumed[0].event.envelope.event_kind,
        EventKind::RunCancelled
    );
}

#[test]
fn filtered_subscription_does_not_require_payload_deserialization() {
    let fixture = Fixture::new();
    let mut frame = lifecycle_frame(
        1,
        fixture.run_id.clone(),
        fixture.agent_id.clone(),
        EventKind::ModelStreamDelta,
        Some(1),
        EventStreamScope::Run(fixture.run_id.clone()),
    );
    frame.event = AgentEvent::with_redacted_summary(
        frame.event.envelope.clone(),
        "payload should not be inspected for envelope-only matching",
    );
    fixture.hub.publish(frame).unwrap();

    let filter = EventFilter::run(fixture.run_id.clone())
        .compile()
        .expect("filter compiles");
    let matched = fixture
        .hub
        .subscribe_events(filter, None)
        .expect("filter subscription")
        .collect::<Vec<_>>();

    assert_eq!(matched.len(), 1);
    assert_eq!(
        matched[0].event.redacted_summary(),
        Some("payload should not be inspected for envelope-only matching")
    );
}

fn lifecycle_frame(
    seq: u64,
    run_id: RunId,
    agent_id: AgentId,
    kind: EventKind,
    journal_seq: Option<u64>,
    scope: EventStreamScope,
) -> EventFrame {
    let family = match kind {
        EventKind::ModelStreamDelta => EventFamily::Model,
        _ => EventFamily::Run,
    };
    let delivery = if journal_seq.is_some() {
        EventDeliverySemantics::JournalBacked
    } else {
        EventDeliverySemantics::BestEffortLive
    };
    let event = AgentEvent::with_redacted_summary(
        envelope(seq, run_id, agent_id, family, kind, delivery, journal_seq),
        "redacted lifecycle event",
    );
    frame(event, scope)
}

fn terminal_frame(
    seq: u64,
    run_id: RunId,
    agent_id: AgentId,
    kind: EventKind,
    journal_seq: u64,
    scope: EventStreamScope,
) -> EventFrame {
    let event = AgentEvent::envelope_only(envelope(
        seq,
        run_id,
        agent_id,
        EventFamily::Run,
        kind,
        EventDeliverySemantics::JournalBacked,
        Some(journal_seq),
    ));
    frame(event, scope)
}

fn frame(event: AgentEvent, scope: EventStreamScope) -> EventFrame {
    let cursor = event.envelope.cursor(scope);
    EventFrame {
        event,
        cursor,
        archive_cursor: None,
        overflow: None,
    }
}

fn envelope(
    seq: u64,
    run_id: RunId,
    agent_id: AgentId,
    family: EventFamily,
    kind: EventKind,
    delivery: EventDeliverySemantics,
    journal_seq: Option<u64>,
) -> EventEnvelope {
    EventEnvelope {
        schema_version: EVENT_SCHEMA_VERSION,
        event_id: EventId::new(format!("event.handle.{seq}")),
        event_seq: seq,
        event_family: family,
        event_kind: kind,
        payload_schema_version: 1,
        timestamp: "2026-05-24T12:00:00Z".to_string(),
        recorded_at: "2026-05-24T12:00:00Z".to_string(),
        run_id: run_id.clone(),
        session_id: None,
        agent_id,
        turn_id: None,
        attempt_id: None,
        message_id: None,
        context_item_id: None,
        trace_id: TraceId::new(format!("trace.handle.{seq}")),
        span_id: SpanId::new(format!("span.handle.{seq}")),
        parent_event_id: None,
        caused_by: None,
        subject_ref: EntityRef::run(run_id),
        related_refs: Vec::new(),
        causal_refs: Vec::new(),
        correlation: EventCorrelation::default(),
        tags: vec![EventTag::new("phase:03c")],
        source: SourceRef::with_kind(SourceKind::Sdk, "source.sdk.run_handle"),
        destination: Some(DestinationRef::with_kind(
            DestinationKind::EventStream,
            "destination.event_stream.run_handle",
        )),
        policy_refs: Vec::new(),
        journal_cursor: journal_seq.map(|seq| JournalCursor::new(format!("journal.{seq}"))),
        state_before: None,
        state_after: None,
        delivery_semantics: delivery,
        privacy: PrivacyClass::ContentRefsOnly,
        content_capture: ContentCaptureMode::Off,
        redaction_policy_id: "policy.redaction.default".to_string(),
        runtime_package_fingerprint: "runtime.package.fingerprint.run_handle".to_string(),
    }
}

fn terminal_record(
    journal_seq: u64,
    run_id: RunId,
    agent_id: AgentId,
    status: RunStatus,
) -> JournalRecord {
    let terminal_status = status
        .as_terminal_str()
        .expect("test terminal status")
        .to_string();
    let source = SourceRef::with_kind(SourceKind::Sdk, "source.sdk.run_handle");
    let destination = Some(DestinationRef::with_kind(
        DestinationKind::Journal,
        "destination.journal.run_handle",
    ));
    let subject_ref = EntityRef::run(run_id.clone());

    JournalRecord {
        journal_schema_version: JOURNAL_SCHEMA_VERSION,
        journal_seq,
        record_id: format!("journal.record.terminal.{journal_seq}"),
        record_kind: JournalRecordKind::Run,
        run_id: run_id.clone(),
        session_id: None,
        agent_id: agent_id.clone(),
        turn_id: None,
        attempt_id: None,
        subject_ref: subject_ref.clone(),
        related_refs: Vec::new(),
        causal_refs: Vec::new(),
        source: source.clone(),
        destination: destination.clone(),
        correlation_keys: Vec::new(),
        tags: vec!["phase:03c".to_string()],
        delivery_semantics: "journal_backed".to_string(),
        event_index: EventIndexProjection {
            run_id: run_id.clone(),
            session_id: None,
            agent_id,
            turn_id: None,
            event_family: "run".to_string(),
            event_kind: terminal_status.clone(),
            source,
            destination,
            subject_ref,
            related_refs: Vec::new(),
            correlation_keys: Vec::new(),
            tags: vec!["phase:03c".to_string()],
            privacy_class: PrivacyClass::ContentRefsOnly,
            delivery_semantics: "journal_backed".to_string(),
        },
        timestamp_millis: 1_779_552_000_000 + journal_seq,
        runtime_package_fingerprint: "runtime.package.fingerprint.run_handle".to_string(),
        privacy: PrivacyClass::ContentRefsOnly,
        content_refs: Vec::new(),
        redaction_policy_id: "policy.redaction.default".to_string(),
        idempotency_key: None,
        dedupe_key: None,
        checkpoint_ref: None,
        payload: JournalRecordPayload::TerminalResult(TerminalResultMarker {
            effect_id: EffectId::new(format!("effect.terminal.{journal_seq}")),
            result_record_id: format!("journal.record.terminal.{journal_seq}"),
            terminal_status,
        }),
    }
}
