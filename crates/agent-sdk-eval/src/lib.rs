//! Optional evaluation framework primitives for Agent SDK runs.
//!
//! This crate owns post-hoc evaluation contracts over core traces and evidence.
//! It does not run agents, append journals, publish events, choose evaluator
//! models, or define product-specific success rubrics.

pub mod comparison;
pub mod evaluator;
pub mod evidence;
pub mod identity;
pub mod report;
pub mod request;
pub mod scope;
pub mod testing;

pub use comparison::ComparisonDesign;
pub use evaluator::Evaluator;
pub use evidence::{EvidenceBundle, EvidenceItem, EvidenceRole, SupportRefValidation};
pub use identity::EvaluationId;
pub use report::{
    EvaluationConfidence, EvaluationMetricDelta, EvaluationReport, EvaluationVerdict,
    EvaluatorJudgment,
};
pub use request::{EvaluationBudget, EvaluationRequest, EvaluationUsage};
pub use scope::{
    EvaluationCriterion, EvaluationScope, EvaluationSubject, EvaluationSubjectRole, ExpectedOutcome,
};
