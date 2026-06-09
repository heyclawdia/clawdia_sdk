use std::path::{Path, PathBuf};

use agent_sdk_core::{
    ContentResolutionError, ContentResolutionErrorKind, ContentResolutionPolicy,
    ContentResolveRequest, ContentResolver, ContentStore, ResolvedContent,
    content::{ContentRef, ResolvedContent as CoreResolvedContent},
};
use rusqlite::params;

use crate::util::{content_error, decode, encode, open, sha256_hex};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS content (
    content_ref TEXT PRIMARY KEY,
    content_json TEXT NOT NULL,
    bytes BLOB NOT NULL,
    byte_len INTEGER NOT NULL
);
";

#[derive(Clone, Debug)]
/// SQLite-backed content resolver and store.
pub struct SqliteContentStore {
    path: PathBuf,
}

impl SqliteContentStore {
    /// Opens or creates a SQLite content store.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, agent_sdk_core::AgentError> {
        crate::util::init(path.as_ref(), SCHEMA)?;
        Ok(Self {
            path: path.as_ref().to_path_buf(),
        })
    }
}

impl ContentResolver for SqliteContentStore {
    fn resolve(
        &self,
        request: ContentResolveRequest,
        policy: ContentResolutionPolicy,
    ) -> Result<ResolvedContent, ContentResolutionError> {
        if request.requested_version != request.content_ref.version {
            return Err(content_error(
                ContentResolutionErrorKind::VersionMismatch,
                request.content_ref,
                policy.policy_refs,
            ));
        }
        let connection = open(&self.path).map_err(|_| {
            content_error(
                ContentResolutionErrorKind::StorageUnavailable,
                request.content_ref.clone(),
                policy.policy_refs.clone(),
            )
        })?;
        let mut statement = connection
            .prepare("SELECT content_json, bytes FROM content WHERE content_ref = ?1")
            .map_err(|_| {
                content_error(
                    ContentResolutionErrorKind::StorageUnavailable,
                    request.content_ref.clone(),
                    policy.policy_refs.clone(),
                )
            })?;
        let row = statement
            .query_row(params![request.content_ref.content_id.as_str()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            })
            .optional()
            .map_err(|_| {
                content_error(
                    ContentResolutionErrorKind::StorageUnavailable,
                    request.content_ref.clone(),
                    policy.policy_refs.clone(),
                )
            })?
            .ok_or_else(|| {
                content_error(
                    ContentResolutionErrorKind::Missing,
                    request.content_ref.clone(),
                    policy.policy_refs.clone(),
                )
            })?;
        let content_ref: ContentRef = decode(&row.0).map_err(|_| {
            content_error(
                ContentResolutionErrorKind::StorageUnavailable,
                request.content_ref.clone(),
                policy.policy_refs.clone(),
            )
        })?;
        if !policy.allow_raw_content {
            return Ok(CoreResolvedContent::redacted(
                content_ref,
                policy.policy_refs,
            ));
        }
        if row.1.len() as u64 > policy.max_bytes {
            return Err(content_error(
                ContentResolutionErrorKind::MaxBytesExceeded,
                request.content_ref,
                policy.policy_refs,
            ));
        }
        if policy.require_hash_match {
            if let Some(expected) = &content_ref.content_hash {
                let actual = format!("sha256:{}", sha256_hex(&row.1));
                if expected != &actual {
                    return Err(content_error(
                        ContentResolutionErrorKind::HashMismatch,
                        request.content_ref,
                        policy.policy_refs,
                    ));
                }
            }
        }
        Ok(ResolvedContent {
            mime: content_ref.mime.clone(),
            redacted_summary: content_ref.redacted_summary.clone(),
            content_ref,
            bytes: Some(row.1),
            policy_refs: policy.policy_refs,
            raw_content_included: true,
        })
    }

    fn store_resolved_content(
        &self,
        content_ref: &ContentRef,
        bytes: Vec<u8>,
    ) -> Result<(), ContentResolutionError> {
        self.put_content(content_ref, bytes)
    }
}

impl ContentStore for SqliteContentStore {
    fn put_content(
        &self,
        content_ref: &ContentRef,
        bytes: Vec<u8>,
    ) -> Result<(), ContentResolutionError> {
        let connection = open(&self.path).map_err(|_| {
            content_error(
                ContentResolutionErrorKind::StorageUnavailable,
                content_ref.clone(),
                Vec::new(),
            )
        })?;
        connection
            .execute(
                "INSERT OR REPLACE INTO content
                 (content_ref, content_json, bytes, byte_len)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    content_ref.content_id.as_str(),
                    encode(content_ref).map_err(|_| {
                        content_error(
                            ContentResolutionErrorKind::StorageUnavailable,
                            content_ref.clone(),
                            Vec::new(),
                        )
                    })?,
                    bytes,
                    bytes.len() as i64,
                ],
            )
            .map_err(|_| {
                content_error(
                    ContentResolutionErrorKind::StorageUnavailable,
                    content_ref.clone(),
                    Vec::new(),
                )
            })?;
        Ok(())
    }
}

use rusqlite::OptionalExtension;
