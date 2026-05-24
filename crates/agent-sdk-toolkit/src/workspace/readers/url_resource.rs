use agent_sdk_core::{AgentError, AgentErrorKind, RetryClassification};
use base64::{Engine as _, engine::general_purpose::STANDARD};

use super::{RenderedRead, RenderedUriRead, add_truncation_guidance, text};
use crate::workspace::{
    read_pipeline::{
        WorkspaceFileKind, WorkspaceFileTypeConfidence, WorkspaceReadDetection,
        WorkspaceReaderStep, WorkspaceResourceMetadata,
    },
    util::{hash_bytes, truncate_bytes},
};

pub(super) fn render_uri(
    uri: &str,
    max_input_bytes: u64,
    max_output_bytes: u64,
) -> Result<RenderedUriRead, AgentError> {
    if uri.len() as u64 > max_input_bytes {
        return Err(AgentError::new(
            AgentErrorKind::PolicyDenial,
            RetryClassification::UserActionNeeded,
            "workspace URI input exceeds max_file_bytes; attach a host resource resolver or pass a smaller data URL",
        ));
    }
    if uri.starts_with("data:") {
        return render_data_uri(uri, max_output_bytes);
    }
    Err(AgentError::new(
        AgentErrorKind::PolicyDenial,
        RetryClassification::UserActionNeeded,
        "workspace_read does not perform ambient network/resource reads; attach a host resource resolver or network policy for this URI",
    ))
}

fn render_data_uri(uri: &str, max_output_bytes: u64) -> Result<RenderedUriRead, AgentError> {
    let Some(comma) = uri.find(',') else {
        return Err(AgentError::new(
            AgentErrorKind::ToolFailure,
            RetryClassification::UserActionNeeded,
            "malformed data URL: missing comma",
        ));
    };
    let meta = &uri[5..comma];
    let encoded = &uri[comma + 1..];
    let base64 = meta
        .split(';')
        .any(|part| part.eq_ignore_ascii_case("base64"));
    let mime_type = meta
        .split(';')
        .find(|part| part.contains('/'))
        .filter(|part| !part.is_empty())
        .unwrap_or("text/plain;charset=US-ASCII")
        .to_string();
    let bytes = if base64 {
        STANDARD.decode(encoded).map_err(|error| {
            AgentError::new(
                AgentErrorKind::ToolFailure,
                RetryClassification::UserActionNeeded,
                format!("malformed data URL base64 payload: {error}"),
            )
        })?
    } else {
        percent_decode(encoded)?
    };
    let valid_text = std::str::from_utf8(&bytes).is_ok() && !bytes.contains(&0);
    let declared_text = mime_type.starts_with("text/") || mime_type == "application/json";
    let binary = !valid_text || !declared_text;
    let detection = WorkspaceReadDetection {
        kind: WorkspaceFileKind::UrlResource,
        mime_type: mime_type.clone(),
        extension: None,
        binary,
        confidence: WorkspaceFileTypeConfidence::Magic,
    };
    let mut rendered = if binary {
        let summary = format!(
            "data URL resource: {mime_type}, {} byte(s). Binary URI content was not emitted as text.",
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
                WorkspaceReaderStep::ReadDataUrl,
                WorkspaceReaderStep::SummarizeBinary,
            ],
            media: None,
            document: None,
            archive: None,
            sqlite: None,
            resource: None,
            warnings: vec!["binary data URL content was summarized, not emitted".to_string()],
        };
        add_truncation_guidance(&mut rendered);
        rendered
    } else {
        let text = String::from_utf8_lossy(&bytes);
        let mut rendered = text::render_text(&text, max_output_bytes, false, false);
        rendered.reader_pipeline = vec![
            WorkspaceReaderStep::DetectFileType,
            WorkspaceReaderStep::ReadDataUrl,
        ];
        rendered
    };
    rendered.resource = Some(WorkspaceResourceMetadata {
        scheme: "data".to_string(),
        source: "inline-data-url".to_string(),
        byte_len: bytes.len() as u64,
        parser: "workspace-data-url:v1".to_string(),
        warnings: if base64 {
            Vec::new()
        } else {
            vec!["data URL percent payload decoded locally".to_string()]
        },
    });
    Ok(RenderedUriRead {
        detection,
        byte_len: bytes.len() as u64,
        content_hash: hash_bytes(&bytes),
        rendered,
    })
}

fn percent_decode(input: &str) -> Result<Vec<u8>, AgentError> {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(AgentError::new(
                    AgentErrorKind::ToolFailure,
                    RetryClassification::UserActionNeeded,
                    "malformed data URL percent escape",
                ));
            }
            let high = hex_value(bytes[index + 1])?;
            let low = hex_value(bytes[index + 2])?;
            output.push((high << 4) | low);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    Ok(output)
}

fn hex_value(byte: u8) -> Result<u8, AgentError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(AgentError::new(
            AgentErrorKind::ToolFailure,
            RetryClassification::UserActionNeeded,
            "malformed data URL percent escape",
        )),
    }
}
