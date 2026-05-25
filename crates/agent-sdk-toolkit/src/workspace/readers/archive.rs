//! Concrete workspace tool helpers layered over core tool/effect contracts. Use these
//! modules for bounded read, search, edit, write, and format-aware extraction
//! behavior under a host-selected workspace policy. Reads search local files;
//! edit/write helpers may mutate files only through explicit executor calls. This
//! file contains the archive portion of that contract.
//!
use std::{
    io::{Cursor, Read},
    path::Path,
};

use flate2::read::GzDecoder;
use zip::ZipArchive;

use super::{RenderedRead, add_truncation_guidance, text};
use crate::workspace::{
    read_pipeline::{WorkspaceArchiveEntry, WorkspaceArchiveMetadata, WorkspaceReaderStep},
    util::truncate_bytes,
};

const MAX_ARCHIVE_ENTRIES: usize = 200;
const MAX_ARCHIVE_DECOMPRESSED_BYTES: u64 = 4 * 1024 * 1024;

/// Render archive.
/// This parses caller-provided bytes into a bounded rendered read response and does not write
/// workspace files.
pub(super) fn render_archive(
    path: &Path,
    bytes: &[u8],
    max_output_bytes: u64,
) -> Result<RenderedRead, agent_sdk_core::AgentError> {
    if bytes.starts_with(b"PK\x03\x04") {
        return render_zip(bytes, max_output_bytes);
    }
    if is_tgz_path(path) || bytes.starts_with(b"\x1f\x8b") && looks_like_tar_gzip(path) {
        return render_tgz(bytes, max_output_bytes);
    }
    if bytes.starts_with(b"\x1f\x8b") {
        return render_gzip(path, bytes, max_output_bytes);
    }
    if bytes.len() > 262 && bytes.get(257..262) == Some(b"ustar".as_slice()) {
        return render_tar(Cursor::new(bytes), "tar:0.4.46", max_output_bytes);
    }
    Ok(archive_warning_read(
        format!(
            "Archive detected ({} bytes), but no supported archive magic matched.",
            bytes.len()
        ),
        None,
        vec!["unsupported or malformed archive format".to_string()],
        max_output_bytes,
    ))
}

fn render_zip(
    bytes: &[u8],
    max_output_bytes: u64,
) -> Result<RenderedRead, agent_sdk_core::AgentError> {
    let mut archive = match ZipArchive::new(Cursor::new(bytes)) {
        Ok(archive) => archive,
        Err(error) => {
            return Ok(archive_warning_read(
                format!(
                    "Archive detected ({} bytes), but ZIP listing failed: {error}",
                    bytes.len()
                ),
                None,
                vec![format!("ZIP listing failed: {error}")],
                max_output_bytes,
            ));
        }
    };
    let entry_count = archive.len();
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    for index in 0..entry_count.min(MAX_ARCHIVE_ENTRIES) {
        let file = archive
            .by_index(index)
            .map_err(|error| super::extraction_error("archive", error))?;
        let name = file.name().to_string();
        if unsafe_archive_path(&name) {
            warnings.push(format!("entry with unsafe path skipped: {name}"));
            continue;
        }
        entries.push(WorkspaceArchiveEntry {
            path: name,
            byte_len: file.size(),
            directory: file.is_dir(),
        });
    }
    Ok(archive_listing_read(
        "zip:2.4.2",
        entry_count,
        entries,
        entry_count > MAX_ARCHIVE_ENTRIES,
        warnings,
        max_output_bytes,
    ))
}

fn render_tar<R: Read>(
    reader: R,
    parser: &str,
    max_output_bytes: u64,
) -> Result<RenderedRead, agent_sdk_core::AgentError> {
    let mut archive = tar::Archive::new(reader);
    let entries_iter = match archive.entries() {
        Ok(entries) => entries,
        Err(error) => {
            return Ok(archive_warning_read(
                format!("TAR archive listing failed: {error}"),
                None,
                vec![format!("TAR listing failed: {error}")],
                max_output_bytes,
            ));
        }
    };
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    let mut entry_count = 0usize;
    for entry in entries_iter {
        entry_count += 1;
        if entries.len() >= MAX_ARCHIVE_ENTRIES {
            continue;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                warnings.push(format!("TAR entry skipped: {error}"));
                continue;
            }
        };
        let path = match entry.path() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(error) => {
                warnings.push(format!("TAR entry path skipped: {error}"));
                continue;
            }
        };
        if unsafe_archive_path(&path) {
            warnings.push(format!("entry with unsafe path skipped: {path}"));
            continue;
        }
        entries.push(WorkspaceArchiveEntry {
            path,
            byte_len: entry.size(),
            directory: entry.header().entry_type().is_dir(),
        });
    }
    Ok(archive_listing_read(
        parser,
        entry_count,
        entries,
        entry_count > MAX_ARCHIVE_ENTRIES,
        warnings,
        max_output_bytes,
    ))
}

fn render_tgz(
    bytes: &[u8],
    max_output_bytes: u64,
) -> Result<RenderedRead, agent_sdk_core::AgentError> {
    let mut decoder = GzDecoder::new(Cursor::new(bytes));
    let mut decoded = Vec::new();
    let mut warnings = Vec::new();
    if let Err(error) = decoder
        .by_ref()
        .take(MAX_ARCHIVE_DECOMPRESSED_BYTES + 1)
        .read_to_end(&mut decoded)
    {
        return Ok(archive_warning_read(
            format!("TGZ archive detected, but decompression failed: {error}"),
            None,
            vec![format!("TGZ decompression failed: {error}")],
            max_output_bytes,
        ));
    }
    let truncated_decompression = decoded.len() as u64 > MAX_ARCHIVE_DECOMPRESSED_BYTES;
    if truncated_decompression {
        decoded.truncate(MAX_ARCHIVE_DECOMPRESSED_BYTES as usize);
        warnings.push("TGZ decompression hit the reader cap".to_string());
    }
    let mut rendered = render_tar(
        Cursor::new(decoded),
        "tar+gzip:flate2:1.1.9/tar:0.4.46",
        max_output_bytes,
    )?;
    if truncated_decompression {
        rendered.truncated = true;
        rendered.warnings.extend(warnings.clone());
        if let Some(archive) = &mut rendered.archive {
            archive.truncated = true;
            archive.warnings.extend(warnings);
        }
        add_truncation_guidance(&mut rendered);
    }
    Ok(rendered)
}

fn render_gzip(
    path: &Path,
    bytes: &[u8],
    max_output_bytes: u64,
) -> Result<RenderedRead, agent_sdk_core::AgentError> {
    let mut decoder = GzDecoder::new(Cursor::new(bytes));
    let mut output = Vec::new();
    let read_result = decoder
        .by_ref()
        .take(MAX_ARCHIVE_DECOMPRESSED_BYTES + 1)
        .read_to_end(&mut output);
    let mut warnings = Vec::new();
    if let Err(error) = read_result {
        warnings.push(format!("GZIP decompression failed: {error}"));
        return Ok(archive_warning_read(
            format!("GZIP archive detected, but decompression failed: {error}"),
            None,
            warnings,
            max_output_bytes,
        ));
    }
    let truncated_decompression = output.len() as u64 > MAX_ARCHIVE_DECOMPRESSED_BYTES;
    if truncated_decompression {
        output.truncate(MAX_ARCHIVE_DECOMPRESSED_BYTES as usize);
        warnings.push("GZIP decompression hit the reader cap".to_string());
    }
    let entry_name = gzip_entry_name(path);
    let archive = WorkspaceArchiveMetadata {
        parser: "gzip:flate2:1.1.9".to_string(),
        entry_count: 1,
        entries: vec![WorkspaceArchiveEntry {
            path: entry_name,
            byte_len: output.len() as u64,
            directory: false,
        }],
        truncated: truncated_decompression,
        warnings: warnings.clone(),
    };
    let content = if let Ok(text) = std::str::from_utf8(&output) {
        let mut rendered = text::render_text(text, max_output_bytes, false, true);
        rendered.reader_pipeline = vec![
            WorkspaceReaderStep::DetectFileType,
            WorkspaceReaderStep::ListArchiveEntries,
        ];
        rendered.archive = Some(archive);
        rendered.warnings.extend(warnings);
        if truncated_decompression {
            rendered.truncated = true;
        }
        add_truncation_guidance(&mut rendered);
        return Ok(rendered);
    } else {
        format!(
            "GZIP archive: 1 decompressed entry, {} byte(s). Binary decompressed content was not emitted.",
            output.len()
        )
    };
    let mut rendered = archive_warning_read(content, Some(archive), warnings, max_output_bytes);
    if truncated_decompression {
        rendered.truncated = true;
        add_truncation_guidance(&mut rendered);
    }
    Ok(rendered)
}

fn archive_listing_read(
    parser: &str,
    entry_count: usize,
    entries: Vec<WorkspaceArchiveEntry>,
    truncated_entries: bool,
    warnings: Vec<String>,
    max_output_bytes: u64,
) -> RenderedRead {
    let mut listing = format!("Archive entries: {entry_count}\n");
    for entry in &entries {
        let suffix = if entry.directory { "/" } else { "" };
        listing.push_str(&format!(
            "- {}{} ({} bytes)\n",
            entry.path, suffix, entry.byte_len
        ));
    }
    if truncated_entries {
        listing.push_str(&format!(
            "[{} archive entries omitted]\n",
            entry_count.saturating_sub(MAX_ARCHIVE_ENTRIES)
        ));
    }
    archive_warning_read(
        listing,
        Some(WorkspaceArchiveMetadata {
            parser: parser.to_string(),
            entry_count,
            entries,
            truncated: truncated_entries,
            warnings: warnings.clone(),
        }),
        warnings,
        max_output_bytes,
    )
}

fn archive_warning_read(
    summary: String,
    archive: Option<WorkspaceArchiveMetadata>,
    warnings: Vec<String>,
    max_output_bytes: u64,
) -> RenderedRead {
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
            WorkspaceReaderStep::ListArchiveEntries,
            WorkspaceReaderStep::SummarizeBinary,
        ],
        media: None,
        document: None,
        archive,
        sqlite: None,
        resource: None,
        warnings,
    };
    add_truncation_guidance(&mut rendered);
    rendered
}

fn is_tgz_path(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    name.ends_with(".tgz") || name.ends_with(".tar.gz")
}

fn looks_like_tar_gzip(path: &Path) -> bool {
    is_tgz_path(path)
}

fn gzip_entry_name(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("gzip-entry");
    name.strip_suffix(".gz").unwrap_or(name).to_string()
}

fn unsafe_archive_path(path: &str) -> bool {
    path.starts_with('/')
        || path
            .split('/')
            .any(|part| matches!(part, "." | "..") || part.contains('\\'))
}
