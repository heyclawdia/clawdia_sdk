use agent_sdk_core::AgentError;

use super::{RenderedRead, add_truncation_guidance, text};
use crate::workspace::{
    read_pipeline::{WorkspaceFileKind, WorkspaceReadDetection, WorkspaceReaderStep},
    util::truncate_bytes,
};

pub(super) fn render_binary_summary(bytes: &[u8], max_output_bytes: u64) -> RenderedRead {
    let mut rendered = RenderedRead {
        content: String::new(),
        content_summary: Some(format!(
            "Binary file detected ({} bytes). No text reader is available for this file type.",
            bytes.len()
        )),
        truncated: bytes.len() as u64 > max_output_bytes,
        binary: true,
        anchors: Vec::new(),
        reader_pipeline: vec![
            WorkspaceReaderStep::DetectFileType,
            WorkspaceReaderStep::SummarizeBinary,
        ],
        media: None,
        document: None,
        archive: None,
        sqlite: None,
        resource: None,
        warnings: vec!["binary content was not emitted as text".to_string()],
    };
    add_truncation_guidance(&mut rendered);
    rendered
}

pub(super) fn render_bounded_prefix_read(
    bytes: &[u8],
    detection: &WorkspaceReadDetection,
    full_byte_len: u64,
    max_output_bytes: u64,
) -> Result<RenderedRead, AgentError> {
    let guidance = format!(
        "file exceeds workspace max_file_bytes; returned a bounded prefix of {} byte(s) from {} byte(s). Use workspace_search/grep or a narrower range read for more.",
        bytes.len(),
        full_byte_len
    );
    let mut rendered = match detection.kind {
        WorkspaceFileKind::Text | WorkspaceFileKind::Markdown | WorkspaceFileKind::Json => {
            let safe_text = String::from_utf8_lossy(bytes);
            let mut rendered = text::render_text(&safe_text, max_output_bytes, true, false);
            if !matches!(std::str::from_utf8(bytes), Ok(_)) {
                rendered.warnings.push(
                    "bounded prefix ended inside an invalid UTF-8 sequence; replacement characters may appear at the prefix boundary"
                        .to_string(),
                );
            }
            rendered
        }
        _ => {
            let summary = format!(
                "{:?} file is larger than the workspace full-read cap ({} bytes). Parser adapters that require the whole file were not run.\n{}",
                detection.kind, full_byte_len, guidance
            );
            let truncated = summary.len() as u64 > max_output_bytes;
            RenderedRead {
                content: if truncated {
                    truncate_bytes(&summary, max_output_bytes as usize)
                } else {
                    summary.clone()
                },
                content_summary: Some(summary),
                truncated: true,
                binary: detection.binary,
                anchors: Vec::new(),
                reader_pipeline: vec![
                    WorkspaceReaderStep::DetectFileType,
                    WorkspaceReaderStep::ReadBoundedPrefix,
                    WorkspaceReaderStep::SummarizeBinary,
                ],
                media: None,
                document: None,
                archive: None,
                sqlite: None,
                resource: None,
                warnings: Vec::new(),
            }
        }
    };
    if !rendered
        .reader_pipeline
        .contains(&WorkspaceReaderStep::ReadBoundedPrefix)
    {
        rendered
            .reader_pipeline
            .push(WorkspaceReaderStep::ReadBoundedPrefix);
    }
    rendered.truncated = true;
    add_truncation_guidance(&mut rendered);
    rendered.warnings.push(guidance);
    rendered
        .warnings
        .push("content_hash covers returned prefix bytes, not the full file".to_string());
    Ok(rendered)
}
