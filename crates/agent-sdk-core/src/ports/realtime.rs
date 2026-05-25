//! Host adapter boundaries for the SDK core. Use these traits and registries when
//! hosts provide providers, journals, sinks, tools, isolation, extensions, telemetry,
//! or subscriptions. Implementations may perform external side effects and must honor
//! policy, redaction, idempotency, and replay contracts. This file contains the
//! realtime portion of that contract.
//!
use serde::{Deserialize, Serialize};

use crate::{
    domain::AgentError,
    realtime_records::{
        RealtimeCloseReason, RealtimeConnectionId, RealtimeInputFrame, RealtimeOutputFrame,
        RealtimeResponseId, RealtimeSessionId,
    },
};

/// Port or behavior contract for realtime provider adapter.
/// Implementors should preserve policy, redaction, idempotency, and
/// replay expectations from the surrounding module. Implementations may
/// perform side effects only as described by the trait methods.
pub trait RealtimeProviderAdapter: Send + Sync {
    /// Returns the adapter ref identifier for this adapter.
    /// This is a side-effect-free adapter identifier accessor.
    fn adapter_ref(&self) -> &str;

    /// Opens or attaches to a realtime adapter session.
    /// Implementations open or attach to a realtime adapter session and return the session
    /// record without appending journal records themselves.
    fn connect(
        &self,
        request: RealtimeConnectRequest,
    ) -> Result<RealtimeConnectResponse, AgentError>;

    /// Sends one input frame to the active realtime adapter session.
    /// Implementations send one realtime input frame to the active adapter session and return
    /// the adapter response record.
    fn send(
        &self,
        session_id: &RealtimeSessionId,
        frame: RealtimeInputFrame,
    ) -> Result<(), AgentError>;

    /// Reads from the in-memory endpoint queue used by protocol tests; it
    /// performs no OS-level I/O.
    fn receive(
        &self,
        session_id: &RealtimeSessionId,
    ) -> Result<Option<RealtimeOutputFrame>, AgentError>;

    /// Sends an interruption request to the active realtime adapter session.
    /// Implementations send an interruption request to the active realtime adapter session.
    fn interrupt(
        &self,
        session_id: &RealtimeSessionId,
        response_id: &RealtimeResponseId,
    ) -> Result<RealtimeAdapterAck, AgentError>;

    /// Restarts or reconnects the active realtime adapter session.
    /// Implementations restart or reconnect the active realtime adapter session and return the
    /// new session record.
    fn restart(
        &self,
        session_id: &RealtimeSessionId,
        current_connection_id: &RealtimeConnectionId,
    ) -> Result<RealtimeConnectResponse, AgentError>;

    /// Closes the active realtime adapter session.
    /// Implementations close or detach the realtime adapter session according to the requested
    /// close reason.
    fn close(
        &self,
        session_id: &RealtimeSessionId,
        reason: RealtimeCloseReason,
    ) -> Result<RealtimeAdapterAck, AgentError>;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries realtime connect request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct RealtimeConnectRequest {
    /// Stable session id used for typed lineage, lookup, or dedupe.
    pub session_id: RealtimeSessionId,
    /// Typed provider route ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub provider_route_ref: String,
    /// Typed realtime capability ref reference. Resolving or executing it is
    /// a separate policy-gated step.
    pub realtime_capability_ref: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries realtime connect response data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct RealtimeConnectResponse {
    /// Stable session id used for typed lineage, lookup, or dedupe.
    pub session_id: RealtimeSessionId,
    /// Stable connection id used for typed lineage, lookup, or dedupe.
    pub connection_id: RealtimeConnectionId,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries realtime adapter ack data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct RealtimeAdapterAck {
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "call", rename_all = "snake_case")]
/// Enumerates the finite realtime adapter call cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RealtimeAdapterCall {
    /// Use this variant when the contract needs to represent connect; selecting it has no side effect by itself.
    Connect {
        /// Stable session id used for typed lineage, lookup, or dedupe.
        session_id: RealtimeSessionId,
    },
    /// Use this variant when the contract needs to represent send; selecting it has no side effect by itself.
    Send {
        /// Stable session id used for typed lineage, lookup, or dedupe.
        session_id: RealtimeSessionId,
        /// Redacted human-readable summary safe for events, telemetry, and
        /// logs.
        redacted_summary: String,
    },
    /// Use this variant when the contract needs to represent receive; selecting it has no side effect by itself.
    Receive {
        /// Stable session id used for typed lineage, lookup, or dedupe.
        session_id: RealtimeSessionId,
    },
    /// Use this variant when the contract needs to represent interrupt; selecting it has no side effect by itself.
    Interrupt {
        /// Stable session id used for typed lineage, lookup, or dedupe.
        session_id: RealtimeSessionId,
        /// Stable response id used for typed lineage, lookup, or dedupe.
        response_id: RealtimeResponseId,
    },
    /// Use this variant when the contract needs to represent restart; selecting it has no side effect by itself.
    Restart {
        /// Stable session id used for typed lineage, lookup, or dedupe.
        session_id: RealtimeSessionId,
        /// Stable current connection id used for typed lineage, lookup, or
        /// dedupe.
        current_connection_id: RealtimeConnectionId,
    },
    /// Use this variant when the contract needs to represent close; selecting it has no side effect by itself.
    Close {
        /// Stable session id used for typed lineage, lookup, or dedupe.
        session_id: RealtimeSessionId,
        /// Redacted explanation for a denial, failure, status, or package
        /// delta.
        reason: RealtimeCloseReason,
    },
}

impl RealtimeAdapterCall {
    /// Reads the stored name without registry or runtime work.
    /// This returns a static reason name and performs no adapter call.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Connect { .. } => "connect",
            Self::Send { .. } => "send",
            Self::Receive { .. } => "receive",
            Self::Interrupt { .. } => "interrupt",
            Self::Restart { .. } => "restart",
            Self::Close { .. } => "close",
        }
    }
}
