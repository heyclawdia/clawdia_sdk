//! Host adapter boundaries for the SDK core. Use these traits and registries when
//! hosts provide providers, journals, sinks, tools, isolation, extensions, telemetry,
//! or subscriptions. Implementations may perform external side effects and must honor
//! policy, redaction, idempotency, and replay contracts. This file contains the
//! content portion of that contract.
//!
use crate::content::{
    ContentRef, ContentResolutionError, ContentResolutionErrorKind, ContentResolutionPolicy,
    ContentResolveRequest, ResolvedContent,
};

/// Port or behavior contract for content resolver. Implementors should
/// preserve policy, redaction, idempotency, and replay expectations
/// from the surrounding module. Implementations may perform side
/// effects only as described by the trait methods.
pub trait ContentResolver {
    /// Resolves resolve through the configured ports::content boundary.
    /// Concrete implementations own any backing-store, filesystem, or network
    /// side effects.
    fn resolve(
        &self,
        request: ContentResolveRequest,
        policy: ContentResolutionPolicy,
    ) -> Result<ResolvedContent, ContentResolutionError>;

    /// Stores resolved content bytes and metadata in the content backing store.
    /// Implementations store the resolved content bytes and metadata in the content resolver
    /// backing store for later policy-checked lookup.
    fn store_resolved_content(
        &self,
        content_ref: &ContentRef,
        _bytes: Vec<u8>,
    ) -> Result<(), ContentResolutionError> {
        Err(ContentResolutionError {
            kind: ContentResolutionErrorKind::StorageUnavailable,
            redacted_summary: content_ref.redacted_summary.clone(),
            content_ref: Box::new(content_ref.clone()),
            policy_refs: Vec::new(),
        })
    }
}

/// Explicit content store contract for durable adapters.
pub trait ContentStore: ContentResolver + Send + Sync {
    /// Stores raw bytes for a content ref. Resolution still goes through
    /// `ContentResolver` and its policy checks.
    fn put_content(
        &self,
        content_ref: &ContentRef,
        bytes: Vec<u8>,
    ) -> Result<(), ContentResolutionError>;
}
