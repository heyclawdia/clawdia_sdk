//! Evaluation report records and confidence validation.

use serde::{Deserialize, Serialize};

use agent_sdk_core::{AgentError, EntityRef};

use crate::{ComparisonDesign, EvaluationId, EvaluationRequest, EvaluationScope, EvaluationUsage};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Confidence level for an evaluation result.
pub enum EvaluationConfidence {
    /// Evidence was available, but no evaluator judgment or cited support exists.
    Available,
    /// The evaluator cited refs that were validated against the evidence bundle.
    Cited,
    /// The evaluator judged the result without measured comparison evidence.
    Judged,
    /// A comparison, baseline, ablation, or repeated experiment produced a metric delta.
    Measured,
    /// Repeated experiments produced statistical evidence.
    Statistical,
}

impl EvaluationConfidence {
    /// Returns true when this confidence claims measured impact.
    pub fn is_measured(&self) -> bool {
        matches!(self, Self::Measured | Self::Statistical)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Top-level evaluation verdict.
pub enum EvaluationVerdict {
    /// Expected outcome passed.
    Passed,
    /// Expected outcome failed.
    Failed,
    /// Evidence was insufficient or ambiguous.
    Inconclusive,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Optional metric delta for measured evaluations.
pub struct EvaluationMetricDelta {
    /// Stable metric reference owned by the host or eval fixture.
    pub metric_ref: String,
    /// Baseline or comparison artifact.
    pub baseline_ref: Option<EntityRef>,
    /// Delta value encoded as a string so the SDK stays metric-neutral.
    pub delta_value: String,
    /// Bounded summary of how the metric was computed.
    pub redacted_summary: String,
}

impl EvaluationMetricDelta {
    /// Creates a metric delta.
    pub fn new(
        metric_ref: impl Into<String>,
        delta_value: impl Into<String>,
        redacted_summary: impl Into<String>,
    ) -> Self {
        Self {
            metric_ref: metric_ref.into(),
            baseline_ref: None,
            delta_value: delta_value.into(),
            redacted_summary: redacted_summary.into(),
        }
    }

    /// Returns this metric delta with a baseline ref attached.
    pub fn with_baseline_ref(mut self, baseline_ref: EntityRef) -> Self {
        self.baseline_ref = Some(baseline_ref);
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Evaluator judgment for one criterion or subject.
pub struct EvaluatorJudgment {
    /// Optional judgment id owned by the evaluator or test fixture.
    pub judgment_id: Option<String>,
    /// Subject this judgment is about.
    pub subject_ref: EntityRef,
    /// Optional criterion id this judgment answers.
    pub criterion_id: Option<String>,
    /// Judgment verdict.
    pub verdict: EvaluationVerdict,
    /// Optional score encoded as a string so the SDK stays rubric-neutral.
    pub score: Option<String>,
    /// Validated support refs.
    pub support_refs: Vec<EntityRef>,
    /// Refs cited by the evaluator but not present in the evidence bundle.
    pub rejected_support_refs: Vec<EntityRef>,
    /// Confidence level for this judgment.
    pub confidence: EvaluationConfidence,
    /// Bounded summary safe for logs and prompts.
    pub redacted_summary: String,
    /// Limitations or validation notes.
    pub limitations: Vec<String>,
}

impl EvaluatorJudgment {
    /// Creates a judgment with no cited refs.
    pub fn new(
        subject_ref: EntityRef,
        verdict: EvaluationVerdict,
        confidence: EvaluationConfidence,
        redacted_summary: impl Into<String>,
    ) -> Self {
        Self {
            judgment_id: None,
            subject_ref,
            criterion_id: None,
            verdict,
            score: None,
            support_refs: Vec::new(),
            rejected_support_refs: Vec::new(),
            confidence,
            redacted_summary: redacted_summary.into(),
            limitations: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Top-level report returned by an evaluator.
pub struct EvaluationReport {
    /// Stable evaluation id.
    pub evaluation_id: EvaluationId,
    /// Scope this report evaluates.
    pub scope: EvaluationScope,
    /// Comparison design actually used.
    pub comparison: ComparisonDesign,
    /// Top-level verdict.
    pub verdict: EvaluationVerdict,
    /// Optional top-level score.
    pub score: Option<String>,
    /// Top-level confidence.
    pub confidence: EvaluationConfidence,
    /// Per-subject or per-criterion judgments.
    pub judgments: Vec<EvaluatorJudgment>,
    /// Metric deltas for measured evaluations.
    pub metric_deltas: Vec<EvaluationMetricDelta>,
    /// Evidence refs used by this report.
    pub evidence_refs: Vec<EntityRef>,
    /// Usage captured during evaluation.
    pub usage: EvaluationUsage,
    /// Bounded report summary.
    pub redacted_summary: String,
    /// Limitations or validation notes.
    pub limitations: Vec<String>,
}

impl EvaluationReport {
    /// Creates a report with no metric deltas.
    pub fn new(
        evaluation_id: EvaluationId,
        scope: EvaluationScope,
        comparison: ComparisonDesign,
        verdict: EvaluationVerdict,
        confidence: EvaluationConfidence,
        redacted_summary: impl Into<String>,
    ) -> Self {
        Self {
            evaluation_id,
            scope,
            comparison,
            verdict,
            score: None,
            confidence,
            judgments: Vec::new(),
            metric_deltas: Vec::new(),
            evidence_refs: Vec::new(),
            usage: EvaluationUsage::default(),
            redacted_summary: redacted_summary.into(),
            limitations: Vec::new(),
        }
    }

    /// Returns this report with usage attached.
    pub fn with_usage(mut self, usage: EvaluationUsage) -> Self {
        self.usage = usage;
        self
    }

    /// Returns this report with one judgment appended.
    pub fn with_judgment(mut self, judgment: EvaluatorJudgment) -> Self {
        self.judgments.push(judgment);
        self
    }

    /// Returns this report with one metric delta appended.
    pub fn with_metric_delta(mut self, metric_delta: EvaluationMetricDelta) -> Self {
        self.metric_deltas.push(metric_delta);
        self
    }

    /// Validates that measured confidence is backed by comparison evidence and
    /// metric deltas.
    pub fn validate_confidence_contract(&self) -> Result<(), AgentError> {
        self.validate_measured_confidence(&self.comparison, &self.metric_deltas)
    }

    /// Validates measured confidence against request-owned metric deltas.
    pub fn validate_confidence_contract_for_request(
        &self,
        request: &EvaluationRequest,
    ) -> Result<(), AgentError> {
        self.validate_measured_confidence(&request.comparison, &request.metric_deltas)?;
        let claims_measured = self.confidence.is_measured()
            || self
                .judgments
                .iter()
                .any(|judgment| judgment.confidence.is_measured());
        if claims_measured && self.comparison != request.comparison {
            return Err(AgentError::contract_violation(
                "measured evaluation comparison must match the evaluation request",
            ));
        }
        if claims_measured && self.metric_deltas != request.metric_deltas {
            return Err(AgentError::contract_violation(
                "measured evaluation metric deltas must come from the evaluation request",
            ));
        }
        Ok(())
    }

    fn validate_measured_confidence(
        &self,
        comparison: &ComparisonDesign,
        metric_deltas: &[EvaluationMetricDelta],
    ) -> Result<(), AgentError> {
        let claims_measured = self.confidence.is_measured()
            || self
                .judgments
                .iter()
                .any(|judgment| judgment.confidence.is_measured());
        if !claims_measured {
            return Ok(());
        }
        if !comparison.supports_measured_confidence() {
            return Err(AgentError::contract_violation(
                "measured evaluation confidence requires a comparison design",
            ));
        }
        if !comparison.has_comparison_evidence() {
            return Err(AgentError::contract_violation(
                "measured evaluation confidence requires comparison evidence refs",
            ));
        }
        if metric_deltas.is_empty() {
            return Err(AgentError::contract_violation(
                "measured evaluation confidence requires at least one metric delta",
            ));
        }
        Ok(())
    }
}
