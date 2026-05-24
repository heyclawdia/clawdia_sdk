//! Workspace tool-pack helpers.
//!
//! Each operation lives in its own module so SDK agents can find the behavior
//! they need without spelunking through a catch-all implementation file.

mod anchor;
mod bounds;
mod edit;
mod grep;
mod policy;
mod read;
mod read_pipeline;
mod readers;
mod util;
mod write;

pub use anchor::HashLineAnchor;
pub use bounds::BoundedWorkspace;
pub use edit::{WorkspaceEditExecutor, WorkspaceEditOutput, WorkspaceEditRequest};
pub use grep::{
    SearchMatch, WorkspaceSearchExecutor, WorkspaceSearchOutput, WorkspaceSearchRequest,
};
pub use policy::{WorkspacePolicy, approval_policy, filesystem_policy};
pub use read::{WorkspaceReadExecutor, WorkspaceReadOutput, WorkspaceReadRequest};
pub use read_pipeline::{
    WorkspaceApplePhotosMetadata, WorkspaceArchiveEntry, WorkspaceArchiveMetadata,
    WorkspaceDocumentMetadata, WorkspaceEmbeddedPreviewMetadata, WorkspaceFileKind,
    WorkspaceFileTypeConfidence, WorkspaceMediaMetadata, WorkspaceOcrMetadata,
    WorkspaceRawSensorMetadata, WorkspaceReadDetection, WorkspaceReaderStep,
    WorkspaceResourceMetadata, WorkspaceSqliteMetadata, WorkspaceSqliteTableMetadata,
};
pub use write::{
    WorkspaceWriteExecutor, WorkspaceWriteMode, WorkspaceWriteOutput, WorkspaceWriteRequest,
};
