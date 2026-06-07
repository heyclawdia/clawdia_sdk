use agent_sdk_core::{
    ContentResolutionError, ContentResolutionErrorKind, ContentResolutionPolicy,
    ContentResolveRequest, ContentResolver, ContentStore, ResolvedContent, content::ContentRef,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::json;

use crate::client::SupabaseClient;

#[derive(Clone)]
/// Supabase-backed content resolver and store.
pub struct SupabaseContentStore {
    client: SupabaseClient,
}

impl SupabaseContentStore {
    pub fn new(client: SupabaseClient) -> Self {
        Self { client }
    }
}

impl ContentResolver for SupabaseContentStore {
    fn resolve(
        &self,
        request: ContentResolveRequest,
        policy: ContentResolutionPolicy,
    ) -> Result<ResolvedContent, ContentResolutionError> {
        if !policy.allow_raw_content {
            return Ok(ResolvedContent::redacted(
                request.content_ref,
                policy.policy_refs,
            ));
        }
        let query = format!(
            "store_scope=eq.{}&content_id=eq.{}&select=content_ref,bytes_base64&limit=1",
            self.client.config().store_scope(),
            request.content_ref.content_id.as_str()
        );
        let response = self
            .client
            .select("agent_sdk_content", &query)
            .map_err(|_| {
                content_error(
                    ContentResolutionErrorKind::StorageUnavailable,
                    request.content_ref.clone(),
                )
            })?;
        if !(200..300).contains(&response.status) {
            return Err(content_error(
                ContentResolutionErrorKind::StorageUnavailable,
                request.content_ref,
            ));
        }
        let rows =
            serde_json::from_slice::<Vec<serde_json::Value>>(&response.body).map_err(|_| {
                content_error(
                    ContentResolutionErrorKind::StorageUnavailable,
                    request.content_ref.clone(),
                )
            })?;
        let Some(row) = rows.into_iter().next() else {
            return Err(content_error(
                ContentResolutionErrorKind::Missing,
                request.content_ref,
            ));
        };
        let content_ref = serde_json::from_value::<ContentRef>(row["content_ref"].clone())
            .unwrap_or(request.content_ref);
        let encoded = row["bytes_base64"].as_str().ok_or_else(|| {
            content_error(
                ContentResolutionErrorKind::StorageUnavailable,
                content_ref.clone(),
            )
        })?;
        let bytes = STANDARD.decode(encoded).map_err(|_| {
            content_error(
                ContentResolutionErrorKind::StorageUnavailable,
                content_ref.clone(),
            )
        })?;
        if bytes.len() as u64 > policy.max_bytes {
            return Err(content_error(
                ContentResolutionErrorKind::MaxBytesExceeded,
                content_ref,
            ));
        }
        Ok(ResolvedContent {
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

impl ContentStore for SupabaseContentStore {
    fn put_content(
        &self,
        content_ref: &ContentRef,
        bytes: Vec<u8>,
    ) -> Result<(), ContentResolutionError> {
        let response = self
            .client
            .insert(
                "agent_sdk_content",
                &json!({
                    "store_scope": self.client.config().store_scope(),
                    "content_id": content_ref.content_id.as_str(),
                    "content_ref": content_ref,
                    "bytes_base64": STANDARD.encode(&bytes),
                    "byte_len": bytes.len(),
                }),
            )
            .map_err(|_| {
                content_error(
                    ContentResolutionErrorKind::StorageUnavailable,
                    content_ref.clone(),
                )
            })?;
        if !(200..300).contains(&response.status) {
            return Err(content_error(
                ContentResolutionErrorKind::StorageUnavailable,
                content_ref.clone(),
            ));
        }
        Ok(())
    }
}

fn content_error(
    kind: ContentResolutionErrorKind,
    content_ref: ContentRef,
) -> ContentResolutionError {
    ContentResolutionError {
        kind,
        redacted_summary: content_ref.redacted_summary.clone(),
        content_ref: Box::new(content_ref),
        policy_refs: Vec::new(),
    }
}
