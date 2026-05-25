//! Concrete workspace tool helpers layered over core tool/effect contracts. Use these
//! modules for bounded read, search, edit, write, and format-aware extraction
//! behavior under a host-selected workspace policy. Reads search local files;
//! edit/write helpers may mutate files only through explicit executor calls. This
//! file contains the pdf portion of that contract.
//!
use std::path::Path;

use super::{RenderedRead, add_truncation_guidance, extraction_error, ocr, text};
use crate::workspace::read_pipeline::{WorkspaceDocumentMetadata, WorkspaceReaderStep};

/// Renders or detects bounded workspace content for workspace::readers::pdf.
/// It may read already-approved local file data but does not mutate the
/// workspace.
pub(super) fn render_pdf(
    path: &Path,
    bytes: &[u8],
    max_output_bytes: u64,
) -> Result<RenderedRead, agent_sdk_core::AgentError> {
    let mut warnings = Vec::new();
    let pages = match pdf_extract::extract_text_from_mem_by_pages(bytes) {
        Ok(pages) => pages,
        Err(error) => {
            if let Some(sidecar) = ocr::read_ocr_sidecar(path, max_output_bytes)? {
                warnings.push(format!(
                    "pdf text extraction failed, using bounded OCR sidecar: {error}"
                ));
                let mut rendered = text::render_text(&sidecar.text, max_output_bytes, false, true);
                rendered.reader_pipeline = vec![
                    WorkspaceReaderStep::DetectFileType,
                    WorkspaceReaderStep::ExtractPdfText,
                    WorkspaceReaderStep::ApplyOcrFallback,
                ];
                rendered.content_summary = Some(format!(
                    "PDF OCR sidecar extraction completed with {} extracted character(s).",
                    sidecar.metadata.extracted_chars
                ));
                rendered.document = Some(WorkspaceDocumentMetadata {
                    parser: "pdf-extract:0.10.0+workspace-ocr-sidecar:v1".to_string(),
                    page_count: None,
                    extracted_chars: sidecar.metadata.extracted_chars,
                    ocr: Some(sidecar.metadata),
                    warnings: warnings.clone(),
                });
                rendered.warnings.extend(warnings);
                add_truncation_guidance(&mut rendered);
                return Ok(rendered);
            }
            return Err(extraction_error("pdf", error));
        }
    };
    let extracted_page_text = pages
        .iter()
        .map(|page| page.trim())
        .filter(|page| !page.is_empty())
        .collect::<Vec<_>>();
    let text = if pages.is_empty() {
        warnings.push("pdf parser returned no pages".to_string());
        String::new()
    } else {
        pages
            .iter()
            .enumerate()
            .map(|(index, page)| format!("[page {}]\n{}", index + 1, page.trim()))
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    let ocr_sidecar = if extracted_page_text.is_empty() {
        warnings.push("pdf contains no extractable text; OCR may be required".to_string());
        ocr::read_ocr_sidecar(path, max_output_bytes)?
    } else {
        None
    };
    let rendered_text = if let Some(sidecar) = &ocr_sidecar {
        warnings.push("using bounded OCR sidecar text for scanned/empty-text PDF".to_string());
        sidecar.text.clone()
    } else {
        text
    };
    let mut rendered = text::render_text(&rendered_text, max_output_bytes, false, true);
    rendered.reader_pipeline = vec![
        WorkspaceReaderStep::DetectFileType,
        WorkspaceReaderStep::ExtractPdfText,
    ];
    if ocr_sidecar.is_some() {
        rendered
            .reader_pipeline
            .push(WorkspaceReaderStep::ApplyOcrFallback);
    }
    rendered.content_summary = Some(format!(
        "PDF text extraction completed with {} page(s), {} extracted character(s).",
        pages.len(),
        rendered_text.chars().count()
    ));
    rendered.document = Some(WorkspaceDocumentMetadata {
        parser: "pdf-extract:0.10.0".to_string(),
        page_count: Some(pages.len()),
        extracted_chars: rendered_text.chars().count(),
        ocr: ocr_sidecar.map(|sidecar| sidecar.metadata),
        warnings: warnings.clone(),
    });
    rendered.warnings.extend(warnings);
    add_truncation_guidance(&mut rendered);
    Ok(rendered)
}
