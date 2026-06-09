use std::path::{Path, PathBuf};

use agent_sdk_core::{
    AgentError, ProviderArgumentStore, domain::ContentRef, tool_records::CanonicalToolName,
};
use rusqlite::{OptionalExtension, params};

use crate::util::{open, sha256_hex, sqlite_error};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS provider_arguments (
    content_ref TEXT PRIMARY KEY,
    provider_ref TEXT NOT NULL,
    call_id TEXT NOT NULL,
    canonical_tool_name TEXT NOT NULL,
    raw_arguments TEXT NOT NULL
);
";

#[derive(Clone, Debug)]
/// SQLite-backed raw provider argument store.
pub struct SqliteProviderArgumentStore {
    path: PathBuf,
}

impl SqliteProviderArgumentStore {
    /// Opens or creates a SQLite provider-argument store.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AgentError> {
        crate::util::init(path.as_ref(), SCHEMA)?;
        Ok(Self {
            path: path.as_ref().to_path_buf(),
        })
    }
}

impl ProviderArgumentStore for SqliteProviderArgumentStore {
    fn store_provider_arguments(
        &self,
        provider_ref: &str,
        call_id: &str,
        canonical_tool_name: &CanonicalToolName,
        raw_arguments: &str,
    ) -> Result<Option<ContentRef>, AgentError> {
        let digest = sha256_hex(raw_arguments.as_bytes());
        let content_ref = ContentRef::new(format!("content.provider_arguments.{}", &digest[..24]));
        let connection = open(&self.path)?;
        connection
            .execute(
                "INSERT OR REPLACE INTO provider_arguments
                 (content_ref, provider_ref, call_id, canonical_tool_name, raw_arguments)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    content_ref.as_str(),
                    provider_ref,
                    call_id,
                    canonical_tool_name.as_str(),
                    raw_arguments,
                ],
            )
            .map_err(sqlite_error)?;
        Ok(Some(content_ref))
    }

    fn load_provider_arguments_json(
        &self,
        content_ref: &ContentRef,
    ) -> Result<serde_json::Value, AgentError> {
        let connection = open(&self.path)?;
        let raw = connection
            .query_row(
                "SELECT raw_arguments FROM provider_arguments WHERE content_ref = ?1",
                params![content_ref.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sqlite_error)?
            .ok_or_else(|| {
                AgentError::contract_violation("provider argument content ref is missing")
            })?;
        serde_json::from_str(&raw).map_err(|error| {
            AgentError::contract_violation(format!(
                "stored provider arguments are not valid JSON: {error}"
            ))
        })
    }
}
