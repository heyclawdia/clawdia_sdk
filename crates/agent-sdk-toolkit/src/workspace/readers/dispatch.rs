use std::path::Path;

use agent_sdk_core::{AgentError, AgentErrorKind, RetryClassification};

use super::super::read_pipeline::{WorkspaceFileKind, WorkspaceReadDetection};
use super::{
    RenderedRead, RenderedUriRead, archive, media, office, pdf, sqlite, summary, text, url_resource,
};

pub(in crate::workspace) fn render_workspace_read(
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

pub(in crate::workspace) fn render_bounded_prefix_read(
    bytes: &[u8],
    detection: &WorkspaceReadDetection,
    full_byte_len: u64,
    max_output_bytes: u64,
) -> Result<RenderedRead, AgentError> {
    summary::render_bounded_prefix_read(bytes, detection, full_byte_len, max_output_bytes)
}

pub(in crate::workspace) fn render_workspace_uri(
    uri: &str,
    max_input_bytes: u64,
    max_output_bytes: u64,
) -> Result<RenderedUriRead, AgentError> {
    url_resource::render_uri(uri, max_input_bytes, max_output_bytes)
}

pub(super) fn extraction_error(kind: &str, error: impl std::fmt::Display) -> AgentError {
    AgentError::new(
        AgentErrorKind::ToolFailure,
        RetryClassification::UserActionNeeded,
        format!("{kind} extraction failed: {error}"),
    )
}
