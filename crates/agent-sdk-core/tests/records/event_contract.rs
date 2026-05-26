use std::num::NonZeroUsize;

use agent_sdk_core::ids::{ArchiveCursorId, SpanId};
use agent_sdk_core::{
    AgentEventBus, AgentId, DestinationKind, DestinationRef, EntityRef, EventId,
    InMemoryAgentEventBus, JournalCursor, PrivacyClass, RunId, SourceKind, SourceRef, TraceId,
    TurnId,
    event::{
        AgentEvent, ArchiveCursor, CompiledEventFilter, ContentCaptureMode, EVENT_SCHEMA_VERSION,
        EventCorrelation, EventDeliverySemantics, EventEnvelope, EventFamily, EventFilter,
        EventFilterSet, EventFrame, EventKind, EventOverflowNotice, EventOverflowReason,
        EventStreamScope, PayloadAccessMode, SubscriberOverflowPolicy, SubscriberQueueConfig,
        cursor_compatible,
    },
    testing::FakeEventConformanceHarness,
};
use serde_json::json;

fn run_started_event(seq: u64, payload_marker: &str) -> AgentEvent {
    AgentEvent::with_redacted_summary(
        envelope(seq, EventFamily::Run, EventKind::RunStarted),
        format!("run started {payload_marker}"),
    )
}

fn run_completed_event(seq: u64, run_id: RunId) -> AgentEvent {
    let mut envelope = envelope(seq, EventFamily::Run, EventKind::RunCompleted);
    envelope.run_id = run_id.clone();
    envelope.subject_ref = EntityRef::run(run_id);
    envelope.journal_cursor = Some(JournalCursor::new(format!("journal.cursor.{seq}")));
    envelope.delivery_semantics = EventDeliverySemantics::JournalBacked;
    AgentEvent::envelope_only(envelope)
}

fn model_delta_event(seq: u64) -> AgentEvent {
    let mut envelope = envelope(seq, EventFamily::Model, EventKind::ModelStreamDelta);
    envelope.privacy = PrivacyClass::ContentRefsOnly;
    envelope.content_capture = ContentCaptureMode::Off;
    AgentEvent::with_redacted_summary(envelope, "model delta redacted")
}

fn envelope(seq: u64, family: EventFamily, kind: EventKind) -> EventEnvelope {
    let run_id = RunId::new("run.event.contract");
    EventEnvelope {
        schema_version: EVENT_SCHEMA_VERSION,
        event_id: EventId::new(format!("event.{seq}")),
        event_seq: seq,
        event_family: family,
        event_kind: kind,
        payload_schema_version: 1,
        timestamp: "2026-05-24T12:00:00Z".to_string(),
        recorded_at: "2026-05-24T12:00:00Z".to_string(),
        run_id: run_id.clone(),
        session_id: None,
        agent_id: AgentId::new("agent.event.contract"),
        turn_id: Some(TurnId::new("turn.event.contract")),
        attempt_id: None,
        message_id: None,
        context_item_id: None,
        trace_id: TraceId::new("trace.event.contract"),
        span_id: SpanId::new(format!("span.{seq}")),
        parent_event_id: None,
        caused_by: None,
        subject_ref: EntityRef::run(run_id),
        related_refs: Vec::new(),
        causal_refs: Vec::new(),
        correlation: EventCorrelation::default(),
        tags: Vec::new(),
        source: SourceRef::with_kind(SourceKind::Sdk, "source.sdk.event"),
        destination: Some(DestinationRef::with_kind(
            DestinationKind::EventStream,
            "destination.event.stream",
        )),
        policy_refs: Vec::new(),
        journal_cursor: None,
        state_before: None,
        state_after: None,
        delivery_semantics: EventDeliverySemantics::BestEffortLive,
        privacy: PrivacyClass::ContentRefsOnly,
        content_capture: ContentCaptureMode::Off,
        redaction_policy_id: "policy.redaction.default".to_string(),
        runtime_package_fingerprint: "sha256:event-contract-package".to_string(),
    }
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

#[test]
fn event_envelope_defaults_are_redacted_and_fixture_stable() {
    let event = run_started_event(1, "fixture");
    let encoded = serde_json::to_value(&event).expect("event serializes");
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/events/run_started.json")).unwrap();

    assert_eq!(encoded, expected);
    assert_eq!(event.redacted_summary(), Some("run started fixture"));
    assert_eq!(event.envelope.content_capture, ContentCaptureMode::Off);
    assert_eq!(event.envelope.privacy, PrivacyClass::ContentRefsOnly);
}

#[test]
fn terminal_event_fixture_is_envelope_only_and_journal_linked() {
    let event = run_completed_event(2, RunId::new("run.event.contract"));
    let encoded = serde_json::to_value(&event).expect("event serializes");
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/events/run_completed.json")).unwrap();

    assert_eq!(encoded, expected);
    assert_eq!(event.redacted_summary(), None);
    assert_eq!(
        event.envelope.delivery_semantics,
        EventDeliverySemantics::JournalBacked
    );
    assert!(event.envelope.journal_cursor.is_some());
}

#[test]
fn compiled_filters_match_only_envelope_fields() {
    let filter = EventFilter {
        run_ids: EventFilterSet::Include(vec![RunId::new("run.event.contract")]),
        families: EventFilterSet::Include(vec![EventFamily::Model]),
        kinds: EventFilterSet::Include(vec![EventKind::ModelStreamDelta]),
        privacy_classes: EventFilterSet::Include(vec![PrivacyClass::ContentRefsOnly]),
        payload_access: PayloadAccessMode::EnvelopeOnly,
        ..EventFilter::default()
    }
    .compile()
    .expect("filter compiles");

    let matching = model_delta_event(3);
    let mut non_matching = model_delta_event(4);
    non_matching.envelope.run_id = RunId::new("run.other");

    assert!(filter.matches_envelope(&matching.envelope));
    assert!(!filter.matches_envelope(&non_matching.envelope));
    assert_eq!(filter.payload_access, PayloadAccessMode::EnvelopeOnly);
    assert!(!filter.indexed_fields.is_empty());
}

#[test]
fn model_delta_fixture_uses_redacted_summary_without_raw_payload() {
    let event = model_delta_event(3);
    let encoded = serde_json::to_value(&event).expect("event serializes");
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/events/model_stream_delta.json")).unwrap();

    assert_eq!(encoded, expected);
    assert_eq!(event.envelope.content_capture, ContentCaptureMode::Off);
    assert_eq!(event.redacted_summary(), Some("model delta redacted"));
}

#[test]
fn archive_cursor_fixture_is_distinct_from_live_and_journal_cursors() {
    let cursor = ArchiveCursor {
        archive_id: ArchiveCursorId::new("archive.events.contract"),
        position: "archive.position.42".to_string(),
        event_id: Some(EventId::new("event.42")),
        watermark: Some("2026-05-24T12:00:42Z".to_string()),
    };
    let encoded = serde_json::to_value(&cursor).expect("archive cursor serializes");
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/events/archive_cursor.json")).unwrap();

    assert_eq!(encoded, expected);
}

#[test]
fn filter_cursor_compatibility_rejects_scope_changes() {
    let filter = EventFilter::terminal_run_events()
        .compile()
        .expect("filter compiles");
    let cursor = EventFrame {
        event: run_completed_event(5, RunId::new("run.event.contract")),
        cursor: EventCursorForTest::filter_cursor(&filter, 5),
        archive_cursor: None,
        overflow: None,
    }
    .cursor;

    assert!(cursor_compatible(&filter.cursor_scope(), Some(&cursor)).is_ok());
    assert!(cursor_compatible(&EventStreamScope::All, Some(&cursor)).is_err());
}

#[test]
fn in_memory_bus_filters_and_resumes_without_payload_lookup() {
    let bus = InMemoryAgentEventBus::default();
    let run_frame = frame(
        run_started_event(1, "bus"),
        EventStreamScope::Run(RunId::new("run.event.contract")),
    );
    let model_frame = frame(model_delta_event(2), EventStreamScope::All);
    let terminal_frame = frame(
        run_completed_event(3, RunId::new("run.event.contract")),
        EventStreamScope::Run(RunId::new("run.event.contract")),
    );

    bus.publish_all([
        run_frame.clone(),
        model_frame.clone(),
        terminal_frame.clone(),
    ])
    .expect("publish succeeds");

    let replayed = bus
        .subscribe_run(RunId::new("run.event.contract"), Some(run_frame.cursor))
        .expect("subscribe run")
        .collect::<Vec<_>>();

    assert_eq!(replayed.len(), 2);
    assert_eq!(replayed[0].event, model_frame.event);
    assert_eq!(
        replayed[0].cursor.scope,
        EventStreamScope::Run(RunId::new("run.event.contract"))
    );
    assert_eq!(replayed[1].event, terminal_frame.event);
    assert_eq!(
        replayed[1].cursor.scope,
        EventStreamScope::Run(RunId::new("run.event.contract"))
    );

    let resumed = bus
        .subscribe_run(
            RunId::new("run.event.contract"),
            Some(replayed[0].cursor.clone()),
        )
        .expect("resubscribe from returned run cursor")
        .collect::<Vec<_>>();
    assert_eq!(resumed.len(), 1);
    assert_eq!(resumed[0].event, terminal_frame.event);
}

#[test]
fn overflow_notice_preserves_terminal_repair_cursor() {
    let terminal = frame(
        run_completed_event(9, RunId::new("run.event.contract")),
        EventStreamScope::Run(RunId::new("run.event.contract")),
    );
    let overflow = EventOverflowNotice {
        policy: SubscriberOverflowPolicy::DropNonTerminal,
        dropped_count: 7,
        gap_start: None,
        gap_end: terminal.cursor.clone(),
        repair_from: Some(JournalCursor::new("journal.cursor.9")),
        terminal_preserved: true,
        reason: EventOverflowReason::PolicyDroppedNonTerminal,
    };

    let value = json!(overflow);
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/events/overflow_notice.json")).unwrap();

    assert_eq!(value, expected);
}

#[test]
fn deterministic_fake_harness_is_reusable_for_conformance() {
    let mut harness = FakeEventConformanceHarness::default();
    harness.push(frame(model_delta_event(11), EventStreamScope::All));
    harness.push(frame(
        run_completed_event(12, RunId::new("run.event.contract")),
        EventStreamScope::Run(RunId::new("run.event.contract")),
    ));

    let terminal_filter = EventFilter {
        terminal_only: true,
        queue: SubscriberQueueConfig {
            capacity: NonZeroUsize::new(8).unwrap(),
            terminal_reserve: NonZeroUsize::new(1).unwrap(),
            overflow: SubscriberOverflowPolicy::DropNonTerminal,
        },
        ..EventFilter::default()
    }
    .compile()
    .expect("filter compiles");

    let matches = harness.matching_frames(&terminal_filter);
    assert_eq!(matches.len(), 1);
    assert_eq!(
        matches[0].event.envelope.event_kind,
        EventKind::RunCompleted
    );
}

struct EventCursorForTest;

impl EventCursorForTest {
    fn filter_cursor(filter: &CompiledEventFilter, seq: u64) -> agent_sdk_core::event::EventCursor {
        agent_sdk_core::event::EventCursor {
            scope: filter.cursor_scope(),
            event_seq: seq,
            event_id: EventId::new(format!("event.{seq}")),
            journal_cursor: None,
        }
    }
}
