//! JSON-RPC protocol primitives and line-transport helpers.
//!
//! Scripted ACP/MCP conformance harnesses live under
//! `agent_sdk_toolkit::testing`, so production-facing wire types stay separate
//! from deterministic test-kit behavior.

mod json_rpc;
mod line_transport;

pub use json_rpc::{
    JsonRpcErrorObject, JsonRpcFrame, JsonRpcId, JsonRpcNotification, JsonRpcRequest,
    JsonRpcResponse,
};
pub use line_transport::{JsonRpcLineCodec, JsonRpcLineEndpoint};

pub(crate) use json_rpc::{expect_notification, expect_response, protocol_violation};
