//! Concrete workspace tool helpers layered over core tool/effect contracts. Use these
//! modules for bounded read, search, edit, write, and format-aware extraction
//! behavior under a host-selected workspace policy. Reads search local files;
//! edit/write helpers may mutate files only through explicit executor calls. This
//! file contains the legacy office portion of that contract.
//!
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use super::{RenderedRead, add_truncation_guidance, text};
use crate::workspace::{
    read_pipeline::{WorkspaceDocumentMetadata, WorkspaceReaderStep},
    util::{tool_failure, truncate_bytes},
};

/// Renders or detects bounded workspace content for
/// workspace::readers::legacy_office. It may read already-approved local file
/// data but does not mutate the workspace.
pub(super) fn render_legacy_office(
    path: &Path,
    bytes: &[u8],
    max_output_bytes: u64,
) -> Result<RenderedRead, agent_sdk_core::AgentError> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("legacy")
        .to_ascii_lowercase();
    let mut warnings = vec![format!(
        "legacy .{extension} reader is a bounded fallback, not full Office binary layout fidelity"
    )];
    let mut parser = "workspace-legacy-office-fallback:v1".to_string();
    let extracted = if let Some(sidecar) = legacy_text_sidecar(path) {
        if fs::symlink_metadata(&sidecar)
            .map_err(tool_failure)?
            .file_type()
            .is_symlink()
        {
            warnings.push("legacy Office text sidecar symlink was ignored".to_string());
            String::new()
        } else {
            parser = "workspace-legacy-office-sidecar:v1".to_string();
            read_sidecar(&sidecar, max_output_bytes, &mut warnings)?
        }
    } else if let Ok(text) = std::str::from_utf8(bytes) {
        if text.contains('\0') {
            String::new()
        } else {
            warnings.push(
                "legacy Office bytes were valid UTF-8 and emitted as fallback text".to_string(),
            );
            truncate_bytes(text, max_output_bytes as usize)
        }
    } else {
        String::new()
    };

    if extracted.trim().is_empty() {
        warnings.push(
            "legacy Office binary text was not extractable without a host converter or sidecar"
                .to_string(),
        );
        let summary = format!(
            "Legacy Office .{extension} document detected ({} bytes). No model-visible binary bytes were emitted.",
            bytes.len()
        );
        let truncated = summary.len() as u64 > max_output_bytes;
        let mut rendered = RenderedRead {
            content: if truncated {
                truncate_bytes(&summary, max_output_bytes as usize)
            } else {
                summary.clone()
            },
            content_summary: Some(summary),
            truncated,
            binary: true,
            anchors: Vec::new(),
            reader_pipeline: vec![
                WorkspaceReaderStep::DetectFileType,
                WorkspaceReaderStep::ExtractLegacyOfficeText,
                WorkspaceReaderStep::SummarizeBinary,
            ],
            media: None,
            document: Some(WorkspaceDocumentMetadata {
                parser,
                page_count: None,
                extracted_chars: 0,
                ocr: None,
                warnings: warnings.clone(),
            }),
            archive: None,
            sqlite: None,
            resource: None,
            warnings,
        };
        add_truncation_guidance(&mut rendered);
        return Ok(rendered);
    }

    let mut rendered = text::render_text(&extracted, max_output_bytes, false, true);
    rendered.reader_pipeline = vec![
        WorkspaceReaderStep::DetectFileType,
        WorkspaceReaderStep::ExtractLegacyOfficeText,
    ];
    rendered.content_summary = Some(format!(
        "Legacy Office fallback extracted {} character(s).",
        extracted.chars().count()
    ));
    rendered.document = Some(WorkspaceDocumentMetadata {
        parser,
        page_count: None,
        extracted_chars: extracted.chars().count(),
        ocr: None,
        warnings: warnings.clone(),
    });
    rendered.warnings.extend(warnings);
    add_truncation_guidance(&mut rendered);
    Ok(rendered)
}

fn legacy_text_sidecar(path: &Path) -> Option<PathBuf> {
    let extension = path.extension()?.to_str()?;
    let candidate = path.with_extension(format!("{extension}.txt"));
    if candidate.exists() {
        return Some(candidate);
    }
    let file_name = path.file_name()?.to_str()?;
    let candidate = path.with_file_name(format!("{file_name}.txt"));
    if candidate.exists() {
        return Some(candidate);
    }
    None
}

fn read_sidecar(
    path: &Path,
    max_output_bytes: u64,
    warnings: &mut Vec<String>,
) -> Result<String, agent_sdk_core::AgentError> {
    let mut file = fs::File::open(path).map_err(tool_failure)?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(max_output_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(tool_failure)?;
    if bytes.len() as u64 > max_output_bytes {
        bytes.truncate(max_output_bytes as usize);
        warnings.push("legacy Office text sidecar was truncated".to_string());
    }
    Ok(String::from_utf8_lossy(&bytes).to_string())
}
