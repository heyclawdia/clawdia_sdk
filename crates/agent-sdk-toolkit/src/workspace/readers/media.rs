//! Concrete workspace tool helpers layered over core tool/effect contracts. Use these
//! modules for bounded read, search, edit, write, and format-aware extraction
//! behavior under a host-selected workspace policy. Reads search local files;
//! edit/write helpers may mutate files only through explicit executor calls. This
//! file contains the media portion of that contract.
//!
use std::{
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

use image::ImageReader;

use super::{RenderedRead, add_truncation_guidance, ocr};
use crate::workspace::{
    read_pipeline::{
        WorkspaceApplePhotosMetadata, WorkspaceDocumentMetadata, WorkspaceEmbeddedPreviewMetadata,
        WorkspaceMediaMetadata, WorkspaceRawSensorMetadata, WorkspaceReadDetection,
        WorkspaceReaderStep,
    },
    util::{hash_bytes, tool_failure, truncate_bytes},
};

/// Renders or detects bounded workspace content for
/// workspace::readers::media. It may read already-approved local file data
/// but does not mutate the workspace.
pub(super) fn render_image(
    path: &Path,
    bytes: &[u8],
    detection: &WorkspaceReadDetection,
    max_output_bytes: u64,
) -> Result<RenderedRead, agent_sdk_core::AgentError> {
    let mut warnings = Vec::new();
    let decoded = false;
    let mut width = None;
    let mut height = None;
    let mut color_type = None;

    match ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .and_then(|reader| {
            let dimensions = reader.into_dimensions().map_err(std::io::Error::other)?;
            Ok(dimensions)
        }) {
        Ok(dimensions) => {
            width = Some(dimensions.0);
            height = Some(dimensions.1);
            color_type = Some("metadata-only".to_string());
        }
        Err(error) => {
            if let Some((bmff_width, bmff_height)) = bmff_ispe_dimensions(bytes) {
                width = Some(bmff_width);
                height = Some(bmff_height);
                warnings.push(format!(
                    "image pixels were not decoded, but HEIF/AVIF-style dimensions were read: {error}"
                ));
            } else {
                warnings.push(format!("image metadata decode failed: {error}"));
            }
        }
    }

    let apple_photos = read_apple_photos_sidecar(path, max_output_bytes)?;
    if let Some(sidecar) = &apple_photos {
        warnings.extend(sidecar.warnings.clone());
    }
    let ocr_sidecar = ocr::read_ocr_sidecar(path, max_output_bytes)?;
    if let Some(sidecar) = &ocr_sidecar {
        warnings.extend(sidecar.metadata.warnings.clone());
    }

    let summary = media_summary(
        "image",
        &detection.mime_type,
        bytes.len() as u64,
        width,
        height,
        decoded,
        &warnings,
    );
    let content = if let Some(sidecar) = &ocr_sidecar {
        format!("{summary}\nOCR fallback text:\n{}", sidecar.text)
    } else {
        summary
    };
    let mut pipeline = vec![
        WorkspaceReaderStep::DetectFileType,
        WorkspaceReaderStep::InspectImageMetadata,
    ];
    if apple_photos.is_some() {
        pipeline.push(WorkspaceReaderStep::InspectApplePhotosAdjustments);
    }
    if ocr_sidecar.is_some() {
        pipeline.push(WorkspaceReaderStep::ApplyOcrFallback);
    }
    pipeline.push(WorkspaceReaderStep::SummarizeBinary);
    Ok(render_media(
        content,
        WorkspaceMediaMetadata {
            format: detection.mime_type.clone(),
            width,
            height,
            color_type,
            decoded,
            parser: if decoded {
                "image:0.25.10".to_string()
            } else {
                "workspace-bmff-metadata".to_string()
            },
            embedded_previews: Vec::new(),
            raw_sensor: None,
            apple_photos,
            warnings: warnings.clone(),
        },
        document_from_ocr(ocr_sidecar),
        pipeline,
        warnings,
        max_output_bytes,
    ))
}

/// Render raw image.
/// This parses caller-provided bytes into a bounded rendered read response and does not write
/// workspace files.
pub(super) fn render_raw_image(
    path: &Path,
    bytes: &[u8],
    detection: &WorkspaceReadDetection,
    max_output_bytes: u64,
) -> Result<RenderedRead, agent_sdk_core::AgentError> {
    let mut warnings = Vec::new();
    let tiff = tiff_raw_metadata(bytes);
    let (width, height, parser) = if let Some(tiff) = &tiff {
        (tiff.width, tiff.height, "workspace-tiff-raw-preview-sensor")
    } else if let Some((width, height)) = bmff_ispe_dimensions(bytes) {
        (Some(width), Some(height), "workspace-bmff-raw-metadata")
    } else {
        warnings.push(
            "raw image dimensions were not found; a platform RAW decoder may be required"
                .to_string(),
        );
        (None, None, "workspace-raw-metadata")
    };
    let embedded_previews = tiff
        .as_ref()
        .map(|metadata| metadata.embedded_previews.clone())
        .unwrap_or_default();
    let raw_sensor = tiff.and_then(|metadata| metadata.raw_sensor);
    if embedded_previews.is_empty() {
        warnings.push("no embedded JPEG preview tags were found".to_string());
    }
    if raw_sensor
        .as_ref()
        .map(|sensor| sensor.decoded_pixels)
        .unwrap_or(false)
    {
        warnings.push(
            "uncompressed RAW sensor strip metadata was decoded; raw pixels remain behind hashes/metadata"
                .to_string(),
        );
    } else {
        warnings.push(
            "raw sensor pixels were not decoded; a camera-specific RAW adapter may be required"
                .to_string(),
        );
    }
    let apple_photos = read_apple_photos_sidecar(path, max_output_bytes)?;
    if let Some(sidecar) = &apple_photos {
        warnings.extend(sidecar.warnings.clone());
    }
    let summary = media_summary(
        "raw image",
        &detection.mime_type,
        bytes.len() as u64,
        width,
        height,
        raw_sensor
            .as_ref()
            .map(|sensor| sensor.decoded_pixels)
            .unwrap_or(false),
        &warnings,
    );
    let mut pipeline = vec![
        WorkspaceReaderStep::DetectFileType,
        WorkspaceReaderStep::InspectRawImageMetadata,
    ];
    if !embedded_previews.is_empty() {
        pipeline.push(WorkspaceReaderStep::InspectRawPreview);
    }
    if apple_photos.is_some() {
        pipeline.push(WorkspaceReaderStep::InspectApplePhotosAdjustments);
    }
    pipeline.push(WorkspaceReaderStep::SummarizeBinary);
    Ok(render_media(
        summary,
        WorkspaceMediaMetadata {
            format: detection.mime_type.clone(),
            width,
            height,
            color_type: None,
            decoded: raw_sensor
                .as_ref()
                .map(|sensor| sensor.decoded_pixels)
                .unwrap_or(false),
            parser: parser.to_string(),
            embedded_previews,
            raw_sensor,
            apple_photos,
            warnings: warnings.clone(),
        },
        None,
        pipeline,
        warnings,
        max_output_bytes,
    ))
}

fn render_media(
    summary: String,
    media: WorkspaceMediaMetadata,
    document: Option<WorkspaceDocumentMetadata>,
    reader_pipeline: Vec<WorkspaceReaderStep>,
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
        reader_pipeline,
        media: Some(media),
        document,
        archive: None,
        sqlite: None,
        resource: None,
        warnings,
    };
    add_truncation_guidance(&mut rendered);
    rendered
}

fn media_summary(
    label: &str,
    mime_type: &str,
    byte_len: u64,
    width: Option<u32>,
    height: Option<u32>,
    decoded: bool,
    warnings: &[String],
) -> String {
    let dimensions = match (width, height) {
        (Some(width), Some(height)) => format!("{width}x{height}"),
        _ => "unknown dimensions".to_string(),
    };
    let mut summary =
        format!("{label}: {mime_type}, {byte_len} bytes, {dimensions}, decoded={decoded}.");
    if !warnings.is_empty() {
        summary.push_str("\nWarnings:\n");
        for warning in warnings {
            summary.push_str("- ");
            summary.push_str(warning);
            summary.push('\n');
        }
    }
    summary
}

fn bmff_ispe_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    for offset in 0..bytes.len().saturating_sub(20) {
        if bytes.get(offset + 4..offset + 8) == Some(b"ispe".as_slice()) {
            let size = u32::from_be_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
            if size < 20 || offset.checked_add(size)? > bytes.len() {
                continue;
            }
            let payload = offset + 8;
            let width = u32::from_be_bytes(bytes[payload + 4..payload + 8].try_into().ok()?);
            let height = u32::from_be_bytes(bytes[payload + 8..payload + 12].try_into().ok()?);
            return Some((width, height));
        }
    }
    None
}

struct TiffInspection {
    width: Option<u32>,
    height: Option<u32>,
    embedded_previews: Vec<WorkspaceEmbeddedPreviewMetadata>,
    raw_sensor: Option<WorkspaceRawSensorMetadata>,
}

fn tiff_raw_metadata(bytes: &[u8]) -> Option<TiffInspection> {
    if bytes.len() < 8 {
        return None;
    }
    let little = match &bytes[0..2] {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    if read_u16(bytes, 2, little)? != 42 {
        return None;
    }
    let ifd_offset = read_u32(bytes, 4, little)? as usize;
    let count = read_u16(bytes, ifd_offset, little)? as usize;
    let mut width = None;
    let mut height = None;
    let mut bits_per_sample = None;
    let mut compression = None;
    let mut photometric = None;
    let mut strip_offsets = Vec::new();
    let mut strip_byte_counts = Vec::new();
    let mut jpeg_offset = None;
    let mut jpeg_len = None;
    for index in 0..count {
        let entry = ifd_offset + 2 + index * 12;
        if entry + 12 > bytes.len() {
            return None;
        }
        let tag = read_u16(bytes, entry, little)?;
        let values = read_tiff_values(bytes, entry, little).unwrap_or_default();
        match tag {
            256 => width = values.first().copied(),
            257 => height = values.first().copied(),
            258 => bits_per_sample = values.first().map(|value| *value as u16),
            259 => compression = values.first().map(|value| *value as u16),
            262 => photometric = values.first().map(|value| *value as u16),
            273 => strip_offsets = values,
            279 => strip_byte_counts = values,
            513 => jpeg_offset = values.first().copied(),
            514 => jpeg_len = values.first().copied(),
            _ => {}
        }
    }
    let mut embedded_previews = Vec::new();
    if let (Some(offset), Some(len)) = (jpeg_offset, jpeg_len) {
        let offset = offset as usize;
        let len = len as usize;
        if offset
            .checked_add(len)
            .map(|end| end <= bytes.len())
            .unwrap_or(false)
        {
            embedded_previews.push(WorkspaceEmbeddedPreviewMetadata {
                mime_type: "image/jpeg".to_string(),
                byte_len: len as u64,
                offset: Some(offset as u64),
                content_hash: hash_bytes(&bytes[offset..offset + len]),
            });
        }
    }
    let strip_byte_len = strip_byte_counts
        .iter()
        .fold(0u64, |sum, value| sum.saturating_add(*value as u64));
    let strips_in_bounds =
        strip_offsets
            .iter()
            .zip(strip_byte_counts.iter())
            .all(|(offset, len)| {
                (*offset as usize)
                    .checked_add(*len as usize)
                    .map(|end| end <= bytes.len())
                    .unwrap_or(false)
            });
    let decoded_pixels = compression == Some(1)
        && width.is_some()
        && height.is_some()
        && bits_per_sample.is_some()
        && !strip_offsets.is_empty()
        && strips_in_bounds;
    let sample_hash = if decoded_pixels {
        let offset = strip_offsets[0] as usize;
        let len = strip_byte_counts[0] as usize;
        Some(hash_bytes(&bytes[offset..offset + len]))
    } else {
        None
    };
    Some(TiffInspection {
        width,
        height,
        embedded_previews,
        raw_sensor: Some(WorkspaceRawSensorMetadata {
            bits_per_sample,
            compression,
            photometric_interpretation: photometric,
            strip_count: strip_offsets.len(),
            strip_byte_len,
            decoded_pixels,
            sample_hash,
            warnings: if decoded_pixels {
                Vec::new()
            } else {
                vec![
                    "TIFF/DNG sensor strip metadata was present but not decodable as uncompressed bounded pixels"
                        .to_string(),
                ]
            },
        }),
    })
}

fn read_tiff_values(bytes: &[u8], entry: usize, little: bool) -> Option<Vec<u32>> {
    let field_type = read_u16(bytes, entry + 2, little)?;
    let count = read_u32(bytes, entry + 4, little)? as usize;
    let type_size = match field_type {
        3 => 2,
        4 => 4,
        _ => return None,
    };
    let byte_len = count.checked_mul(type_size)?;
    let value_offset = if byte_len <= 4 {
        entry + 8
    } else {
        read_u32(bytes, entry + 8, little)? as usize
    };
    if value_offset.checked_add(byte_len)? > bytes.len() {
        return None;
    }
    let mut values = Vec::new();
    for index in 0..count {
        let offset = value_offset + index * type_size;
        values.push(match field_type {
            3 => read_u16(bytes, offset, little)? as u32,
            4 => read_u32(bytes, offset, little)?,
            _ => return None,
        });
    }
    Some(values)
}

fn document_from_ocr(sidecar: Option<ocr::OcrSidecar>) -> Option<WorkspaceDocumentMetadata> {
    sidecar.map(|sidecar| WorkspaceDocumentMetadata {
        parser: sidecar.metadata.parser.clone(),
        page_count: None,
        extracted_chars: sidecar.metadata.extracted_chars,
        ocr: Some(sidecar.metadata.clone()),
        warnings: sidecar.metadata.warnings,
    })
}

fn read_apple_photos_sidecar(
    path: &Path,
    max_output_bytes: u64,
) -> Result<Option<WorkspaceApplePhotosMetadata>, agent_sdk_core::AgentError> {
    let Some(sidecar) = apple_photos_sidecar_path(path) else {
        return Ok(None);
    };
    if fs::symlink_metadata(&sidecar)
        .map_err(tool_failure)?
        .file_type()
        .is_symlink()
    {
        return Ok(Some(WorkspaceApplePhotosMetadata {
            sidecar_path: display_name(&sidecar),
            byte_len: 0,
            adjustment_count: 0,
            parser: "workspace-apple-photos-aae:v1".to_string(),
            warnings: vec!["Apple Photos sidecar symlink was ignored".to_string()],
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
    let text = String::from_utf8_lossy(&bytes);
    let adjustment_count = text
        .matches("adjustment")
        .count()
        .max(text.matches("Adjustment").count());
    let mut warnings = vec![
        "Apple Photos adjustment sidecar metadata was summarized; adjustment operations were not applied to pixels"
            .to_string(),
    ];
    if truncated {
        warnings.push("Apple Photos sidecar was truncated".to_string());
    }
    Ok(Some(WorkspaceApplePhotosMetadata {
        sidecar_path: display_name(&sidecar),
        byte_len,
        adjustment_count,
        parser: "workspace-apple-photos-aae:v1".to_string(),
        warnings,
    }))
}

fn apple_photos_sidecar_path(path: &Path) -> Option<PathBuf> {
    let extension = path.extension()?.to_str()?;
    for candidate_extension in [
        format!("{extension}.aae"),
        format!("{extension}.AAE"),
        "aae".to_string(),
        "AAE".to_string(),
    ] {
        let candidate = path.with_extension(candidate_extension);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    let file_name = path.file_name()?.to_str()?;
    for suffix in [".aae", ".AAE"] {
        let candidate = path.with_file_name(format!("{file_name}{suffix}"));
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("sidecar")
        .to_string()
}

fn read_u16(bytes: &[u8], offset: usize, little: bool) -> Option<u16> {
    let slice: [u8; 2] = bytes.get(offset..offset + 2)?.try_into().ok()?;
    Some(if little {
        u16::from_le_bytes(slice)
    } else {
        u16::from_be_bytes(slice)
    })
}

fn read_u32(bytes: &[u8], offset: usize, little: bool) -> Option<u32> {
    let slice: [u8; 4] = bytes.get(offset..offset + 4)?.try_into().ok()?;
    Some(if little {
        u32::from_le_bytes(slice)
    } else {
        u32::from_be_bytes(slice)
    })
}
