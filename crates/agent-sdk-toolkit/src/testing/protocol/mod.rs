//! Deterministic protocol conformance harnesses for SDK consumers.

mod acp;
mod isolation;
mod mcp;

pub use acp::{ScriptedAcpAgent, ScriptedAcpClient};
pub use isolation::IsolatedJsonRpcProcess;
pub use mcp::{McpHostProxy, ScriptedMcpServer};
