//! Host adapter boundaries for the SDK core. Use these traits and registries when
//! hosts provide providers, journals, sinks, tools, isolation, extensions, telemetry,
//! or subscriptions. Implementations may perform external side effects and must honor
//! policy, redaction, idempotency, and replay contracts. This file contains the tool
//! pack portion of that contract.
//!
use std::{collections::BTreeMap, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::domain::{
    AgentError, AgentErrorKind, ContentRef, PolicyRef, PrivacyClass, RetentionClass,
    RetryClassification, SourceRef,
};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Carries resource scheme data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ResourceScheme(String);

impl ResourceScheme {
    /// Creates a new ports::tool_pack value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
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

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries resource read request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ResourceReadRequest {
    /// Resource URI selected for explicit resolution.
    pub uri: String,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Maximum byte budget the caller requested before truncation or summary
    /// behavior is applied.
    pub max_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries resource resolution data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ResourceResolution {
    /// Resource URI selected for explicit resolution.
    pub uri: String,
    /// URI scheme resolved by the resource reader.
    pub scheme: ResourceScheme,
    /// Content reference where payload bytes or structured tool output are
    /// stored.
    pub content_ref: ContentRef,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Observed byte length for the source, sidecar, or extracted record.
    pub byte_len: u64,
    /// Whether output was shortened by byte, item, page, archive, or parser
    /// limits.
    pub truncated: bool,
    /// Version string for this capability, package, or protocol surface.
    /// Use it for compatibility checks during package or adapter resolution.
    pub parser_version: String,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention: RetentionClass,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

/// Port or behavior contract for resource resolver. Implementors should
/// preserve policy, redaction, idempotency, and replay expectations
/// from the surrounding module. Implementations may perform side
/// effects only as described by the trait methods.
pub trait ResourceResolver: Send + Sync {
    /// Returns the scheme identifier for this adapter.
    /// This returns resource routing metadata and does not resolve the resource.
    fn scheme(&self) -> &ResourceScheme;

    /// Resolves resolve through the configured ports::tool_pack boundary.
    /// Concrete implementations own any backing-store, filesystem, or network
    /// side effects.
    fn resolve(&self, request: &ResourceReadRequest) -> Result<ResourceResolution, AgentError>;
}

#[derive(Clone, Default)]
/// Carries resource router data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ResourceRouter {
    resolvers: BTreeMap<ResourceScheme, Arc<dyn ResourceResolver>>,
}

impl ResourceRouter {
    /// Creates a new ports::tool_pack value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds data to this in-memory ports::tool_pack collection. It does not
    /// perform external I/O, execute tools, or append journals.
    pub fn register(&mut self, resolver: Arc<dyn ResourceResolver>) {
        self.resolvers.insert(resolver.scheme().clone(), resolver);
    }

    /// Builds the register static value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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

    /// Resolves resolve through the configured ports::tool_pack boundary.
    /// Concrete implementations own any backing-store, filesystem, or network
    /// side effects.
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
