use agent_sdk_core::{
    AgentError, AgentErrorKind, RetryClassification, ToolExecutionRequest, domain::ContentRef,
};
use sha2::{Digest, Sha256};

pub(super) fn first_arg_ref(request: &ToolExecutionRequest) -> Result<&ContentRef, AgentError> {
    request
        .effect_intent
        .content_refs
        .first()
        .ok_or_else(|| AgentError::missing_required_field("tool_execution.argument_content_ref"))
}

pub(super) fn content_ref_for(request: &ToolExecutionRequest, suffix: &str) -> ContentRef {
    ContentRef::new(format!(
        "content.{}.{}",
        request.resolved_call.request.tool_call_id.as_str(),
        suffix
    ))
}

pub(super) fn tool_failure(error: std::io::Error) -> AgentError {
    AgentError::new(
        AgentErrorKind::ToolFailure,
        RetryClassification::UserActionNeeded,
        error.to_string(),
    )
}

pub(super) fn policy_denial(message: impl Into<String>) -> AgentError {
    AgentError::new(
        AgentErrorKind::PolicyDenial,
        RetryClassification::UserActionNeeded,
        message,
    )
}

pub(super) fn hash_line(line: &str) -> String {
    hash_bytes(line.as_bytes())
}

pub(super) fn hash_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::from("sha256:");
    for byte in digest {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

pub(super) fn truncate_bytes(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let mut end = max_bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}
