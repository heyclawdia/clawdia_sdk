use std::path::PathBuf;

use agent_sdk_core::{
    ContentResolutionError, ContentResolutionErrorKind, ContentResolutionPolicy,
    ContentResolveRequest, ContentResolver, ContentStore, ResolvedContent,
    content::{ContentRef, ResolvedContent as CoreResolvedContent},
};

use crate::util::{
    content_error, read_bytes, read_json, root_join, safe_segment, sha256_hex, write_bytes,
    write_json,
};

#[derive(Clone, Debug)]
/// Filesystem-backed content resolver and store.
pub struct FileContentStore {
    root: PathBuf,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct ContentMetadata {
    content_ref: ContentRef,
    byte_len: u64,
}

impl FileContentStore {
    /// Creates a content store rooted under the provided directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn content_dir(&self, content_ref: &ContentRef) -> PathBuf {
        root_join(
            &self.root,
            &[
                "content".to_string(),
                safe_segment(content_ref.content_id.as_str()),
            ],
        )
    }

    fn metadata_path(&self, content_ref: &ContentRef) -> PathBuf {
        self.content_dir(content_ref).join("metadata.json")
    }

    fn bytes_path(&self, content_ref: &ContentRef) -> PathBuf {
        self.content_dir(content_ref).join("content.bin")
    }
}

impl ContentResolver for FileContentStore {
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
        let metadata = read_json::<ContentMetadata>(&self.metadata_path(&request.content_ref))
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
        if !policy.allow_raw_content {
            return Ok(CoreResolvedContent::redacted(
                metadata.content_ref,
                policy.policy_refs,
            ));
        }
        let bytes = read_bytes(&self.bytes_path(&request.content_ref)).map_err(|_| {
            content_error(
                ContentResolutionErrorKind::StorageUnavailable,
                request.content_ref.clone(),
                policy.policy_refs.clone(),
            )
        })?;
        if bytes.len() as u64 > policy.max_bytes {
            return Err(content_error(
                ContentResolutionErrorKind::MaxBytesExceeded,
                request.content_ref,
                policy.policy_refs,
            ));
        }
        if policy.require_hash_match {
            if let Some(expected) = &metadata.content_ref.content_hash {
                let actual = format!("sha256:{}", sha256_hex(&bytes));
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
            mime: metadata.content_ref.mime.clone(),
            redacted_summary: metadata.content_ref.redacted_summary.clone(),
            content_ref: metadata.content_ref,
            bytes: Some(bytes),
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

impl ContentStore for FileContentStore {
    fn put_content(
        &self,
        content_ref: &ContentRef,
        bytes: Vec<u8>,
    ) -> Result<(), ContentResolutionError> {
        let metadata = ContentMetadata {
            content_ref: content_ref.clone(),
            byte_len: bytes.len() as u64,
        };
        write_bytes(&self.bytes_path(content_ref), &bytes).map_err(|_| {
            content_error(
                ContentResolutionErrorKind::StorageUnavailable,
                content_ref.clone(),
                Vec::new(),
            )
        })?;
        write_json(&self.metadata_path(content_ref), &metadata).map_err(|_| {
            content_error(
                ContentResolutionErrorKind::StorageUnavailable,
                content_ref.clone(),
                Vec::new(),
            )
        })
    }
}
