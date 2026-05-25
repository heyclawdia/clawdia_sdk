//! Durable and observable SDK records. Use these DTOs for events, journals, effects,
//! context, output, and feature evidence. Constructing records is data-only;
//! persistence, publication, and external actions happen through ports or application
//! coordinators. This file contains the content portion of that contract.
//!
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
/// Carries the artifact version record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ArtifactVersion(String);

impl ArtifactVersion {
    /// Creates a new records::content value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Carries the content version record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ContentVersion(String);

impl ContentVersion {
    /// Creates a new records::content value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite content kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ContentKind {
    /// Use this variant when the contract needs to represent text; selecting it has no side effect by itself.
    Text,
    /// Use this variant when the contract needs to represent document; selecting it has no side effect by itself.
    Document,
    /// Use this variant when the contract needs to represent image; selecting it has no side effect by itself.
    Image,
    /// Use this variant when the contract needs to represent audio; selecting it has no side effect by itself.
    Audio,
    /// Use this variant when the contract needs to represent video; selecting it has no side effect by itself.
    Video,
    /// Use this variant when the contract needs to represent file; selecting it has no side effect by itself.
    File,
    /// Use this variant when the contract needs to represent tool result; selecting it has no side effect by itself.
    ToolResult,
    /// Use this variant when the contract needs to represent schema; selecting it has no side effect by itself.
    Schema,
    /// Use this variant when the contract needs to represent stdout; selecting it has no side effect by itself.
    Stdout,
    /// Use this variant when the contract needs to represent stderr; selecting it has no side effect by itself.
    Stderr,
    /// Use this variant when the contract needs to represent output payload; selecting it has no side effect by itself.
    OutputPayload,
    /// Use this variant when the contract needs to represent memory record; selecting it has no side effect by itself.
    MemoryRecord,
    /// Use this variant when the contract needs to represent external; selecting it has no side effect by itself.
    External,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite content scope cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ContentScope {
    /// Use this variant when the contract needs to represent run; selecting it has no side effect by itself.
    Run,
    /// Use this variant when the contract needs to represent session; selecting it has no side effect by itself.
    Session,
    /// Use this variant when the contract needs to represent host workspace; selecting it has no side effect by itself.
    HostWorkspace,
    /// Use this variant when the contract needs to represent external; selecting it has no side effect by itself.
    External,
    /// Use this variant when the contract needs to represent persistent store; selecting it has no side effect by itself.
    PersistentStore,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the artifact ref record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ArtifactRef {
    /// Stable artifact id used for typed lineage, lookup, or dedupe.
    pub artifact_id: ArtifactId,
    /// Version string for this capability, package, or protocol surface.
    /// Use it for compatibility checks during package or adapter resolution.
    pub version: ArtifactVersion,
    /// Scope used by this record or request.
    pub scope: ContentScope,
    /// Typed storage ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub storage_ref: AdapterRef,
    /// Optional mime value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub mime: Option<String>,
    /// size bytes used for bounds checks, summaries, or truncation evidence.
    pub size_bytes: Option<u64>,
    /// Stable hash for the bytes or canonical payload used for stale checks
    /// and fingerprints.
    pub content_hash: Option<String>,
    /// Typed producer ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub producer_ref: EntityRef,
    /// Typed source reference that records where this item originated.
    pub source_ref: SourceRef,
    /// Typed destination reference that records where this item is being sent
    /// or projected.
    pub destination_ref: Option<DestinationRef>,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy_class: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention_class: RetentionClass,
    /// Trust class used when deciding whether context or capabilities may be
    /// admitted.
    pub trust_class: TrustClass,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

impl ArtifactRef {
    /// Creates a new records::content value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
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
/// Carries the content ref record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ContentRef {
    /// Stable content id used for typed lineage, lookup, or dedupe.
    pub content_id: ContentId,
    /// Version string for this capability, package, or protocol surface.
    /// Use it for compatibility checks during package or adapter resolution.
    pub version: ContentVersion,
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: ContentKind,
    /// Typed artifact ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub artifact_ref: Option<ArtifactRef>,
    /// Scope used by this record or request.
    pub scope: ContentScope,
    /// Typed producer ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub producer_ref: EntityRef,
    /// Typed source reference that records where this item originated.
    pub source_ref: SourceRef,
    /// Typed destination reference that records where this item is being sent
    /// or projected.
    pub destination_ref: Option<DestinationRef>,
    /// Optional mime value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub mime: Option<String>,
    /// size bytes used for bounds checks, summaries, or truncation evidence.
    pub size_bytes: Option<u64>,
    /// Stable hash for the bytes or canonical payload used for stale checks
    /// and fingerprints.
    pub content_hash: Option<String>,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy_class: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention_class: RetentionClass,
    /// Trust class used when deciding whether context or capabilities may be
    /// admitted.
    pub trust_class: TrustClass,
    /// Typed resolver ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub resolver_ref: AdapterRef,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

impl ContentRef {
    /// Creates a new records::content value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    #[expect(
        clippy::too_many_arguments,
        reason = "ContentRef::new is the explicit durable DTO constructor; a builder would be a separate public API ergonomics pass"
    )]
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

    /// Returns summary for default events for callers that need to inspect the contract state.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn summary_for_default_events(&self) -> &str {
        &self.redacted_summary
    }

    /// Returns whether provider visible without context admission applies for this state.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn provider_visible_without_context_admission(&self) -> bool {
        false
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite content resolution purpose cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ContentResolutionPurpose {
    /// Use this variant when the contract needs to represent context projection; selecting it has no side effect by itself.
    ContextProjection,
    /// Use this variant when the contract needs to represent output validation; selecting it has no side effect by itself.
    OutputValidation,
    /// Use this variant when the contract needs to represent output delivery; selecting it has no side effect by itself.
    OutputDelivery,
    /// Use this variant when the contract needs to represent replay; selecting it has no side effect by itself.
    Replay,
    /// Use this variant when the contract needs to represent host inspection; selecting it has no side effect by itself.
    HostInspection,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite retention use cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RetentionUse {
    /// Use this variant when the contract needs to represent read only; selecting it has no side effect by itself.
    ReadOnly,
    /// Use this variant when the contract needs to represent provider projection; selecting it has no side effect by itself.
    ProviderProjection,
    /// Use this variant when the contract needs to represent validation; selecting it has no side effect by itself.
    Validation,
    /// Use this variant when the contract needs to represent delivery; selecting it has no side effect by itself.
    Delivery,
    /// Use this variant when the contract needs to represent replay; selecting it has no side effect by itself.
    Replay,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite missing content policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum MissingContentPolicy {
    /// Use this variant when the contract needs to represent fail; selecting it has no side effect by itself.
    Fail,
    /// Use this variant when the contract needs to represent recoverable replay gap; selecting it has no side effect by itself.
    RecoverableReplayGap,
    /// Use this variant when the contract needs to represent omit with projection audit; selecting it has no side effect by itself.
    OmitWithProjectionAudit,
    /// Use this variant when the contract needs to represent request host repair; selecting it has no side effect by itself.
    RequestHostRepair,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the content resolution policy record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ContentResolutionPolicy {
    /// Typed caller ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub caller_ref: EntityRef,
    /// Typed destination reference that records where this item is being sent
    /// or projected.
    pub destination_ref: DestinationRef,
    /// Purpose used by this record or request.
    pub purpose: ContentResolutionPurpose,
    /// Privacy classification for the value.
    /// Projection, telemetry, and delivery paths use it to enforce redaction and retention.
    pub allowed_privacy_classes: Vec<PrivacyClass>,
    /// Maximum byte budget the caller requested before truncation or summary
    /// behavior is applied.
    pub max_bytes: u64,
    /// Boolean policy/capability flag for whether require hash match is
    /// enabled.
    pub require_hash_match: bool,
    /// Retention class for referenced content or records.
    /// Stores and telemetry sinks use it to decide how long evidence may be kept.
    pub retention_use: RetentionUse,
    /// On missing used by this record or request.
    pub on_missing: MissingContentPolicy,
    /// Boolean policy/capability flag for whether allow raw content is
    /// enabled.
    pub allow_raw_content: bool,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
}

impl ContentResolutionPolicy {
    /// Builds the redacted context value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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

    /// Builds the raw context value with the documented defaults.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
/// Carries the content resolve request record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ContentResolveRequest {
    /// Content reference where payload bytes or structured tool output are
    /// stored.
    pub content_ref: ContentRef,
    /// Version string for this capability, package, or protocol surface.
    /// Use it for compatibility checks during package or adapter resolution.
    pub requested_version: ContentVersion,
}

impl ContentResolveRequest {
    /// Creates a new records::content value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(content_ref: ContentRef) -> Self {
        Self {
            requested_version: content_ref.version.clone(),
            content_ref,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the resolved content record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ResolvedContent {
    /// Content reference where payload bytes or structured tool output are
    /// stored.
    pub content_ref: ContentRef,
    /// Optional mime value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub mime: Option<String>,
    /// Byte size or byte limit for bytes.
    /// Use it to enforce bounded reads, writes, summaries, or parser output.
    pub bytes: Option<Vec<u8>>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Raw content or raw-content control for this value.
    /// Use it only when policy explicitly allows raw content capture or delivery.
    pub raw_content_included: bool,
}

impl ResolvedContent {
    /// Returns an updated value with redacted configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
/// Enumerates the finite content resolution error kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ContentResolutionErrorKind {
    /// Use this variant when the contract needs to represent missing; selecting it has no side effect by itself.
    Missing,
    /// Use this variant when the contract needs to represent expired; selecting it has no side effect by itself.
    Expired,
    /// Use this variant when the contract needs to represent permission denied; selecting it has no side effect by itself.
    PermissionDenied,
    /// Use this variant when the contract needs to represent storage unavailable; selecting it has no side effect by itself.
    StorageUnavailable,
    /// Use this variant when the contract needs to represent version mismatch; selecting it has no side effect by itself.
    VersionMismatch,
    /// Use this variant when the contract needs to represent hash mismatch; selecting it has no side effect by itself.
    HashMismatch,
    /// Use this variant when the contract needs to represent unsupported mime; selecting it has no side effect by itself.
    UnsupportedMime,
    /// Use this variant when the contract needs to represent raw content not allowed; selecting it has no side effect by itself.
    RawContentNotAllowed,
    /// Use this variant when the contract needs to represent max bytes exceeded; selecting it has no side effect by itself.
    MaxBytesExceeded,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the content resolution error record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ContentResolutionError {
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: ContentResolutionErrorKind,
    /// Content reference where payload bytes or structured tool output are
    /// stored.
    pub content_ref: Box<ContentRef>,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

impl ContentResolutionError {
    /// Converts this value into agent error data.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

/// Builds the resolution error value.
/// This is data construction and performs no I/O, journal append, event publication, or process
pub(crate) fn resolution_error(
    kind: ContentResolutionErrorKind,
    content_ref: ContentRef,
    policy_refs: Vec<PolicyRef>,
) -> ContentResolutionError {
    ContentResolutionError {
        kind,
        redacted_summary: content_ref.redacted_summary.clone(),
        content_ref: Box::new(content_ref),
        policy_refs,
    }
}
