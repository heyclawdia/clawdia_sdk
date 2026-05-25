//! Host adapter boundaries for the SDK core. Use these traits and registries when
//! hosts provide providers, journals, sinks, tools, isolation, extensions, telemetry,
//! or subscriptions. Implementations may perform external side effects and must honor
//! policy, redaction, idempotency, and replay contracts. This file contains the
//! approval portion of that contract.
//!
use crate::{
    approval_records::{ApprovalDecision, ApprovalRequest},
    domain::AgentError,
};

/// Type alias used by the ports::approval contract. Prefer this alias
/// where it makes SDK ownership and boundary intent clearer than the
/// underlying type.
pub type ApprovalDispatchRequest = ApprovalRequest;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Enumerates the finite approval dispatch response cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ApprovalDispatchResponse {
    /// Use this variant when the contract needs to represent decision; selecting it has no side effect by itself.
    Decision(ApprovalDecision),
    /// Use this variant when the contract needs to represent timed out; selecting it has no side effect by itself.
    TimedOut,
    /// Use this variant when the contract needs to represent cancelled; selecting it has no side effect by itself.
    Cancelled,
    /// Use this variant when the contract needs to represent unavailable; selecting it has no side effect by itself.
    Unavailable {
        /// Stable reason code for unavailable or degraded host behavior.
        reason_code: String,
    },
}

impl ApprovalDispatchResponse {
    /// Returns decision for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn decision(decision: ApprovalDecision) -> Self {
        Self::Decision(decision)
    }

    /// Returns unavailable for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn unavailable(reason_code: impl Into<String>) -> Self {
        Self::Unavailable {
            reason_code: reason_code.into(),
        }
    }
}

/// Port or behavior contract for approval dispatcher. Implementors
/// should preserve policy, redaction, idempotency, and replay
/// expectations from the surrounding module. Implementations may
/// perform side effects only as described by the trait methods.
pub trait ApprovalDispatcher: Send + Sync {
    /// Dispatches an approval request to the configured approver.
    /// Implementations may contact host UI or policy services; the runtime
    /// owns denial fallback and journal records around the dispatch.
    fn dispatch(
        &self,
        request: ApprovalDispatchRequest,
    ) -> Result<ApprovalDispatchResponse, AgentError>;
}
