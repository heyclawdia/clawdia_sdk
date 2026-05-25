use super::super::{
    anchor::HashLineAnchor,
    read_pipeline::{
        WorkspaceArchiveMetadata, WorkspaceDocumentMetadata, WorkspaceMediaMetadata,
        WorkspaceReadDetection, WorkspaceReaderStep, WorkspaceResourceMetadata,
        WorkspaceSqliteMetadata,
    },
};

pub(in crate::workspace) struct RenderedRead {
    pub content: String,
    pub content_summary: Option<String>,
    pub truncated: bool,
    pub binary: bool,
    pub anchors: Vec<HashLineAnchor>,
    pub reader_pipeline: Vec<WorkspaceReaderStep>,
    pub media: Option<WorkspaceMediaMetadata>,
    pub document: Option<WorkspaceDocumentMetadata>,
    pub archive: Option<WorkspaceArchiveMetadata>,
    pub sqlite: Option<WorkspaceSqliteMetadata>,
    pub resource: Option<WorkspaceResourceMetadata>,
    pub warnings: Vec<String>,
}

pub(in crate::workspace) struct RenderedUriRead {
    pub detection: WorkspaceReadDetection,
    pub byte_len: u64,
    pub content_hash: String,
    pub rendered: RenderedRead,
}

pub(super) const TRUNCATION_GUIDANCE: &str = "read output was truncated to the requested/policy byte limit; use workspace_search/grep or a narrower/range read for more context";

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
