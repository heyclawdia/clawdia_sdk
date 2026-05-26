use std::{num::NonZeroUsize, sync::Arc};

use agent_sdk_core::{
    AdapterRef, AgentEvent, AgentEventBus, AgentId, CapabilityId, CapabilityNamespace, ContentHash,
    ContentId, ContentKind, ContentResolutionPolicy, ContentResolveRequest, ContentResolver,
    ContentScope, ContentVersion, ContextBudgetSummary, ContextContribution, ContextContributionId,
    ContextContributionKind, ContextItem, ContextProjection, ContextSelectionDecision,
    DestinationKind, DestinationRef, EffectId, EffectIntent, EffectKind, EffectResult, EntityKind,
    EntityRef, EventEnvelope, EventFamily, EventId, EventKind, ExecutorRef, IdempotencyKey,
    InMemoryAgentEventBus, JournalCursor, JournalRecord, JournalRecordBase, JournalRecordKind,
    JournalRecordPayload, LineageId, LineageRef, MessageId, OutputLineage, OutputSchemaId,
    PackageSidecarRef, PolicyDecision, PolicyKind, PolicyOutcome, PolicyRef, PolicyStage,
    PrivacyClass, ProjectionRole, RetentionClass, RiskClass, RunId, SchemaVersion, SourceKind,
    SourceRef, StreamAction, StreamChannel, StreamCursor, StreamDelta, StreamMatcher, StreamRule,
    StreamRuleEngine, ToolCallId, TraceId, TrustClass, TypedResultPublicationRecord,
    ValidatedOutput, ValidatedOutputId, ValidatedOutputParams, ValidationAttemptId,
    ValidationReportRecord,
    content::ContentRef as StoredContentRef,
    domain::ContentRef as ContentRefId,
    event::{
        ContentCaptureMode as EventContentCaptureMode, EVENT_SCHEMA_VERSION, EventCorrelation,
        EventDeliverySemantics, EventFilter, EventFilterSet, EventFrame, EventIndexField,
        EventOverflowNotice, EventOverflowReason, EventStreamScope, PayloadAccessMode,
        SubscriberOverflowPolicy, SubscriberQueueConfig, SubscriptionOptions,
    },
    ids::SpanId,
    policy::{ContentCaptureMode as PolicyContentCaptureMode, EffectClass},
    telemetry::{
        TelemetryContentCaptureRequest, TelemetryFanout, TelemetryFanoutConfig,
        TelemetryOverflowPolicy, evaluate_content_capture, terminal_run_projection_from_event,
    },
    telemetry_ports::TelemetrySinkSpec,
    telemetry_records::{
        TELEMETRY_SCHEMA_VERSION, TelemetryContentCaptureMode, TelemetryProjection,
        TelemetryProjectionId, TelemetryProjectionKind, TelemetryRecordPayload,
        TelemetrySourceCursor, TelemetrySourceRecord,
    },
    testing::{FakeContentResolver, ScriptedTelemetrySink, normalize_json_value, read_fixture},
    tool_records::{CanonicalToolName, ToolCallRecord, ToolCallRecordParams, ToolCallRecordStatus},
};
use serde::Serialize;
use serde_json::json;

const RAW_SENTINEL: &str = "RAW_PRIVACY_SENTINEL_DO_NOT_EXPORT";

#[test]
fn redaction_matrix_covers_default_and_opt_in_paths() {
    let event = model_delta_event(1);
    assert_eq!(event.envelope.content_capture, EventContentCaptureMode::Off);
    assert_no_raw_sentinel(&event);

    let journal_record = message_journal_record();
    assert_eq!(journal_record.privacy, PrivacyClass::ContentRefsOnly);
    assert_no_raw_sentinel(&journal_record);

    let (telemetry_default, telemetry_raw_opt_in) = telemetry_raw_capture_pair();
    assert_eq!(
        telemetry_default.raw_content, None,
        "default telemetry sink must strip raw content"
    );
    assert_eq!(
        telemetry_default.content_capture,
        TelemetryContentCaptureMode::RedactedSummary
    );
    assert_eq!(
        telemetry_raw_opt_in.raw_content.as_deref(),
        Some(RAW_SENTINEL),
        "raw telemetry is allowed only for an explicitly opted-in sink"
    );

    let context = context_resolution_pair();
    assert!(!context.redacted.raw_content_included);
    assert_eq!(context.redacted.bytes, None);
    assert!(context.raw.raw_content_included);
    assert_eq!(context.raw.bytes.as_deref(), Some(RAW_SENTINEL.as_bytes()));

    let stream_delta = StreamDelta::visible_text(
        "stream.delta.privacy.1",
        StreamChannel::AssistantText,
        StreamCursor::chunk(1),
        RAW_SENTINEL,
        source(SourceKind::Sdk, "source.stream.privacy"),
    );
    assert!(stream_delta.serialized_raw_text_absent());
    let stream_intervention = stream_intervention_for_raw_text();
    assert_no_raw_sentinel(&stream_intervention);

    let tool_record = tool_record_refs_only();
    assert_eq!(tool_record.status, ToolCallRecordStatus::Completed);
    assert_eq!(
        tool_record.requested_args_refs,
        vec![ContentRefId::new("content.tool.args.secret")]
    );
    assert_no_raw_sentinel(&tool_record);

    let output = validated_output_refs_only();
    let publication = TypedResultPublicationRecord::published(&output).expect("publication");
    assert_eq!(output.privacy, PrivacyClass::ContentRefsOnly);
    assert_no_raw_sentinel(&output);
    assert_no_raw_sentinel(&publication);

    let matrix = normalize_json_value(json!({
        "event_default": {
            "content_capture": event.envelope.content_capture,
            "privacy": event.envelope.privacy,
            "raw_content_absent": !contains_raw_sentinel(&event),
        },
        "journal_default": {
            "content_refs_only": journal_record.privacy == PrivacyClass::ContentRefsOnly,
            "raw_content_absent": !contains_raw_sentinel(&journal_record),
        },
        "telemetry": {
            "default_strips_raw": telemetry_default.raw_content.is_none(),
            "default_effective_capture": telemetry_default.content_capture,
            "raw_opt_in_keeps_raw": telemetry_raw_opt_in.raw_content.as_deref() == Some(RAW_SENTINEL),
        },
        "context": {
            "default_redacted_has_bytes": context.redacted.bytes.is_some(),
            "default_raw_content_included": context.redacted.raw_content_included,
            "raw_opt_in_has_bytes": context.raw.bytes.is_some(),
            "raw_opt_in_raw_content_included": context.raw.raw_content_included,
        },
        "stream_rule": {
            "serialized_delta_raw_absent": stream_delta.serialized_raw_text_absent(),
            "intervention_raw_absent": !contains_raw_sentinel(&stream_intervention),
        },
        "tool": {
            "args_are_refs": tool_record.requested_args_refs.len(),
            "result_is_refs": tool_record.result_content_refs.len(),
            "raw_content_absent": !contains_raw_sentinel(&tool_record),
        },
        "output": {
            "validated_output_privacy": output.privacy,
            "validated_output_raw_absent": !contains_raw_sentinel(&output),
            "publication_raw_absent": !contains_raw_sentinel(&publication),
        }
    }));

    assert_eq!(
        matrix,
        read_fixture("tests/fixtures/privacy/redaction-matrix.json")
            .expect("redaction matrix fixture")
    );
}

#[test]
fn telemetry_raw_content_requires_all_gates_and_sink_permission() {
    let raw_sink = TelemetrySinkSpec::safe_local_diagnostic("telemetry.raw.privacy")
        .with_content_capture(TelemetryContentCaptureMode::RawContent);
    let allowed = evaluate_content_capture(&TelemetryContentCaptureRequest {
        policy: raw_policy(),
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

    for (label, source_permits, retention_active, sampled, bytes, sink) in [
        ("source_denied", false, true, true, 64, raw_sink.clone()),
        ("retention_expired", true, false, true, 64, raw_sink.clone()),
        ("sample_denied", true, true, false, 64, raw_sink.clone()),
        ("byte_limit_zero", true, true, true, 0, raw_sink.clone()),
        (
            "sink_denied",
            true,
            true,
            true,
            64,
            raw_sink.with_content_capture(TelemetryContentCaptureMode::Off),
        ),
    ] {
        let denied = evaluate_content_capture(&TelemetryContentCaptureRequest {
            policy: raw_policy(),
            sink,
            requested_mode: TelemetryContentCaptureMode::RawContent,
            source_permits_content: source_permits,
            retention_active,
            deterministic_sample_included: sampled,
            requested_bytes: bytes,
            redaction_policy_id: format!("redaction.telemetry.{label}"),
        });

        assert!(!denied.allowed, "{label} should deny raw telemetry");
        assert_eq!(
            denied.effective_mode,
            TelemetryContentCaptureMode::RedactedSummary
        );
    }
}

#[test]
fn event_filters_are_envelope_only_and_do_not_need_content_store_or_journal() {
    let filter = EventFilter {
        run_ids: EventFilterSet::Include(vec![RunId::new("run.privacy")]),
        families: EventFilterSet::Include(vec![EventFamily::Model]),
        kinds: EventFilterSet::Include(vec![EventKind::ModelStreamDelta]),
        privacy_classes: EventFilterSet::Include(vec![PrivacyClass::ContentRefsOnly]),
        payload_access: PayloadAccessMode::EnvelopeOnly,
        queue: SubscriberQueueConfig {
            capacity: NonZeroUsize::new(4).expect("nonzero capacity"),
            terminal_reserve: NonZeroUsize::new(1).expect("nonzero terminal reserve"),
            overflow: SubscriberOverflowPolicy::DropNonTerminal,
        },
        ..EventFilter::default()
    }
    .compile()
    .expect("filter compiles");

    let event_with_raw_payload = AgentEvent::with_redacted_summary(
        event_envelope(77, EventFamily::Model, EventKind::ModelStreamDelta),
        RAW_SENTINEL,
    );
    let store_probe = FakeContentResolver::default();
    let journal_probe = Vec::<JournalRecord>::new();

    assert!(filter.matches_envelope(&event_with_raw_payload.envelope));
    assert_eq!(filter.payload_access, PayloadAccessMode::EnvelopeOnly);
    assert!(filter.indexed_fields.contains(&EventIndexField::RunId));
    assert!(filter.indexed_fields.contains(&EventIndexField::EventKind));
    assert!(filter.indexed_fields.contains(&EventIndexField::Privacy));

    let hot_path_audit = normalize_json_value(json!({
        "matched_envelope": true,
        "payload_access": filter.payload_access,
        "indexed_fields": filter.indexed_fields,
        "content_store_lookup_count": store_probe.resolve_count(),
        "journal_scan_count": journal_probe.len(),
        "payload_parse_count": 0,
        "queue": {
            "capacity": filter.queue.capacity.get(),
            "terminal_reserve": filter.queue.terminal_reserve.get(),
            "overflow": filter.queue.overflow,
        }
    }));

    assert_no_raw_sentinel(&hot_path_audit);
    assert_eq!(
        hot_path_audit,
        read_fixture("tests/fixtures/privacy/hot-path-event-filter.json")
            .expect("hot path fixture")
    );
}

#[test]
fn event_subscriber_queue_is_bounded_and_preserves_terminal() {
    let bus = InMemoryAgentEventBus::default();
    bus.publish_all([
        frame(
            model_delta_event(1),
            EventStreamScope::Run(RunId::new("run.privacy")),
        ),
        frame(
            model_delta_event(2),
            EventStreamScope::Run(RunId::new("run.privacy")),
        ),
        frame(
            model_delta_event(3),
            EventStreamScope::Run(RunId::new("run.privacy")),
        ),
        terminal_event_frame(4),
    ])
    .expect("publish events");

    let frames = bus
        .subscribe_run_with_options(
            RunId::new("run.privacy"),
            None,
            SubscriptionOptions {
                queue: SubscriberQueueConfig {
                    capacity: NonZeroUsize::new(2).expect("nonzero capacity"),
                    terminal_reserve: NonZeroUsize::new(1).expect("nonzero terminal reserve"),
                    overflow: SubscriberOverflowPolicy::DropNonTerminal,
                },
                payload_access: PayloadAccessMode::EnvelopeOnly,
            },
        )
        .expect("bounded subscription")
        .collect::<Vec<_>>();

    assert_eq!(frames.len(), 2);
    assert_eq!(
        frames[0].event.envelope.event_kind,
        EventKind::ModelStreamDelta
    );
    assert_eq!(frames[1].event.envelope.event_kind, EventKind::RunCompleted);
    let overflow = frames[1]
        .overflow
        .as_ref()
        .expect("terminal frame carries overflow notice");
    assert_eq!(overflow.policy, SubscriberOverflowPolicy::DropNonTerminal);
    assert_eq!(overflow.dropped_count, 2);
    assert!(overflow.terminal_preserved);
    assert_eq!(
        overflow.reason,
        EventOverflowReason::PolicyDroppedNonTerminal
    );

    let audit = normalize_json_value(json!({
        "schema_version": 1,
        "bounded_capacity": 2,
        "delivered_kinds": frames
            .iter()
            .map(|frame| frame.event.envelope.event_kind.clone())
            .collect::<Vec<_>>(),
        "overflow": overflow,
        "raw_content_absent": !contains_raw_sentinel(&frames),
    }));
    assert_eq!(
        audit,
        read_fixture("tests/fixtures/privacy/event-subscriber-overflow.json")
            .expect("event subscriber overflow fixture")
    );
}

#[test]
fn event_subscriber_overflow_policies_apply_distinct_semantics() {
    let drop_progress = subscribe_with_overflow_policy(SubscriberOverflowPolicy::DropProgress)
        .expect("drop progress subscription")
        .collect::<Vec<_>>();
    assert_eq!(drop_progress.len(), 2);
    assert_eq!(
        drop_progress[0].event.envelope.event_kind,
        EventKind::MessageAccepted
    );
    assert_eq!(
        drop_progress[1].event.envelope.event_kind,
        EventKind::RunCompleted
    );
    let drop_progress_notice = drop_progress[0]
        .overflow
        .as_ref()
        .expect("non-progress frame carries progress drop notice");
    assert_eq!(
        drop_progress_notice.policy,
        SubscriberOverflowPolicy::DropProgress
    );
    assert_eq!(drop_progress_notice.dropped_count, 2);
    assert_eq!(
        drop_progress_notice.reason,
        EventOverflowReason::PolicyDroppedProgress
    );
    assert!(drop_progress_notice.terminal_preserved);

    let summarized = subscribe_with_overflow_policy(SubscriberOverflowPolicy::SummarizeAndContinue)
        .expect("summarize subscription")
        .collect::<Vec<_>>();
    assert_eq!(summarized.len(), 2);
    assert_eq!(
        summarized[0].event.envelope.event_kind,
        EventKind::ModelStreamDelta
    );
    assert!(
        summarized[0]
            .event
            .redacted_summary()
            .expect("summary frame")
            .contains("dropped progress frames")
    );
    let summary_notice = summarized[0]
        .overflow
        .as_ref()
        .expect("summary frame carries overflow notice");
    assert_eq!(
        summary_notice.policy,
        SubscriberOverflowPolicy::SummarizeAndContinue
    );
    assert_eq!(summary_notice.dropped_count, 2);
    assert_eq!(
        summary_notice.reason,
        EventOverflowReason::PolicyDroppedProgress
    );
    assert_eq!(
        summarized[1].event.envelope.event_kind,
        EventKind::RunCompleted
    );

    let failed = subscribe_with_overflow_policy(SubscriberOverflowPolicy::FailSubscriber)
        .expect("fail subscriber subscription")
        .collect::<Vec<_>>();
    assert_eq!(failed.len(), 1);
    let failed_notice = failed[0]
        .overflow
        .as_ref()
        .expect("final frame carries fail-subscriber overflow notice");
    assert_eq!(
        failed_notice.policy,
        SubscriberOverflowPolicy::FailSubscriber
    );
    assert_eq!(failed_notice.dropped_count, 1);
    assert_eq!(
        failed_notice.reason,
        EventOverflowReason::SubscriberQueueFull
    );

    let backpressure_error =
        subscribe_with_overflow_policy(SubscriberOverflowPolicy::BackpressureCaller)
            .expect_err("live event bus rejects backpressure policy");
    assert!(
        backpressure_error
            .context()
            .message
            .contains("InvalidOverflowPolicy")
    );
}

#[test]
fn bounded_telemetry_fanout_isolates_slow_sink_and_preserves_terminal() {
    let slow = Arc::new(ScriptedTelemetrySink::new(
        TelemetrySinkSpec::test(
            "telemetry.slow.privacy",
            NonZeroUsize::new(2).expect("nonzero capacity"),
        )
        .with_terminal_reserve(NonZeroUsize::new(1).expect("nonzero reserve")),
    ));
    let healthy = Arc::new(ScriptedTelemetrySink::new(TelemetrySinkSpec::test(
        "telemetry.healthy.privacy",
        NonZeroUsize::new(4).expect("nonzero capacity"),
    )));
    let slow_id = slow.sink_spec().sink_id.clone();
    let healthy_id = healthy.sink_spec().sink_id.clone();
    let mut fanout_config = TelemetryFanoutConfig::safe_defaults();
    fanout_config.queue_capacity = NonZeroUsize::new(8).expect("nonzero queue capacity");
    fanout_config.terminal_reserve = NonZeroUsize::new(1).expect("nonzero terminal reserve");
    fanout_config.overflow = TelemetryOverflowPolicy::DropNonTerminalProgress;
    let mut fanout = TelemetryFanout::new(fanout_config);
    fanout.register_sink(slow).expect("slow sink");
    fanout.register_sink(healthy.clone()).expect("healthy sink");

    let first = fanout.try_record(progress_projection("telemetry.progress.privacy.1"));
    let second = fanout.try_record(progress_projection("telemetry.progress.privacy.2"));
    let terminal = fanout.try_record(terminal_projection());

    let slow_queue = fanout.queued_for_sink(&slow_id);
    let healthy_queue = fanout.queued_for_sink(&healthy_id);
    let healthy_drain = fanout.drain_sink(&healthy_id).expect("healthy drain");

    assert_eq!(first.enqueued, 2);
    assert_eq!(second.dropped, 1, "slow sink drops one progress frame");
    assert_eq!(terminal.enqueued, 2);
    assert_eq!(slow_queue.len(), 2);
    assert!(
        slow_queue
            .iter()
            .any(|projection| projection.projection_kind == TelemetryProjectionKind::RunTerminal)
    );
    assert_eq!(healthy_queue.len(), 3);
    assert_eq!(healthy_drain.exported, 3);
    assert_eq!(healthy.exports().len(), 3);
    assert!(terminal.records.iter().all(|record| match &record.payload {
        TelemetryRecordPayload::SinkFailed(failure) => failure.terminal_preserved,
        _ => true,
    }));
}

#[test]
fn overflow_notices_and_failure_records_preserve_terminal_repair_cursor() {
    let terminal = terminal_event_frame(9);
    let overflow = EventOverflowNotice {
        policy: SubscriberOverflowPolicy::DropNonTerminal,
        dropped_count: 5,
        gap_start: None,
        gap_end: terminal.cursor.clone(),
        repair_from: Some(JournalCursor::new("journal.terminal.9")),
        terminal_preserved: true,
        reason: EventOverflowReason::PolicyDroppedNonTerminal,
    };

    let sink = Arc::new(ScriptedTelemetrySink::new(
        TelemetrySinkSpec::test(
            "telemetry.terminal.overflow",
            NonZeroUsize::new(2).expect("nonzero capacity"),
        )
        .with_terminal_reserve(NonZeroUsize::new(1).expect("nonzero reserve")),
    ));
    let sink_id = sink.sink_spec().sink_id.clone();
    let mut fanout = TelemetryFanout::new(TelemetryFanoutConfig::tiny_for_tests());
    fanout.register_sink(sink).expect("sink registers");
    fanout.try_record(progress_projection("telemetry.progress.before_terminal"));
    fanout.try_record(terminal_projection());
    let report = fanout.try_record(terminal_projection());
    let queued = fanout.queued_for_sink(&sink_id);

    let TelemetryRecordPayload::SinkFailed(failure) = &report.records[0].payload else {
        panic!("terminal overflow should produce a sink failure repair record");
    };
    assert!(failure.terminal_preserved);
    assert!(matches!(
        failure.repair_cursor,
        Some(TelemetrySourceCursor::Journal(_))
    ));
    assert_eq!(queued.len(), 2);
    assert_eq!(
        queued[0].projection_kind,
        TelemetryProjectionKind::RunTerminal
    );

    let audit = normalize_json_value(json!({
        "event_overflow": overflow,
        "telemetry_failure": failure,
        "queued_projection_kind": queued[0].projection_kind,
    }));
    assert_eq!(
        audit,
        read_fixture("tests/fixtures/privacy/terminal-overflow-preservation.json")
            .expect("terminal overflow fixture")
    );
}

struct ContextResolutionPair {
    redacted: agent_sdk_core::ResolvedContent,
    raw: agent_sdk_core::ResolvedContent,
}

fn telemetry_raw_capture_pair() -> (TelemetryProjection, TelemetryProjection) {
    let default_sink = Arc::new(ScriptedTelemetrySink::new(TelemetrySinkSpec::test(
        "telemetry.default.redacted",
        NonZeroUsize::new(8).expect("nonzero"),
    )));
    let raw_sink = Arc::new(ScriptedTelemetrySink::new(
        TelemetrySinkSpec::test(
            "telemetry.raw.opt_in",
            NonZeroUsize::new(8).expect("nonzero"),
        )
        .with_content_capture(TelemetryContentCaptureMode::RawContent),
    ));
    let default_id = default_sink.sink_spec().sink_id.clone();
    let raw_id = raw_sink.sink_spec().sink_id.clone();
    let mut fanout = TelemetryFanout::new(TelemetryFanoutConfig::safe_defaults());
    fanout.register_sink(default_sink).expect("default sink");
    fanout.register_sink(raw_sink).expect("raw sink");

    let mut projection = progress_projection("telemetry.progress.raw.capture");
    projection.content_capture = TelemetryContentCaptureMode::RawContent;
    projection.raw_content = Some(RAW_SENTINEL.to_string());
    fanout.try_record(projection);

    (
        fanout
            .queued_for_sink(&default_id)
            .pop()
            .expect("default projection"),
        fanout
            .queued_for_sink(&raw_id)
            .pop()
            .expect("raw projection"),
    )
}

fn context_resolution_pair() -> ContextResolutionPair {
    let content_ref = stored_content_ref(
        "content.context.raw.secret",
        ContentKind::Document,
        "private context summary",
    );
    let resolver = FakeContentResolver::default();
    resolver.insert_text(&content_ref, RAW_SENTINEL);

    let redacted = resolver
        .resolve(
            ContentResolveRequest::new(content_ref.clone()),
            ContentResolutionPolicy::redacted_context(
                producer_ref(),
                destination(DestinationKind::Provider, "destination.provider.context"),
                policy(PolicyKind::Context, "policy.context.redacted"),
            ),
        )
        .expect("redacted context resolution");
    let raw = resolver
        .resolve(
            ContentResolveRequest::new(content_ref),
            ContentResolutionPolicy::raw_context(
                producer_ref(),
                destination(DestinationKind::Provider, "destination.provider.context"),
                policy(PolicyKind::Context, "policy.context.raw"),
                1024,
            ),
        )
        .expect("raw context resolution");

    let contribution = ContextContribution::new(
        ContextContributionId::new("context.contribution.privacy"),
        ContextContributionKind::HostContext,
        producer_ref(),
        source(SourceKind::Host, "source.context.privacy"),
        policy(PolicyKind::Context, "policy.context.projection"),
        "private context summary",
    )
    .with_content_ref(stored_content_ref(
        "content.context.projected",
        ContentKind::Document,
        "projected context summary",
    ));
    let item = ContextItem::admit(
        contribution,
        "context.item.privacy".into(),
        destination(DestinationKind::Provider, "destination.provider.context"),
        ProjectionRole::User,
    );
    let projection = ContextProjection::build(
        "context.projection.privacy".into(),
        Vec::new(),
        vec![item],
        vec![ContextSelectionDecision::omitted(
            &ContextContribution::new(
                ContextContributionId::new("context.contribution.omitted"),
                ContextContributionKind::MemoryRecall,
                producer_ref(),
                source(SourceKind::Memory, "source.memory.privacy"),
                policy(PolicyKind::Context, "policy.context.omitted"),
                "omitted memory summary",
            ),
            agent_sdk_core::ContextSelectionReason::OmittedPolicy,
        )],
        destination(DestinationKind::Provider, "destination.provider.context"),
        ContextBudgetSummary::default(),
        policy(PolicyKind::Redaction, "policy.redaction.context"),
        "sha256:context.privacy",
    )
    .expect("context projection");

    assert!(
        projection
            .projected_parts
            .iter()
            .all(|part| !part.raw_content_included)
    );
    assert_no_raw_sentinel(&projection.projected_parts);
    assert_no_raw_sentinel(&projection.audit);

    ContextResolutionPair { redacted, raw }
}

fn stream_intervention_for_raw_text() -> agent_sdk_core::StreamIntervention {
    let rule = StreamRule::builder(agent_sdk_core::StreamRuleId::new("rule.privacy.mask"))
        .source(source(SourceKind::Host, "source.host.stream_rules"))
        .matcher(StreamMatcher::literal(RAW_SENTINEL, true, 256))
        .on(StreamChannel::AssistantText)
        .action(StreamAction::mask_and_continue("[redacted]"))
        .policy(policy(PolicyKind::Privacy, "policy.stream.privacy"))
        .build()
        .expect("stream rule builds");
    let mut engine = StreamRuleEngine::new(vec![rule]).expect("engine");
    engine
        .observe_delta(StreamDelta::visible_text(
            "stream.delta.privacy.2",
            StreamChannel::AssistantText,
            StreamCursor::chunk(1),
            RAW_SENTINEL,
            source(SourceKind::Sdk, "source.stream.privacy"),
        ))
        .expect("stream observed")
        .pop()
        .expect("intervention")
}

fn tool_record_refs_only() -> ToolCallRecord {
    let mut result = EffectResult::completed(
        EffectId::new("effect.tool.privacy"),
        "tool result stored behind content refs",
    );
    result.content_refs = vec![ContentRefId::new("content.tool.result.secret")];

    ToolCallRecord::requested(ToolCallRecordParams {
        tool_call_id: ToolCallId::new("tool.call.privacy"),
        run_id: RunId::new("run.privacy"),
        turn_id: None,
        capability_id: CapabilityId::new("capability.tool.privacy"),
        canonical_tool_name: CanonicalToolName::new("privacy_read"),
        namespace: CapabilityNamespace::new("privacy"),
        source: source(SourceKind::Tool, "source.tool.privacy"),
        destination: destination(DestinationKind::Tool, "destination.tool.privacy"),
        executor_ref: Some(ExecutorRef::new("executor.tool.privacy")),
        policy_refs: vec![policy(PolicyKind::Approval, "policy.tool.privacy")],
        sidecar_refs: vec![PackageSidecarRef::new("sidecar.tool.privacy", "tool", "v1")],
        effect_class: EffectClass::Read,
        risk_class: RiskClass::Low,
        privacy: PrivacyClass::ContentRefsOnly,
        retention: RetentionClass::RunScoped,
        requested_args_refs: vec![ContentRefId::new("content.tool.args.secret")],
        redacted_args_summary: "tool arguments are stored behind content refs".to_string(),
        idempotency_key: Some(IdempotencyKey::new("idempotency.tool.privacy")),
    })
    .with_intent(EffectIntent::new(
        EffectId::new("effect.tool.privacy"),
        EffectKind::ToolExecution,
        EntityRef::new(EntityKind::ToolCall, ToolCallId::new("tool.call.privacy")),
        source(SourceKind::Tool, "source.tool.privacy"),
        "execute privacy tool with redacted arguments",
    ))
    .with_result(result, allow_outcome(PolicyStage::PostTool))
}

fn validated_output_refs_only() -> ValidatedOutput {
    let report = validation_report();
    ValidatedOutput::from_validation_report(validated_output_params(), &report)
        .expect("validated output refs-only")
}

fn validation_report() -> ValidationReportRecord {
    let mut report = ValidationReportRecord::passed(
        ValidationAttemptId::new("validation.attempt.privacy"),
        schema_id(),
        SchemaVersion::new(1, 0, 0),
        agent_sdk_core::AttemptId::new("attempt.output.privacy"),
        stored_content_ref(
            "content.output.candidate.secret",
            ContentKind::OutputPayload,
            "candidate output stored by content ref",
        ),
        stored_content_ref(
            "content.validation.report.privacy",
            ContentKind::Document,
            "validation report redacted summary",
        ),
        "validation passed with redacted report only",
    );
    report.policy_refs = vec![policy(PolicyKind::Privacy, "policy.output.privacy")];
    report
}

fn validated_output_params() -> ValidatedOutputParams {
    ValidatedOutputParams {
        output_id: ValidatedOutputId::new("validated.output.privacy"),
        schema_id: schema_id(),
        schema_version: SchemaVersion::new(1, 0, 0),
        schema_fingerprint: ContentHash::new(
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ),
        canonical_value_ref: stored_content_ref(
            "content.output.canonical.secret",
            ContentKind::OutputPayload,
            "validated output canonical value ref",
        ),
        repair_attempts: Vec::new(),
        source_attempt_ids: Vec::new(),
        content_refs: Vec::new(),
        lineage: OutputLineage {
            lineage_ref: LineageRef {
                lineage_id: LineageId::new("lineage.output.privacy"),
                source: source(SourceKind::Sdk, "source.output.validator"),
                destination: Some(destination(
                    DestinationKind::Host,
                    "destination.output.consumer",
                )),
                policy_refs: vec![policy(PolicyKind::Privacy, "policy.output.privacy")],
            },
            produced_by: EntityRef::new(
                EntityKind::Attempt,
                agent_sdk_core::AttemptId::new("attempt.output.privacy"),
            ),
            derived_from: vec![EntityRef::new(
                EntityKind::Content,
                "content.output.candidate.secret",
            )],
        },
        policy_refs: vec![policy(PolicyKind::Privacy, "policy.output.privacy")],
        privacy: PrivacyClass::ContentRefsOnly,
        redacted_summary: "validated output with refs only".to_string(),
    }
}

fn message_journal_record() -> JournalRecord {
    let mut base = journal_base(1, "journal.record.message.privacy");
    base.privacy = PrivacyClass::ContentRefsOnly;
    JournalRecord::feature_record(
        base,
        JournalRecordKind::Message,
        "message",
        "accepted",
        EntityRef::message(MessageId::new("message.privacy")),
        vec![EntityRef::new(
            EntityKind::Content,
            "content.message.secret",
        )],
        vec![ContentRefId::new("content.message.secret")],
        JournalRecordPayload::Message(agent_sdk_core::MessageRecord {
            message_id: MessageId::new("message.privacy"),
            role: "user".to_string(),
            redacted_summary: "user message content redacted by default".to_string(),
        }),
    )
}

fn progress_projection(projection_id: &str) -> TelemetryProjection {
    let event = model_delta_event(1);
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
        destination: Some(destination(
            DestinationKind::Telemetry,
            "destination.telemetry.progress",
        )),
        subject_ref: envelope.subject_ref,
        policy_refs: envelope.policy_refs,
        privacy: envelope.privacy,
        retention: RetentionClass::RunScoped,
        content_capture: TelemetryContentCaptureMode::Off,
        redaction_policy_id: envelope.redaction_policy_id,
        provider_id: Some("provider.fake".to_string()),
        model_id: Some("model.fake".to_string()),
        tool_name: None,
        usage: None,
        cost: None,
        terminal_status: None,
        sink_health: None,
        redacted_summary: "model progress redacted".to_string(),
        raw_content: None,
    }
}

fn terminal_projection() -> TelemetryProjection {
    terminal_run_projection_from_event(run_completed_event(9))
}

fn model_delta_event(seq: u64) -> AgentEvent {
    AgentEvent::with_redacted_summary(
        event_envelope(seq, EventFamily::Model, EventKind::ModelStreamDelta),
        "model delta redacted summary",
    )
}

fn message_accepted_event(seq: u64) -> AgentEvent {
    AgentEvent::envelope_only(event_envelope(
        seq,
        EventFamily::Message,
        EventKind::MessageAccepted,
    ))
}

fn run_completed_event(seq: u64) -> AgentEvent {
    let mut envelope = event_envelope(seq, EventFamily::Run, EventKind::RunCompleted);
    envelope.journal_cursor = Some(JournalCursor::new(format!("journal.terminal.{seq}")));
    envelope.delivery_semantics = EventDeliverySemantics::JournalBacked;
    AgentEvent::envelope_only(envelope)
}

fn subscribe_with_overflow_policy(
    overflow: SubscriberOverflowPolicy,
) -> Result<agent_sdk_core::AgentEventStream, agent_sdk_core::AgentError> {
    let bus = InMemoryAgentEventBus::default();
    bus.publish_all([
        frame(
            model_delta_event(1),
            EventStreamScope::Run(RunId::new("run.privacy")),
        ),
        frame(
            model_delta_event(2),
            EventStreamScope::Run(RunId::new("run.privacy")),
        ),
        frame(
            message_accepted_event(3),
            EventStreamScope::Run(RunId::new("run.privacy")),
        ),
        terminal_event_frame(4),
    ])
    .expect("publish overflow policy events");

    bus.subscribe_run_with_options(
        RunId::new("run.privacy"),
        None,
        SubscriptionOptions {
            queue: SubscriberQueueConfig {
                capacity: NonZeroUsize::new(2).expect("nonzero capacity"),
                terminal_reserve: NonZeroUsize::new(1).expect("nonzero terminal reserve"),
                overflow,
            },
            payload_access: PayloadAccessMode::EnvelopeOnly,
        },
    )
}

fn terminal_event_frame(seq: u64) -> EventFrame {
    let event = run_completed_event(seq);
    let cursor = event
        .envelope
        .cursor(EventStreamScope::Run(RunId::new("run.privacy")));
    EventFrame {
        event,
        cursor,
        archive_cursor: None,
        overflow: None,
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

fn event_envelope(seq: u64, family: EventFamily, kind: EventKind) -> EventEnvelope {
    let run_id = RunId::new("run.privacy");
    EventEnvelope {
        schema_version: EVENT_SCHEMA_VERSION,
        event_id: EventId::new(format!("event.privacy.{seq}")),
        event_seq: seq,
        event_family: family,
        event_kind: kind,
        payload_schema_version: 1,
        timestamp: "2026-05-24T00:00:00Z".to_string(),
        recorded_at: "2026-05-24T00:00:00Z".to_string(),
        run_id: run_id.clone(),
        session_id: None,
        agent_id: AgentId::new("agent.privacy"),
        turn_id: None,
        attempt_id: None,
        message_id: None,
        context_item_id: None,
        trace_id: TraceId::new("trace.privacy"),
        span_id: SpanId::new(format!("span.privacy.{seq}")),
        parent_event_id: None,
        caused_by: None,
        subject_ref: EntityRef::run(run_id),
        related_refs: Vec::new(),
        causal_refs: Vec::new(),
        correlation: EventCorrelation::default(),
        tags: Vec::new(),
        source: source(SourceKind::Sdk, "source.agent.loop"),
        destination: Some(destination(
            DestinationKind::EventStream,
            "destination.event.stream",
        )),
        policy_refs: vec![policy(PolicyKind::Privacy, "policy.privacy.default")],
        journal_cursor: None,
        state_before: None,
        state_after: None,
        delivery_semantics: EventDeliverySemantics::BestEffortLive,
        privacy: PrivacyClass::ContentRefsOnly,
        content_capture: EventContentCaptureMode::Off,
        redaction_policy_id: "redaction.default".to_string(),
        runtime_package_fingerprint: "sha256:privacy-runtime-package".to_string(),
    }
}

fn raw_policy() -> agent_sdk_core::ContentCapturePolicy {
    let mut policy = agent_sdk_core::ContentCapturePolicy::safe_defaults(policy(
        PolicyKind::Privacy,
        "policy.raw",
    ));
    policy.mode = PolicyContentCaptureMode::RawContent;
    policy.source_permits_content = true;
    policy.sink_permits_content = true;
    policy.redaction_required = true;
    policy.retention_required = true;
    policy.sampling_required = true;
    policy.byte_limit = 1024;
    policy
}

fn allow_outcome(stage: PolicyStage) -> PolicyOutcome {
    PolicyOutcome {
        stage,
        decision: PolicyDecision::allow("policy.allow"),
        subject: None,
        source: Some(source(SourceKind::Sdk, "source.policy")),
        destination: Some(destination(DestinationKind::Host, "destination.policy")),
        policy_refs: vec![policy(PolicyKind::Privacy, "policy.allow")],
        privacy: PrivacyClass::ContentRefsOnly,
        retention: RetentionClass::RunScoped,
    }
}

fn stored_content_ref(id: &str, kind: ContentKind, summary: &str) -> StoredContentRef {
    let mut content_ref = StoredContentRef::new(
        ContentId::new(id),
        ContentVersion::new("v1"),
        kind,
        ContentScope::Run,
        producer_ref(),
        source(SourceKind::Sdk, "source.content.privacy"),
        AdapterRef::new("resolver.content.privacy"),
        summary,
    );
    content_ref.mime = Some("text/plain".to_string());
    content_ref.size_bytes = Some(64);
    content_ref.content_hash =
        Some("sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_string());
    content_ref.privacy_class = PrivacyClass::ContentRefsOnly;
    content_ref.retention_class = RetentionClass::RunScoped;
    content_ref.trust_class = TrustClass::SdkGenerated;
    content_ref
}

fn journal_base(seq: u64, record_id: &str) -> JournalRecordBase {
    JournalRecordBase::new(
        seq,
        record_id,
        RunId::new("run.privacy"),
        AgentId::new("agent.privacy"),
        source(SourceKind::Sdk, "source.journal.privacy"),
    )
}

fn producer_ref() -> EntityRef {
    EntityRef::new(EntityKind::Agent, AgentId::new("agent.privacy"))
}

fn source(kind: SourceKind, id: &str) -> SourceRef {
    SourceRef::with_kind(kind, id)
}

fn destination(kind: DestinationKind, id: &str) -> DestinationRef {
    DestinationRef::with_kind(kind, id)
}

fn policy(kind: PolicyKind, id: &str) -> PolicyRef {
    PolicyRef::with_kind(kind, id)
}

fn schema_id() -> OutputSchemaId {
    OutputSchemaId::new("schema.privacy.output")
}

fn contains_raw_sentinel(value: &impl Serialize) -> bool {
    serde_json::to_string(value)
        .expect("serializes")
        .contains(RAW_SENTINEL)
}

fn assert_no_raw_sentinel(value: &impl Serialize) {
    assert!(
        !contains_raw_sentinel(value),
        "raw sentinel leaked through serialized contract"
    );
}

trait ResolveCount {
    fn resolve_count(&self) -> usize;
}

impl ResolveCount for FakeContentResolver {
    fn resolve_count(&self) -> usize {
        0
    }
}
