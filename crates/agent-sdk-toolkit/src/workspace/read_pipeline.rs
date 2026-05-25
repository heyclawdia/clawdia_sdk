//! Format-aware workspace read pipeline and metadata records. Use this module to
//! detect file kind, choose bounded extraction behavior, and describe truncation or
//! parser fallbacks. Pipeline functions read local files but must not leak raw binary
//! content by default.
//!
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Workspace workspace read detection request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceReadDetection {
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: WorkspaceFileKind,
    /// Detected or declared MIME type used for reader selection and
    /// provider-safe summaries.
    pub mime_type: String,
    /// Lowercase file extension used as one detection signal; it is not
    /// trusted as sole authority.
    pub extension: Option<String>,
    /// Whether the input is treated as binary so raw bytes are not exposed by
    /// default.
    pub binary: bool,
    /// Confidence level for file-kind detection.
    pub confidence: WorkspaceFileTypeConfidence,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Workspace workspace media metadata request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceMediaMetadata {
    /// Detected media, document, archive, or parser format.
    pub format: String,
    /// Detected image or media width in pixels when available.
    pub width: Option<u32>,
    /// Detected image or media height in pixels when available.
    pub height: Option<u32>,
    /// Decoded image color type when the parser can determine it.
    pub color_type: Option<String>,
    /// Whether the parser decoded the media/document enough to produce
    /// structured metadata.
    pub decoded: bool,
    /// Parser or fallback path that produced this metadata.
    pub parser: String,
    /// Descriptions of embedded previews discovered in RAW or container
    /// media.
    pub embedded_previews: Vec<WorkspaceEmbeddedPreviewMetadata>,
    /// RAW sensor metadata discovered without demosaicing full image data.
    pub raw_sensor: Option<WorkspaceRawSensorMetadata>,
    /// Apple Photos adjustment sidecar metadata, when a sidecar is present.
    pub apple_photos: Option<WorkspaceApplePhotosMetadata>,
    /// Non-fatal warnings from bounded readers, parsers, or policy
    /// downgrades.
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Workspace workspace document metadata request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceDocumentMetadata {
    /// Parser or fallback path that produced this metadata.
    pub parser: String,
    /// Count of page items observed or included in this record.
    pub page_count: Option<usize>,
    /// Number of text characters extracted before truncation or parser
    /// limits.
    pub extracted_chars: usize,
    /// OCR requirement or sidecar metadata for scanned PDFs/images.
    pub ocr: Option<WorkspaceOcrMetadata>,
    /// Non-fatal warnings from bounded readers, parsers, or policy
    /// downgrades.
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Workspace workspace ocr metadata request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceOcrMetadata {
    /// Parser or fallback path that produced this metadata.
    pub parser: String,
    /// Path to a sidecar file used for OCR, Apple Photos adjustments, or
    /// legacy extraction.
    pub sidecar_path: Option<String>,
    /// Observed byte length for the source, sidecar, or extracted record.
    pub byte_len: u64,
    /// Number of text characters extracted before truncation or parser
    /// limits.
    pub extracted_chars: usize,
    /// Whether output was shortened by byte, item, page, archive, or parser
    /// limits.
    pub truncated: bool,
    /// Non-fatal warnings from bounded readers, parsers, or policy
    /// downgrades.
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Workspace workspace embedded preview metadata request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceEmbeddedPreviewMetadata {
    /// Detected or declared MIME type used for reader selection and
    /// provider-safe summaries.
    pub mime_type: String,
    /// Observed byte length for the source, sidecar, or extracted record.
    pub byte_len: u64,
    /// Byte offset where this excerpt, prefix, or sample begins.
    pub offset: Option<u64>,
    /// Stable hash for the bytes or canonical payload used for stale checks
    /// and fingerprints.
    pub content_hash: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Workspace workspace raw sensor metadata request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceRawSensorMetadata {
    /// Optional bits per sample value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub bits_per_sample: Option<u16>,
    /// Optional compression value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub compression: Option<u16>,
    /// Optional photometric interpretation value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub photometric_interpretation: Option<u16>,
    /// Count of strip items observed or included in this record.
    pub strip_count: usize,
    /// strip byte len used for bounds checks, summaries, or truncation
    /// evidence.
    pub strip_byte_len: u64,
    /// Whether decoded pixels is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub decoded_pixels: bool,
    /// Deterministic sample hash used for stale checks, package evidence, or
    /// replay comparisons.
    pub sample_hash: Option<String>,
    /// Non-fatal warnings from bounded readers, parsers, or policy
    /// downgrades.
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Workspace workspace apple photos metadata request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceApplePhotosMetadata {
    /// Path to a sidecar file used for OCR, Apple Photos adjustments, or
    /// legacy extraction.
    pub sidecar_path: String,
    /// Observed byte length for the source, sidecar, or extracted record.
    pub byte_len: u64,
    /// Count of adjustment items observed or included in this record.
    pub adjustment_count: usize,
    /// Parser or fallback path that produced this metadata.
    pub parser: String,
    /// Non-fatal warnings from bounded readers, parsers, or policy
    /// downgrades.
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Workspace workspace archive entry request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceArchiveEntry {
    /// Workspace-relative or resource path selected by the request or result.
    pub path: String,
    /// Observed byte length for the source, sidecar, or extracted record.
    pub byte_len: u64,
    /// Whether directory is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub directory: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Workspace workspace archive metadata request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceArchiveMetadata {
    /// Parser or fallback path that produced this metadata.
    pub parser: String,
    /// Count of entry items observed or included in this record.
    pub entry_count: usize,
    /// Bounded entries included in this record. Limits and truncation are
    /// represented by companion metadata when applicable.
    pub entries: Vec<WorkspaceArchiveEntry>,
    /// Whether output was shortened by byte, item, page, archive, or parser
    /// limits.
    pub truncated: bool,
    /// Non-fatal warnings from bounded readers, parsers, or policy
    /// downgrades.
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Workspace workspace sqlite table metadata request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceSqliteTableMetadata {
    /// Human-readable or protocol-visible name for this SDK item.
    pub name: String,
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: String,
    /// Bounded columns included in this record. Limits and truncation are
    /// represented by companion metadata when applicable.
    pub columns: Vec<String>,
    /// Bounded sample rows included in this record. Limits and truncation are
    /// represented by companion metadata when applicable.
    pub sample_rows: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Workspace workspace sqlite metadata request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceSqliteMetadata {
    /// Parser or fallback path that produced this metadata.
    pub parser: String,
    /// Count of table items observed or included in this record.
    pub table_count: usize,
    /// Bounded tables included in this record. Limits and truncation are
    /// represented by companion metadata when applicable.
    pub tables: Vec<WorkspaceSqliteTableMetadata>,
    /// Whether output was shortened by byte, item, page, archive, or parser
    /// limits.
    pub truncated: bool,
    /// Non-fatal warnings from bounded readers, parsers, or policy
    /// downgrades.
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Workspace workspace resource metadata request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceResourceMetadata {
    /// URI scheme resolved by the resource reader.
    pub scheme: String,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: String,
    /// Observed byte length for the source, sidecar, or extracted record.
    pub byte_len: u64,
    /// Parser or fallback path that produced this metadata.
    pub parser: String,
    /// Non-fatal warnings from bounded readers, parsers, or policy
    /// downgrades.
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite workspace file kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum WorkspaceFileKind {
    /// Use this variant when the contract needs to represent text; selecting it has no side effect by itself.
    Text,
    /// Use this variant when the contract needs to represent markdown; selecting it has no side effect by itself.
    Markdown,
    /// Use this variant when the contract needs to represent json; selecting it has no side effect by itself.
    Json,
    /// Use this variant when the contract needs to represent pdf; selecting it has no side effect by itself.
    Pdf,
    /// Use this variant when the contract needs to represent image; selecting it has no side effect by itself.
    Image,
    /// Use this variant when the contract needs to represent raw image; selecting it has no side effect by itself.
    RawImage,
    /// Use this variant when the contract needs to represent office document; selecting it has no side effect by itself.
    OfficeDocument,
    /// Use this variant when the contract needs to represent archive; selecting it has no side effect by itself.
    Archive,
    /// Use this variant when the contract needs to represent sqlite database; selecting it has no side effect by itself.
    SqliteDatabase,
    /// Use this variant when the contract needs to represent url resource; selecting it has no side effect by itself.
    UrlResource,
    /// Use this variant when the contract needs to represent binary; selecting it has no side effect by itself.
    Binary,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite workspace file type confidence cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum WorkspaceFileTypeConfidence {
    /// Use this variant when the contract needs to represent magic; selecting it has no side effect by itself.
    Magic,
    /// Use this variant when the contract needs to represent extension; selecting it has no side effect by itself.
    Extension,
    /// Use this variant when the contract needs to represent utf8; selecting it has no side effect by itself.
    Utf8,
    /// Use this variant when the contract needs to represent fallback; selecting it has no side effect by itself.
    Fallback,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite workspace reader step cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum WorkspaceReaderStep {
    /// Use this variant when the contract needs to represent detect file type; selecting it has no side effect by itself.
    DetectFileType,
    /// Use this variant when the contract needs to represent decode utf8 text; selecting it has no side effect by itself.
    DecodeUtf8Text,
    /// Use this variant when the contract needs to represent extract pdf text; selecting it has no side effect by itself.
    ExtractPdfText,
    /// Use this variant when the contract needs to represent inspect image metadata; selecting it has no side effect by itself.
    InspectImageMetadata,
    /// Use this variant when the contract needs to represent inspect raw image metadata; selecting it has no side effect by itself.
    InspectRawImageMetadata,
    /// Use this variant when the contract needs to represent inspect raw preview; selecting it has no side effect by itself.
    InspectRawPreview,
    /// Use this variant when the contract needs to represent inspect apple photos adjustments; selecting it has no side effect by itself.
    InspectApplePhotosAdjustments,
    /// Use this variant when the contract needs to represent apply ocr fallback; selecting it has no side effect by itself.
    ApplyOcrFallback,
    /// Use this variant when the contract needs to represent extract office text; selecting it has no side effect by itself.
    ExtractOfficeText,
    /// Use this variant when the contract needs to represent extract legacy office text; selecting it has no side effect by itself.
    ExtractLegacyOfficeText,
    /// Use this variant when the contract needs to represent list archive entries; selecting it has no side effect by itself.
    ListArchiveEntries,
    /// Use this variant when the contract needs to represent inspect sqlite database; selecting it has no side effect by itself.
    InspectSqliteDatabase,
    /// Use this variant when the contract needs to represent read data url; selecting it has no side effect by itself.
    ReadDataUrl,
    /// Use this variant when the contract needs to represent fail closed external resource; selecting it has no side effect by itself.
    FailClosedExternalResource,
    /// Use this variant when the contract needs to represent read bounded prefix; selecting it has no side effect by itself.
    ReadBoundedPrefix,
    /// Use this variant when the contract needs to represent summarize binary; selecting it has no side effect by itself.
    SummarizeBinary,
}

/// Detect workspace file.
/// This inspects the path and byte prefix to choose a reader route and performs no I/O.
pub fn detect_workspace_file(path: &Path, bytes: &[u8]) -> WorkspaceReadDetection {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase());

    if bytes.starts_with(b"%PDF-") {
        return detected(
            WorkspaceFileKind::Pdf,
            "application/pdf",
            extension,
            true,
            WorkspaceFileTypeConfidence::Magic,
        );
    }

    if bytes.starts_with(b"SQLite format 3\0") {
        return detected(
            WorkspaceFileKind::SqliteDatabase,
            "application/vnd.sqlite3",
            extension,
            true,
            WorkspaceFileTypeConfidence::Magic,
        );
    }

    if let Some((kind, mime_type)) = detect_magic_image(bytes, extension.as_deref()) {
        return detected(
            kind,
            mime_type,
            extension,
            true,
            WorkspaceFileTypeConfidence::Magic,
        );
    }

    if bytes.starts_with(b"PK\x03\x04") {
        return match extension.as_deref() {
            Some("docx") => detected(
                WorkspaceFileKind::OfficeDocument,
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                extension,
                true,
                WorkspaceFileTypeConfidence::Magic,
            ),
            Some("xlsx") => detected(
                WorkspaceFileKind::OfficeDocument,
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                extension,
                true,
                WorkspaceFileTypeConfidence::Magic,
            ),
            Some("pptx") => detected(
                WorkspaceFileKind::OfficeDocument,
                "application/vnd.openxmlformats-officedocument.presentationml.presentation",
                extension,
                true,
                WorkspaceFileTypeConfidence::Magic,
            ),
            _ => detected(
                WorkspaceFileKind::Archive,
                "application/zip",
                extension,
                true,
                WorkspaceFileTypeConfidence::Magic,
            ),
        };
    }

    if bytes.starts_with(b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1") {
        return detected(
            WorkspaceFileKind::OfficeDocument,
            "application/vnd.ms-office",
            extension,
            true,
            WorkspaceFileTypeConfidence::Magic,
        );
    }

    if bytes.starts_with(b"\x1f\x8b") {
        return detected(
            WorkspaceFileKind::Archive,
            "application/gzip",
            extension,
            true,
            WorkspaceFileTypeConfidence::Magic,
        );
    }

    if is_tar_archive(bytes) {
        return detected(
            WorkspaceFileKind::Archive,
            "application/x-tar",
            extension,
            true,
            WorkspaceFileTypeConfidence::Magic,
        );
    }

    if is_raw_extension(extension.as_deref()) {
        return detected(
            WorkspaceFileKind::RawImage,
            "image/x-raw",
            extension,
            true,
            WorkspaceFileTypeConfidence::Extension,
        );
    }

    if let Some((kind, mime_type)) = detect_extension_kind(extension.as_deref()) {
        if matches!(
            kind,
            WorkspaceFileKind::Text | WorkspaceFileKind::Markdown | WorkspaceFileKind::Json
        ) && !is_utf8_text(bytes)
        {
            return detected(
                WorkspaceFileKind::Binary,
                "application/octet-stream",
                extension,
                true,
                WorkspaceFileTypeConfidence::Fallback,
            );
        }
        let binary = !matches!(
            kind,
            WorkspaceFileKind::Text | WorkspaceFileKind::Markdown | WorkspaceFileKind::Json
        );
        return detected(
            kind,
            mime_type,
            extension,
            binary,
            WorkspaceFileTypeConfidence::Extension,
        );
    }

    if is_utf8_text(bytes) {
        return detected(
            WorkspaceFileKind::Text,
            "text/plain; charset=utf-8",
            extension,
            false,
            WorkspaceFileTypeConfidence::Utf8,
        );
    }

    detected(
        WorkspaceFileKind::Binary,
        "application/octet-stream",
        extension,
        true,
        WorkspaceFileTypeConfidence::Fallback,
    )
}

fn detected(
    kind: WorkspaceFileKind,
    mime_type: &str,
    extension: Option<String>,
    binary: bool,
    confidence: WorkspaceFileTypeConfidence,
) -> WorkspaceReadDetection {
    WorkspaceReadDetection {
        kind,
        mime_type: mime_type.to_string(),
        extension,
        binary,
        confidence,
    }
}

fn detect_magic_image(
    bytes: &[u8],
    extension: Option<&str>,
) -> Option<(WorkspaceFileKind, &'static str)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some((WorkspaceFileKind::Image, "image/png"));
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return Some((WorkspaceFileKind::Image, "image/jpeg"));
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some((WorkspaceFileKind::Image, "image/gif"));
    }
    if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
        if is_raw_extension(extension) {
            return Some((WorkspaceFileKind::RawImage, "image/x-raw"));
        }
        return Some((WorkspaceFileKind::Image, "image/tiff"));
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some((WorkspaceFileKind::Image, "image/webp"));
    }
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if matches!(
            brand,
            b"heic" | b"heix" | b"hevc" | b"hevx" | b"mif1" | b"msf1"
        ) {
            return Some((WorkspaceFileKind::Image, "image/heic"));
        }
        if matches!(brand, b"avif" | b"avis") {
            return Some((WorkspaceFileKind::Image, "image/avif"));
        }
        if matches!(brand, b"crx " | b"crx2" | b"crx3") {
            return Some((WorkspaceFileKind::RawImage, "image/x-canon-cr3"));
        }
    }
    None
}

fn detect_extension_kind(extension: Option<&str>) -> Option<(WorkspaceFileKind, &'static str)> {
    match extension? {
        "md" | "markdown" => Some((WorkspaceFileKind::Markdown, "text/markdown; charset=utf-8")),
        "json" => Some((WorkspaceFileKind::Json, "application/json")),
        "txt" | "rs" | "toml" | "yaml" | "yml" | "js" | "ts" | "tsx" | "jsx" | "py" | "go"
        | "java" | "c" | "cc" | "cpp" | "h" | "hpp" | "swift" | "sh" | "zsh" | "bash" => {
            Some((WorkspaceFileKind::Text, "text/plain; charset=utf-8"))
        }
        "pdf" => Some((WorkspaceFileKind::Pdf, "application/pdf")),
        "png" => Some((WorkspaceFileKind::Image, "image/png")),
        "jpg" | "jpeg" => Some((WorkspaceFileKind::Image, "image/jpeg")),
        "gif" => Some((WorkspaceFileKind::Image, "image/gif")),
        "webp" => Some((WorkspaceFileKind::Image, "image/webp")),
        "heic" | "heif" => Some((WorkspaceFileKind::Image, "image/heic")),
        "avif" => Some((WorkspaceFileKind::Image, "image/avif")),
        "tif" | "tiff" => Some((WorkspaceFileKind::Image, "image/tiff")),
        "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "rtf" | "epub" => Some((
            WorkspaceFileKind::OfficeDocument,
            "application/octet-stream",
        )),
        "sqlite" | "sqlite3" | "db" => {
            Some((WorkspaceFileKind::SqliteDatabase, "application/vnd.sqlite3"))
        }
        "zip" | "tar" | "tgz" | "gz" => {
            Some((WorkspaceFileKind::Archive, "application/octet-stream"))
        }
        _ => None,
    }
}

fn is_raw_extension(extension: Option<&str>) -> bool {
    matches!(
        extension,
        Some(
            "dng"
                | "cr2"
                | "cr3"
                | "nef"
                | "arw"
                | "raf"
                | "rw2"
                | "orf"
                | "pef"
                | "srw"
                | "x3f"
                | "erf"
                | "kdc"
        )
    )
}

fn is_utf8_text(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).is_ok() && !bytes.contains(&0)
}

fn is_tar_archive(bytes: &[u8]) -> bool {
    bytes.len() > 262 && bytes.get(257..262) == Some(b"ustar".as_slice())
}
