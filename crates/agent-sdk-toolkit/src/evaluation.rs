//! Toolkit helpers for post-hoc agent-run evaluation.

use std::sync::Arc;

use serde::Deserialize;

use agent_sdk_core::{
    AgentError, EntityKind, EntityRef, PrivacyClass, ProviderAdapter, ProviderMessage,
    ProviderMessageRole, ProviderRequest, RunId, RunTrace, SessionTimeline, TurnTrace,
    domain::EntityId,
};
use agent_sdk_eval::{
    ComparisonDesign, EvaluationBudget, EvaluationConfidence, EvaluationId, EvaluationReport,
    EvaluationRequest, EvaluationSubject, EvaluationSubjectRole, EvaluationUsage,
    EvaluationVerdict, Evaluator, EvaluatorJudgment, EvidenceBundle, EvidenceRole, ExpectedOutcome,
    TraceMetrics, TraceMetricsComparison,
};

/// Builder for evaluating an agent trace with an arbitrary evaluator.
#[derive(Clone, Debug)]
pub struct AgentTraceEvaluation {
    request: EvaluationRequest,
    evidence: EvidenceBundle,
    metrics: TraceMetrics,
    metrics_comparison: Option<TraceMetricsComparison>,
}

impl AgentTraceEvaluation {
    /// Builds an evaluation from a turn trace.
    pub fn turn(trace: &TurnTrace, expected_outcome: ExpectedOutcome) -> Result<Self, AgentError> {
        let evidence = EvidenceBundle::from_turn_trace(trace)?;
        let metrics = TraceMetrics::from_turn_trace(trace)?;
        let turn_id = trace.turn_id.clone().ok_or_else(|| {
            AgentError::contract_violation("turn trace is missing turn id for evaluation")
        })?;
        let request = EvaluationRequest::new(
            EvaluationId::new(format!("evaluation.turn.{}", turn_id.as_str())),
            evidence.scope.clone(),
            expected_outcome,
        )
        .with_subject(agent_trace_subject(&evidence)?);
        Ok(Self {
            request,
            evidence,
            metrics,
            metrics_comparison: None,
        })
    }

    /// Builds an evaluation from a run trace.
    pub fn run(trace: &RunTrace, expected_outcome: ExpectedOutcome) -> Result<Self, AgentError> {
        let evidence = EvidenceBundle::from_run_trace(trace)?;
        let metrics = TraceMetrics::from_run_trace(trace)?;
        let run_id = trace.run_id.clone().ok_or_else(|| {
            AgentError::contract_violation("run trace is missing run id for evaluation")
        })?;
        let request = EvaluationRequest::new(
            EvaluationId::new(format!("evaluation.run.{}", run_id.as_str())),
            evidence.scope.clone(),
            expected_outcome,
        )
        .with_subject(agent_trace_subject(&evidence)?);
        Ok(Self {
            request,
            evidence,
            metrics,
            metrics_comparison: None,
        })
    }

    /// Builds an evaluation from a session timeline.
    pub fn session(
        timeline: &SessionTimeline,
        expected_outcome: ExpectedOutcome,
    ) -> Result<Self, AgentError> {
        let evidence = EvidenceBundle::from_session_timeline(timeline)?;
        let metrics = TraceMetrics::from_session_timeline(timeline);
        let request = EvaluationRequest::new(
            EvaluationId::new(format!(
                "evaluation.session.{}",
                timeline.session_id.as_str()
            )),
            evidence.scope.clone(),
            expected_outcome,
        )
        .with_subject(agent_trace_subject(&evidence)?);
        Ok(Self {
            request,
            evidence,
            metrics,
            metrics_comparison: None,
        })
    }

    /// Builds a deterministic comparison between two session timelines.
    pub fn compare_sessions(
        observed: &SessionTimeline,
        baseline: &SessionTimeline,
        expected_outcome: ExpectedOutcome,
    ) -> Result<Self, AgentError> {
        let mut evidence = EvidenceBundle::from_session_timeline(observed)?;
        let baseline_evidence = EvidenceBundle::from_session_timeline(baseline)?;
        let baseline_outcome_ref = baseline_evidence.outcome_ref.clone();
        let comparison_metrics = TraceMetricsComparison::sessions(observed, baseline);
        for mut item in baseline_evidence.items {
            item.role = EvidenceRole::Baseline;
            evidence = evidence.with_item(item);
        }
        let mut request = EvaluationRequest::new(
            EvaluationId::new(format!(
                "evaluation.session.compare.{}.{}",
                observed.session_id.as_str(),
                baseline.session_id.as_str()
            )),
            evidence.scope.clone(),
            expected_outcome,
        )
        .with_subject(agent_trace_subject(&evidence)?)
        .with_comparison(ComparisonDesign::PairedScopes {
            observed_scope: comparison_metrics.observed.scope.clone(),
            comparison_scope: comparison_metrics.baseline.scope.clone(),
        })
        .with_metric_deltas(comparison_metrics.metric_deltas.clone());

        if let Some(subject_ref) = baseline_outcome_ref {
            request = request.with_subject(EvaluationSubject {
                subject_ref,
                role: EvaluationSubjectRole::Baseline,
                redacted_summary: Some("baseline session outcome".to_string()),
            });
        }

        Ok(Self {
            request,
            evidence,
            metrics: comparison_metrics.observed.clone(),
            metrics_comparison: Some(comparison_metrics),
        })
    }

    /// Returns this evaluation with its comparison design replaced.
    pub fn compare(mut self, comparison: ComparisonDesign) -> Self {
        self.request.comparison = comparison;
        self
    }

    /// Returns this evaluation with its evaluator budget replaced.
    pub fn with_budget(mut self, budget: EvaluationBudget) -> Self {
        self.request.budget = budget;
        self
    }

    /// Evaluates this trace using the supplied evaluator.
    pub fn evaluate<E: Evaluator>(&self, evaluator: &E) -> Result<EvaluationReport, AgentError> {
        let report = evaluator.evaluate(&self.request, &self.evidence)?;
        report.validate_confidence_contract_for_request(&self.request)?;
        Ok(report)
    }

    /// Returns the request this builder will evaluate.
    pub fn request(&self) -> &EvaluationRequest {
        &self.request
    }

    /// Returns the evidence bundle this builder will pass to the evaluator.
    pub fn evidence(&self) -> &EvidenceBundle {
        &self.evidence
    }

    /// Returns deterministic metrics for the observed trace.
    pub fn metrics(&self) -> &TraceMetrics {
        &self.metrics
    }

    /// Returns deterministic comparison metrics when this evaluation compares traces.
    pub fn metrics_comparison(&self) -> Option<&TraceMetricsComparison> {
        self.metrics_comparison.as_ref()
    }
}

/// Provider-backed evaluator that asks for cited evidence refs and validates
/// them against the supplied bundle.
pub struct AiTraceEvaluator {
    provider: Arc<dyn ProviderAdapter>,
}

impl AiTraceEvaluator {
    /// Creates an AI evaluator from a provider adapter.
    pub fn new(provider: Arc<dyn ProviderAdapter>) -> Self {
        Self { provider }
    }

    /// Creates an AI evaluator from a concrete provider adapter.
    pub fn from_provider<P>(provider: P) -> Self
    where
        P: ProviderAdapter + 'static,
    {
        Self::new(Arc::new(provider))
    }
}

impl Evaluator for AiTraceEvaluator {
    fn evaluate(
        &self,
        request: &EvaluationRequest,
        evidence: &EvidenceBundle,
    ) -> Result<EvaluationReport, AgentError> {
        request.budget.require_provider_call()?;
        let prompt = ai_evaluator_prompt(request, evidence)?;
        let provider_request = ProviderRequest {
            schema_version: ProviderRequest::SCHEMA_VERSION,
            projection_policy_ref: "policy.evaluation.content_refs_only".to_string(),
            messages: vec![ProviderMessage {
                role: ProviderMessageRole::Developer,
                content: prompt,
                privacy: PrivacyClass::ContentRefsOnly,
                projected_metadata: None,
            }],
            projection_item_count: evidence.items.len(),
            structured_output_hint: None,
        };
        let response = self.provider.complete(&provider_request)?;
        let usage = EvaluationUsage {
            provider_calls: 1,
            provider_usage: Some(self.provider.extract_usage(&response)),
        };
        report_from_provider_response(request, evidence, &response.output_text, usage)
    }
}

#[derive(Deserialize)]
struct AiEvaluatorReply {
    #[serde(default)]
    verdict: Option<EvaluationVerdict>,
    #[serde(default)]
    score: Option<String>,
    #[serde(default)]
    confidence: Option<EvaluationConfidence>,
    #[serde(default)]
    redacted_summary: Option<String>,
    #[serde(default)]
    support_refs: Vec<WireEntityRef>,
    #[serde(default)]
    limitations: Vec<String>,
}

#[derive(Deserialize)]
struct WireEntityRef {
    kind: EntityKind,
    id: String,
}

fn agent_trace_subject(
    evidence: &EvidenceBundle,
) -> Result<agent_sdk_eval::EvaluationSubject, AgentError> {
    let subject_ref = evidence
        .outcome_ref
        .clone()
        .or_else(|| evidence.items.first().map(|item| item.evidence_ref.clone()))
        .ok_or_else(|| AgentError::contract_violation("evaluation evidence bundle is empty"))?;
    Ok(agent_sdk_eval::EvaluationSubject::primary(subject_ref))
}

fn ai_evaluator_prompt(
    request: &EvaluationRequest,
    evidence: &EvidenceBundle,
) -> Result<String, AgentError> {
    let payload = serde_json::json!({
        "evaluation_id": request.evaluation_id.as_str(),
        "scope": request.scope,
        "expected_outcome": request.expected_outcome,
        "comparison": request.comparison,
        "deterministic_metric_deltas": request.metric_deltas,
        "subjects": request.subjects,
        "available_evidence": evidence.items,
        "evidence_summary": evidence.redacted_summary,
    });
    let payload = serde_json::to_string(&payload).map_err(|error| {
        AgentError::contract_violation(format!("evaluation prompt JSON encode failed: {error}"))
    })?;
    let prompt = format!(
        concat!(
            "Evaluate this recorded agent outcome using only the supplied redacted summaries and ids. ",
            "Return compact JSON only with keys verdict, score, confidence, redacted_summary, support_refs, and limitations. ",
            "Cite support_refs only from available_evidence. ",
            "Use confidence=measured only when deterministic_metric_deltas and comparison evidence support it. ",
            "Do not compute or return metric_deltas; deterministic_metric_deltas are the only metric authority. ",
            "Do not include raw private content.\n",
            "EVALUATION_PAYLOAD={payload}"
        ),
        payload = payload
    );
    Ok(truncate_chars(prompt, request.budget.max_prompt_chars))
}

fn report_from_provider_response(
    request: &EvaluationRequest,
    evidence: &EvidenceBundle,
    output_text: &str,
    usage: EvaluationUsage,
) -> Result<EvaluationReport, AgentError> {
    let mut limitations = Vec::new();
    let parsed = serde_json::from_str::<AiEvaluatorReply>(output_text).map_err(|error| {
        AgentError::contract_violation(format!("evaluation provider JSON parse failed: {error}"))
    })?;
    let (support_refs, parse_limitations) = raw_support_refs(parsed.support_refs);
    limitations.extend(parse_limitations);
    let support = evidence.validate_support_refs(support_refs, request.budget.max_support_refs);
    limitations.extend(parsed.limitations);

    let fallback_confidence = if support.accepted_refs.is_empty() {
        EvaluationConfidence::Judged
    } else {
        EvaluationConfidence::Cited
    };
    let confidence = parsed.confidence.unwrap_or(fallback_confidence.clone());
    let subject_ref = request
        .subjects
        .first()
        .map(|subject| subject.subject_ref.clone())
        .or_else(|| evidence.outcome_ref.clone())
        .unwrap_or_else(|| EntityRef::run(RunId::new("run.evaluation.unknown")));
    let mut judgment = EvaluatorJudgment::new(
        subject_ref,
        parsed
            .verdict
            .clone()
            .unwrap_or(EvaluationVerdict::Inconclusive),
        confidence.clone(),
        parsed
            .redacted_summary
            .clone()
            .unwrap_or_else(|| "AI evaluator returned a judgment".to_string()),
    );
    judgment.score = parsed.score.clone();
    judgment.support_refs = support.accepted_refs.clone();
    judgment.rejected_support_refs = support.rejected_refs.clone();

    let mut report = EvaluationReport::new(
        request.evaluation_id.clone(),
        request.scope.clone(),
        request.comparison.clone(),
        parsed.verdict.unwrap_or(EvaluationVerdict::Inconclusive),
        confidence,
        parsed
            .redacted_summary
            .unwrap_or_else(|| "AI evaluator returned a report".to_string()),
    )
    .with_usage(usage)
    .with_judgment(judgment);
    report.score = parsed.score;
    report.evidence_refs = support.accepted_refs;
    report.metric_deltas = request.metric_deltas.clone();
    report.limitations = limitations;

    let claims_measured = report.confidence.is_measured()
        || report
            .judgments
            .iter()
            .any(|judgment| judgment.confidence.is_measured());
    if report
        .validate_confidence_contract_for_request(request)
        .is_err()
        && claims_measured
    {
        report.confidence = fallback_confidence.clone();
        for judgment in &mut report.judgments {
            if judgment.confidence.is_measured() {
                judgment.confidence = fallback_confidence.clone();
            }
        }
        report.limitations.push(
            "evaluator requested measured confidence without sufficient comparison evidence; downgraded"
                .to_string(),
        );
    }
    report.validate_confidence_contract_for_request(request)?;
    Ok(report)
}

fn raw_support_refs(raw_refs: Vec<WireEntityRef>) -> (Vec<EntityRef>, Vec<String>) {
    let mut refs = Vec::new();
    let mut limitations = Vec::new();
    for raw_ref in raw_refs {
        match EntityId::try_new(raw_ref.id) {
            Ok(entity_id) => refs.push(EntityRef::new(raw_ref.kind, entity_id)),
            Err(error) => {
                limitations.push(format!("provider cited an invalid support ref: {error}"))
            }
        }
    }
    (refs, limitations)
}

fn truncate_chars(value: String, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value;
    }
    value.chars().take(max_chars).collect()
}
