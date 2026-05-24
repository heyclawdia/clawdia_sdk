use std::{
    io::{Cursor, Read},
    path::Path,
};

use quick_xml::{Reader, events::Event};
use zip::ZipArchive;

use super::{RenderedRead, add_truncation_guidance, extraction_error, legacy_office, text};
use crate::workspace::read_pipeline::{WorkspaceDocumentMetadata, WorkspaceReaderStep};

const MAX_XML_ENTRY_BYTES: u64 = 1024 * 1024;

pub(super) fn render_office(
    path: &Path,
    bytes: &[u8],
    max_output_bytes: u64,
) -> Result<RenderedRead, agent_sdk_core::AgentError> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(extension.as_str(), "doc" | "xls" | "ppt") {
        return legacy_office::render_legacy_office(path, bytes, max_output_bytes);
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| extraction_error("office document", error))?;
    let mut warnings = Vec::new();
    let mut sections = Vec::new();
    let entry_limit = max_output_bytes.clamp(4096, MAX_XML_ENTRY_BYTES);

    match extension.as_str() {
        "docx" => {
            push_xml_entry(
                &mut archive,
                "word/document.xml",
                "document",
                entry_limit,
                &mut sections,
                &mut warnings,
            )?;
        }
        "pptx" => {
            push_matching_xml_entries(
                &mut archive,
                "ppt/slides/slide",
                ".xml",
                "slide",
                entry_limit,
                &mut sections,
                &mut warnings,
            )?;
        }
        "xlsx" => {
            push_xml_entry(
                &mut archive,
                "xl/sharedStrings.xml",
                "shared strings",
                entry_limit,
                &mut sections,
                &mut warnings,
            )?;
            push_matching_xml_entries(
                &mut archive,
                "xl/worksheets/sheet",
                ".xml",
                "sheet",
                entry_limit,
                &mut sections,
                &mut warnings,
            )?;
        }
        _ => warnings.push(format!(
            "office/document extension .{extension} is detected, but only docx/pptx/xlsx XML extraction is implemented"
        )),
    }

    let extracted = sections.join("\n\n");
    if extracted.trim().is_empty() {
        warnings.push("document parser found no extractable XML text".to_string());
    }
    let mut rendered = text::render_text(&extracted, max_output_bytes, false, true);
    rendered.reader_pipeline = vec![
        WorkspaceReaderStep::DetectFileType,
        WorkspaceReaderStep::ExtractOfficeText,
    ];
    rendered.content_summary = Some(format!(
        "Office/document extraction completed with {} section(s), {} extracted character(s).",
        sections.len(),
        extracted.chars().count()
    ));
    rendered.document = Some(WorkspaceDocumentMetadata {
        parser: "zip-ooxml-quick-xml:0.1".to_string(),
        page_count: None,
        extracted_chars: extracted.chars().count(),
        ocr: None,
        warnings: warnings.clone(),
    });
    rendered.warnings.extend(warnings);
    add_truncation_guidance(&mut rendered);
    Ok(rendered)
}

fn push_matching_xml_entries(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    prefix: &str,
    suffix: &str,
    label: &str,
    entry_limit: u64,
    sections: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Result<(), agent_sdk_core::AgentError> {
    let mut names = Vec::new();
    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|error| extraction_error("office document", error))?;
        let name = file.name().to_string();
        if name.starts_with(prefix) && name.ends_with(suffix) {
            if name.split('/').any(|part| part == "..") {
                warnings.push(format!(
                    "office entry with parent traversal skipped: {name}"
                ));
                continue;
            }
            names.push(name);
        }
    }
    names.sort();
    for (index, name) in names.iter().enumerate() {
        push_xml_entry(
            archive,
            name,
            &format!("{label} {}", index + 1),
            entry_limit,
            sections,
            warnings,
        )?;
    }
    Ok(())
}

fn push_xml_entry(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
    label: &str,
    entry_limit: u64,
    sections: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Result<(), agent_sdk_core::AgentError> {
    let file = match archive.by_name(name) {
        Ok(file) => file,
        Err(error) => {
            warnings.push(format!("missing {name}: {error}"));
            return Ok(());
        }
    };
    if file.size() > entry_limit {
        warnings.push(format!(
            "skipped {name}: uncompressed entry size {} exceeds limit {entry_limit}",
            file.size()
        ));
        return Ok(());
    }
    let mut xml = String::new();
    let mut limited = file.take(entry_limit + 1);
    limited
        .read_to_string(&mut xml)
        .map_err(|error| extraction_error("office document", error))?;
    if xml.len() as u64 > entry_limit {
        warnings.push(format!(
            "skipped {name}: entry exceeded read limit {entry_limit}"
        ));
        return Ok(());
    }
    let text = xml_text(&xml).map_err(|error| extraction_error("office document xml", error))?;
    if !text.trim().is_empty() {
        sections.push(format!("[{label}]\n{text}"));
    }
    Ok(())
}

fn xml_text(xml: &str) -> Result<String, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut output = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Text(text)) => {
                let value = text.xml10_content().map_err(|error| error.to_string())?;
                push_word(&mut output, value.trim());
            }
            Ok(Event::CData(text)) => {
                let value = text.decode().map_err(|error| error.to_string())?;
                push_word(&mut output, value.trim());
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(error.to_string()),
            _ => {}
        }
    }
    Ok(output)
}

fn push_word(output: &mut String, value: &str) {
    if value.is_empty() {
        return;
    }
    if !output.is_empty() {
        output.push(' ');
    }
    output.push_str(value);
}
