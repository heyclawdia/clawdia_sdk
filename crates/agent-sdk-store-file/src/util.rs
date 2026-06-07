use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use agent_sdk_core::{
    AgentError, AgentErrorKind, ContentResolutionError, ContentResolutionErrorKind, PolicyRef,
    RetryClassification, content::ContentRef,
};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

pub(crate) fn store_error(message: impl Into<String>) -> AgentError {
    AgentError::new(
        AgentErrorKind::RecoveryRepairNeeded,
        RetryClassification::Retryable,
        message,
    )
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

pub(crate) fn safe_segment(value: &str) -> String {
    let visible = value
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => character,
            _ => '_',
        })
        .take(80)
        .collect::<String>();
    let visible = if visible.is_empty() {
        "id".to_string()
    } else {
        visible
    };
    format!("{visible}-{}", &sha256_hex(value.as_bytes())[..12])
}

pub(crate) fn ensure_parent(path: &Path) -> Result<(), AgentError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| store_error(error.to_string()))?;
    }
    Ok(())
}

pub(crate) fn read_json<T>(path: &Path) -> Result<Option<T>, AgentError>
where
    T: DeserializeOwned,
{
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path).map_err(|error| store_error(error.to_string()))?;
    serde_json::from_slice(&bytes).map(Some).map_err(|error| {
        store_error(format!(
            "failed to decode JSON at {}: {error}",
            path.display()
        ))
    })
}

pub(crate) fn write_json<T>(path: &Path, value: &T) -> Result<(), AgentError>
where
    T: Serialize,
{
    ensure_parent(path)?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| store_error(error.to_string()))?;
    fs::write(path, bytes).map_err(|error| store_error(error.to_string()))
}

pub(crate) fn append_json_line<T>(path: &Path, value: &T) -> Result<(), AgentError>
where
    T: Serialize,
{
    ensure_parent(path)?;
    let line = serde_json::to_vec(value).map_err(|error| store_error(error.to_string()))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| store_error(error.to_string()))?;
    file.write_all(&line)
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|error| store_error(error.to_string()))
}

pub(crate) fn read_json_lines<T>(path: &Path) -> Result<Vec<T>, AgentError>
where
    T: DeserializeOwned,
{
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(path).map_err(|error| store_error(error.to_string()))?;
    let mut output = Vec::new();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|error| store_error(error.to_string()))?;
        if line.trim().is_empty() {
            continue;
        }
        let value = serde_json::from_str(&line).map_err(|error| {
            store_error(format!(
                "failed to decode NDJSON line {} at {}: {error}",
                index + 1,
                path.display()
            ))
        })?;
        output.push(value);
    }
    Ok(output)
}

pub(crate) fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), AgentError> {
    ensure_parent(path)?;
    fs::write(path, bytes).map_err(|error| store_error(error.to_string()))
}

pub(crate) fn read_bytes(path: &Path) -> Result<Vec<u8>, AgentError> {
    fs::read(path).map_err(|error| store_error(error.to_string()))
}

pub(crate) fn remove_file_if_exists(path: &Path) -> Result<(), AgentError> {
    if path.exists() {
        fs::remove_file(path).map_err(|error| store_error(error.to_string()))?;
    }
    Ok(())
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn parse_cursor_seq(cursor: &str, prefix: &str) -> Option<u64> {
    cursor.strip_prefix(prefix).unwrap_or(cursor).parse().ok()
}

pub(crate) fn root_join(root: &Path, parts: &[String]) -> PathBuf {
    parts
        .iter()
        .fold(root.to_path_buf(), |path, part| path.join(part))
}
