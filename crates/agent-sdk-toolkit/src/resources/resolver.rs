use std::sync::Arc;

use agent_sdk_core::{
    AgentError, PolicyRef, ResourceReadRequest, ResourceResolution, ResourceResolver,
    ResourceScheme, RetentionClass, domain::ContentRef,
};

#[derive(Clone)]
pub struct InMemoryResourceResolver {
    scheme: ResourceScheme,
    content_ref: ContentRef,
    source: agent_sdk_core::SourceRef,
    policy_ref: PolicyRef,
}

impl InMemoryResourceResolver {
    pub fn new(
        scheme: &str,
        content_ref: ContentRef,
        source: agent_sdk_core::SourceRef,
        policy_ref: PolicyRef,
    ) -> Arc<Self> {
        Arc::new(Self {
            scheme: ResourceScheme::new(scheme),
            content_ref,
            source,
            policy_ref,
        })
    }
}

impl ResourceResolver for InMemoryResourceResolver {
    fn scheme(&self) -> &ResourceScheme {
        &self.scheme
    }

    fn resolve(&self, request: &ResourceReadRequest) -> Result<ResourceResolution, AgentError> {
        Ok(ResourceResolution {
            uri: request.uri.clone(),
            scheme: self.scheme.clone(),
            content_ref: self.content_ref.clone(),
            source: self.source.clone(),
            policy_refs: vec![self.policy_ref.clone()],
            byte_len: 0,
            truncated: false,
            parser_version: "toolkit.in_memory_resource.v1".to_string(),
            privacy: agent_sdk_core::PrivacyClass::ContentRefsOnly,
            retention: RetentionClass::RunScoped,
            redacted_summary: "in-memory resource resolved to content ref".to_string(),
        })
    }
}
