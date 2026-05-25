//! Concrete workspace tool helpers layered over core tool/effect contracts. Use these
//! modules for bounded read, search, edit, write, and format-aware extraction
//! behavior under a host-selected workspace policy. Reads search local files;
//! edit/write helpers may mutate files only through explicit executor calls. This
//! file contains the rendered portion of that contract.
//!
use super::super::{
    anchor::HashLineAnchor,
    read_pipeline::{
        WorkspaceArchiveMetadata, WorkspaceDocumentMetadata, WorkspaceMediaMetadata,
        WorkspaceReadDetection, WorkspaceReaderStep, WorkspaceResourceMetadata,
        WorkspaceSqliteMetadata,
    },
};

/// Workspace rendered read request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub(in crate::workspace) struct RenderedRead {
    /// Bounded textual content extracted for caller use; absent for binary
    /// summaries or denied raw access.
    pub content: String,
    /// Redacted or bounded summary used when raw content is absent or
    /// truncated.
    pub content_summary: Option<String>,
    /// Whether output was shortened by byte, item, page, archive, or parser
    /// limits.
    pub truncated: bool,
    /// Whether the input is treated as binary so raw bytes are not exposed by
    /// default.
    pub binary: bool,
    /// Hashline anchors and line metadata used for stale-read detection and
    /// edit planning.
    pub anchors: Vec<HashLineAnchor>,
    /// Collection of reader pipeline values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub reader_pipeline: Vec<WorkspaceReaderStep>,
    /// Media metadata extracted without exposing raw media bytes.
    pub media: Option<WorkspaceMediaMetadata>,
    /// Document metadata such as parser, page/slide/sheet counts, and
    /// extraction warnings.
    pub document: Option<WorkspaceDocumentMetadata>,
    /// Archive listing metadata with truncation and decompression warnings.
    pub archive: Option<WorkspaceArchiveMetadata>,
    /// SQLite schema/sample metadata gathered under bounded read limits.
    pub sqlite: Option<WorkspaceSqliteMetadata>,
    /// Resource/URI metadata resolved through an explicit resolver.
    pub resource: Option<WorkspaceResourceMetadata>,
    /// Non-fatal warnings from bounded readers, parsers, or policy
    /// downgrades.
    pub warnings: Vec<String>,
}

/// Workspace rendered uri read request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub(in crate::workspace) struct RenderedUriRead {
    /// Detection used by this record or request.
    pub detection: WorkspaceReadDetection,
    /// Observed byte length for the source, sidecar, or extracted record.
    pub byte_len: u64,
    /// Stable hash for the bytes or canonical payload used for stale checks
    /// and fingerprints.
    pub content_hash: String,
    /// Rendered used by this record or request.
    pub rendered: RenderedRead,
}

/// Constant value for the workspace::readers::rendered contract. Use it
/// to keep SDK records and tests aligned on the same stable value.
pub(super) const TRUNCATION_GUIDANCE: &str = "read output was truncated to the requested/policy byte limit; use workspace_search/grep or a narrower/range read for more context";

/// Builds the add truncation guidance value.
/// This is data construction and performs no I/O, journal append, event publication, or process
pub(super) fn add_truncation_guidance(rendered: &mut RenderedRead) {
    if rendered.truncated
        && !rendered
            .warnings
            .iter()
            .any(|warning| warning == TRUNCATION_GUIDANCE)
    {
        rendered.warnings.push(TRUNCATION_GUIDANCE.to_string());
    }
}
