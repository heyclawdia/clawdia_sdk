//! Concrete workspace tool helpers layered over core tool/effect contracts. Use these
//! modules for bounded read, search, edit, write, and format-aware extraction
//! behavior under a host-selected workspace policy. Reads search local files;
//! edit/write helpers may mutate files only through explicit executor calls. This
//! file contains the ocr portion of that contract.
//!
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use agent_sdk_core::AgentError;

use crate::workspace::{
    read_pipeline::WorkspaceOcrMetadata,
    util::{tool_failure, truncate_bytes},
};

/// Workspace ocr sidecar request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub(super) struct OcrSidecar {
    /// Text used by this record or request.
    pub text: String,
    /// Metadata used by this record or request.
    pub metadata: WorkspaceOcrMetadata,
}

/// Read ocr sidecar.
/// This reads the configured OCR sidecar file when present and returns bounded extracted text
/// metadata.
pub(super) fn read_ocr_sidecar(
    path: &Path,
    max_output_bytes: u64,
) -> Result<Option<OcrSidecar>, AgentError> {
    let Some(sidecar) = ocr_sidecar_path(path) else {
        return Ok(None);
    };
    if fs::symlink_metadata(&sidecar)
        .map_err(tool_failure)?
        .file_type()
        .is_symlink()
    {
        return Ok(Some(OcrSidecar {
            text: String::new(),
            metadata: WorkspaceOcrMetadata {
                parser: "workspace-ocr-sidecar:v1".to_string(),
                sidecar_path: Some(display_name(&sidecar)),
                byte_len: 0,
                extracted_chars: 0,
                truncated: false,
                warnings: vec!["OCR sidecar symlink was ignored".to_string()],
            },
        }));
    }
    let byte_len = fs::metadata(&sidecar).map_err(tool_failure)?.len();
    let mut file = fs::File::open(&sidecar).map_err(tool_failure)?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(max_output_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(tool_failure)?;
    let truncated = bytes.len() as u64 > max_output_bytes;
    if truncated {
        bytes.truncate(max_output_bytes as usize);
    }
    let text = String::from_utf8_lossy(&bytes).to_string();
    let text = if truncated {
        truncate_bytes(&text, max_output_bytes as usize)
    } else {
        text
    };
    let mut warnings = vec![
        "OCR text came from a bounded host/precomputed sidecar, not an ambient OCR engine"
            .to_string(),
    ];
    if truncated {
        warnings.push("OCR sidecar output was truncated".to_string());
    }
    Ok(Some(OcrSidecar {
        metadata: WorkspaceOcrMetadata {
            parser: "workspace-ocr-sidecar:v1".to_string(),
            sidecar_path: Some(display_name(&sidecar)),
            byte_len,
            extracted_chars: text.chars().count(),
            truncated,
            warnings: warnings.clone(),
        },
        text,
    }))
}

fn ocr_sidecar_path(path: &Path) -> Option<PathBuf> {
    let extension = path.extension()?.to_str()?;
    let candidate = path.with_extension(format!("{extension}.ocr.txt"));
    if candidate.exists() {
        return Some(candidate);
    }
    let file_name = path.file_name()?.to_str()?;
    let candidate = path.with_file_name(format!("{file_name}.ocr.txt"));
    if candidate.exists() {
        return Some(candidate);
    }
    None
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("ocr-sidecar")
        .to_string()
}
