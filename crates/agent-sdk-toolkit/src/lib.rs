//! Optional concrete tool-pack helpers for the Agent SDK.
//!
//! This crate owns filesystem, search, edit, write, shell, resource-reader,
//! discovery, and adapter-conformance helpers. `agent-sdk-core` stays
//! product-neutral and only sees runtime-package capabilities, tool executor
//! refs, policy refs, content refs, and effect lineage.

/// Public agent-pool toolkit namespace. Use it for concrete pool-store
/// adapters layered over `agent-sdk-core` coordination ports.
pub mod agent_pool;
pub mod discovery;
/// Public environment namespace. Use it for data-only helpers that lower
/// portable environment policy into core isolation contracts.
pub mod environment;
/// Public evaluation namespace. Use it for optional post-hoc agent-run
/// evaluation helpers layered over `agent-sdk-eval` and core traces.
pub mod evaluation;
/// Public packs namespace. Use it for the documented packs API surface;
/// prefer crate-root re-exports for common imports. Module items must
/// preserve the toolkit ownership and side-effect boundaries described
/// in this file.
pub mod packs;
/// Public protocol namespace. Use it for the documented protocol API
/// surface; prefer crate-root re-exports for common imports. Module
/// items must preserve the toolkit ownership and side-effect boundaries
/// described in this file.
pub mod protocol;
/// Public resources namespace. Use it for the documented resources API
/// surface; prefer crate-root re-exports for common imports. Module
/// items must preserve the toolkit ownership and side-effect boundaries
/// described in this file.
pub mod resources;
/// Public shell namespace. Use it for the documented shell API surface;
/// prefer crate-root re-exports for common imports. Module items must
/// preserve the toolkit ownership and side-effect boundaries described
/// in this file.
pub mod shell;
/// Public testing namespace. Use it for the documented testing API
/// surface; prefer crate-root re-exports for common imports. Module
/// items must preserve the toolkit ownership and side-effect boundaries
/// described in this file.
pub mod testing;
/// Public workspace namespace. Use it for the documented workspace API
/// surface; prefer crate-root re-exports for common imports. Module
/// items must preserve the toolkit ownership and side-effect boundaries
/// described in this file.
pub mod workspace;

pub use agent_pool::SqliteAgentPoolStore;
pub use discovery::{ToolDiscoveryExecutor, ToolDiscoveryIndex, ToolDiscoveryRequest};
pub use environment::{
    AgentWorkspaceEnvironment, AgentWorkspaceEnvironmentProfile, EgressAllowlist, EgressProtocol,
    EgressTarget, EnvironmentRuntime,
};
pub use evaluation::{AgentTraceEvaluation, AiTraceEvaluator};
pub use packs::{
    AsyncTool, Tool, ToolBuilder, ToolPackBuilder, ToolkitPackBundle, ToolkitToolExecutionMode,
    tool_snapshot,
};
pub use protocol::{
    JsonRpcErrorObject, JsonRpcFrame, JsonRpcId, JsonRpcLineCodec, JsonRpcLineEndpoint,
    JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
};
pub use resources::{InMemoryResourceResolver, ResourceReaderExecutor, ResourceReaderRequest};
pub use shell::{ShellExecutionPolicy, ShellExecutor, ShellRequest, ShellResult};
pub use testing::{InMemoryJsonArgumentStore, InMemoryToolkitContentStore};
pub use workspace::{
    BoundedWorkspace, HashLineAnchor, SearchMatch, WorkspaceApplePhotosMetadata,
    WorkspaceArchiveEntry, WorkspaceArchiveMetadata, WorkspaceDocumentMetadata,
    WorkspaceEditExecutor, WorkspaceEditOutput, WorkspaceEditRequest,
    WorkspaceEmbeddedPreviewMetadata, WorkspaceFileKind, WorkspaceFileTypeConfidence,
    WorkspaceMediaMetadata, WorkspaceOcrMetadata, WorkspacePolicy, WorkspaceRawSensorMetadata,
    WorkspaceReadDetection, WorkspaceReadExecutor, WorkspaceReadOutput, WorkspaceReadRequest,
    WorkspaceReaderStep, WorkspaceResourceMetadata, WorkspaceSearchExecutor, WorkspaceSearchOutput,
    WorkspaceSearchRequest, WorkspaceSqliteMetadata, WorkspaceSqliteTableMetadata,
    WorkspaceWriteExecutor, WorkspaceWriteMode, WorkspaceWriteOutput, WorkspaceWriteRequest,
};
