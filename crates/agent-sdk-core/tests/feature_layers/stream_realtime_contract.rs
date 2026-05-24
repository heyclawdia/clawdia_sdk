use std::sync::Arc;

use agent_sdk_core::{
    AgentId, DestinationKind, DestinationRef, EffectId, EffectKind, JournalRecordBase,
    JournalRecordPayload, PolicyKind, PolicyRef, ProviderRouteSnapshot, RealtimeBackpressureAction,
    RealtimeCompletionGate, RealtimeInputFrame, RealtimeMediaKind, RealtimeSessionController,
    RealtimeSessionSidecar, RealtimeSessionStatus, RunId, RuntimePackage, RuntimePackageId,
    SourceKind, SourceRef, StreamAction, StreamChannel, StreamCursor, StreamDelta, StreamDirection,
    StreamIntervention, StreamMatcher, StreamRule, StreamRuleEngine, StreamRuleSidecar,
    domain::ContentRef as ContentRefId,
    testing::{FakeJournalStore, ScriptedRealtimeAdapter, normalize_json_value, read_fixture},
};
use serde_json::{Value, json};

#[test]
fn stream_rule_helper_lowers_to_sidecar_and_changes_package_fingerprint() {
    let helper_rule = StreamRule::mask_regex("rule.mask.secret", "sk-[A-Za-z0-9]{6,}")
        .on(StreamChannel::AssistantText)
        .policy(policy("policy.stream.mask_secret"))
        .build()
        .expect("helper builds canonical rule");

    let explicit_rule = StreamRule::builder(helper_rule.id.clone())
        .source(source("source.host.stream_rules"))
        .matcher(StreamMatcher::regex_with_limits(
            "sk-[A-Za-z0-9]{6,}",
            4096,
            25,
        ))
        .on(StreamChannel::AssistantText)
        .action(StreamAction::mask_and_continue("[redacted]"))
        .policy(policy("policy.stream.mask_secret"))
        .build()
        .expect("explicit rule builds");

    assert_eq!(
        helper_rule, explicit_rule,
        "helper must lower into the same explicit StreamRule contract"
    );

    let sidecar = StreamRuleSidecar::new(
        "sidecar.stream.mask_secret",
        source("source.host.stream_rules"),
        vec![helper_rule],
        policy("policy.redaction.stream"),
        policy("policy.content_capture.off"),
    )
    .expect("stream sidecar validates");
    let sidecar_snapshot = sidecar
        .to_package_sidecar_snapshot()
        .expect("sidecar lowers to package snapshot");

    let base = base_package("package.stream.base");
    let with_stream = RuntimePackage::builder(RuntimePackageId::new("package.stream.base"))
        .agent(agent_sdk_core::AgentSnapshot {
            agent_id: AgentId::new("agent.stream"),
            name: "stream".to_string(),
            default_behavior_refs: Vec::new(),
        })
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake"))
        .sidecar(sidecar_snapshot.clone())
        .build()
        .expect("package builds");

    assert_ne!(
        base.fingerprint().unwrap(),
        with_stream.fingerprint().unwrap(),
        "stream sidecar fields must participate in runtime package fingerprinting"
    );

    let summary = json!({
        "sidecar_kind": sidecar_snapshot.kind,
        "sidecar_version": sidecar_snapshot.version,
        "content_hash_is_sha256": sidecar_snapshot.content_hash.starts_with("sha256:"),
        "policy_refs": sidecar_snapshot.policy_refs.iter().map(|policy| policy.as_str()).collect::<Vec<_>>(),
        "fingerprint_changed": true,
    });
    assert_eq!(
        normalize_json_value(summary),
        read_fixture("tests/fixtures/stream_realtime/stream_sidecar_lowering.json")
            .expect("sidecar fixture")
    );
}

#[test]
fn split_chunk_regex_matches_only_policy_visible_channels() {
    let rule = StreamRule::mask_regex("rule.mask.key", "sk-[A-Za-z0-9]{6,}")
        .on(StreamChannel::AssistantText)
        .policy(policy("policy.stream.mask"))
        .build()
        .expect("rule builds");
    let mut engine = StreamRuleEngine::new(vec![rule]).expect("engine compiles");

    assert!(
        engine
            .observe_delta(hidden_delta("sk-ABCDEF"))
            .expect("hidden channel ignored")
            .is_empty(),
        "hidden chain-of-thought must not be matchable"
    );

    assert!(
        engine
            .observe_delta(assistant_delta(1, "prefix sk-"))
            .expect("first chunk observed")
            .is_empty()
    );
    let interventions = engine
        .observe_delta(assistant_delta(2, "ABCDEF suffix"))
        .expect("split chunk observed");

    assert_eq!(interventions.len(), 1);
    assert_eq!(
        interventions[0].requested_action.action_kind(),
        "mask_and_continue"
    );
    assert_eq!(interventions[0].redacted_match.byte_len, 9);
    assert!(
        !serde_json::to_string(&interventions[0])
            .expect("json")
            .contains("sk-ABCDEF"),
        "intervention payloads must not serialize raw matched content"
    );
    let journal_record =
        agent_sdk_core::StreamRuleRecord::matched(&engine.rules()[0], &interventions[0])
            .to_journal_record(JournalRecordBase::new(
                1,
                "journal.record.stream.rule.match",
                RunId::new("run.stream.rule"),
                AgentId::new("agent.stream"),
                source("source.sdk.stream_rule"),
            ));
    assert!(matches!(
        journal_record.payload,
        JournalRecordPayload::StreamRule(_)
    ));

    assert_eq!(
        intervention_summary(&interventions),
        read_fixture("tests/fixtures/stream_realtime/stream_rule_match_record.json")
            .expect("match fixture")
    );
}

#[test]
fn regex_backtracking_shape_is_rejected_before_runtime_matching() {
    let error = StreamRule::mask_regex("rule.bad.regex", "(a+)+b")
        .on(StreamChannel::AssistantText)
        .policy(policy("policy.stream.mask"))
        .build()
        .expect_err("nested quantifier shape should fail safe regex validation");

    assert!(matches!(
        error,
        agent_sdk_core::AgentError::ContractViolation { .. }
    ));
}

#[test]
fn marker_match_and_repeat_state_restore_are_bounded_and_deduped() {
    let rule = StreamRule::builder(agent_sdk_core::StreamRuleId::new("rule.marker.stop"))
        .source(source("source.host.stream_rules"))
        .matcher(StreamMatcher::marker("marker.stop"))
        .on(StreamChannel::RealtimeTranscript)
        .action(StreamAction::emit_only("marker_seen"))
        .repeat(agent_sdk_core::RepeatPolicy::OncePerRun)
        .policy(policy("policy.stream.marker"))
        .build()
        .expect("marker rule builds");
    let mut engine = StreamRuleEngine::new(vec![rule]).expect("engine compiles");

    let first = engine
        .observe_delta(
            StreamDelta::marker(
                "delta.marker.1",
                StreamChannel::RealtimeTranscript,
                StreamCursor::chunk(1),
                "marker.stop",
                source("source.realtime.fake"),
            )
            .with_run(
                RunId::new("run.stream.marker"),
                AgentId::new("agent.stream"),
            ),
        )
        .expect("marker observed");
    assert_eq!(first.len(), 1);

    let restored = StreamRuleEngine::restore(engine.rules().to_vec(), engine.snapshot_state())
        .expect("repeat state restores");
    let mut restored = restored;
    let second = restored
        .observe_delta(
            StreamDelta::marker(
                "delta.marker.2",
                StreamChannel::RealtimeTranscript,
                StreamCursor::chunk(2),
                "marker.stop",
                source("source.realtime.fake"),
            )
            .with_run(
                RunId::new("run.stream.marker"),
                AgentId::new("agent.stream"),
            ),
        )
        .expect("marker observed after restore");

    assert!(
        second.is_empty(),
        "OncePerRun repeat state must survive snapshot/restore"
    );
}

#[test]
fn stream_intervention_maps_to_existing_effect_shapes_only() {
    let rule = StreamRule::builder(agent_sdk_core::StreamRuleId::new("rule.retry"))
        .source(source("source.host.stream_rules"))
        .matcher(StreamMatcher::literal("retry me", true, 128))
        .on(StreamChannel::AssistantText)
        .action(StreamAction::abort_and_retry(
            "stream rule requested retry context contribution",
            policy("policy.retry.stream"),
        ))
        .policy(policy("policy.stream.retry"))
        .build()
        .expect("retry rule builds");
    let mut engine = StreamRuleEngine::new(vec![rule]).expect("engine compiles");
    let intervention = engine
        .observe_delta(assistant_delta(1, "retry me"))
        .expect("match observed")
        .pop()
        .expect("intervention");

    let intervention = intervention.with_effect_intent(EffectId::new("effect.provider.retry"));
    assert_eq!(
        intervention
            .effect_intent
            .as_ref()
            .expect("effect intent")
            .kind,
        EffectKind::ProviderRequest
    );
    assert_eq!(intervention.effect_kind_name(), Some("provider_request"));
    assert!(!format!("{:?}", intervention).contains("StreamInterventionKind"));

    assert_eq!(
        stream_effect_summary(&intervention),
        read_fixture("tests/fixtures/stream_realtime/stream_intervention_effect_map.json")
            .expect("effect map fixture")
    );
}

#[test]
fn realtime_sidecar_port_restart_interruption_and_backpressure_are_recorded() {
    let sidecar = RealtimeSessionSidecar::voice_defaults(
        "sidecar.realtime.voice",
        "provider.fake",
        "provider.capability.realtime.v1",
        policy("policy.realtime.media"),
    )
    .expect("realtime sidecar validates");
    let adapter = Arc::new(ScriptedRealtimeAdapter::new("adapter.realtime.fake"));
    adapter.push_output(
        agent_sdk_core::RealtimeOutputFrame::transcript(
            "response.1",
            "content.transcript.1",
            "assistant transcript ready",
        )
        .with_cursor(StreamCursor::chunk(1)),
    );
    let journal = FakeJournalStore::default();
    let mut controller = RealtimeSessionController::new(
        sidecar.clone(),
        adapter.clone(),
        Arc::new(journal.clone()),
        RunId::new("run.realtime"),
        AgentId::new("agent.realtime"),
        source("source.host.voice"),
        "runtime.package.fingerprint.realtime",
    );

    let connected = controller.connect().expect("connect records");
    assert_eq!(connected.status, RealtimeSessionStatus::Connected);

    let sent = controller
        .send(RealtimeInputFrame::media_ref(
            RealtimeMediaKind::Audio,
            ContentRefId::new("content.audio.1"),
            "audio frame content ref",
        ))
        .expect("send records");
    let received = controller.receive().expect("receive records").unwrap();
    assert_ne!(sent.send_cursor, received.receive_cursor);
    assert_eq!(
        sent.content_refs,
        vec![ContentRefId::new("content.audio.1")]
    );

    let interrupted = controller
        .interrupt("response.1")
        .expect("interrupt records before adapter cancellation result");
    assert_eq!(interrupted.event_kind_name(), "realtime_interrupted");

    let mut restart_records = controller.begin_restart().expect("restart begins");
    let gated = controller
        .send(RealtimeInputFrame::media_ref(
            RealtimeMediaKind::Audio,
            ContentRefId::new("content.audio.gated"),
            "audio gated during restart",
        ))
        .expect("send during restart is gated");
    assert_eq!(
        gated.backpressure_state.last_action,
        Some(RealtimeBackpressureAction::Gate)
    );
    restart_records.push(gated);
    restart_records.extend(controller.complete_restart().expect("restart completes"));

    assert_eq!(
        adapter.call_names(),
        vec!["connect", "send", "receive", "interrupt", "restart"]
    );
    let journal_payloads = journal
        .records()
        .into_iter()
        .filter_map(|record| match record.payload {
            JournalRecordPayload::RealtimeSession(record) => Some(record.event_kind_name()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        journal_payloads,
        vec![
            "realtime_connect_requested",
            "realtime_connected",
            "realtime_input_send_requested",
            "realtime_input_sent",
            "realtime_output_receive_requested",
            "realtime_output_received",
            "realtime_interrupt_requested",
            "realtime_interrupted",
            "realtime_restart_requested",
            "realtime_restart_started",
            "realtime_backpressure_applied",
            "realtime_restart_completed",
        ]
    );
    assert_eq!(
        realtime_records_summary(&restart_records),
        read_fixture("tests/fixtures/stream_realtime/realtime_restart_backpressure_records.json")
            .expect("realtime fixture")
    );
}

#[test]
fn realtime_adapter_calls_are_journal_gated() {
    let sidecar = RealtimeSessionSidecar::voice_defaults(
        "sidecar.realtime.gated",
        "provider.fake",
        "provider.capability.realtime.v1",
        policy("policy.realtime.media"),
    )
    .expect("realtime sidecar validates");
    let adapter = Arc::new(ScriptedRealtimeAdapter::new("adapter.realtime.fake"));
    let journal = FakeJournalStore::default();
    journal.fail_next_append("journal unavailable before realtime connect");
    let mut controller = RealtimeSessionController::new(
        sidecar,
        adapter.clone(),
        Arc::new(journal),
        RunId::new("run.realtime.gated"),
        AgentId::new("agent.realtime"),
        source("source.host.voice"),
        "runtime.package.fingerprint.realtime",
    );

    let error = controller
        .connect()
        .expect_err("journal failure gates adapter connect");

    assert_eq!(error.kind(), agent_sdk_core::AgentErrorKind::JournalFailure);
    assert!(
        adapter.call_names().is_empty(),
        "adapter must not be called if durable realtime request cannot be appended"
    );
}

#[test]
fn final_visible_text_waits_for_stream_realtime_and_output_drain() {
    let mut gate = RealtimeCompletionGate::default();
    gate.mark_final_visible_output();
    assert!(
        !gate.can_complete_run(),
        "final visible text is renderable but not terminal run completion"
    );

    gate.mark_terminal_event_replayable();
    gate.mark_stream_interventions_terminal();
    gate.mark_realtime_sessions_terminal();
    gate.mark_output_delivery_terminal();
    gate.mark_approvals_terminal();
    gate.mark_journal_terminal();

    assert!(gate.can_complete_run());
    assert_eq!(
        normalize_json_value(json!(gate)),
        read_fixture("tests/fixtures/stream_realtime/completion_after_drain_gate.json")
            .expect("completion gate fixture")
    );
}

#[test]
fn otel_projection_fixture_uses_agent_sdk_fields_without_raw_content() {
    let projection = normalize_json_value(json!({
        "schema_url": "https://opentelemetry.io/schemas/1.41.0",
        "stability_opt_in": "gen_ai_latest_experimental",
        "content_capture": "off",
        "span_events": [
            {
                "name": "stream_rule.intervention",
                "attributes": {
                    "agent_sdk.stream.rule.id": "rule.mask.key",
                    "agent_sdk.stream.channel": "assistant_text",
                    "agent_sdk.stream.action": "mask_and_continue",
                    "agent_sdk.stream.match.redaction": "hash_length_summary"
                }
            },
            {
                "name": "realtime.restart",
                "attributes": {
                    "agent_sdk.realtime.session.id": "realtime.session.run.realtime",
                    "agent_sdk.realtime.restart.count": 1,
                    "agent_sdk.realtime.backpressure.policy": "gate_during_restart"
                }
            }
        ],
        "raw_content_attributes_present": false
    }));

    assert_eq!(
        projection,
        read_fixture("tests/fixtures/stream_realtime/otel/stream_realtime_projection_v1.json")
            .expect("otel projection fixture")
    );
}

fn base_package(package_id: &str) -> RuntimePackage {
    RuntimePackage::builder(RuntimePackageId::new(package_id))
        .agent(agent_sdk_core::AgentSnapshot {
            agent_id: AgentId::new("agent.stream"),
            name: "stream".to_string(),
            default_behavior_refs: Vec::new(),
        })
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake"))
        .build()
        .expect("base package builds")
}

fn assistant_delta(chunk_sequence: u64, text: &str) -> StreamDelta {
    StreamDelta::visible_text(
        format!("delta.assistant.{chunk_sequence}"),
        StreamChannel::AssistantText,
        StreamCursor::chunk(chunk_sequence),
        text,
        source("source.provider.fake"),
    )
    .with_run(RunId::new("run.stream"), AgentId::new("agent.stream"))
    .with_direction(StreamDirection::OutputFromProvider)
    .with_destination(destination_provider())
}

fn hidden_delta(text: &str) -> StreamDelta {
    StreamDelta::visible_text(
        "delta.hidden.1",
        StreamChannel::HiddenChainOfThought,
        StreamCursor::chunk(1),
        text,
        source("source.provider.fake"),
    )
    .with_run(RunId::new("run.stream"), AgentId::new("agent.stream"))
    .with_direction(StreamDirection::OutputFromProvider)
    .with_destination(destination_provider())
}

fn intervention_summary(interventions: &[StreamIntervention]) -> Value {
    normalize_json_value(json!({
        "interventions": interventions.iter().map(|intervention| json!({
            "action": intervention.requested_action.action_kind(),
            "channel": intervention.redacted_match.channel,
            "direction": intervention.redacted_match.direction,
            "byte_len": intervention.redacted_match.byte_len,
            "hash_is_sha256": intervention.redacted_match.text_hash.starts_with("sha256:"),
            "summary": intervention.redacted_match.redacted_summary,
            "raw_match_serialized": serde_json::to_string(intervention).unwrap().contains("sk-ABCDEF"),
        })).collect::<Vec<_>>()
    }))
}

fn stream_effect_summary(intervention: &StreamIntervention) -> Value {
    let intent = intervention.effect_intent.as_ref().expect("effect intent");
    normalize_json_value(json!({
        "action": intervention.requested_action.action_kind(),
        "effect_kind": intent.kind,
        "effect_id": intent.effect_id.as_str(),
        "subject_kind": intent.subject_ref.kind,
        "policy_refs": intent.policy_refs.iter().map(|policy| policy.as_str()).collect::<Vec<_>>(),
        "redacted_summary": intent.redacted_summary,
    }))
}

fn realtime_records_summary(records: &[agent_sdk_core::RealtimeSessionRecord]) -> Value {
    normalize_json_value(json!({
        "records": records.iter().map(|record| json!({
            "kind": record.event_kind_name(),
            "status": record.status,
            "restart_count": record.restart_count,
            "send_cursor": record.send_cursor.chunk_sequence,
            "receive_cursor": record.receive_cursor.chunk_sequence,
            "backpressure_action": record.backpressure_state.last_action,
            "content_refs": record.content_refs.iter().map(|content_ref| content_ref.as_str()).collect::<Vec<_>>(),
            "raw_media_serialized": serde_json::to_string(record).unwrap().contains("audio bytes"),
        })).collect::<Vec<_>>()
    }))
}

fn source(id: &str) -> SourceRef {
    SourceRef::with_kind(SourceKind::Host, id)
}

fn destination_provider() -> DestinationRef {
    DestinationRef::with_kind(DestinationKind::Provider, "destination.provider.fake")
}

fn policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::RuntimePackage, id)
}
