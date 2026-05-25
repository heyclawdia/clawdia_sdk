//! Toolkit-specific deterministic test helpers. Use these fakes for content stores,
//! argument stores, and scripted protocol harnesses without live editors, MCP
//! servers, or product hosts. Helpers mutate only in-memory state unless noted.
//!
pub mod protocol;

mod stores;

pub use protocol::{
    IsolatedJsonRpcProcess, McpHostProxy, ScriptedAcpAgent, ScriptedAcpClient, ScriptedMcpServer,
};
pub use stores::{InMemoryJsonArgumentStore, InMemoryToolkitContentStore};
