use std::{num::NonZeroUsize, sync::Arc};

use agent_sdk_core::{
    AgentEvent, AgentId, DestinationKind, DestinationRef, EntityRef, EventEnvelope, EventFamily,
    EventId, EventKind, JournalCursor, PolicyRef, PrivacyClass, RunId, SourceKind, SourceRef,
    TraceId,
    event::{
        ContentCaptureMode as EventContentCaptureMode, EVENT_SCHEMA_VERSION, EventCorrelation,
        EventDeliverySemantics, EventStreamScope,
    },
    ids::SpanId,
    telemetry::{
        TelemetryContentCaptureRequest, TelemetryFanout, TelemetryFanoutConfig,
        TelemetryOverflowPolicy, TelemetryUsageExtractionInput, TelemetryUsageExtractor,
        evaluate_content_capture, telemetry_authority_boundary, terminal_run_projection_from_event,
    },
    telemetry_ports::TelemetrySinkSpec,
    telemetry_records::{
        TELEMETRY_SCHEMA_VERSION, TelemetryContentCaptureMode, TelemetryExportCursor,
        TelemetryProjection, TelemetryProjectionId, TelemetryProjectionKind, TelemetryRecord,
        TelemetryRecordId, TelemetryRecordPayload, TelemetrySinkId, TelemetrySourceCursor,
        TelemetrySourceRecord, TelemetryUsageRecordId, UsageUnits,
    },
    testing::ScriptedTelemetrySink,
    testing::{normalize_json_value, read_fixture},
};
use serde_json::Value;

#[test]
fn safe_telemetry_defaults_lower_to_content_capture_off() {
    let fanout = TelemetryFanout::safe_defaults();
    let sink = TelemetrySinkSpec::safe_local_diagnostic("telemetry.local.safe");
    let policy = agent_sdk_core::ContentCapturePolicy::safe_defaults(PolicyRef::new(
        "policy.telemetry.safe_defaults",
    ));

    let decision = evaluate_content_capture(&TelemetryContentCaptureRequest {
        policy,
        sink: sink.clone(),
        requested_mode: TelemetryContentCaptureMode::RawContent,
        source_permits_content: false,
        retention_active: true,
        deterministic_sample_included: true,
        requested_bytes: 32,
        redaction_policy_id: "redaction.telemetry.default".to_string(),
    });

    assert_eq!(fanout.sink_queue_len(&sink.sink_id), None);
    assert_eq!(sink.content_capture, TelemetryContentCaptureMode::Off);
    assert!(!decision.allowed);
    assert_eq!(
        decision.effective_mode,
        TelemetryContentCaptureMode::RedactedSummary
    );
}

#[test]
fn telemetry_helper_and_explicit_sink_emit_equivalent_usage_records() {
    let event = model_usage_event();
    let event_cursor = event
        .envelope
        .cursor(EventStreamScope::Run(RunId::new("run.telemetry.usage")));
    let projection = TelemetryUsageExtractor::extract_from_event(TelemetryUsageExtractionInput {
        event,
        event_cursor: Some(event_cursor),
        provider_id: Some("provider.fake".to_string()),
        model_id: Some("model.fake".to_string()),
        usage: UsageUnits {
            input_tokens: Some(12),
            output_tokens: Some(5),
            total_tokens: Some(17),
            bytes: None,
            media_duration_ms: None,
        },
    })
    .expect("usage projection");

    let helper_record =
        TelemetryUsageExtractor::usage_record(&projection, "telemetry.usage.record.1");
    let explicit_record = TelemetryRecord::usage(
        TelemetryRecordId::new("telemetry.record.telemetry.usage.event.model.1"),
        &projection,
        TelemetryUsageRecordId::new("telemetry.usage.record.1"),
    );

    assert_eq!(helper_record, explicit_record);
    assert_eq!(
        normalize(serde_json::to_value(helper_record).expect("usage record JSON")),
        read_fixture("tests/fixtures/telemetry/usage-record.json").expect("usage fixture loads")
    );
}

#[test]
fn telemetry_sink_cannot_escalate_content_capture() {
    let sink = Arc::new(ScriptedTelemetrySink::new(TelemetrySinkSpec::test(
        "telemetry.no.raw",
        NonZeroUsize::new(8).expect("nonzero"),
    )));
    let sink_id = sink.sink_spec().sink_id.clone();
    let mut fanout = TelemetryFanout::new(TelemetryFanoutConfig::safe_defaults());
    fanout.register_sink(sink).expect("sink registers");

    let mut projection = progress_projection("telemetry.progress.raw");
    projection.content_capture = TelemetryContentCaptureMode::RawContent;
    projection.raw_content = Some("do not export this raw text".to_string());

    let report = fanout.try_record(projection);
    let queued = fanout.queued_for_sink(&sink_id);

    assert_eq!(report.enqueued, 1);
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].raw_content, None);
    assert_eq!(
        queued[0].content_capture,
        TelemetryContentCaptureMode::RedactedSummary
    );
}

#[test]
fn telemetry_content_capture_requires_redaction_retention_sampling_and_sink_permission() {
    let raw_sink = TelemetrySinkSpec::safe_local_diagnostic("telemetry.raw")
        .with_content_capture(TelemetryContentCaptureMode::RawContent);

    let mut allowed_policy = raw_policy();
    let allowed = evaluate_content_capture(&TelemetryContentCaptureRequest {
        policy: allowed_policy.clone(),
        sink: raw_sink.clone(),
        requested_mode: TelemetryContentCaptureMode::RawContent,
        source_permits_content: true,
        retention_active: true,
        deterministic_sample_included: true,
        requested_bytes: 64,
        redaction_policy_id: "redaction.telemetry.raw".to_string(),
    });
    assert!(allowed.allowed);
    assert_eq!(
        allowed.effective_mode,
        TelemetryContentCaptureMode::RawContent
    );

    allowed_policy.redaction_required = false;
    assert_raw_denied(
        allowed_policy.clone(),
        raw_sink.clone(),
        true,
        true,
        true,
        64,
    );

    allowed_policy = raw_policy();
    allowed_policy.retention_required = false;
    assert_raw_denied(
        allowed_policy.clone(),
        raw_sink.clone(),
        true,
        true,
        true,
        64,
    );

    allowed_policy = raw_policy();
    allowed_policy.sampling_required = false;
    assert_raw_denied(
        allowed_policy.clone(),
        raw_sink.clone(),
        true,
        true,
        true,
        64,
    );

    allowed_policy = raw_policy();
    assert_raw_denied(
        allowed_policy.clone(),
        raw_sink.clone(),
        false,
        true,
        true,
        64,
    );

    let no_raw_sink = raw_sink
        .clone()
        .with_content_capture(TelemetryContentCaptureMode::Off);
    assert_raw_denied(raw_policy(), no_raw_sink, true, true, true, 64);

    assert_raw_denied(raw_policy(), raw_sink, true, true, false, 64);
}

#[test]
fn telemetry_content_capture_denies_expired_retention_on_repair() {
    let decision = evaluate_content_capture(&TelemetryContentCaptureRequest {
        policy: raw_policy(),
        sink: TelemetrySinkSpec::safe_local_diagnostic("telemetry.repair.raw")
            .with_content_capture(TelemetryContentCaptureMode::RawContent),
        requested_mode: TelemetryContentCaptureMode::RawContent,
        source_permits_content: true,
        retention_active: false,
        deterministic_sample_included: true,
        requested_bytes: 64,
        redaction_policy_id: "redaction.telemetry.repair".to_string(),
    });

    assert!(!decision.allowed);
    assert_eq!(
        normalize(serde_json::to_value(decision).expect("content decision JSON")),
        read_fixture("tests/fixtures/telemetry/content-capture-denial.json")
            .expect("content capture fixture loads")
    );
}

#[test]
fn cost_accounting_runs_without_raw_prompt_tool_or_model_content() {
    let projection = TelemetryUsageExtractor::extract_from_event(TelemetryUsageExtractionInput {
        event: model_usage_event(),
        event_cursor: None,
        provider_id: Some("provider.fake".to_string()),
        model_id: Some("model.fake".to_string()),
        usage: UsageUnits {
            input_tokens: Some(10),
            output_tokens: Some(7),
            total_tokens: Some(17),
            bytes: None,
            media_duration_ms: None,
        },
    })
    .expect("usage projection");

    assert_eq!(projection.content_capture, TelemetryContentCaptureMode::Off);
    assert_eq!(projection.raw_content, None);
    assert_eq!(projection.usage.as_ref().unwrap().total_tokens, Some(17));
    assert!(
        projection
            .redacted_summary
            .contains("without raw prompt, tool, or model content")
    );
}

#[test]
fn slow_telemetry_sink_overflow_does_not_block_run_and_preserves_terminal() {
    let sink = Arc::new(ScriptedTelemetrySink::new(
        TelemetrySinkSpec::test(
            "telemetry.slow",
            NonZeroUsize::new(2).expect("nonzero capacity"),
        )
        .with_terminal_reserve(NonZeroUsize::new(1).expect("nonzero reserve")),
    ));
    let sink_id = sink.sink_spec().sink_id.clone();
    let mut fanout = TelemetryFanout::new(TelemetryFanoutConfig::tiny_for_tests());
    fanout.register_sink(sink).expect("sink registers");

    let first = fanout.try_record(progress_projection("telemetry.progress.1"));
    let second = fanout.try_record(progress_projection("telemetry.progress.2"));
    let terminal = fanout.try_record(terminal_projection());
    let queued = fanout.queued_for_sink(&sink_id);

    assert_eq!(first.enqueued, 1);
    assert_eq!(second.dropped, 1);
    assert_eq!(terminal.enqueued, 1);
    assert!(terminal.records.iter().all(|record| match &record.payload {
        TelemetryRecordPayload::SinkFailed(failure) => failure.terminal_preserved,
        _ => true,
    }));
    assert_eq!(queued.len(), 2);
    assert!(
        queued
            .iter()
            .any(|projection| projection.projection_kind == TelemetryProjectionKind::RunTerminal)
    );
}

#[test]
fn terminal_usage_record_survives_overflow() {
    let sink = Arc::new(ScriptedTelemetrySink::new(
        TelemetrySinkSpec::test(
            "telemetry.usage.overflow",
            NonZeroUsize::new(1).expect("nonzero capacity"),
        )
        .with_terminal_reserve(NonZeroUsize::new(1).expect("nonzero reserve")),
    ));
    let sink_id = sink.sink_spec().sink_id.clone();
    let mut fanout = TelemetryFanout::new(TelemetryFanoutConfig {
        overflow: TelemetryOverflowPolicy::DropNonTerminalProgress,
        ..TelemetryFanoutConfig::tiny_for_tests()
    });
    fanout.register_sink(sink).expect("sink registers");

    fanout.try_record(progress_projection("telemetry.progress.before_usage"));
    let usage = fanout.try_record(usage_projection());
    let queued = fanout.queued_for_sink(&sink_id);

    assert_eq!(usage.enqueued, 1);
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].projection_kind, TelemetryProjectionKind::Usage);
}

#[test]
fn telemetry_sink_failure_isolates_one_sink() {
    let failing = Arc::new(ScriptedTelemetrySink::new(TelemetrySinkSpec::test(
        "telemetry.failing",
        NonZeroUsize::new(4).expect("nonzero"),
    )));
    failing.fail_next("collector unavailable");
    let healthy = Arc::new(ScriptedTelemetrySink::new(TelemetrySinkSpec::test(
        "telemetry.healthy",
        NonZeroUsize::new(4).expect("nonzero"),
    )));
    let failing_id = failing.sink_spec().sink_id.clone();
    let healthy_id = healthy.sink_spec().sink_id.clone();
    let mut fanout = TelemetryFanout::new(TelemetryFanoutConfig::safe_defaults());
    fanout.register_sink(failing.clone()).expect("failing sink");
    fanout.register_sink(healthy.clone()).expect("healthy sink");
    fanout.try_record(terminal_projection());

    let failing_report = fanout.drain_sink(&failing_id).expect("failing drain");
    let healthy_report = fanout.drain_sink(&healthy_id).expect("healthy drain");

    assert_eq!(failing_report.exported, 0);
    assert!(matches!(
        &failing_report.records[0].payload,
        TelemetryRecordPayload::SinkFailed(_)
    ));
    assert_eq!(healthy_report.exported, 1);
    assert_eq!(healthy.exports().len(), 1);
    assert_eq!(failing.exports().len(), 0);
}

#[test]
fn sink_failure_and_recovery_records_carry_repair_cursor() {
    let sink = Arc::new(ScriptedTelemetrySink::new(TelemetrySinkSpec::test(
        "telemetry.repairable",
        NonZeroUsize::new(4).expect("nonzero"),
    )));
    sink.fail_next("temporary sink outage");
    let sink_id = sink.sink_spec().sink_id.clone();
    let mut fanout = TelemetryFanout::new(TelemetryFanoutConfig::safe_defaults());
    fanout.register_sink(sink.clone()).expect("sink registers");
    fanout.try_record(terminal_projection());

    let failure = fanout.drain_sink(&sink_id).expect("first drain");
    let recovery = fanout.drain_sink(&sink_id).expect("second drain");

    let TelemetryRecordPayload::SinkFailed(failure_record) = &failure.records[0].payload else {
        panic!("expected sink failure record");
    };
    assert!(matches!(
        failure_record.repair_cursor,
        Some(TelemetrySourceCursor::Journal(_))
    ));
    assert!(failure_record.terminal_preserved);

    let TelemetryRecordPayload::SinkRecovered(recovery_record) = &recovery.records[0].payload
    else {
        panic!("expected sink recovery record");
    };
    assert_eq!(recovery.exported, 1);
    assert_eq!(recovery_record.export_cursor.export_seq, 1);
    assert_eq!(
        normalize(serde_json::to_value(&failure.records[0]).expect("failure record JSON")),
        read_fixture("tests/fixtures/telemetry/sink-failure-record.json")
            .expect("sink failure fixture loads")
    );
}

#[test]
fn telemetry_export_cursor_is_distinct_from_event_and_journal_cursor() {
    let projection = terminal_projection();
    let source_cursor = projection
        .source_record
        .source_cursor
        .clone()
        .expect("terminal projection has durable source cursor");
    let export_cursor = TelemetryExportCursor::new(TelemetrySinkId::new("telemetry.cursor"));

    assert!(matches!(source_cursor, TelemetrySourceCursor::Journal(_)));
    assert_eq!(export_cursor.sink_id.as_str(), "telemetry.cursor");
    assert_eq!(export_cursor.export_seq, 0);
    assert_eq!(export_cursor.last_acknowledged_source, None);
}

#[test]
fn telemetry_cannot_decide_run_state_policy_output_or_side_effect_status() {
    let boundary = telemetry_authority_boundary();

    assert!(!boundary.can_decide_run_state);
    assert!(!boundary.can_decide_policy_outcome);
    assert!(!boundary.can_decide_output_delivery);
    assert!(!boundary.can_decide_side_effect_status);

    let application_source = include_str!("../../src/application/telemetry.rs");
    for forbidden in [
        "RunStatus",
        "PolicyDecision",
        "OutputDispatchRecord",
        "EffectTerminalStatus",
    ] {
        assert!(
            !application_source.contains(forbidden),
            "telemetry application must not import or decide {forbidden}"
        );
    }
}

#[test]
fn telemetry_repair_replay_does_not_execute_side_effects() {
    let application_source = include_str!("../../src/application/telemetry.rs");
    for forbidden in [
        ".complete(",
        ".stream(",
        ".execute(",
        "OutputSinkPort",
        "RunJournal",
        "append_before_effect",
        "append_result_or_recovery",
    ] {
        assert!(
            !application_source.contains(forbidden),
            "telemetry repair/export paths must not execute side effects through {forbidden}"
        );
    }
}

fn assert_raw_denied(
    policy: agent_sdk_core::ContentCapturePolicy,
    sink: TelemetrySinkSpec,
    source_permits_content: bool,
    retention_active: bool,
    deterministic_sample_included: bool,
    requested_bytes: u64,
) {
    let decision = evaluate_content_capture(&TelemetryContentCaptureRequest {
        policy,
        sink,
        requested_mode: TelemetryContentCaptureMode::RawContent,
        source_permits_content,
        retention_active,
        deterministic_sample_included,
        requested_bytes,
        redaction_policy_id: "redaction.telemetry.raw".to_string(),
    });

    assert!(!decision.allowed);
    assert_eq!(
        decision.effective_mode,
        TelemetryContentCaptureMode::RedactedSummary
    );
}

fn raw_policy() -> agent_sdk_core::ContentCapturePolicy {
    let mut policy =
        agent_sdk_core::ContentCapturePolicy::safe_defaults(PolicyRef::new("policy.telemetry.raw"));
    policy.mode = agent_sdk_core::policy::ContentCaptureMode::RawContent;
    policy.source_permits_content = true;
    policy.sink_permits_content = true;
    policy.redaction_required = true;
    policy.retention_required = true;
    policy.sampling_required = true;
    policy.byte_limit = 1024;
    policy
}

fn model_usage_event() -> AgentEvent {
    base_event(
        EventFamily::Model,
        EventKind::ModelMessageCompleted,
        "event.model.1",
        Some(JournalCursor::new("journal.usage.1")),
    )
}

fn terminal_projection() -> TelemetryProjection {
    terminal_run_projection_from_event(base_event(
        EventFamily::Run,
        EventKind::RunCompleted,
        "event.run.completed.1",
        Some(JournalCursor::new("journal.terminal.1")),
    ))
}

fn usage_projection() -> TelemetryProjection {
    TelemetryUsageExtractor::extract_from_event(TelemetryUsageExtractionInput {
        event: model_usage_event(),
        event_cursor: None,
        provider_id: Some("provider.fake".to_string()),
        model_id: Some("model.fake".to_string()),
        usage: UsageUnits {
            input_tokens: Some(8),
            output_tokens: Some(3),
            total_tokens: Some(11),
            bytes: None,
            media_duration_ms: None,
        },
    })
    .expect("usage projection")
}

fn progress_projection(projection_id: &str) -> TelemetryProjection {
    let event = base_event(
        EventFamily::Model,
        EventKind::ModelStreamDelta,
        "event.progress.1",
        None,
    );
    let envelope = event.envelope;
    TelemetryProjection {
        schema_version: TELEMETRY_SCHEMA_VERSION,
        projection_id: TelemetryProjectionId::new(projection_id),
        projection_kind: TelemetryProjectionKind::Progress,
        source_record: TelemetrySourceRecord {
            event_family: envelope.event_family.clone(),
            event_kind: envelope.event_kind.clone(),
            event_cursor: Some(envelope.cursor(EventStreamScope::Run(envelope.run_id.clone()))),
            source_cursor: None,
        },
        run_id: envelope.run_id,
        agent_id: envelope.agent_id,
        turn_id: envelope.turn_id,
        attempt_id: envelope.attempt_id,
        event_id: Some(envelope.event_id),
        journal_cursor: envelope.journal_cursor,
        trace_id: Some(envelope.trace_id),
        span_id: Some(envelope.span_id),
        runtime_package_fingerprint: envelope.runtime_package_fingerprint,
        source: envelope.source,
        destination: Some(DestinationRef::with_kind(
            DestinationKind::Telemetry,
            "destination.telemetry.progress",
        )),
        subject_ref: envelope.subject_ref,
        policy_refs: envelope.policy_refs,
        privacy: envelope.privacy,
        retention: agent_sdk_core::RetentionClass::RunScoped,
        content_capture: TelemetryContentCaptureMode::Off,
        redaction_policy_id: envelope.redaction_policy_id,
        provider_id: Some("provider.fake".to_string()),
        model_id: Some("model.fake".to_string()),
        tool_name: None,
        usage: None,
        cost: None,
        terminal_status: None,
        sink_health: None,
        redacted_summary: "coalescible model progress".to_string(),
        raw_content: None,
    }
}

fn base_event(
    family: EventFamily,
    kind: EventKind,
    event_id: &str,
    journal_cursor: Option<JournalCursor>,
) -> AgentEvent {
    let run_id = RunId::new("run.telemetry.usage");
    AgentEvent::with_redacted_summary(
        EventEnvelope {
            schema_version: EVENT_SCHEMA_VERSION,
            event_id: EventId::new(event_id),
            event_seq: 1,
            event_family: family,
            event_kind: kind,
            payload_schema_version: 1,
            timestamp: "2026-05-24T00:00:00Z".to_string(),
            recorded_at: "2026-05-24T00:00:00Z".to_string(),
            run_id: run_id.clone(),
            agent_id: AgentId::new("agent.telemetry"),
            turn_id: None,
            attempt_id: None,
            message_id: None,
            context_item_id: None,
            trace_id: TraceId::new("trace.telemetry.1"),
            span_id: SpanId::new("span.telemetry.1"),
            parent_event_id: None,
            caused_by: None,
            subject_ref: EntityRef::run(run_id),
            related_refs: Vec::new(),
            causal_refs: Vec::new(),
            correlation: EventCorrelation::default(),
            tags: Vec::new(),
            source: SourceRef::with_kind(SourceKind::Sdk, "source.agent.loop"),
            destination: Some(DestinationRef::with_kind(
                DestinationKind::Telemetry,
                "destination.telemetry",
            )),
            policy_refs: vec![PolicyRef::new("policy.telemetry.derived")],
            journal_cursor,
            state_before: None,
            state_after: None,
            delivery_semantics: EventDeliverySemantics::JournalBacked,
            privacy: PrivacyClass::ContentRefsOnly,
            content_capture: EventContentCaptureMode::Off,
            redaction_policy_id: "redaction.telemetry.default".to_string(),
            runtime_package_fingerprint: "sha256:runtime.package.telemetry".to_string(),
        },
        "redacted telemetry source event",
    )
}

fn normalize(value: Value) -> Value {
    normalize_json_value(value)
}
