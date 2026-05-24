mod support;

use agent_sdk_core::ids::SpanId;
use agent_sdk_core::{
    AgentId, DestinationKind, DestinationRef, EntityRef, EventFrame, EventId, JournalRecord,
    JournalRecordBase, PrivacyClass, ProviderAdapter, ProviderMessage, ProviderMessageRole,
    ProviderRequest, RecoveryMarker, RunId, RunJournal, SourceKind, SourceRef, TraceId, TurnId,
    event::{
        AgentEvent, ContentCaptureMode, EVENT_SCHEMA_VERSION, EventCorrelation,
        EventDeliverySemantics, EventEnvelope, EventFamily, EventKind, EventStreamScope,
    },
    testing::{
        FIXTURE_SCHEMA_VERSION, FakeFixtureHarness, FixtureManifest, FixtureManifestEntry,
        normalize_json_value, write_fixture,
    },
};
use serde_json::json;

#[test]
fn deterministic_id_and_clock_sequences_are_seeded() {
    let harness = FakeFixtureHarness::with_seed(7);

    assert_eq!(harness.ids.next_raw("run"), "run.0007.0000");
    assert_eq!(harness.ids.next_raw("run"), "run.0007.0001");
    assert_eq!(harness.clock.next_millis(), 7);
    assert_eq!(harness.clock.next_millis(), 8);
}

#[test]
fn fake_content_store_exposes_redacted_manifest() {
    let harness = FakeFixtureHarness::default();
    let content_ref = harness.ids.next_content_ref();

    harness
        .content_store
        .put_text(content_ref.clone(), "raw fixture bytes");

    let stored = harness.content_store.get(&content_ref).expect("content");
    assert_eq!(stored.bytes, b"raw fixture bytes");

    let manifest = harness.content_store.manifest();
    assert_eq!(manifest.len(), 1);
    assert_eq!(manifest[0].content_ref, "content.0000.0000");
    assert_eq!(manifest[0].byte_len, 17);
    assert_eq!(manifest[0].redacted_summary, "text content");
}

#[test]
fn fake_journal_store_is_append_only_and_cursor_ordered() {
    let harness = FakeFixtureHarness::default();

    let first = harness
        .journal_store
        .append(fake_journal_record(1))
        .expect("first append");
    let second = harness
        .journal_store
        .append(fake_journal_record(2))
        .expect("second append");

    assert_ne!(first.as_str(), second.as_str());
    assert_eq!(
        harness.journal_store.normalized_records(),
        vec![
            normalize_json_value(
                serde_json::to_value(fake_journal_record(1)).expect("record JSON")
            ),
            normalize_json_value(
                serde_json::to_value(fake_journal_record(2)).expect("record JSON")
            ),
        ]
    );
}

#[test]
fn fake_event_sink_records_normalized_metadata_without_payload_lookup() {
    let harness = FakeFixtureHarness::default();
    let event = fake_event(1);

    harness.event_sink.emit(EventFrame {
        cursor: event.envelope.cursor(EventStreamScope::All),
        event,
        archive_cursor: None,
        overflow: None,
    });

    assert_eq!(
        harness.event_sink.normalized_events(),
        vec![json!({
            "schema_version": FIXTURE_SCHEMA_VERSION,
            "event_seq": 1,
            "event": {
                "event_id": "event.1",
                "run_id": "run.1",
                "agent_id": "agent.1",
                "family": "Run",
                "kind": "RunStarted",
                "privacy": "Internal",
            },
        })]
    );
}

#[test]
fn fake_provider_records_prompts_and_exhausts_declared_outputs() {
    let provider = agent_sdk_core::testing::FakeProvider::with_responses(["one"]);
    let request = fake_provider_request("prompt words");
    let exhausted_request = fake_provider_request("again");

    let response = ProviderAdapter::complete(&provider, &request).expect("response");
    assert_eq!(response.output_text, "one");
    assert!(
        ProviderAdapter::complete(&provider, &exhausted_request).is_err(),
        "declared fake responses should be deterministic and finite"
    );
    assert_eq!(provider.requests(), vec![request, exhausted_request]);
}

#[test]
fn fixture_writer_round_trips_normalized_json() {
    let dir = support::temp_fixture_dir("fixture-writer").expect("temp dir");
    let path = dir.join("events/run-started.json");
    let value = json!({
        "z": 1,
        "a": {
            "c": true,
            "b": ["kept", "ordered"],
        },
    });
    let expected = normalize_json_value(value);

    write_fixture(&path, &expected).expect("write fixture");
    support::assert_fixture_round_trip(&path, &expected).expect("read fixture");
}

#[test]
fn fixture_manifest_documents_schema_paths_and_redaction() {
    let mut manifest = FixtureManifest::new("phase-01-fakes");
    manifest.entries.push(FixtureManifestEntry {
        path: "events/run-started.json".to_string(),
        contract: "event-schema".to_string(),
        schema_version: FIXTURE_SCHEMA_VERSION,
    });

    let normalized = normalize_json_value(serde_json::to_value(manifest).expect("manifest JSON"));
    assert_eq!(normalized["schema_version"], FIXTURE_SCHEMA_VERSION);
    assert_eq!(normalized["entries"][0]["path"], "events/run-started.json");
    assert!(
        normalized["redaction"]
            .as_str()
            .expect("redaction")
            .contains("redacted summaries")
    );
}

fn fake_journal_record(journal_seq: u64) -> JournalRecord {
    JournalRecord::recovery(
        JournalRecordBase::new(
            journal_seq,
            format!("journal.record.{journal_seq}"),
            RunId::new("run.fake.fixture"),
            AgentId::new("agent.fake.fixture"),
            SourceRef::with_kind(SourceKind::Sdk, "source.fake.fixture"),
        ),
        RecoveryMarker {
            unsafe_pending: Vec::new(),
            recovery_reason: format!("fake recovery marker {journal_seq}"),
            policy_refs: Vec::new(),
        },
    )
}

fn fake_event(seq: u64) -> AgentEvent {
    let run_id = RunId::new("run.1");
    AgentEvent::envelope_only(EventEnvelope {
        schema_version: EVENT_SCHEMA_VERSION,
        event_id: EventId::new(format!("event.{seq}")),
        event_seq: seq,
        event_family: EventFamily::Run,
        event_kind: EventKind::RunStarted,
        payload_schema_version: 1,
        timestamp: "2026-05-24T12:00:00Z".to_string(),
        recorded_at: "2026-05-24T12:00:00Z".to_string(),
        run_id: run_id.clone(),
        agent_id: AgentId::new("agent.1"),
        turn_id: Some(TurnId::new("turn.1")),
        attempt_id: None,
        message_id: None,
        context_item_id: None,
        trace_id: TraceId::new("trace.1"),
        span_id: SpanId::new("span.1"),
        parent_event_id: None,
        caused_by: None,
        subject_ref: EntityRef::run(run_id),
        related_refs: Vec::new(),
        causal_refs: Vec::new(),
        correlation: EventCorrelation::default(),
        tags: Vec::new(),
        source: SourceRef::with_kind(SourceKind::Sdk, "source.fake.fixture"),
        destination: Some(DestinationRef::with_kind(
            DestinationKind::EventStream,
            "destination.event.stream",
        )),
        policy_refs: Vec::new(),
        journal_cursor: None,
        state_before: None,
        state_after: None,
        delivery_semantics: EventDeliverySemantics::BestEffortLive,
        privacy: PrivacyClass::Internal,
        content_capture: ContentCaptureMode::Off,
        redaction_policy_id: "policy.redaction.default".to_string(),
        runtime_package_fingerprint: "sha256:fake-fixture-package".to_string(),
    })
}

fn fake_provider_request(content: &str) -> ProviderRequest {
    ProviderRequest {
        schema_version: ProviderRequest::SCHEMA_VERSION,
        projection_policy_ref: "policy.provider.fake-fixture".to_string(),
        messages: vec![ProviderMessage {
            role: ProviderMessageRole::User,
            content: content.to_string(),
            privacy: PrivacyClass::ContentRefsOnly,
            projected_metadata: None,
        }],
        projection_item_count: 1,
        structured_output_hint: None,
    }
}
