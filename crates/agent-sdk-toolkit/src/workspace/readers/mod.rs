use std::path::Path;

use agent_sdk_core::{AgentError, AgentErrorKind, RetryClassification};

use super::{
    anchor::HashLineAnchor,
    read_pipeline::{
        WorkspaceArchiveMetadata, WorkspaceDocumentMetadata, WorkspaceFileKind,
        WorkspaceMediaMetadata, WorkspaceReadDetection, WorkspaceReaderStep,
        WorkspaceResourceMetadata, WorkspaceSqliteMetadata,
    },
};

mod archive;
mod legacy_office;
mod media;
mod ocr;
mod office;
mod pdf;
mod sqlite;
mod summary;
mod text;
mod url_resource;

pub(super) struct RenderedRead {
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

pub(super) struct RenderedUriRead {
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

pub(super) fn render_workspace_read(
    path: &Path,
    bytes: &[u8],
    detection: &WorkspaceReadDetection,
    max_output_bytes: u64,
) -> Result<RenderedRead, AgentError> {
    match detection.kind {
        WorkspaceFileKind::Text | WorkspaceFileKind::Markdown | WorkspaceFileKind::Json => {
            text::render_utf8_text(bytes, max_output_bytes, true)
        }
        WorkspaceFileKind::Pdf => pdf::render_pdf(path, bytes, max_output_bytes),
        WorkspaceFileKind::Image => media::render_image(path, bytes, detection, max_output_bytes),
        WorkspaceFileKind::RawImage => {
            media::render_raw_image(path, bytes, detection, max_output_bytes)
        }
        WorkspaceFileKind::OfficeDocument => office::render_office(path, bytes, max_output_bytes),
        WorkspaceFileKind::Archive => archive::render_archive(path, bytes, max_output_bytes),
        WorkspaceFileKind::SqliteDatabase => sqlite::render_sqlite(path, max_output_bytes),
        WorkspaceFileKind::UrlResource => Err(AgentError::contract_violation(
            "URL resources are rendered before file dispatch",
        )),
        WorkspaceFileKind::Binary => Ok(summary::render_binary_summary(bytes, max_output_bytes)),
    }
}

pub(super) fn render_bounded_prefix_read(
    bytes: &[u8],
    detection: &WorkspaceReadDetection,
    full_byte_len: u64,
    max_output_bytes: u64,
) -> Result<RenderedRead, AgentError> {
    summary::render_bounded_prefix_read(bytes, detection, full_byte_len, max_output_bytes)
}

pub(super) fn render_workspace_uri(
    uri: &str,
    max_input_bytes: u64,
    max_output_bytes: u64,
) -> Result<RenderedUriRead, AgentError> {
    url_resource::render_uri(uri, max_input_bytes, max_output_bytes)
}

fn extraction_error(kind: &str, error: impl std::fmt::Display) -> AgentError {
    AgentError::new(
        AgentErrorKind::ToolFailure,
        RetryClassification::UserActionNeeded,
        format!("{kind} extraction failed: {error}"),
    )
}
