use serde::{Deserialize, Serialize};

use crate::error::RetryClassification;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryFailureKind {
    ProviderFailure,
    ToolInterrupted,
    ToolFailure,
    ApprovalTransportUnknown,
    JournalAppendAfterEffect,
    MissingContentRef,
    PackageFingerprintMismatch,
    InvariantFailed,
    UnsafeSideEffect,
    HostPolicyRequired,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryClassification {
    RetryableSafeStep,
    ReconcileRequired,
    RepairRequired,
    UserActionRequired,
    HostConfigurationRequired,
    Irrecoverable,
}

impl RecoveryClassification {
    pub fn retry_classification(self) -> RetryClassification {
        match self {
            Self::RetryableSafeStep => RetryClassification::Retryable,
            Self::ReconcileRequired | Self::RepairRequired => RetryClassification::RepairNeeded,
            Self::UserActionRequired => RetryClassification::UserActionNeeded,
            Self::HostConfigurationRequired => RetryClassification::HostConfigurationNeeded,
            Self::Irrecoverable => RetryClassification::NotRetryable,
        }
    }

    pub fn requires_repair_plan(self) -> bool {
        matches!(
            self,
            Self::ReconcileRequired | Self::RepairRequired | Self::HostConfigurationRequired
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryAction {
    RetrySafeStep,
    ReconcilePendingSideEffect,
    RestoreFromJournal,
    RequestHostRepair,
    SurfaceRepairNeeded,
    FailClosed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecoveryDecision {
    pub failure_kind: RecoveryFailureKind,
    pub classification: RecoveryClassification,
    pub action: RecoveryAction,
    pub retry: RetryClassification,
    pub repair_plan_required: bool,
    pub idempotent_retry_allowed: bool,
}

impl RecoveryDecision {
    pub fn classify(failure_kind: RecoveryFailureKind) -> Self {
        classify_recovery(failure_kind)
    }
}

pub fn classify_recovery(failure_kind: RecoveryFailureKind) -> RecoveryDecision {
    let (classification, action, idempotent_retry_allowed) = match failure_kind {
        RecoveryFailureKind::ProviderFailure => (
            RecoveryClassification::RetryableSafeStep,
            RecoveryAction::RetrySafeStep,
            true,
        ),
        RecoveryFailureKind::ToolInterrupted => (
            RecoveryClassification::RetryableSafeStep,
            RecoveryAction::RestoreFromJournal,
            true,
        ),
        RecoveryFailureKind::ToolFailure | RecoveryFailureKind::InvariantFailed => (
            RecoveryClassification::RepairRequired,
            RecoveryAction::SurfaceRepairNeeded,
            false,
        ),
        RecoveryFailureKind::ApprovalTransportUnknown
        | RecoveryFailureKind::JournalAppendAfterEffect
        | RecoveryFailureKind::UnsafeSideEffect => (
            RecoveryClassification::ReconcileRequired,
            RecoveryAction::ReconcilePendingSideEffect,
            false,
        ),
        RecoveryFailureKind::MissingContentRef => (
            RecoveryClassification::UserActionRequired,
            RecoveryAction::SurfaceRepairNeeded,
            false,
        ),
        RecoveryFailureKind::PackageFingerprintMismatch
        | RecoveryFailureKind::HostPolicyRequired => (
            RecoveryClassification::HostConfigurationRequired,
            RecoveryAction::RequestHostRepair,
            false,
        ),
    };

    RecoveryDecision {
        failure_kind,
        classification,
        action,
        retry: classification.retry_classification(),
        repair_plan_required: classification.requires_repair_plan(),
        idempotent_retry_allowed,
    }
}
