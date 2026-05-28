//! Evaluation scopes, subjects, and expected outcomes.

use serde::{Deserialize, Serialize};

use agent_sdk_core::{EntityRef, RunId, SessionId, TurnId};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
/// The durable boundary an evaluation is about.
pub enum EvaluationScope {
    /// Evaluate one run.
    Run {
        /// Run identifier used for lineage, filtering, replay, and dedupe.
        run_id: RunId,
    },
    /// Evaluate one turn, optionally grouped by session.
    Turn {
        /// Optional host-provided session identifier for grouping related turns.
        session_id: Option<SessionId>,
        /// Turn identifier for one loop turn within a run.
        turn_id: TurnId,
    },
    /// Evaluate one session timeline.
    Session {
        /// Session identifier for grouping related turns.
        session_id: SessionId,
    },
    /// Evaluate a host-defined scope represented by an entity ref.
    Custom {
        /// Scope ref owned by the host or optional integration layer.
        scope_ref: EntityRef,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Role a subject plays in an evaluation.
pub enum EvaluationSubjectRole {
    /// Main thing being evaluated.
    Primary,
    /// Candidate evidence that may have helped the outcome.
    CandidateEvidence,
    /// Baseline subject used for comparison.
    Baseline,
    /// Comparator subject used in paired or ablation evals.
    Comparator,
    /// Constraint that shaped the expected result.
    Constraint,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// One entity being evaluated or compared.
pub struct EvaluationSubject {
    /// Entity ref for the subject.
    pub subject_ref: EntityRef,
    /// Subject role in this evaluation.
    pub role: EvaluationSubjectRole,
    /// Bounded summary safe for logs, journals, events, and telemetry.
    pub redacted_summary: Option<String>,
}

impl EvaluationSubject {
    /// Creates a primary evaluation subject.
    pub fn primary(subject_ref: EntityRef) -> Self {
        Self {
            subject_ref,
            role: EvaluationSubjectRole::Primary,
            redacted_summary: None,
        }
    }

    /// Returns this subject with a safe summary attached.
    pub fn with_redacted_summary(mut self, redacted_summary: impl Into<String>) -> Self {
        self.redacted_summary = Some(redacted_summary.into());
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// One expected-outcome criterion.
pub struct EvaluationCriterion {
    /// Stable criterion id owned by the host or eval fixture.
    pub criterion_id: String,
    /// Bounded summary of the condition to check.
    pub redacted_summary: String,
    /// Optional weight encoded as a string so the SDK stays metric-neutral.
    pub weight: Option<String>,
}

impl EvaluationCriterion {
    /// Creates a criterion from a stable id and safe summary.
    pub fn new(criterion_id: impl Into<String>, redacted_summary: impl Into<String>) -> Self {
        Self {
            criterion_id: criterion_id.into(),
            redacted_summary: redacted_summary.into(),
            weight: None,
        }
    }

    /// Returns this criterion with a host-defined weight attached.
    pub fn with_weight(mut self, weight: impl Into<String>) -> Self {
        self.weight = Some(weight.into());
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Expected result supplied by a test, host, human reviewer, or eval fixture.
pub struct ExpectedOutcome {
    /// Optional outcome entity being checked.
    pub outcome_ref: Option<EntityRef>,
    /// Criteria the evaluator should judge.
    pub criteria: Vec<EvaluationCriterion>,
    /// Bounded summary safe for provider prompts and test output.
    pub redacted_summary: String,
}

impl ExpectedOutcome {
    /// Creates an expected outcome from a safe summary.
    pub fn new(redacted_summary: impl Into<String>) -> Self {
        Self {
            outcome_ref: None,
            criteria: Vec::new(),
            redacted_summary: redacted_summary.into(),
        }
    }

    /// Creates a common completion expectation.
    pub fn completed() -> Self {
        Self::new("agent completed the requested task").with_criterion(EvaluationCriterion::new(
            "criterion.completed",
            "run completed",
        ))
    }

    /// Returns this expected outcome with an outcome ref attached.
    pub fn with_outcome_ref(mut self, outcome_ref: EntityRef) -> Self {
        self.outcome_ref = Some(outcome_ref);
        self
    }

    /// Returns this expected outcome with one criterion appended.
    pub fn with_criterion(mut self, criterion: EvaluationCriterion) -> Self {
        self.criteria.push(criterion);
        self
    }
}
