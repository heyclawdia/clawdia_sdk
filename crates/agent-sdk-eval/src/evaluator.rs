//! Evaluator trait for post-hoc evaluation implementations.

use agent_sdk_core::AgentError;

use crate::{EvaluationReport, EvaluationRequest, EvidenceBundle};

/// Evaluates an evidence bundle against an evaluation request.
pub trait Evaluator {
    /// Runs the evaluator and returns a validated report.
    fn evaluate(
        &self,
        request: &EvaluationRequest,
        evidence: &EvidenceBundle,
    ) -> Result<EvaluationReport, AgentError>;
}

impl<T> Evaluator for &T
where
    T: Evaluator + ?Sized,
{
    fn evaluate(
        &self,
        request: &EvaluationRequest,
        evidence: &EvidenceBundle,
    ) -> Result<EvaluationReport, AgentError> {
        (*self).evaluate(request, evidence)
    }
}
