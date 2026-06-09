use agent_sdk_core::{
    ContentResolutionError, ContentResolutionErrorKind, ContentResolutionPolicy,
    ContentResolveRequest, ContentResolver, ContentStore,
    content::{ContentRef, ResolvedContent as CoreResolvedContent},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde_json::Value;

use crate::{
    PostgresStoreClient,
    util::{content_error, decode_row},
};

#[derive(Clone)]
pub struct PostgresContentStore {
    client: PostgresStoreClient,
}

impl PostgresContentStore {
    pub fn new(client: PostgresStoreClient) -> Self {
        Self { client }
    }
}

impl ContentResolver for PostgresContentStore {
    fn resolve(
        &self,
        request: ContentResolveRequest,
        policy: ContentResolutionPolicy,
    ) -> Result<agent_sdk_core::ResolvedContent, ContentResolutionError> {
        if request.requested_version != request.content_ref.version {
            return Err(content_error(
                ContentResolutionErrorKind::VersionMismatch,
                request.content_ref,
                policy.policy_refs,
            ));
        }
        let response = self
            .client
            .execute(
                format!("select content_json, bytes_base64 from {} where store_scope = $1 and content_id = $2", self.client.table("agent_sdk_content")),
                vec![self.client.scope(), Value::String(request.content_ref.content_id.as_str().to_string())],
            )
            .map_err(|_| content_error(ContentResolutionErrorKind::StorageUnavailable, request.content_ref.clone(), policy.policy_refs.clone()))?;
        let row = response.rows.into_iter().next().ok_or_else(|| {
            content_error(
                ContentResolutionErrorKind::Missing,
                request.content_ref.clone(),
                policy.policy_refs.clone(),
            )
        })?;
        let content_ref: ContentRef = decode_row(row.clone(), "content_json").map_err(|_| {
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
        let bytes_base64 = row
            .get("bytes_base64")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let bytes = BASE64.decode(bytes_base64).map_err(|_| {
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
        Ok(agent_sdk_core::ResolvedContent {
            mime: content_ref.mime.clone(),
            redacted_summary: content_ref.redacted_summary.clone(),
            content_ref,
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

impl ContentStore for PostgresContentStore {
    fn put_content(
        &self,
        content_ref: &ContentRef,
        bytes: Vec<u8>,
    ) -> Result<(), ContentResolutionError> {
        self.client
            .execute(
                format!("insert into {} (store_scope, content_id, content_json, bytes_base64) values ($1, $2, $3, $4) on conflict (store_scope, content_id) do update set content_json = excluded.content_json, bytes_base64 = excluded.bytes_base64", self.client.table("agent_sdk_content")),
                vec![
                    self.client.scope(),
                    Value::String(content_ref.content_id.as_str().to_string()),
                    serde_json::to_value(content_ref).map_err(|_| content_error(ContentResolutionErrorKind::StorageUnavailable, content_ref.clone(), Vec::new()))?,
                    Value::String(BASE64.encode(bytes)),
                ],
            )
            .map_err(|_| content_error(ContentResolutionErrorKind::StorageUnavailable, content_ref.clone(), Vec::new()))?;
        Ok(())
    }
}
