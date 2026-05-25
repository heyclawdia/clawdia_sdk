//! Application-layer coordination over core primitives. Use these services to lower
//! helpers, drive runs, validate output, coordinate tools, approvals, delivery,
//! isolation, telemetry, and feature layers. Methods in this layer may call
//! configured ports, mutate in-memory stores, append journals, or publish events as
//! documented. This file contains the recovery portion of that contract.
//!
use serde::{Deserialize, Serialize};

use crate::error::RetryClassification;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite recovery failure kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RecoveryFailureKind {
    /// Use this variant when the contract needs to represent provider failure; selecting it has no side effect by itself.
    ProviderFailure,
    /// Use this variant when the contract needs to represent tool interrupted; selecting it has no side effect by itself.
    ToolInterrupted,
    /// Use this variant when the contract needs to represent tool failure; selecting it has no side effect by itself.
    ToolFailure,
    /// Use this variant when the contract needs to represent approval transport unknown; selecting it has no side effect by itself.
    ApprovalTransportUnknown,
    /// Use this variant when the contract needs to represent journal append after effect; selecting it has no side effect by itself.
    JournalAppendAfterEffect,
    /// Use this variant when the contract needs to represent missing content ref; selecting it has no side effect by itself.
    MissingContentRef,
    /// Use this variant when the contract needs to represent package fingerprint mismatch; selecting it has no side effect by itself.
    PackageFingerprintMismatch,
    /// Use this variant when the contract needs to represent invariant failed; selecting it has no side effect by itself.
    InvariantFailed,
    /// Use this variant when the contract needs to represent unsafe side effect; selecting it has no side effect by itself.
    UnsafeSideEffect,
    /// Use this variant when the contract needs to represent host policy required; selecting it has no side effect by itself.
    HostPolicyRequired,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite recovery classification cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RecoveryClassification {
    /// Use this variant when the contract needs to represent retryable safe step; selecting it has no side effect by itself.
    RetryableSafeStep,
    /// Use this variant when the contract needs to represent reconcile required; selecting it has no side effect by itself.
    ReconcileRequired,
    /// Use this variant when the contract needs to represent repair required; selecting it has no side effect by itself.
    RepairRequired,
    /// Use this variant when the contract needs to represent user action required; selecting it has no side effect by itself.
    UserActionRequired,
    /// Use this variant when the contract needs to represent host configuration required; selecting it has no side effect by itself.
    HostConfigurationRequired,
    /// Use this variant when the contract needs to represent irrecoverable; selecting it has no side effect by itself.
    Irrecoverable,
}

impl RecoveryClassification {
    /// Computes or returns retry classification for the application::recovery
    /// contract without external I/O or side effects.
    pub fn retry_classification(self) -> RetryClassification {
        match self {
            Self::RetryableSafeStep => RetryClassification::Retryable,
            Self::ReconcileRequired | Self::RepairRequired => RetryClassification::RepairNeeded,
            Self::UserActionRequired => RetryClassification::UserActionNeeded,
            Self::HostConfigurationRequired => RetryClassification::HostConfigurationNeeded,
            Self::Irrecoverable => RetryClassification::NotRetryable,
        }
    }

    /// Returns whether requires repair plan applies for this contract.
    /// This derives recovery or repair data from the supplied failure state and does not
    /// perform the repair by itself.
    pub fn requires_repair_plan(self) -> bool {
        matches!(
            self,
            Self::ReconcileRequired | Self::RepairRequired | Self::HostConfigurationRequired
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite recovery action cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RecoveryAction {
    /// Use this variant when the contract needs to represent retry safe step; selecting it has no side effect by itself.
    RetrySafeStep,
    /// Use this variant when the contract needs to represent reconcile pending side effect; selecting it has no side effect by itself.
    ReconcilePendingSideEffect,
    /// Use this variant when the contract needs to represent restore from journal; selecting it has no side effect by itself.
    RestoreFromJournal,
    /// Use this variant when the contract needs to represent request host repair; selecting it has no side effect by itself.
    RequestHostRepair,
    /// Use this variant when the contract needs to represent surface repair needed; selecting it has no side effect by itself.
    SurfaceRepairNeeded,
    /// Use this variant when the contract needs to represent fail closed; selecting it has no side effect by itself.
    FailClosed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds recovery decision application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct RecoveryDecision {
    /// Kind discriminator for failure kind.
    /// Use it to route finite match arms without parsing display text.
    pub failure_kind: RecoveryFailureKind,
    /// Classification used by this record or request.
    pub classification: RecoveryClassification,
    /// Action used by this record or request.
    pub action: RecoveryAction,
    /// Retry used by this record or request.
    pub retry: RetryClassification,
    /// Whether the recovery path requires an explicit repair plan before mutation.
    /// Use it to fail closed when retrying would be unsafe or insufficiently audited.
    pub repair_plan_required: bool,
    /// Allowlist for this policy or contract.
    /// Validation uses it to reject undeclared or policy-denied values.
    pub idempotent_retry_allowed: bool,
}

impl RecoveryDecision {
    /// Builds the classify value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn classify(failure_kind: RecoveryFailureKind) -> Self {
        classify_recovery(failure_kind)
    }
}

/// Classify recovery.
/// This derives recovery or repair data from the supplied failure state and does not perform
/// the repair by itself.
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
