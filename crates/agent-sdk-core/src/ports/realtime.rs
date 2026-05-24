use serde::{Deserialize, Serialize};

use crate::{
    domain::AgentError,
    realtime_records::{
        RealtimeCloseReason, RealtimeConnectionId, RealtimeInputFrame, RealtimeOutputFrame,
        RealtimeResponseId, RealtimeSessionId,
    },
};

pub trait RealtimeProviderAdapter: Send + Sync {
    fn adapter_ref(&self) -> &str;

    fn connect(
        &self,
        request: RealtimeConnectRequest,
    ) -> Result<RealtimeConnectResponse, AgentError>;

    fn send(
        &self,
        session_id: &RealtimeSessionId,
        frame: RealtimeInputFrame,
    ) -> Result<(), AgentError>;

    fn receive(
        &self,
        session_id: &RealtimeSessionId,
    ) -> Result<Option<RealtimeOutputFrame>, AgentError>;

    fn interrupt(
        &self,
        session_id: &RealtimeSessionId,
        response_id: &RealtimeResponseId,
    ) -> Result<RealtimeAdapterAck, AgentError>;

    fn restart(
        &self,
        session_id: &RealtimeSessionId,
        current_connection_id: &RealtimeConnectionId,
    ) -> Result<RealtimeConnectResponse, AgentError>;

    fn close(
        &self,
        session_id: &RealtimeSessionId,
        reason: RealtimeCloseReason,
    ) -> Result<RealtimeAdapterAck, AgentError>;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RealtimeConnectRequest {
    pub session_id: RealtimeSessionId,
    pub provider_route_ref: String,
    pub realtime_capability_ref: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RealtimeConnectResponse {
    pub session_id: RealtimeSessionId,
    pub connection_id: RealtimeConnectionId,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RealtimeAdapterAck {
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "call", rename_all = "snake_case")]
pub enum RealtimeAdapterCall {
    Connect {
        session_id: RealtimeSessionId,
    },
    Send {
        session_id: RealtimeSessionId,
        redacted_summary: String,
    },
    Receive {
        session_id: RealtimeSessionId,
    },
    Interrupt {
        session_id: RealtimeSessionId,
        response_id: RealtimeResponseId,
    },
    Restart {
        session_id: RealtimeSessionId,
        current_connection_id: RealtimeConnectionId,
    },
    Close {
        session_id: RealtimeSessionId,
        reason: RealtimeCloseReason,
    },
}

impl RealtimeAdapterCall {
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
