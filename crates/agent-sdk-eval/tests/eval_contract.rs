use agent_sdk_core::{EntityKind, EntityRef, MessageId, RunId, SessionId, TurnId, TurnTrace};
use agent_sdk_eval::{
    ComparisonDesign, EvaluationConfidence, EvaluationId, EvaluationMetricDelta, EvaluationReport,
    EvaluationRequest, EvaluationVerdict, Evaluator, EvidenceBundle, ExpectedOutcome,
    testing::ScriptedEvaluator,
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
    let request = EvaluationRequest::new(
        EvaluationId::new("evaluation.baseline.measured"),
        agent_sdk_eval::EvaluationScope::Run {
            run_id: RunId::new("run.observed"),
        },
        ExpectedOutcome::completed(),
    )
    .with_comparison(comparison.clone());
    let report = EvaluationReport::new(
        request.evaluation_id.clone(),
        request.scope.clone(),
        comparison,
        EvaluationVerdict::Passed,
        EvaluationConfidence::Measured,
        "baseline improved the expected metric",
    )
    .with_metric_delta(
        EvaluationMetricDelta::new("metric.success", "+1", "observed beat baseline")
            .with_baseline_ref(baseline_ref),
    );
    let evaluator = ScriptedEvaluator::new(report);
    let evidence = EvidenceBundle::new(request.scope.clone(), "comparison evidence");

    let report = evaluator
        .evaluate(&request, &evidence)
        .expect("measured baseline report is valid");

    assert_eq!(report.confidence, EvaluationConfidence::Measured);
    assert_eq!(report.metric_deltas.len(), 1);
}
