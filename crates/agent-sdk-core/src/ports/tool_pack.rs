use std::{collections::BTreeMap, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::domain::{
    AgentError, AgentErrorKind, ContentRef, PolicyRef, PrivacyClass, RetentionClass,
    RetryClassification, SourceRef,
};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ResourceScheme(String);

impl ResourceScheme {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(!value.is_empty(), "ResourceScheme must not be empty");
        assert!(
            value
                .chars()
                .all(|character| character.is_ascii_alphanumeric()
                    || matches!(character, '+' | '-' | '.')),
            "ResourceScheme must use URI scheme characters"
        );
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResourceReadRequest {
    pub uri: String,
    pub source: SourceRef,
    pub policy_refs: Vec<PolicyRef>,
    pub max_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResourceResolution {
    pub uri: String,
    pub scheme: ResourceScheme,
    pub content_ref: ContentRef,
    pub source: SourceRef,
    pub policy_refs: Vec<PolicyRef>,
    pub byte_len: u64,
    pub truncated: bool,
    pub parser_version: String,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
    pub redacted_summary: String,
}

pub trait ResourceResolver: Send + Sync {
    fn scheme(&self) -> &ResourceScheme;

    fn resolve(&self, request: &ResourceReadRequest) -> Result<ResourceResolution, AgentError>;
}

#[derive(Clone, Default)]
pub struct ResourceRouter {
    resolvers: BTreeMap<ResourceScheme, Arc<dyn ResourceResolver>>,
}

impl ResourceRouter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, resolver: Arc<dyn ResourceResolver>) {
        self.resolvers.insert(resolver.scheme().clone(), resolver);
    }

    pub fn register_static(
        &mut self,
        scheme: ResourceScheme,
        content_ref: ContentRef,
        source: SourceRef,
        policy_ref: PolicyRef,
    ) {
        self.register(Arc::new(StaticResourceResolver {
            scheme,
            content_ref,
            source,
            policy_ref,
        }));
    }

    pub fn resolve(&self, request: &ResourceReadRequest) -> Result<ResourceResolution, AgentError> {
        let scheme = parse_scheme(&request.uri)?;
        let Some(resolver) = self.resolvers.get(&scheme) else {
            return Err(AgentError::new(
                AgentErrorKind::PolicyDenial,
                RetryClassification::HostConfigurationNeeded,
                "resource URI scheme has no registered resolver in the runtime package boundary",
            ));
        };
        resolver.resolve(request)
    }
}

#[derive(Clone)]
struct StaticResourceResolver {
    scheme: ResourceScheme,
    content_ref: ContentRef,
    source: SourceRef,
    policy_ref: PolicyRef,
}

impl ResourceResolver for StaticResourceResolver {
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
            parser_version: "static.resource.resolver.v1".to_string(),
            privacy: PrivacyClass::ContentRefsOnly,
            retention: RetentionClass::RunScoped,
            redacted_summary: "resource resolved to content ref".to_string(),
        })
    }
}

fn parse_scheme(uri: &str) -> Result<ResourceScheme, AgentError> {
    let Some((scheme, _rest)) = uri.split_once("://") else {
        return Err(AgentError::contract_violation(
            "resource URI must include a scheme",
        ));
    };
    if scheme.is_empty() {
        return Err(AgentError::contract_violation(
            "resource URI scheme must not be empty",
        ));
    }
    Ok(ResourceScheme::new(scheme))
}
