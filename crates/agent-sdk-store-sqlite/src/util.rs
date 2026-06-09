use std::path::Path;

use agent_sdk_core::{
    AgentError, AgentErrorKind, ContentResolutionError, ContentResolutionErrorKind, PolicyRef,
    RetryClassification, content::ContentRef,
};
use rusqlite::Connection;
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

pub(crate) fn sqlite_error(error: rusqlite::Error) -> AgentError {
    AgentError::new(
        AgentErrorKind::RecoveryRepairNeeded,
        RetryClassification::Retryable,
        format!("sqlite store error: {error}"),
    )
}

pub(crate) fn serde_error(error: impl std::fmt::Display) -> AgentError {
    AgentError::contract_violation(format!("sqlite store JSON error: {error}"))
}

pub(crate) fn journal_error(message: impl Into<String>) -> AgentError {
    AgentError::new(
        AgentErrorKind::JournalFailure,
        RetryClassification::RepairNeeded,
        message,
    )
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

pub(crate) fn open(path: &Path) -> Result<Connection, AgentError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            AgentError::new(
                AgentErrorKind::RecoveryRepairNeeded,
                RetryClassification::Retryable,
                format!("sqlite store directory error: {error}"),
            )
        })?;
    }
    Connection::open(path).map_err(sqlite_error)
}

pub(crate) fn init(path: &Path, schema: &str) -> Result<(), AgentError> {
    let connection = open(path)?;
    connection.execute_batch(schema).map_err(sqlite_error)
}

pub(crate) fn encode<T: Serialize>(value: &T) -> Result<String, AgentError> {
    serde_json::to_string(value).map_err(serde_error)
}

pub(crate) fn decode<T: DeserializeOwned>(value: &str) -> Result<T, AgentError> {
    serde_json::from_str(value).map_err(serde_error)
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
