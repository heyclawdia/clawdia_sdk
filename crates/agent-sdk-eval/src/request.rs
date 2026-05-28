//! Evaluation request, budget, and usage records.

use serde::{Deserialize, Serialize};

use agent_sdk_core::{AgentError, ProviderUsage};

use crate::{ComparisonDesign, EvaluationId, EvaluationScope, EvaluationSubject, ExpectedOutcome};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Budget for evaluator work. Provider-backed evaluators should reject requests
/// that cannot fit this budget rather than silently making extra calls.
pub struct EvaluationBudget {
    /// Maximum provider calls an evaluator may make for this evaluation.
    pub max_provider_calls: u32,
    /// Maximum prompt characters sent to a provider-backed evaluator.
    pub max_prompt_chars: usize,
    /// Maximum cited support refs accepted from evaluator output.
    pub max_support_refs: usize,
}

impl Default for EvaluationBudget {
    fn default() -> Self {
        Self {
            max_provider_calls: 1,
            max_prompt_chars: 4_096,
            max_support_refs: 8,
        }
    }
}

impl EvaluationBudget {
    /// Ensures a provider-backed evaluator may spend one provider call.
    pub fn require_provider_call(&self) -> Result<(), AgentError> {
        if self.max_provider_calls == 0 {
            return Err(AgentError::contract_violation(
                "evaluation budget allows zero provider calls",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Usage captured by an evaluator run.
pub struct EvaluationUsage {
    /// Number of provider calls made by this evaluator.
    pub provider_calls: u32,
    /// Provider usage accounting when the adapter reports it.
    pub provider_usage: Option<ProviderUsage>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Request passed to an evaluator.
pub struct EvaluationRequest {
    /// Stable evaluation id for lineage and test output.
    pub evaluation_id: EvaluationId,
    /// Durable scope being evaluated.
    pub scope: EvaluationScope,
    /// Subjects being evaluated or compared.
    pub subjects: Vec<EvaluationSubject>,
    /// Expected outcome supplied by a test, host, or reviewer.
    pub expected_outcome: ExpectedOutcome,
    /// Comparison design for the evaluation.
    pub comparison: ComparisonDesign,
    /// Budget for evaluator work.
    pub budget: EvaluationBudget,
    /// Bounded request summary safe for logs and prompts.
    pub redacted_summary: String,
}

impl EvaluationRequest {
    /// Creates an evaluation request with observed-only comparison defaults.
    pub fn new(
        evaluation_id: EvaluationId,
        scope: EvaluationScope,
        expected_outcome: ExpectedOutcome,
    ) -> Self {
        Self {
            evaluation_id,
            scope,
            subjects: Vec::new(),
            redacted_summary: expected_outcome.redacted_summary.clone(),
            expected_outcome,
            comparison: ComparisonDesign::ObservedOnly,
            budget: EvaluationBudget::default(),
        }
    }

    /// Returns this request with one subject appended.
    pub fn with_subject(mut self, subject: EvaluationSubject) -> Self {
        self.subjects.push(subject);
        self
    }

    /// Returns this request with its comparison design replaced.
    pub fn with_comparison(mut self, comparison: ComparisonDesign) -> Self {
        self.comparison = comparison;
        self
    }

    /// Returns this request with its budget replaced.
    pub fn with_budget(mut self, budget: EvaluationBudget) -> Self {
        self.budget = budget;
        self
    }
}
