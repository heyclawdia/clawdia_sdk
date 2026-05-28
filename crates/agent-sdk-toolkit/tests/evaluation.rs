use std::sync::Arc;

use agent_sdk_core::{
    AgentError, MessageId, RunId, SessionId, SessionTimeline, TurnId, TurnTrace,
    testing::FakeProvider,
};
use agent_sdk_eval::{
    ComparisonDesign, EvaluationBudget, EvaluationConfidence, EvaluationMetricDelta,
    EvaluationReport, EvaluationRequest, EvaluationVerdict, Evaluator, EvidenceBundle,
    ExpectedOutcome,
};
use agent_sdk_toolkit::{AgentTraceEvaluation, AiTraceEvaluator};

#[test]
fn ai_trace_evaluator_validates_cited_support_and_captures_usage() {
    let provider = FakeProvider::with_responses([r#"{
        "verdict":"passed",
        "score":"1.0",
        "redacted_summary":"the available run evidence supports the expected outcome",
        "support_refs":[
          {"kind":"run","id":"run.toolkit.eval"},
          {"kind":"context_item","id":"context.item.not.visible"}
        ],
        "limitations":["mock evaluator"]
    }"#]);
    let evaluator = AiTraceEvaluator::new(Arc::new(provider.clone()));
    let trace = turn_trace();
    let evaluation =
        AgentTraceEvaluation::turn(&trace, ExpectedOutcome::completed()).expect("trace evaluation");

    let report = evaluation.evaluate(&evaluator).expect("AI eval succeeds");

    assert_eq!(report.verdict, agent_sdk_eval::EvaluationVerdict::Passed);
    assert_eq!(report.confidence, EvaluationConfidence::Cited);
    assert_eq!(report.evidence_refs.len(), 1);
    assert_eq!(report.judgments[0].support_refs.len(), 1);
    assert_eq!(report.judgments[0].rejected_support_refs.len(), 1);
    assert_eq!(report.usage.provider_calls, 1);
    assert!(report.usage.provider_usage.unwrap().total_tokens.is_some());

    let requests = provider.requests();
    assert_eq!(
        requests.len(),
        1,
        "post-hoc eval spends exactly one provider call"
    );
    assert_eq!(
        requests[0].projection_item_count,
        evaluation.evidence().items.len()
    );
    let prompt = &requests[0].messages[0].content;
    assert!(prompt.contains("Evaluate this recorded agent outcome"));
    assert!(prompt.contains("run.toolkit.eval"));
    assert!(!prompt.contains("secret final output"));
}

#[test]
fn observed_only_ai_eval_cannot_produce_measured_confidence() {
    let provider = FakeProvider::with_responses([r#"{
        "verdict":"passed",
        "confidence":"measured",
        "redacted_summary":"the evaluator tried to claim measurement",
        "support_refs":[{"kind":"run","id":"run.toolkit.eval"}],
        "metric_deltas":[{"metric_ref":"metric.success","baseline_ref":null,"delta_value":"+1","redacted_summary":"invalid measured claim"}]
    }"#]);
    let evaluator = AiTraceEvaluator::new(Arc::new(provider));
    let evaluation = AgentTraceEvaluation::turn(&turn_trace(), ExpectedOutcome::completed())
        .expect("trace evaluation");

    let report = evaluation.evaluate(&evaluator).expect("AI eval succeeds");

    assert_eq!(report.confidence, EvaluationConfidence::Cited);
    assert!(report.metric_deltas.is_empty());
    assert!(
        report
            .limitations
            .iter()
            .any(|limitation| limitation.contains("downgraded"))
    );
}

#[test]
fn zero_provider_call_budget_blocks_ai_eval_without_calling_provider() {
    let provider = FakeProvider::with_responses([r#"{"verdict":"passed"}"#]);
    let evaluator = AiTraceEvaluator::new(Arc::new(provider.clone()));
    let budget = EvaluationBudget {
        max_provider_calls: 0,
        ..EvaluationBudget::default()
    };
    let evaluation = AgentTraceEvaluation::turn(&turn_trace(), ExpectedOutcome::completed())
        .expect("trace evaluation")
        .with_budget(budget);

    let error = evaluation
        .evaluate(&evaluator)
        .expect_err("zero-call budget rejects AI eval");

    assert!(error.context().message.contains("zero provider calls"));
    assert!(provider.requests().is_empty());
}

#[test]
fn compare_sessions_exposes_metrics_and_defers_provider_calls() {
    let provider = FakeProvider::with_responses([r#"{"verdict":"passed"}"#]);
    let observed = session_timeline(
        "session.toolkit.observed",
        "turn.toolkit.observed",
        "run.toolkit.observed",
    );
    let baseline = session_timeline(
        "session.toolkit.baseline",
        "turn.toolkit.baseline",
        "run.toolkit.baseline",
    );

    let evaluation =
        AgentTraceEvaluation::compare_sessions(&observed, &baseline, ExpectedOutcome::completed())
            .expect("session comparison evaluation");

    assert!(provider.requests().is_empty());
    assert!(evaluation.metrics_comparison().is_some());
    assert!(!evaluation.request().metric_deltas.is_empty());
    assert_eq!(evaluation.metrics().turn_count, 1);
    assert!(
        evaluation
            .request()
            .metric_deltas
            .iter()
            .any(|delta| delta.metric_ref == "trace.provider_call_count")
    );
}

#[test]
fn compare_sessions_allows_measured_ai_confidence_from_deterministic_deltas() {
    let provider = FakeProvider::with_responses([r#"{
        "verdict":"passed",
        "confidence":"measured",
        "redacted_summary":"deterministic comparison metrics support the judgment",
        "support_refs":[{"kind":"run","id":"run.toolkit.observed"}],
        "limitations":["mock evaluator"]
    }"#]);
    let evaluator = AiTraceEvaluator::new(Arc::new(provider.clone()));
    let observed = session_timeline(
        "session.toolkit.observed",
        "turn.toolkit.observed",
        "run.toolkit.observed",
    );
    let baseline = session_timeline(
        "session.toolkit.baseline",
        "turn.toolkit.baseline",
        "run.toolkit.baseline",
    );
    let evaluation =
        AgentTraceEvaluation::compare_sessions(&observed, &baseline, ExpectedOutcome::completed())
            .expect("session comparison evaluation");

    let report = evaluation.evaluate(&evaluator).expect("AI eval succeeds");

    assert_eq!(report.confidence, EvaluationConfidence::Measured);
    assert_eq!(report.metric_deltas, evaluation.request().metric_deltas);
    let prompt = &provider.requests()[0].messages[0].content;
    assert!(prompt.contains("deterministic_metric_deltas"));
    assert!(prompt.contains("trace.provider_call_count"));
}

#[test]
fn trace_eval_wrapper_rejects_bad_custom_measured_evaluator() {
    let observed = session_timeline(
        "session.toolkit.observed",
        "turn.toolkit.observed",
        "run.toolkit.observed",
    );
    let baseline = session_timeline(
        "session.toolkit.baseline",
        "turn.toolkit.baseline",
        "run.toolkit.baseline",
    );
    let evaluation =
        AgentTraceEvaluation::compare_sessions(&observed, &baseline, ExpectedOutcome::completed())
            .expect("session comparison evaluation");

    let error = evaluation
        .evaluate(&BadMeasuredEvaluator)
        .expect_err("wrapper rejects invalid custom evaluator output");

    assert!(
        error
            .context()
            .message
            .contains("comparison must match the evaluation request")
    );
}

#[test]
fn trace_eval_builder_exposes_the_same_evidence_bundle_as_eval_crate() {
    let trace = turn_trace();
    let evaluation =
        AgentTraceEvaluation::turn(&trace, ExpectedOutcome::completed()).expect("trace evaluation");
    let direct_bundle = EvidenceBundle::from_turn_trace(&trace).expect("direct bundle");

    assert_eq!(evaluation.evidence(), &direct_bundle);
    assert_eq!(evaluation.request().subjects.len(), 1);
}

struct BadMeasuredEvaluator;

impl Evaluator for BadMeasuredEvaluator {
    fn evaluate(
        &self,
        request: &EvaluationRequest,
        _evidence: &EvidenceBundle,
    ) -> Result<EvaluationReport, AgentError> {
        Ok(EvaluationReport::new(
            request.evaluation_id.clone(),
            request.scope.clone(),
            ComparisonDesign::ObservedOnly,
            EvaluationVerdict::Passed,
            EvaluationConfidence::Measured,
            "invalid measured report",
        )
        .with_metric_delta(EvaluationMetricDelta::new(
            "trace.provider_call_count",
            "+1",
            "invented by custom evaluator",
        )))
    }
}

fn session_timeline(session_id: &str, turn_id: &str, run_id: &str) -> SessionTimeline {
    SessionTimeline {
        session_id: SessionId::new(session_id),
        turns: vec![TurnTrace {
            session_id: Some(SessionId::new(session_id)),
            turn_id: Some(TurnId::new(turn_id)),
            run_ids: vec![RunId::new(run_id)],
            attempt_ids: Vec::new(),
            message_ids: vec![MessageId::new(format!("message.{turn_id}.input"))],
            context_projection_ids: Vec::new(),
            effect_ids: Vec::new(),
            tool_call_ids: Vec::new(),
            event_indexes: Vec::new(),
            records: Vec::new(),
        }],
    }
}

fn turn_trace() -> TurnTrace {
    TurnTrace {
        session_id: None,
        turn_id: Some(TurnId::new("turn.toolkit.eval")),
        run_ids: vec![RunId::new("run.toolkit.eval")],
        attempt_ids: Vec::new(),
        message_ids: vec![MessageId::new("message.toolkit.eval.input")],
        context_projection_ids: Vec::new(),
        effect_ids: Vec::new(),
        tool_call_ids: Vec::new(),
        event_indexes: Vec::new(),
        records: Vec::new(),
    }
}
