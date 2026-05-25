//! Capability identity, projection, executable routes, and readiness records. Use
//! this module when a runtime package exposes callable or discoverable behavior.
//! Projection helpers are pure and must not execute tools.
//!
use core::fmt;

use serde::{Deserialize, Deserializer, Serialize, de::Error as DeError};

use crate::{
    domain::{
        AdapterRef, AgentError, IdValidationError, PolicyKind, PolicyRef, PrivacyClass, SourceRef,
    },
    ids::validate_identifier,
};

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Describes the capability id portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct CapabilityId(String);

impl CapabilityId {
    /// Creates a new package::capability value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    ///
    /// # Panics
    ///
    /// Panics if constructor invariants fail, such as invalid identifier
    /// text or constructor-specific bounds. Use a fallible constructor such as
    /// `try_new` when one is available for untrusted input.
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("CapabilityId must be valid")
    }

    /// Creates a new package::capability value after validation.
    /// Returns an SDK error instead of panicking when the identifier or
    /// input does not satisfy the contract.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdValidationError> {
        let value = value.into();
        validate_identifier(&value)?;
        Ok(Self(value))
    }

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for CapabilityId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for CapabilityId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(D::Error::custom)
    }
}

impl fmt::Debug for CapabilityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CapabilityId(redacted)")
    }
}

impl fmt::Display for CapabilityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CapabilityId(redacted)")
    }
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Describes the capability namespace portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct CapabilityNamespace(String);

impl CapabilityNamespace {
    /// Creates a new package::capability value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    ///
    /// # Panics
    ///
    /// Panics if constructor invariants fail, such as invalid identifier
    /// text or constructor-specific bounds. Use a fallible constructor such as
    /// `try_new` when one is available for untrusted input.
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("CapabilityNamespace must be valid")
    }

    /// Creates a new package::capability value after validation.
    /// Returns an SDK error instead of panicking when the identifier or
    /// input does not satisfy the contract.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdValidationError> {
        let value = value.into();
        validate_identifier(&value)?;
        Ok(Self(value))
    }

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for CapabilityNamespace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(D::Error::custom)
    }
}

impl fmt::Debug for CapabilityNamespace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CapabilityNamespace(redacted)")
    }
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Describes the capability version portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct CapabilityVersion(String);

impl CapabilityVersion {
    /// Creates a new package::capability value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    ///
    /// # Panics
    ///
    /// Panics if constructor invariants fail, such as invalid identifier
    /// text or constructor-specific bounds. Use a fallible constructor such as
    /// `try_new` when one is available for untrusted input.
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("CapabilityVersion must be valid")
    }

    /// Creates a new package::capability value after validation.
    /// Returns an SDK error instead of panicking when the identifier or
    /// input does not satisfy the contract.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdValidationError> {
        let value = value.into();
        validate_identifier(&value)?;
        Ok(Self(value))
    }

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for CapabilityVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(D::Error::custom)
    }
}

impl fmt::Debug for CapabilityVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CapabilityVersion(redacted)")
    }
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Describes the executor ref portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ExecutorRef(String);

impl ExecutorRef {
    /// Creates a new package::capability value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    ///
    /// # Panics
    ///
    /// Panics if constructor invariants fail, such as invalid identifier
    /// text or constructor-specific bounds. Use a fallible constructor such as
    /// `try_new` when one is available for untrusted input.
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("ExecutorRef must be valid")
    }

    /// Creates a new package::capability value after validation.
    /// Returns an SDK error instead of panicking when the identifier or
    /// input does not satisfy the contract.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdValidationError> {
        let value = value.into();
        validate_identifier(&value)?;
        Ok(Self(value))
    }

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ExecutorRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(D::Error::custom)
    }
}

impl fmt::Debug for ExecutorRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ExecutorRef(redacted)")
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite capability kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum CapabilityKind {
    /// Use this variant when the contract needs to represent tool; selecting it has no side effect by itself.
    Tool,
    /// Use this variant when the contract needs to represent mcp tool; selecting it has no side effect by itself.
    McpTool,
    /// Use this variant when the contract needs to represent mcp resource; selecting it has no side effect by itself.
    McpResource,
    /// Use this variant when the contract needs to represent tool discovery candidate; selecting it has no side effect by itself.
    ToolDiscoveryCandidate,
    /// Use this variant when the contract needs to represent agent as tool; selecting it has no side effect by itself.
    AgentAsTool,
    /// Use this variant when the contract needs to represent extension action; selecting it has no side effect by itself.
    ExtensionAction,
    /// Use this variant when the contract needs to represent stream control; selecting it has no side effect by itself.
    StreamControl,
    /// Use this variant when the contract needs to represent realtime action; selecting it has no side effect by itself.
    RealtimeAction,
}

impl CapabilityKind {
    /// Reports whether this value is reserved. The check is pure and
    /// does not mutate SDK or host state.
    pub fn is_reserved(&self) -> bool {
        !matches!(self, Self::Tool)
    }

    /// Returns owner role for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn owner_role(&self) -> &'static str {
        match self {
            Self::Tool => "02-runtime-package-p0-fake-tool",
            Self::McpTool | Self::McpResource | Self::ToolDiscoveryCandidate => {
                "04-tools-approval-toolpacks"
            }
            Self::AgentAsTool => "07-subagents-coordination",
            Self::ExtensionAction => "08-extension-sdk-packaging",
            Self::StreamControl | Self::RealtimeAction => "05-streaming-realtime-rules",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite capability source kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum CapabilitySourceKind {
    /// Use this variant when the contract needs to represent sdk built in; selecting it has no side effect by itself.
    SdkBuiltIn,
    /// Use this variant when the contract needs to represent host provided; selecting it has no side effect by itself.
    HostProvided,
    /// Use this variant when the contract needs to represent tool pack; selecting it has no side effect by itself.
    ToolPack,
    /// Use this variant when the contract needs to represent mcp server; selecting it has no side effect by itself.
    McpServer,
    /// Use this variant when the contract needs to represent extension; selecting it has no side effect by itself.
    Extension,
    /// Use this variant when the contract needs to represent subagent; selecting it has no side effect by itself.
    Subagent,
    /// Use this variant when the contract needs to represent discovery index; selecting it has no side effect by itself.
    DiscoveryIndex,
    /// Use this variant when the contract needs to represent test only fake; selecting it has no side effect by itself.
    TestOnlyFake,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the capability source portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct CapabilitySource {
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: CapabilitySourceKind,
    /// Typed source reference that records where this item originated.
    pub source_ref: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed adapter ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub adapter_ref: Option<AdapterRef>,
}

impl CapabilitySource {
    /// Builds the test fake value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn test_fake(source_ref: SourceRef) -> Self {
        Self {
            kind: CapabilitySourceKind::TestOnlyFake,
            source_ref,
            adapter_ref: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite capability visibility cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum CapabilityVisibility {
    /// Use this variant when the contract needs to represent active; selecting it has no side effect by itself.
    Active,
    /// Use this variant when the contract needs to represent hidden; selecting it has no side effect by itself.
    Hidden,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite projection mode cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ProjectionMode {
    /// Use this variant when the contract needs to represent not projected; selecting it has no side effect by itself.
    NotProjected,
    /// Use this variant when the contract needs to represent descriptor only; selecting it has no side effect by itself.
    DescriptorOnly,
    /// Use this variant when the contract needs to represent provider tool schema; selecting it has no side effect by itself.
    ProviderToolSchema {
        /// Typed schema ref reference. Resolving or executing it is a
        /// separate policy-gated step.
        schema_ref: PackageSidecarRef,
    },
    /// Use this variant when the contract needs to represent produces context items; selecting it has no side effect by itself.
    ProducesContextItems {
        /// Kinds this capability is allowed to produce.
        /// Package validation uses the list to reject undeclared context item kinds.
        allowed_kinds: Vec<String>,
    },
    /// Use this variant when the contract needs to represent projects context refs; selecting it has no side effect by itself.
    ProjectsContextRefs {
        /// Reference kinds this capability is allowed to project.
        /// Projection validation uses the list to reject undeclared reference kinds.
        allowed_ref_kinds: Vec<String>,
    },
}

impl ProjectionMode {
    /// Reports whether this value is projected. The check is pure and
    /// does not mutate SDK or host state.
    pub fn is_projected(&self) -> bool {
        !matches!(self, Self::NotProjected)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the package sidecar ref portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct PackageSidecarRef {
    /// Identifier for the typed package sidecar.
    pub sidecar_id: String,
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: String,
    /// Version string for this capability, package, or protocol surface.
    /// Use it for compatibility checks during package or adapter resolution.
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable hash for the bytes or canonical payload used for stale checks
    /// and fingerprints.
    pub content_hash: Option<String>,
}

impl PackageSidecarRef {
    /// Creates a new package::capability value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(
        sidecar_id: impl Into<String>,
        kind: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            sidecar_id: sidecar_id.into(),
            kind: kind.into(),
            version: version.into(),
            content_hash: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite capability readiness status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum CapabilityReadinessStatus {
    /// Use this variant when the contract needs to represent active; selecting it has no side effect by itself.
    Active,
    /// Use this variant when the contract needs to represent reserved inactive; selecting it has no side effect by itself.
    ReservedInactive,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the capability readiness portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct CapabilityReadiness {
    /// Finite status for this record or lifecycle stage.
    pub status: CapabilityReadinessStatus,
    /// Implementation owner responsible for this capability surface.
    /// Use it to route follow-up work and validation ownership.
    pub owner_role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Sidecar contract that carries typed capability data.
    /// Package validation uses it to connect capability specs to executable sidecars.
    pub typed_sidecar_contract: Option<String>,
    #[serde(default)]
    /// Deterministic fingerprint fields used for stale checks, package
    /// evidence, or replay comparisons.
    pub fingerprint_fields: Vec<String>,
    #[serde(default)]
    /// Event kinds emitted by this capability or feature.
    /// Use them to keep event fixtures and subscriptions aligned with the public contract.
    pub emitted_events: Vec<String>,
    #[serde(default)]
    /// Journal record kinds produced by this capability or feature.
    /// Use them to keep replay and recovery fixtures aligned with the public contract.
    pub journal_records: Vec<String>,
    #[serde(default)]
    /// Acceptance tests that prove the capability contract is implemented.
    /// Use them as release-readiness evidence before marking the capability active.
    pub acceptance_tests: Vec<String>,
}

impl CapabilityReadiness {
    /// Returns active tool for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn active_tool() -> Self {
        Self {
            status: CapabilityReadinessStatus::Active,
            owner_role: CapabilityKind::Tool.owner_role().to_string(),
            typed_sidecar_contract: Some(
                "capability.tool.schema_ref.executor_ref.policy_ref".to_string(),
            ),
            fingerprint_fields: vec![
                "capability_id".to_string(),
                "kind".to_string(),
                "namespace".to_string(),
                "version".to_string(),
                "projection".to_string(),
                "executor_ref".to_string(),
                "policy_ref".to_string(),
                "sidecar_refs".to_string(),
                "source".to_string(),
            ],
            emitted_events: vec![
                "capability_loaded".to_string(),
                "tool_requested".to_string(),
                "tool_completed".to_string(),
            ],
            journal_records: vec![
                "package_catalog_snapshot".to_string(),
                "package_delta".to_string(),
                "tool_execution_intent".to_string(),
                "tool_execution_result".to_string(),
            ],
            acceptance_tests: vec![
                "provider_visible_capability_requires_executor_and_policy_refs".to_string(),
                "projection_and_execution_hashes_match".to_string(),
            ],
        }
    }

    /// Returns reserved for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn reserved(kind: &CapabilityKind) -> Self {
        Self {
            status: CapabilityReadinessStatus::ReservedInactive,
            owner_role: kind.owner_role().to_string(),
            typed_sidecar_contract: None,
            fingerprint_fields: Vec::new(),
            emitted_events: Vec::new(),
            journal_records: Vec::new(),
            acceptance_tests: Vec::new(),
        }
    }

    /// Reports whether this value is active. The check is pure and does
    /// not mutate SDK or host state.
    pub fn is_active(&self) -> bool {
        matches!(self.status, CapabilityReadinessStatus::Active)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the capability spec portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct CapabilitySpec {
    /// Stable capability identifier used for package projection and
    /// executable routing.
    pub capability_id: CapabilityId,
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: CapabilityKind,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: CapabilitySource,
    /// Namespace that groups this capability or identifier.
    /// Use it to avoid collisions between packages, hosts, and extensions.
    pub namespace: CapabilityNamespace,
    /// Version string for this capability, package, or protocol surface.
    /// Use it for compatibility checks during package or adapter resolution.
    pub version: CapabilityVersion,
    /// Visibility class for the capability or result.
    /// Discovery and projection use it to decide what callers or models can see.
    pub visibility: CapabilityVisibility,
    /// Projection controls for exposing data to a provider or subscriber.
    /// Use it to keep provider-visible data separate from private SDK state.
    pub projection: ProjectionMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed executor ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub executor_ref: Option<ExecutorRef>,
    /// Policy reference that must be resolved by the host or runtime before
    /// execution.
    pub policy_ref: PolicyRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// References to typed package sidecars needed by this capability.
    pub sidecar_refs: Vec<PackageSidecarRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed isolation ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub isolation_ref: Option<PackageSidecarRef>,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Readiness state for a capability or package feature.
    /// Launch and package validation use it to distinguish active, reserved, and blocked
    /// surfaces.
    pub readiness: CapabilityReadiness,
}

impl CapabilitySpec {
    /// Returns fake tool for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn fake_tool(
        capability_id: impl Into<CapabilityId>,
        name: impl Into<String>,
        schema_ref: PackageSidecarRef,
        executor_ref: ExecutorRef,
        policy_ref: PolicyRef,
        source: SourceRef,
    ) -> Self {
        let name = name.into();
        Self {
            capability_id: capability_id.into(),
            kind: CapabilityKind::Tool,
            source: CapabilitySource::test_fake(source),
            namespace: CapabilityNamespace::new(format!("tool.{name}")),
            version: CapabilityVersion::new("v1"),
            visibility: CapabilityVisibility::Active,
            projection: ProjectionMode::ProviderToolSchema { schema_ref },
            executor_ref: Some(executor_ref),
            policy_ref,
            sidecar_refs: Vec::new(),
            isolation_ref: None,
            privacy: PrivacyClass::ContentRefsOnly,
            readiness: CapabilityReadiness::active_tool(),
        }
    }

    /// Builds the reserved inactive value with the documented defaults.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn reserved_inactive(
        capability_id: impl Into<CapabilityId>,
        kind: CapabilityKind,
        policy_ref: PolicyRef,
        source: SourceRef,
    ) -> Self {
        let owner = kind.owner_role().replace('-', ".");
        Self {
            capability_id: capability_id.into(),
            kind: kind.clone(),
            source: CapabilitySource::test_fake(source),
            namespace: CapabilityNamespace::new(format!("reserved.{owner}")),
            version: CapabilityVersion::new("reserved"),
            visibility: CapabilityVisibility::Hidden,
            projection: ProjectionMode::NotProjected,
            executor_ref: None,
            policy_ref,
            sidecar_refs: Vec::new(),
            isolation_ref: None,
            privacy: PrivacyClass::Internal,
            readiness: CapabilityReadiness::reserved(&kind),
        }
    }

    /// Computes or returns provider visible for the package::capability
    /// contract without external I/O or side effects.
    pub fn provider_visible(&self) -> bool {
        self.visibility == CapabilityVisibility::Active && self.projection.is_projected()
    }

    /// Returns whether executable applies for this state.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn executable(&self) -> bool {
        self.executor_ref.is_some()
    }

    /// Computes or returns project for provider for the package::capability
    /// contract without external I/O or side effects.
    pub fn project_for_provider(&self) -> Result<Option<ProviderCapabilityProjection>, AgentError> {
        self.validate()?;
        if !self.provider_visible() {
            return Ok(None);
        }
        Ok(Some(ProviderCapabilityProjection {
            capability_id: self.capability_id.clone(),
            namespace: self.namespace.clone(),
            projection: self.projection.clone(),
            policy_ref: self.policy_ref.clone(),
        }))
    }

    /// Computes or returns executable route for the package::capability
    /// contract without external I/O or side effects.
    pub fn executable_route(&self) -> Result<Option<ExecutableCapabilityRoute>, AgentError> {
        self.validate()?;
        let Some(executor_ref) = self.executor_ref.clone() else {
            return Ok(None);
        };
        Ok(Some(ExecutableCapabilityRoute {
            capability_id: self.capability_id.clone(),
            executor_ref,
            policy_ref: self.policy_ref.clone(),
        }))
    }

    /// Validates the package::capability invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
    pub fn validate(&self) -> Result<(), AgentError> {
        if self.policy_ref.as_str().is_empty() {
            return Err(AgentError::missing_required_field("capability.policy_ref"));
        }
        if self.kind.is_reserved() && self.readiness.is_active() {
            return Err(AgentError::contract_violation(format!(
                "reserved capability {} cannot be active until its owner supplies sidecar, fingerprint, event, journal, and acceptance-test evidence",
                self.capability_id.as_str()
            )));
        }
        if self.kind.is_reserved() && self.projection.is_projected() {
            return Err(AgentError::contract_violation(format!(
                "reserved capability {} cannot be projected while inactive",
                self.capability_id.as_str()
            )));
        }
        if self.kind.is_reserved() && self.executor_ref.is_some() {
            return Err(AgentError::contract_violation(format!(
                "reserved capability {} cannot execute while inactive",
                self.capability_id.as_str()
            )));
        }
        if self.provider_visible() && self.executor_ref.is_none() {
            return Err(AgentError::missing_required_field(
                "provider_visible_capability.executor_ref",
            ));
        }
        if self.provider_visible() && self.policy_ref.kind == PolicyKind::Host {
            return Err(AgentError::contract_violation(
                "provider-visible capability must use an explicit non-host policy ref",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the provider capability projection portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ProviderCapabilityProjection {
    /// Stable capability identifier used for package projection and
    /// executable routing.
    pub capability_id: CapabilityId,
    /// Namespace that groups this capability or identifier.
    /// Use it to avoid collisions between packages, hosts, and extensions.
    pub namespace: CapabilityNamespace,
    /// Projection controls for exposing data to a provider or subscriber.
    /// Use it to keep provider-visible data separate from private SDK state.
    pub projection: ProjectionMode,
    /// Policy reference that must be resolved by the host or runtime before
    /// execution.
    pub policy_ref: PolicyRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the executable capability route portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ExecutableCapabilityRoute {
    /// Stable capability identifier used for package projection and
    /// executable routing.
    pub capability_id: CapabilityId,
    /// Typed executor ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub executor_ref: ExecutorRef,
    /// Policy reference that must be resolved by the host or runtime before
    /// execution.
    pub policy_ref: PolicyRef,
}
