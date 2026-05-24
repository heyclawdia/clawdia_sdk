use crate::content::{
    ContentRef, ContentResolutionError, ContentResolutionErrorKind, ContentResolutionPolicy,
    ContentResolveRequest, ResolvedContent,
};

pub trait ContentResolver {
    fn resolve(
        &self,
        request: ContentResolveRequest,
        policy: ContentResolutionPolicy,
    ) -> Result<ResolvedContent, ContentResolutionError>;

    fn store_resolved_content(
        &self,
        content_ref: &ContentRef,
        _bytes: Vec<u8>,
    ) -> Result<(), ContentResolutionError> {
        Err(ContentResolutionError {
            kind: ContentResolutionErrorKind::StorageUnavailable,
            redacted_summary: content_ref.redacted_summary.clone(),
            content_ref: content_ref.clone(),
            policy_refs: Vec::new(),
        })
    }
}
