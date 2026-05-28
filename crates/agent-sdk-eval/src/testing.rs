//! Deterministic evaluator fakes for SDK consumers.

use agent_sdk_core::AgentError;

use crate::{EvaluationReport, EvaluationRequest, Evaluator, EvidenceBundle};

#[derive(Clone, Debug)]
/// Scripted evaluator that returns a prebuilt report after validating its
/// confidence contract.
pub struct ScriptedEvaluator {
    report: EvaluationReport,
}

impl ScriptedEvaluator {
    /// Creates a scripted evaluator from a fixed report.
    pub fn new(report: EvaluationReport) -> Self {
        Self { report }
    }
}

impl Evaluator for ScriptedEvaluator {
    fn evaluate(
        &self,
        request: &EvaluationRequest,
        _evidence: &EvidenceBundle,
    ) -> Result<EvaluationReport, AgentError> {
        let mut report = self.report.clone();
        report.metric_deltas = request.metric_deltas.clone();
        report.validate_confidence_contract_for_request(request)?;
        Ok(report)
    }
}
