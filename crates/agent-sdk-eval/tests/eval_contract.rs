use agent_sdk_core::{
    AgentId, AttemptId, CapabilityId, CapabilityNamespace, DestinationKind, DestinationRef,
    EffectClass, EffectId, EffectIntent, EffectKind, EffectResult, EntityKind, EntityRef,
    JournalRecord, JournalRecordBase, JournalRecordKind, JournalRecordPayload, MessageId,
    ModelAttemptRecord, PolicyDecision, PolicyOutcome, PolicyStage, PrivacyClass,
    ProviderStopReason, ProviderUsage, RetentionClass, RiskClass, RunId, SessionId, SourceKind,
    SourceRef, ToolCallId, ToolCallRecord, ToolCallRecordParams, TurnId, TurnTrace,
    tool_call_journal_record,
};
use agent_sdk_eval::{
    ComparisonDesign, EvaluationConfidence, EvaluationId, EvaluationMetricDelta, EvaluationReport,
    EvaluationRequest, EvaluationScope, EvaluationVerdict, Evaluator, EvidenceBundle,
    ExpectedOutcome, TraceMetrics, TraceMetricsComparison, testing::ScriptedEvaluator,
};

#[test]
fn turn_trace_evidence_validates_cited_support_refs() {
    let trace = TurnTrace {
        session_id: Some(SessionId::new("session.eval.validation")),
        turn_id: Some(TurnId::new("turn.eval.validation")),
        run_ids: vec![RunId::new("run.eval.validation")],
        attempt_ids: Vec::new(),
        message_ids: vec![MessageId::new("message.eval.validation.input")],
        context_projection_ids: Vec::new(),
        effect_ids: Vec::new(),
        tool_call_ids: Vec::new(),
        event_indexes: Vec::new(),
        records: Vec::new(),
    };

    let bundle = EvidenceBundle::from_turn_trace(&trace).expect("bundle from trace");
    let validation = bundle.validate_support_refs(
        [
            EntityRef::run(RunId::new("run.eval.validation")),
            EntityRef::new(EntityKind::ContextItem, "context.item.not.visible"),
        ],
        8,
    );

    assert_eq!(validation.accepted_refs.len(), 1);
    assert_eq!(validation.accepted_refs[0].kind, EntityKind::Run);
    assert_eq!(validation.rejected_refs.len(), 1);
    assert_eq!(
        validation.rejected_refs[0].id.as_str(),
        "context.item.not.visible"
    );
}

#[test]
fn scripted_evaluator_rejects_measured_confidence_without_comparison() {
    let request = EvaluationRequest::new(
        EvaluationId::new("evaluation.observed.only"),
        agent_sdk_eval::EvaluationScope::Run {
            run_id: RunId::new("run.observed.only"),
        },
        ExpectedOutcome::completed(),
    );
    let report = EvaluationReport::new(
        request.evaluation_id.clone(),
        request.scope.clone(),
        ComparisonDesign::ObservedOnly,
        EvaluationVerdict::Passed,
        EvaluationConfidence::Measured,
        "invalid measured claim",
    )
    .with_metric_delta(EvaluationMetricDelta::new(
        "metric.success",
        "+1",
        "metric was claimed without comparison evidence",
    ));
    let evaluator = ScriptedEvaluator::new(report);
    let evidence = EvidenceBundle::new(request.scope.clone(), "observed evidence");

    let error = evaluator
        .evaluate(&request, &evidence)
        .expect_err("observed-only measured confidence is rejected");

    assert!(
        error
            .context()
            .message
            .contains("requires a comparison design")
    );
}

#[test]
fn baseline_comparison_allows_measured_confidence_with_metric_delta() {
    let baseline_ref = EntityRef::run(RunId::new("run.baseline"));
    let comparison = ComparisonDesign::BaselineRun {
        baseline_ref: baseline_ref.clone(),
    };
    let metric_delta = EvaluationMetricDelta::new("metric.success", "+1", "observed beat baseline")
        .with_baseline_ref(baseline_ref);
    let request = EvaluationRequest::new(
        EvaluationId::new("evaluation.baseline.measured"),
        agent_sdk_eval::EvaluationScope::Run {
            run_id: RunId::new("run.observed"),
        },
        ExpectedOutcome::completed(),
    )
    .with_comparison(comparison.clone())
    .with_metric_delta(metric_delta);
    let report = EvaluationReport::new(
        request.evaluation_id.clone(),
        request.scope.clone(),
        comparison,
        EvaluationVerdict::Passed,
        EvaluationConfidence::Measured,
        "baseline improved the expected metric",
    );
    let evaluator = ScriptedEvaluator::new(report);
    let evidence = EvidenceBundle::new(request.scope.clone(), "comparison evidence");

    let report = evaluator
        .evaluate(&request, &evidence)
        .expect("measured baseline report is valid");

    assert_eq!(report.confidence, EvaluationConfidence::Measured);
    assert_eq!(report.metric_deltas.len(), 1);
}

#[test]
fn trace_metrics_count_provider_tool_and_elapsed_time() {
    let session_id = SessionId::new("session.metrics.observed");
    let turn_id = TurnId::new("turn.metrics.observed");
    let run_id = RunId::new("run.metrics.observed");
    let tool_call_id = ToolCallId::new("tool.call.metrics.observed");
    let effect_id = EffectId::new("effect.tool.metrics.observed");
    let records = [
        model_attempt_record(
            1,
            100,
            &session_id,
            &turn_id,
            &run_id,
            "attempt.metrics.observed",
            usage(10, 5, 15),
        ),
        tool_intent_record(
            2,
            120,
            &session_id,
            &turn_id,
            &run_id,
            &tool_call_id,
            &effect_id,
        ),
        tool_result_record(
            3,
            170,
            &session_id,
            &turn_id,
            &run_id,
            &tool_call_id,
            &effect_id,
        ),
    ];
    let trace = TurnTrace::from_records(&turn_id, records.iter());

    let metrics = TraceMetrics::from_turn_trace(&trace).expect("turn metrics");

    assert_eq!(metrics.started_at_millis, Some(100));
    assert_eq!(metrics.ended_at_millis, Some(170));
    assert_eq!(metrics.elapsed_ms, Some(70));
    assert_eq!(metrics.record_count, 3);
    assert_eq!(metrics.provider_call_count, 1);
    assert_eq!(metrics.provider_input_tokens, 10);
    assert_eq!(metrics.provider_output_tokens, 5);
    assert_eq!(metrics.provider_total_tokens, 15);
    assert_eq!(metrics.tool_call_count, 1);
    assert_eq!(metrics.tool_completed_count, 1);
    assert_eq!(metrics.tool_total_elapsed_ms, Some(50));
    assert_eq!(metrics.tools[0].started_at_millis, Some(120));
    assert_eq!(metrics.tools[0].ended_at_millis, Some(170));
    assert_eq!(metrics.tools[0].elapsed_ms, Some(50));
}

#[test]
fn tool_elapsed_time_is_unavailable_for_non_monotonic_evidence() {
    let session_id = SessionId::new("session.metrics.non_monotonic");
    let turn_id = TurnId::new("turn.metrics.non_monotonic");
    let run_id = RunId::new("run.metrics.non_monotonic");
    let tool_call_id = ToolCallId::new("tool.call.metrics.non_monotonic");
    let effect_id = EffectId::new("effect.tool.metrics.non_monotonic");
    let records = [
        tool_intent_record(
            1,
            200,
            &session_id,
            &turn_id,
            &run_id,
            &tool_call_id,
            &effect_id,
        ),
        tool_result_record(
            2,
            150,
            &session_id,
            &turn_id,
            &run_id,
            &tool_call_id,
            &effect_id,
        ),
    ];
    let trace = TurnTrace::from_records(&turn_id, records.iter());

    let metrics = TraceMetrics::from_turn_trace(&trace).expect("turn metrics");

    assert_eq!(metrics.tool_call_count, 1);
    assert_eq!(metrics.tool_total_elapsed_ms, None);
    assert_eq!(metrics.tools[0].started_at_millis, Some(200));
    assert_eq!(metrics.tools[0].ended_at_millis, None);
    assert_eq!(metrics.tools[0].elapsed_ms, None);
}

#[test]
fn session_metrics_comparison_computes_deterministic_deltas() {
    let observed_session = SessionId::new("session.metrics.observed");
    let baseline_session = SessionId::new("session.metrics.baseline");
    let observed_turn = TurnId::new("turn.metrics.observed");
    let baseline_turn = TurnId::new("turn.metrics.baseline");
    let observed_run = RunId::new("run.metrics.observed");
    let baseline_run = RunId::new("run.metrics.baseline");
    let observed_tool = ToolCallId::new("tool.call.metrics.observed");
    let baseline_tool = ToolCallId::new("tool.call.metrics.baseline");
    let observed_effect = EffectId::new("effect.tool.metrics.observed");
    let baseline_effect = EffectId::new("effect.tool.metrics.baseline");
    let observed_records = [
        model_attempt_record(
            1,
            100,
            &observed_session,
            &observed_turn,
            &observed_run,
            "attempt.metrics.observed",
            usage(20, 10, 30),
        ),
        tool_intent_record(
            2,
            120,
            &observed_session,
            &observed_turn,
            &observed_run,
            &observed_tool,
            &observed_effect,
        ),
        tool_result_record(
            3,
            180,
            &observed_session,
            &observed_turn,
            &observed_run,
            &observed_tool,
            &observed_effect,
        ),
    ];
    let baseline_records = [
        model_attempt_record(
            1,
            10,
            &baseline_session,
            &baseline_turn,
            &baseline_run,
            "attempt.metrics.baseline",
            usage(8, 4, 12),
        ),
        tool_intent_record(
            2,
            20,
            &baseline_session,
            &baseline_turn,
            &baseline_run,
            &baseline_tool,
            &baseline_effect,
        ),
        tool_result_record(
            3,
            50,
            &baseline_session,
            &baseline_turn,
            &baseline_run,
            &baseline_tool,
            &baseline_effect,
        ),
    ];
    let observed =
        agent_sdk_core::SessionTimeline::from_records(&observed_session, observed_records.iter());
    let baseline =
        agent_sdk_core::SessionTimeline::from_records(&baseline_session, baseline_records.iter());

    let comparison = TraceMetricsComparison::sessions(&observed, &baseline);

    assert_eq!(comparison.observed.provider_total_tokens, 30);
    assert_eq!(comparison.baseline.provider_total_tokens, 12);
    assert_eq!(
        delta_value(&comparison.metric_deltas, "trace.provider_total_tokens"),
        "+18"
    );
    assert_eq!(
        delta_value(&comparison.metric_deltas, "trace.tool_total_elapsed_ms"),
        "+30"
    );
}

#[test]
fn paired_scopes_allow_measured_confidence_with_deterministic_delta() {
    let comparison = ComparisonDesign::PairedScopes {
        observed_scope: EvaluationScope::Session {
            session_id: SessionId::new("session.metrics.observed"),
        },
        comparison_scope: EvaluationScope::Session {
            session_id: SessionId::new("session.metrics.baseline"),
        },
    };
    let metric_delta = EvaluationMetricDelta::new(
        "trace.tool_total_elapsed_ms",
        "-42",
        "observed session spent less tool time than baseline",
    );
    let request = EvaluationRequest::new(
        EvaluationId::new("evaluation.metrics.paired_scopes"),
        EvaluationScope::Session {
            session_id: SessionId::new("session.metrics.observed"),
        },
        ExpectedOutcome::completed(),
    )
    .with_comparison(comparison.clone())
    .with_metric_delta(metric_delta.clone());
    let report = EvaluationReport::new(
        EvaluationId::new("evaluation.metrics.paired_scopes"),
        EvaluationScope::Session {
            session_id: SessionId::new("session.metrics.observed"),
        },
        comparison,
        EvaluationVerdict::Passed,
        EvaluationConfidence::Measured,
        "deterministic session metrics improved",
    )
    .with_metric_delta(metric_delta);

    report
        .validate_confidence_contract_for_request(&request)
        .expect("paired scopes with metric deltas support measured confidence");
}

#[test]
fn request_validation_rejects_evaluator_invented_metric_deltas() {
    let comparison = ComparisonDesign::PairedScopes {
        observed_scope: EvaluationScope::Session {
            session_id: SessionId::new("session.metrics.observed"),
        },
        comparison_scope: EvaluationScope::Session {
            session_id: SessionId::new("session.metrics.baseline"),
        },
    };
    let request = EvaluationRequest::new(
        EvaluationId::new("evaluation.metrics.request_owned"),
        EvaluationScope::Session {
            session_id: SessionId::new("session.metrics.observed"),
        },
        ExpectedOutcome::completed(),
    )
    .with_comparison(comparison.clone())
    .with_metric_delta(EvaluationMetricDelta::new(
        "trace.provider_call_count",
        "+0",
        "request-owned deterministic delta",
    ));
    let report = EvaluationReport::new(
        request.evaluation_id.clone(),
        request.scope.clone(),
        comparison,
        EvaluationVerdict::Passed,
        EvaluationConfidence::Measured,
        "evaluator attempted to invent a metric delta",
    )
    .with_metric_delta(EvaluationMetricDelta::new(
        "trace.tool_total_elapsed_ms",
        "+1",
        "invented by evaluator",
    ));

    let error = report
        .validate_confidence_contract_for_request(&request)
        .expect_err("request-owned validation rejects report-only metric deltas");

    assert!(
        error
            .context()
            .message
            .contains("metric deltas must come from the evaluation request")
    );
}

#[test]
fn request_validation_rejects_mismatched_measured_comparison() {
    let comparison = ComparisonDesign::PairedScopes {
        observed_scope: EvaluationScope::Session {
            session_id: SessionId::new("session.metrics.observed"),
        },
        comparison_scope: EvaluationScope::Session {
            session_id: SessionId::new("session.metrics.baseline"),
        },
    };
    let metric_delta = EvaluationMetricDelta::new(
        "trace.provider_call_count",
        "+0",
        "request-owned deterministic delta",
    );
    let request = EvaluationRequest::new(
        EvaluationId::new("evaluation.metrics.comparison_match"),
        EvaluationScope::Session {
            session_id: SessionId::new("session.metrics.observed"),
        },
        ExpectedOutcome::completed(),
    )
    .with_comparison(comparison)
    .with_metric_delta(metric_delta.clone());
    let report = EvaluationReport::new(
        request.evaluation_id.clone(),
        request.scope.clone(),
        ComparisonDesign::ObservedOnly,
        EvaluationVerdict::Passed,
        EvaluationConfidence::Measured,
        "evaluator attempted to change the comparison design",
    )
    .with_metric_delta(metric_delta);

    let error = report
        .validate_confidence_contract_for_request(&request)
        .expect_err("request-owned validation rejects mismatched comparison");

    assert!(
        error
            .context()
            .message
            .contains("comparison must match the evaluation request")
    );
}

fn model_attempt_record(
    journal_seq: u64,
    timestamp_millis: u64,
    session_id: &SessionId,
    turn_id: &TurnId,
    run_id: &RunId,
    attempt_id: &str,
    usage: ProviderUsage,
) -> JournalRecord {
    let attempt_id = AttemptId::new(attempt_id);
    let mut base = base_record(
        journal_seq,
        timestamp_millis,
        session_id,
        turn_id,
        run_id,
        "model",
    );
    base.attempt_id = Some(attempt_id.clone());
    JournalRecord::feature_record(
        base,
        JournalRecordKind::ModelAttempt,
        "model",
        "completed",
        EntityRef::new(EntityKind::Attempt, attempt_id),
        Vec::new(),
        Vec::new(),
        JournalRecordPayload::ModelAttempt(ModelAttemptRecord {
            provider_route_id: "provider.route.fake".to_string(),
            provider_model_id: "provider.model.fake".to_string(),
            request_message_count: 1,
            stop_reason: Some(ProviderStopReason::EndTurn),
            usage: Some(usage),
        }),
    )
}

fn tool_intent_record(
    journal_seq: u64,
    timestamp_millis: u64,
    session_id: &SessionId,
    turn_id: &TurnId,
    run_id: &RunId,
    tool_call_id: &ToolCallId,
    effect_id: &EffectId,
) -> JournalRecord {
    let record = tool_record(run_id, turn_id, tool_call_id).with_intent(EffectIntent::new(
        effect_id.clone(),
        EffectKind::ToolExecution,
        EntityRef::new(EntityKind::ToolCall, tool_call_id.clone()),
        source(SourceKind::Tool, "source.tool.metrics"),
        "execute metrics tool",
    ));
    tool_call_journal_record(
        base_record(
            journal_seq,
            timestamp_millis,
            session_id,
            turn_id,
            run_id,
            "tool.intent",
        ),
        record,
        "intent_recorded",
    )
}

fn tool_result_record(
    journal_seq: u64,
    timestamp_millis: u64,
    session_id: &SessionId,
    turn_id: &TurnId,
    run_id: &RunId,
    tool_call_id: &ToolCallId,
    effect_id: &EffectId,
) -> JournalRecord {
    let record = tool_record(run_id, turn_id, tool_call_id).with_result(
        EffectResult::completed(effect_id.clone(), "tool completed"),
        allow_outcome(PolicyStage::PostTool),
    );
    tool_call_journal_record(
        base_record(
            journal_seq,
            timestamp_millis,
            session_id,
            turn_id,
            run_id,
            "tool.result",
        ),
        record,
        "completed",
    )
}

fn tool_record(run_id: &RunId, turn_id: &TurnId, tool_call_id: &ToolCallId) -> ToolCallRecord {
    ToolCallRecord::requested(ToolCallRecordParams {
        tool_call_id: tool_call_id.clone(),
        run_id: run_id.clone(),
        turn_id: Some(turn_id.clone()),
        capability_id: CapabilityId::new("capability.metrics.tool"),
        canonical_tool_name: "workspace_read".into(),
        namespace: CapabilityNamespace::new("metrics"),
        source: source(SourceKind::Tool, "source.tool.metrics"),
        destination: destination(DestinationKind::Tool, "destination.tool.metrics"),
        executor_ref: None,
        policy_refs: Vec::new(),
        sidecar_refs: Vec::new(),
        effect_class: EffectClass::Read,
        risk_class: RiskClass::Low,
        privacy: PrivacyClass::ContentRefsOnly,
        retention: RetentionClass::RunScoped,
        requested_args_refs: Vec::new(),
        redacted_args_summary: "read metrics fixture".to_string(),
        idempotency_key: None,
    })
}

fn base_record(
    journal_seq: u64,
    timestamp_millis: u64,
    session_id: &SessionId,
    turn_id: &TurnId,
    run_id: &RunId,
    kind: &str,
) -> JournalRecordBase {
    let mut base = JournalRecordBase::new(
        journal_seq,
        format!("journal.record.metrics.{kind}.{journal_seq}"),
        run_id.clone(),
        AgentId::new("agent.metrics"),
        source(SourceKind::Sdk, "source.sdk.metrics"),
    );
    base.session_id = Some(session_id.clone());
    base.turn_id = Some(turn_id.clone());
    base.timestamp_millis = timestamp_millis;
    base
}

fn usage(input_tokens: u32, output_tokens: u32, total_tokens: u32) -> ProviderUsage {
    ProviderUsage {
        input_tokens: Some(input_tokens),
        output_tokens: Some(output_tokens),
        total_tokens: Some(total_tokens),
    }
}

fn allow_outcome(stage: PolicyStage) -> PolicyOutcome {
    PolicyOutcome {
        stage,
        decision: PolicyDecision::allow("policy.allow"),
        subject: None,
        source: Some(source(SourceKind::Sdk, "source.policy.metrics")),
        destination: Some(destination(
            DestinationKind::Host,
            "destination.policy.metrics",
        )),
        policy_refs: Vec::new(),
        privacy: PrivacyClass::ContentRefsOnly,
        retention: RetentionClass::RunScoped,
    }
}

fn source(kind: SourceKind, id: &str) -> SourceRef {
    SourceRef::with_kind(kind, id)
}

fn destination(kind: DestinationKind, id: &str) -> DestinationRef {
    DestinationRef::with_kind(kind, id)
}

fn delta_value<'a>(deltas: &'a [EvaluationMetricDelta], metric_ref: &str) -> &'a str {
    deltas
        .iter()
        .find(|delta| delta.metric_ref == metric_ref)
        .map(|delta| delta.delta_value.as_str())
        .expect("metric delta is present")
}
