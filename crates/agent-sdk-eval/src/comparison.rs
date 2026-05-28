//! Comparison designs for evaluation reports.

use serde::{Deserialize, Serialize};

use agent_sdk_core::EntityRef;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "design", rename_all = "snake_case")]
/// How an evaluation should compare an observed outcome.
pub enum ComparisonDesign {
    /// Judge the observed run or turn only.
    #[default]
    ObservedOnly,
    /// Judge the observed result against expected criteria only.
    ExpectedOutcome,
    /// Compare the observed result with a baseline run.
    BaselineRun {
        /// Baseline run or trace ref used as comparison evidence.
        baseline_ref: EntityRef,
    },
    /// Compare two recorded runs.
    PairedRuns {
        /// Observed run ref.
        observed_ref: EntityRef,
        /// Comparison run ref.
        comparison_ref: EntityRef,
    },
    /// Compare the observed result with one or more evidence refs removed.
    Ablation {
        /// Evidence refs removed for the ablation comparison.
        removed_refs: Vec<EntityRef>,
    },
    /// Ask for a counterfactual judgment without claiming measurement.
    Counterfactual {
        /// Safe summary of the counterfactual condition.
        redacted_summary: String,
    },
    /// Compare a cohort of repeated experiments.
    RepeatedExperiment {
        /// Cohort or experiment ref used as comparison evidence.
        cohort_ref: EntityRef,
    },
}

impl ComparisonDesign {
    /// Returns true when this design can support measured confidence if metric
    /// deltas are also present.
    pub fn supports_measured_confidence(&self) -> bool {
        matches!(
            self,
            Self::BaselineRun { .. }
                | Self::PairedRuns { .. }
                | Self::Ablation { .. }
                | Self::RepeatedExperiment { .. }
        )
    }

    /// Returns comparison refs available for validation and report evidence.
    pub fn comparison_refs(&self) -> Vec<EntityRef> {
        match self {
            Self::ObservedOnly | Self::ExpectedOutcome | Self::Counterfactual { .. } => Vec::new(),
            Self::BaselineRun { baseline_ref } => vec![baseline_ref.clone()],
            Self::PairedRuns {
                observed_ref,
                comparison_ref,
            } => vec![observed_ref.clone(), comparison_ref.clone()],
            Self::Ablation { removed_refs } => removed_refs.clone(),
            Self::RepeatedExperiment { cohort_ref } => vec![cohort_ref.clone()],
        }
    }

    /// Returns true when this design carries comparison evidence.
    pub fn has_comparison_evidence(&self) -> bool {
        !self.comparison_refs().is_empty()
    }
}
