use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        AdapterRef, AgentError, ArtifactId, ContentId, DestinationRef, EntityRef, PolicyRef,
        PrivacyClass, RetentionClass, SourceRef, TrustClass,
    },
    error::{AgentErrorKind, RetryClassification},
};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ArtifactVersion(String);

impl ArtifactVersion {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ContentVersion(String);

impl ContentVersion {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentKind {
    Text,
    Document,
    Image,
    Audio,
    Video,
    File,
    ToolResult,
    Schema,
    Stdout,
    Stderr,
    OutputPayload,
    MemoryRecord,
    External,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentScope {
    Run,
    Session,
    HostWorkspace,
    External,
    PersistentStore,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactRef {
    pub artifact_id: ArtifactId,
    pub version: ArtifactVersion,
    pub scope: ContentScope,
    pub storage_ref: AdapterRef,
    pub mime: Option<String>,
    pub size_bytes: Option<u64>,
    pub content_hash: Option<String>,
    pub producer_ref: EntityRef,
    pub source_ref: SourceRef,
    pub destination_ref: Option<DestinationRef>,
    pub privacy_class: PrivacyClass,
    pub retention_class: RetentionClass,
    pub trust_class: TrustClass,
    pub redacted_summary: String,
}

impl ArtifactRef {
    pub fn new(
        artifact_id: ArtifactId,
        version: ArtifactVersion,
        scope: ContentScope,
        storage_ref: AdapterRef,
        producer_ref: EntityRef,
        source_ref: SourceRef,
        redacted_summary: impl Into<String>,
    ) -> Self {
        Self {
            artifact_id,
            version,
            scope,
            storage_ref,
            mime: None,
            size_bytes: None,
            content_hash: None,
            producer_ref,
            source_ref,
            destination_ref: None,
            privacy_class: PrivacyClass::ContentRefsOnly,
            retention_class: RetentionClass::RunScoped,
            trust_class: TrustClass::HostProvided,
            redacted_summary: redacted_summary.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContentRef {
    pub content_id: ContentId,
    pub version: ContentVersion,
    pub kind: ContentKind,
    pub artifact_ref: Option<ArtifactRef>,
    pub scope: ContentScope,
    pub producer_ref: EntityRef,
    pub source_ref: SourceRef,
    pub destination_ref: Option<DestinationRef>,
    pub mime: Option<String>,
    pub size_bytes: Option<u64>,
    pub content_hash: Option<String>,
    pub privacy_class: PrivacyClass,
    pub retention_class: RetentionClass,
    pub trust_class: TrustClass,
    pub resolver_ref: AdapterRef,
    pub redacted_summary: String,
}

impl ContentRef {
    pub fn new(
        content_id: ContentId,
        version: ContentVersion,
        kind: ContentKind,
        scope: ContentScope,
        producer_ref: EntityRef,
        source_ref: SourceRef,
        resolver_ref: AdapterRef,
        redacted_summary: impl Into<String>,
    ) -> Self {
        Self {
            content_id,
            version,
            kind,
            artifact_ref: None,
            scope,
            producer_ref,
            source_ref,
            destination_ref: None,
            mime: None,
            size_bytes: None,
            content_hash: None,
            privacy_class: PrivacyClass::ContentRefsOnly,
            retention_class: RetentionClass::RunScoped,
            trust_class: TrustClass::HostProvided,
            resolver_ref,
            redacted_summary: redacted_summary.into(),
        }
    }

    pub fn summary_for_default_events(&self) -> &str {
        &self.redacted_summary
    }

    pub fn provider_visible_without_context_admission(&self) -> bool {
        false
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentResolutionPurpose {
    ContextProjection,
    OutputValidation,
    OutputDelivery,
    Replay,
    HostInspection,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionUse {
    ReadOnly,
    ProviderProjection,
    Validation,
    Delivery,
    Replay,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingContentPolicy {
    Fail,
    RecoverableReplayGap,
    OmitWithProjectionAudit,
    RequestHostRepair,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContentResolutionPolicy {
    pub caller_ref: EntityRef,
    pub destination_ref: DestinationRef,
    pub purpose: ContentResolutionPurpose,
    pub allowed_privacy_classes: Vec<PrivacyClass>,
    pub max_bytes: u64,
    pub require_hash_match: bool,
    pub retention_use: RetentionUse,
    pub on_missing: MissingContentPolicy,
    pub allow_raw_content: bool,
    pub policy_refs: Vec<PolicyRef>,
}

impl ContentResolutionPolicy {
    pub fn redacted_context(
        caller_ref: EntityRef,
        destination_ref: DestinationRef,
        policy_ref: PolicyRef,
    ) -> Self {
        Self {
            caller_ref,
            destination_ref,
            purpose: ContentResolutionPurpose::ContextProjection,
            allowed_privacy_classes: vec![PrivacyClass::Public, PrivacyClass::ContentRefsOnly],
            max_bytes: 0,
            require_hash_match: true,
            retention_use: RetentionUse::ProviderProjection,
            on_missing: MissingContentPolicy::Fail,
            allow_raw_content: false,
            policy_refs: vec![policy_ref],
        }
    }

    pub fn raw_context(
        caller_ref: EntityRef,
        destination_ref: DestinationRef,
        policy_ref: PolicyRef,
        max_bytes: u64,
    ) -> Self {
        let mut policy = Self::redacted_context(caller_ref, destination_ref, policy_ref);
        policy.allowed_privacy_classes = vec![PrivacyClass::Public, PrivacyClass::ContentRefsOnly];
        policy.max_bytes = max_bytes;
        policy.allow_raw_content = true;
        policy
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContentResolveRequest {
    pub content_ref: ContentRef,
    pub requested_version: ContentVersion,
}

impl ContentResolveRequest {
    pub fn new(content_ref: ContentRef) -> Self {
        Self {
            requested_version: content_ref.version.clone(),
            content_ref,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResolvedContent {
    pub content_ref: ContentRef,
    pub mime: Option<String>,
    pub bytes: Option<Vec<u8>>,
    pub redacted_summary: String,
    pub policy_refs: Vec<PolicyRef>,
    pub raw_content_included: bool,
}

impl ResolvedContent {
    pub fn redacted(content_ref: ContentRef, policy_refs: Vec<PolicyRef>) -> Self {
        Self {
            mime: content_ref.mime.clone(),
            redacted_summary: content_ref.redacted_summary.clone(),
            content_ref,
            bytes: None,
            policy_refs,
            raw_content_included: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentResolutionErrorKind {
    Missing,
    Expired,
    PermissionDenied,
    StorageUnavailable,
    VersionMismatch,
    HashMismatch,
    UnsupportedMime,
    RawContentNotAllowed,
    MaxBytesExceeded,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContentResolutionError {
    pub kind: ContentResolutionErrorKind,
    pub content_ref: ContentRef,
    pub policy_refs: Vec<PolicyRef>,
    pub redacted_summary: String,
}

impl ContentResolutionError {
    pub fn to_agent_error(&self) -> AgentError {
        let retry = match self.kind {
            ContentResolutionErrorKind::Missing => RetryClassification::RepairNeeded,
            ContentResolutionErrorKind::StorageUnavailable => RetryClassification::Retryable,
            _ => RetryClassification::NotRetryable,
        };
        AgentError::new(
            AgentErrorKind::ProjectionFailure,
            retry,
            format!("content resolution failed: {:?}", self.kind),
        )
    }
}

impl fmt::Display for ContentResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "content resolution failed: {:?}", self.kind)
    }
}

impl std::error::Error for ContentResolutionError {}

pub(crate) fn resolution_error(
    kind: ContentResolutionErrorKind,
    content_ref: ContentRef,
    policy_refs: Vec<PolicyRef>,
) -> ContentResolutionError {
    ContentResolutionError {
        kind,
        redacted_summary: content_ref.redacted_summary.clone(),
        content_ref,
        policy_refs,
    }
}
