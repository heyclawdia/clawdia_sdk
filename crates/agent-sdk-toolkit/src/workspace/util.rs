//! Concrete workspace tool helpers layered over core tool/effect contracts. Use these
//! modules for bounded read, search, edit, write, and format-aware extraction
//! behavior under a host-selected workspace policy. Reads search local files;
//! edit/write helpers may mutate files only through explicit executor calls. This
//! file contains the util portion of that contract.
//!
use agent_sdk_core::{
    AgentError, AgentErrorKind, RetryClassification, ToolExecutionRequest, domain::ContentRef,
};
use sha2::{Digest, Sha256};

/// Returns first arg ref for the current value.
/// This is a read-only or data-construction helper unless the method body explicitly calls a
/// port or store.
pub(super) fn first_arg_ref(request: &ToolExecutionRequest) -> Result<&ContentRef, AgentError> {
    request
        .effect_intent
        .content_refs
        .first()
        .ok_or_else(|| AgentError::missing_required_field("tool_execution.argument_content_ref"))
}

/// Returns content ref for for the current value.
/// This is a read-only or data-construction helper unless the method body explicitly calls a
/// port or store.
pub(super) fn content_ref_for(request: &ToolExecutionRequest, suffix: &str) -> ContentRef {
    ContentRef::new(format!(
        "content.{}.{}",
        request.resolved_call.request.tool_call_id.as_str(),
        suffix
    ))
}

/// Returns tool failure for the current value.
/// This is a read-only or data-construction helper unless the method body explicitly calls a
/// port or store.
pub(super) fn tool_failure(error: std::io::Error) -> AgentError {
    AgentError::new(
        AgentErrorKind::ToolFailure,
        RetryClassification::UserActionNeeded,
        error.to_string(),
    )
}

/// Returns policy denial for the current value.
/// This is a read-only or data-construction helper unless the method body explicitly calls a
/// port or store.
pub(super) fn policy_denial(message: impl Into<String>) -> AgentError {
    AgentError::new(
        AgentErrorKind::PolicyDenial,
        RetryClassification::UserActionNeeded,
        message,
    )
}

/// Computes the stable hash line for this workspace::util value. The
/// computation is deterministic and side-effect free so it can be used
/// in package, journal, or test evidence.
pub(super) fn hash_line(line: &str) -> String {
    hash_bytes(line.as_bytes())
}

/// Computes the stable hash bytes for this workspace::util value. The
/// computation is deterministic and side-effect free so it can be used
/// in package, journal, or test evidence.
pub(super) fn hash_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::from("sha256:");
    for byte in digest {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

/// Renders or detects bounded workspace content for workspace::util. It may
/// read already-approved local file data but does not mutate the workspace.
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
