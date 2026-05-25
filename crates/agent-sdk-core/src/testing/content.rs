//! Deterministic test-kit helpers for SDK consumers. Use these fakes and harnesses to
//! exercise public contracts without live providers, real stores, product UI, network
//! telemetry, or wall-clock-dependent infrastructure. They mutate only their
//! in-memory state unless noted. This file contains the content portion of that
//! contract.
//!
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use crate::{
    content::{
        ContentRef, ContentResolutionError, ContentResolutionErrorKind, ContentResolutionPolicy,
        ContentResolveRequest, ContentVersion, MissingContentPolicy, ResolvedContent,
        resolution_error,
    },
    content_ports::ContentResolver,
    domain::ContentId,
};

#[derive(Clone, Debug, Default)]
/// In-memory fake content resolver fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct FakeContentResolver {
    entries: Arc<Mutex<BTreeMap<ContentId, FakeResolvedContent>>>,
}

impl FakeContentResolver {
    /// Insert text.
    /// This reads or mutates deterministic in-memory test state unless the method explicitly
    /// names a fixture file.
    pub fn insert_text(&self, content_ref: &ContentRef, text: impl Into<String>) {
        self.store_resolved_content(content_ref, text.into().into_bytes())
            .expect("fake content resolver insert");
    }

    /// Assert conformance.
    /// This reads or mutates deterministic in-memory test state unless the method explicitly
    /// names a fixture file.
    pub fn assert_conformance<R: ContentResolver>(
        resolver: &R,
        present_ref: ContentRef,
        missing_ref: ContentRef,
        policy: ContentResolutionPolicy,
    ) {
        let present = resolver.resolve(ContentResolveRequest::new(present_ref), policy.clone());
        assert!(
            present.is_ok(),
            "present ref must resolve under supplied policy"
        );

        let missing = resolver.resolve(ContentResolveRequest::new(missing_ref), policy);
        assert!(
            matches!(
                missing,
                Err(ContentResolutionError {
                    kind: ContentResolutionErrorKind::Missing,
                    ..
                })
            ),
            "missing ref must return a typed Missing error"
        );
    }
}

impl ContentResolver for FakeContentResolver {
    fn store_resolved_content(
        &self,
        content_ref: &ContentRef,
        bytes: Vec<u8>,
    ) -> Result<(), ContentResolutionError> {
        self.entries
            .lock()
            .expect("fake content resolver lock")
            .insert(
                content_ref.content_id.clone(),
                FakeResolvedContent {
                    version: content_ref.version.clone(),
                    bytes,
                    mime: content_ref.mime.clone(),
                    content_hash: content_ref.content_hash.clone(),
                },
            );
        Ok(())
    }

    fn resolve(
        &self,
        request: ContentResolveRequest,
        policy: ContentResolutionPolicy,
    ) -> Result<ResolvedContent, ContentResolutionError> {
        if request.requested_version != request.content_ref.version {
            return Err(resolution_error(
                ContentResolutionErrorKind::VersionMismatch,
                request.content_ref,
                policy.policy_refs,
            ));
        }
        if !policy
            .allowed_privacy_classes
            .contains(&request.content_ref.privacy_class)
        {
            return Err(resolution_error(
                ContentResolutionErrorKind::PermissionDenied,
                request.content_ref,
                policy.policy_refs,
            ));
        }

        let stored = match self
            .entries
            .lock()
            .expect("fake content resolver lock")
            .get(&request.content_ref.content_id)
            .cloned()
        {
            Some(stored) => stored,
            None => {
                return match policy.on_missing {
                    MissingContentPolicy::RecoverableReplayGap
                    | MissingContentPolicy::OmitWithProjectionAudit
                    | MissingContentPolicy::RequestHostRepair
                    | MissingContentPolicy::Fail => Err(resolution_error(
                        ContentResolutionErrorKind::Missing,
                        request.content_ref,
                        policy.policy_refs,
                    )),
                };
            }
        };

        if stored.version != request.requested_version {
            return Err(resolution_error(
                ContentResolutionErrorKind::VersionMismatch,
                request.content_ref,
                policy.policy_refs,
            ));
        }
        if policy.require_hash_match && request.content_ref.content_hash != stored.content_hash {
            return Err(resolution_error(
                ContentResolutionErrorKind::HashMismatch,
                request.content_ref,
                policy.policy_refs,
            ));
        }
        if !policy.allow_raw_content {
            return Ok(ResolvedContent::redacted(
                request.content_ref,
                policy.policy_refs,
            ));
        }
        if stored.bytes.len() as u64 > policy.max_bytes {
            return Err(resolution_error(
                ContentResolutionErrorKind::MaxBytesExceeded,
                request.content_ref,
                policy.policy_refs,
            ));
        }

        Ok(ResolvedContent {
            content_ref: request.content_ref,
            mime: stored.mime,
            bytes: Some(stored.bytes),
            redacted_summary: "raw content resolved by explicit policy".to_string(),
            policy_refs: policy.policy_refs,
            raw_content_included: true,
        })
    }
}

#[derive(Clone, Debug)]
struct FakeResolvedContent {
    version: ContentVersion,
    bytes: Vec<u8>,
    mime: Option<String>,
    content_hash: Option<String>,
}
