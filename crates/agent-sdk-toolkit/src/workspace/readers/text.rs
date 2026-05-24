use agent_sdk_core::{AgentError, AgentErrorKind, RetryClassification};

use super::{RenderedRead, TRUNCATION_GUIDANCE};
use crate::workspace::{
    anchor::HashLineAnchor,
    read_pipeline::WorkspaceReaderStep,
    util::{hash_line, truncate_bytes},
};

pub(super) fn render_utf8_text(
    bytes: &[u8],
    max_output_bytes: u64,
    editable: bool,
) -> Result<RenderedRead, AgentError> {
    let text = String::from_utf8(bytes.to_vec()).map_err(|error| {
        AgentError::new(
            AgentErrorKind::ToolFailure,
            RetryClassification::UserActionNeeded,
            format!("workspace read expected UTF-8 text after detection: {error}"),
        )
    })?;
    Ok(render_text(&text, max_output_bytes, editable, false))
}

pub(super) fn render_text(
    text: &str,
    max_output_bytes: u64,
    editable: bool,
    binary_source: bool,
) -> RenderedRead {
    let truncated = text.len() as u64 > max_output_bytes;
    let content = if truncated {
        truncate_bytes(text, max_output_bytes as usize)
    } else {
        text.to_string()
    };
    let anchors = if editable {
        text.lines()
            .enumerate()
            .map(|(index, line)| HashLineAnchor {
                line: index + 1,
                before_hash: hash_line(line),
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    RenderedRead {
        content,
        content_summary: None,
        truncated,
        binary: binary_source,
        anchors,
        reader_pipeline: vec![
            WorkspaceReaderStep::DetectFileType,
            WorkspaceReaderStep::DecodeUtf8Text,
        ],
        media: None,
        document: None,
        archive: None,
        sqlite: None,
        resource: None,
        warnings: if truncated {
            vec![TRUNCATION_GUIDANCE.to_string()]
        } else {
            Vec::new()
        },
    }
}
