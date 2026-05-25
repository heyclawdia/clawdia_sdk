//! Concrete workspace tool helpers layered over core tool/effect contracts. Use these
//! modules for bounded read, search, edit, write, and format-aware extraction
//! behavior under a host-selected workspace policy. Reads search local files;
//! edit/write helpers may mutate files only through explicit executor calls. This
//! file contains the dispatch portion of that contract.
//!
use std::path::Path;

use agent_sdk_core::{AgentError, AgentErrorKind, RetryClassification};

use super::super::read_pipeline::{WorkspaceFileKind, WorkspaceReadDetection};
use super::{
    RenderedRead, RenderedUriRead, archive, media, office, pdf, sqlite, summary, text, url_resource,
};

/// Render workspace read.
/// This parses caller-provided bytes into a bounded rendered read response and does not write
/// workspace files.
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

/// Renders or detects bounded workspace content for
/// workspace::readers::dispatch. It may read already-approved local file data
/// but does not mutate the workspace.
pub(in crate::workspace) fn render_bounded_prefix_read(
    bytes: &[u8],
    detection: &WorkspaceReadDetection,
    full_byte_len: u64,
    max_output_bytes: u64,
) -> Result<RenderedRead, AgentError> {
    summary::render_bounded_prefix_read(bytes, detection, full_byte_len, max_output_bytes)
}

/// Render workspace uri.
/// This parses caller-provided bytes into a bounded rendered read response and does not write
/// workspace files.
pub(in crate::workspace) fn render_workspace_uri(
    uri: &str,
    max_input_bytes: u64,
    max_output_bytes: u64,
) -> Result<RenderedUriRead, AgentError> {
    url_resource::render_uri(uri, max_input_bytes, max_output_bytes)
}

/// Builds the extraction error value.
/// This is data construction and performs no I/O, journal append, event publication, or process
pub(super) fn extraction_error(kind: &str, error: impl std::fmt::Display) -> AgentError {
    AgentError::new(
        AgentErrorKind::ToolFailure,
        RetryClassification::UserActionNeeded,
        format!("{kind} extraction failed: {error}"),
    )
}
