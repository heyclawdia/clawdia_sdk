//! Optional concrete tool-pack helpers for the Agent SDK.
//!
//! This crate owns filesystem, search, edit, write, shell, resource-reader,
//! discovery, and adapter-conformance helpers. `agent-sdk-core` stays
//! product-neutral and only sees runtime-package capabilities, tool executor
//! refs, policy refs, content refs, and effect lineage.

pub mod discovery;
pub mod packs;
pub mod protocol;
pub mod resources;
pub mod shell;
pub mod testing;
pub mod workspace;

pub use discovery::{ToolDiscoveryExecutor, ToolDiscoveryIndex, ToolDiscoveryRequest};
pub use packs::{ToolkitPackBundle, tool_snapshot};
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
