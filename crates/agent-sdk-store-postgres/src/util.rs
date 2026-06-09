use agent_sdk_core::{
    AgentError, ContentResolutionError, ContentResolutionErrorKind, PolicyRef, content::ContentRef,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) fn decode_row<T>(row: Value, field: &str) -> Result<T, AgentError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(row.get(field).cloned().unwrap_or(Value::Null)).map_err(|error| {
        AgentError::contract_violation(format!("Postgres row decode failed: {error}"))
    })
}

pub(crate) fn json_value<T: Serialize>(value: &T) -> Result<Value, AgentError> {
    serde_json::to_value(value)
        .map_err(|error| AgentError::contract_violation(format!("JSON encode failed: {error}")))
}

pub(crate) fn content_error(
    kind: ContentResolutionErrorKind,
    content_ref: ContentRef,
    policy_refs: Vec<PolicyRef>,
) -> ContentResolutionError {
    ContentResolutionError {
        kind,
        redacted_summary: content_ref.redacted_summary.clone(),
        content_ref: Box::new(content_ref),
        policy_refs,
    }
}
