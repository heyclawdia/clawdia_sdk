use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceReadDetection {
    pub kind: WorkspaceFileKind,
    pub mime_type: String,
    pub extension: Option<String>,
    pub binary: bool,
    pub confidence: WorkspaceFileTypeConfidence,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceMediaMetadata {
    pub format: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub color_type: Option<String>,
    pub decoded: bool,
    pub parser: String,
    pub embedded_previews: Vec<WorkspaceEmbeddedPreviewMetadata>,
    pub raw_sensor: Option<WorkspaceRawSensorMetadata>,
    pub apple_photos: Option<WorkspaceApplePhotosMetadata>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceDocumentMetadata {
    pub parser: String,
    pub page_count: Option<usize>,
    pub extracted_chars: usize,
    pub ocr: Option<WorkspaceOcrMetadata>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceOcrMetadata {
    pub parser: String,
    pub sidecar_path: Option<String>,
    pub byte_len: u64,
    pub extracted_chars: usize,
    pub truncated: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceEmbeddedPreviewMetadata {
    pub mime_type: String,
    pub byte_len: u64,
    pub offset: Option<u64>,
    pub content_hash: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceRawSensorMetadata {
    pub bits_per_sample: Option<u16>,
    pub compression: Option<u16>,
    pub photometric_interpretation: Option<u16>,
    pub strip_count: usize,
    pub strip_byte_len: u64,
    pub decoded_pixels: bool,
    pub sample_hash: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceApplePhotosMetadata {
    pub sidecar_path: String,
    pub byte_len: u64,
    pub adjustment_count: usize,
    pub parser: String,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceArchiveEntry {
    pub path: String,
    pub byte_len: u64,
    pub directory: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceArchiveMetadata {
    pub parser: String,
    pub entry_count: usize,
    pub entries: Vec<WorkspaceArchiveEntry>,
    pub truncated: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceSqliteTableMetadata {
    pub name: String,
    pub kind: String,
    pub columns: Vec<String>,
    pub sample_rows: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceSqliteMetadata {
    pub parser: String,
    pub table_count: usize,
    pub tables: Vec<WorkspaceSqliteTableMetadata>,
    pub truncated: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceResourceMetadata {
    pub scheme: String,
    pub source: String,
    pub byte_len: u64,
    pub parser: String,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceFileKind {
    Text,
    Markdown,
    Json,
    Pdf,
    Image,
    RawImage,
    OfficeDocument,
    Archive,
    SqliteDatabase,
    UrlResource,
    Binary,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceFileTypeConfidence {
    Magic,
    Extension,
    Utf8,
    Fallback,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceReaderStep {
    DetectFileType,
    DecodeUtf8Text,
    ExtractPdfText,
    InspectImageMetadata,
    InspectRawImageMetadata,
    InspectRawPreview,
    InspectApplePhotosAdjustments,
    ApplyOcrFallback,
    ExtractOfficeText,
    ExtractLegacyOfficeText,
    ListArchiveEntries,
    InspectSqliteDatabase,
    ReadDataUrl,
    FailClosedExternalResource,
    ReadBoundedPrefix,
    SummarizeBinary,
}

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
