//! Runtime-package records and builders. Use these items to describe the immutable
//! per-run package that freezes provider route, capabilities, policies, sidecars,
//! catalogs, and fingerprints. Builders are data-only and must not perform discovery
//! or execution side effects. This file contains the extension portion of that
//! contract.
//!
use core::fmt;
use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize, de::Error as DeError};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    capability::{
        CapabilityId, CapabilityKind, CapabilityNamespace, CapabilityReadiness, CapabilitySource,
        CapabilitySourceKind, CapabilityVersion, CapabilityVisibility, PackageSidecarRef,
        ProjectionMode,
    },
    domain::{
        AgentError, DestinationRef, IdValidationError, PolicyRef, PrivacyClass, SourceRef,
        TrustClass,
    },
    ids::validate_identifier,
    package::{CapabilityCatalogSnapshot, PackageSidecarSnapshot},
    policy::RiskClass,
};

/// Constant value for the package::extension contract. Use it to keep
/// SDK records and tests aligned on the same stable value.
pub const EXTENSION_ACTION_SIDECAR_KIND: &str = "extension_action";
/// Constant value for the package::extension contract. Use it to keep
/// SDK records and tests aligned on the same stable value.
pub const EXTENSION_ACTION_SIDECAR_VERSION: &str = "v1";

macro_rules! extension_id {
    ($name:ident, $debug:literal) => {
        #[doc = concat!(
                    "Typed extension identifier for `",
                    stringify!($name),
                    "`. Use it to refer to extension capabilities, actions, and bridge resources ",
                    "without granting extensions ambient authority; constructing it is data-only."
                )]
        #[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Creates a new package::extension value with explicit
            /// caller-provided inputs. This constructor is data-only
            /// and performs no I/O or external side effects.
            ///
            /// # Panics
            ///
            /// Panics if constructor invariants fail, such as invalid identifier
            /// text or constructor-specific bounds. Use a fallible constructor such as
            /// `try_new` when one is available for untrusted input.
            pub fn new(value: impl Into<String>) -> Self {
                Self::try_new(value).expect(concat!($debug, " must be valid"))
            }

            /// Creates a new package::extension value after validation.
            /// Returns an SDK error instead of panicking when the
            /// identifier or input does not satisfy the contract.
            pub fn try_new(value: impl Into<String>) -> Result<Self, IdValidationError> {
                let value = value.into();
                validate_identifier(&value)?;
                Ok(Self(value))
            }

            /// Returns this value as str. The accessor is side-effect
            /// free and keeps ownership with the caller.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::try_new(value).map_err(D::Error::custom)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!($debug, "(redacted)"))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!($debug, "(redacted)"))
            }
        }
    };
}

extension_id!(ExtensionId, "ExtensionId");
extension_id!(ExtensionVersion, "ExtensionVersion");
extension_id!(ExtensionActionId, "ExtensionActionId");
extension_id!(ExtensionBridgeRef, "ExtensionBridgeRef");
extension_id!(ExtensionActionRequestId, "ExtensionActionRequestId");

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the core extension capabilities portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct CoreExtensionCapabilities {
    /// Stable extension id used for typed lineage, lookup, or dedupe.
    pub extension_id: ExtensionId,
    /// Version string for this capability, package, or protocol surface.
    /// Use it for compatibility checks during package or adapter resolution.
    pub version: ExtensionVersion,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of tools values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub tools: Vec<ExtensionToolCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of hooks values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub hooks: Vec<ExtensionHookCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of providers values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub providers: Vec<ExtensionProviderCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of subagents values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub subagents: Vec<ExtensionSubagentCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of actions values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub actions: Vec<ExtensionActionCapability>,
}

impl CoreExtensionCapabilities {
    /// Starts a builder for this package::extension value. Building is
    /// data-only; runtime side effects occur only when a later
    /// coordinator or host port executes the built configuration.
    pub fn builder(extension_id: ExtensionId) -> CoreExtensionCapabilitiesBuilder {
        CoreExtensionCapabilitiesBuilder {
            capabilities: Self {
                extension_id,
                version: ExtensionVersion::new("0.0.0"),
                tools: Vec::new(),
                hooks: Vec::new(),
                providers: Vec::new(),
                subagents: Vec::new(),
                actions: Vec::new(),
            },
        }
    }

    /// Validates the package::extension invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
    pub fn validate(&self) -> Result<(), AgentError> {
        if self.actions.is_empty()
            && self.tools.is_empty()
            && self.hooks.is_empty()
            && self.providers.is_empty()
            && self.subagents.is_empty()
        {
            return Err(AgentError::missing_required_field(
                "core_extension_capabilities.capabilities",
            ));
        }
        for action in &self.actions {
            action.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
/// Describes the core extension capabilities builder portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct CoreExtensionCapabilitiesBuilder {
    capabilities: CoreExtensionCapabilities,
}

impl CoreExtensionCapabilitiesBuilder {
    /// Returns an updated package::extension value with version applied. This
    /// is data construction only and does not execute the configured
    /// behavior.
    pub fn version(mut self, version: ExtensionVersion) -> Self {
        self.capabilities.version = version;
        self
    }

    /// Returns an updated package::extension value with tool applied. This is
    /// data construction only and does not execute the configured behavior.
    pub fn tool(mut self, name: impl Into<String>) -> Self {
        self.capabilities
            .tools
            .push(ExtensionToolCapability::new(name));
        self
    }

    /// Returns an updated package::extension value with action applied. This
    /// is data construction only and does not execute the configured
    /// behavior.
    pub fn action(
        mut self,
        action_id: ExtensionActionId,
        action_kind: ExtensionActionKind,
        requested_destination: DestinationRef,
    ) -> Self {
        self.capabilities
            .actions
            .push(ExtensionActionCapability::new(
                action_id,
                action_kind,
                requested_destination,
            ));
        self
    }

    /// Finishes builder validation and returns the configured value.
    /// This is data-only unless the surrounding builder explicitly
    /// documents adapter or store access.
    pub fn build(self) -> Result<CoreExtensionCapabilities, AgentError> {
        self.capabilities.validate()?;
        Ok(self.capabilities)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the extension tool capability portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ExtensionToolCapability {
    /// Human-readable or protocol-visible name for this SDK item.
    pub name: String,
}

impl ExtensionToolCapability {
    /// Creates a new package::extension value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the extension hook capability portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ExtensionHookCapability {
    /// Stable hook id used for typed lineage, lookup, or dedupe.
    pub hook_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the extension provider capability portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ExtensionProviderCapability {
    /// Stable provider id used for typed lineage, lookup, or dedupe.
    pub provider_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the extension subagent capability portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ExtensionSubagentCapability {
    /// Stable subagent id used for typed lineage, lookup, or dedupe.
    pub subagent_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the extension action capability portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ExtensionActionCapability {
    /// Stable action id used for typed lineage, lookup, or dedupe.
    pub action_id: ExtensionActionId,
    /// Kind discriminator for action kind.
    /// Use it to route finite match arms without parsing display text.
    pub action_kind: ExtensionActionKind,
    /// Requested destination used by this record or request.
    pub requested_destination: DestinationRef,
    /// Risk classification for the operation or capability.
    /// Policy uses it to decide whether approval, sandboxing, or denial is required.
    pub risk_class: RiskClass,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed input schema ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub input_schema_ref: Option<PackageSidecarRef>,
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency: ExtensionActionIdempotency,
}

impl ExtensionActionCapability {
    /// Creates a new package::extension value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(
        action_id: ExtensionActionId,
        action_kind: ExtensionActionKind,
        requested_destination: DestinationRef,
    ) -> Self {
        Self {
            action_id,
            action_kind,
            requested_destination,
            risk_class: RiskClass::Medium,
            input_schema_ref: None,
            idempotency: ExtensionActionIdempotency::IdempotentWhenKeyed,
        }
    }

    /// Validates the package::extension invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
    pub fn validate(&self) -> Result<(), AgentError> {
        if self.action_id.as_str().is_empty() {
            return Err(AgentError::missing_required_field(
                "extension_action.action_id",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite extension action kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ExtensionActionKind {
    /// Use this variant when the contract needs to represent host action; selecting it has no side effect by itself.
    HostAction,
    /// Use this variant when the contract needs to represent output delivery; selecting it has no side effect by itself.
    OutputDelivery,
    /// Use this variant when the contract needs to represent tool request; selecting it has no side effect by itself.
    ToolRequest,
}

impl ExtensionActionKind {
    /// Returns effect class for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn effect_class(&self) -> crate::policy::EffectClass {
        match self {
            Self::HostAction | Self::OutputDelivery => crate::policy::EffectClass::Write,
            Self::ToolRequest => crate::policy::EffectClass::Network,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite extension action idempotency cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ExtensionActionIdempotency {
    /// Use this variant when the contract needs to represent idempotent when keyed; selecting it has no side effect by itself.
    IdempotentWhenKeyed,
    /// Use this variant when the contract needs to represent host dedupe required; selecting it has no side effect by itself.
    HostDedupeRequired,
    /// Use this variant when the contract needs to represent non idempotent; selecting it has no side effect by itself.
    NonIdempotent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the extension package resolution portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ExtensionPackageResolution {
    /// Typed source reference that records where this item originated.
    pub source_ref: SourceRef,
    /// Stable catalog id used for typed lineage, lookup, or dedupe.
    pub catalog_id: String,
    /// Typed activation policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub activation_policy_ref: PolicyRef,
    /// Typed bridge ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub bridge_ref: ExtensionBridgeRef,
    /// Typed action policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub action_policy_ref: PolicyRef,
    /// Typed approval policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub approval_policy_ref: PolicyRef,
    /// Stable redaction policy id used for typed lineage, lookup, or dedupe.
    pub redaction_policy_id: String,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the resolved extension package portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ResolvedExtensionPackage {
    /// Stable extension id used for typed lineage, lookup, or dedupe.
    pub extension_id: ExtensionId,
    /// Version string for this capability, package, or protocol surface.
    /// Use it for compatibility checks during package or adapter resolution.
    pub version: ExtensionVersion,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
    /// Catalog snapshot used by this record or request.
    pub catalog_snapshot: CapabilityCatalogSnapshot,
    /// Collection of action capabilities values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub action_capabilities: Vec<ExtensionPackageCapability>,
    /// Collection of action sidecars values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub action_sidecars: Vec<ResolvedExtensionActionSidecar>,
    /// Collection of package sidecars values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub package_sidecars: Vec<PackageSidecarSnapshot>,
}

impl ResolvedExtensionPackage {
    /// Constructs this value from core capabilities. Use it when
    /// adapting canonical SDK records without introducing a second
    /// behavior path.
    pub fn from_core_capabilities(
        capabilities: CoreExtensionCapabilities,
        resolution: ExtensionPackageResolution,
    ) -> Result<Self, AgentError> {
        capabilities.validate()?;
        let mut action_capabilities = Vec::new();
        let mut action_sidecars = Vec::new();
        let mut package_sidecars = Vec::new();

        for action in &capabilities.actions {
            let action_ref =
                ExtensionActionRef::from_capability(&capabilities.extension_id, action);
            let sidecar = ResolvedExtensionActionSidecar {
                sidecar_id: format!(
                    "sidecar.{}.action.{}",
                    capabilities.extension_id.as_str(),
                    action.action_id.as_str()
                ),
                action_ref: action_ref.clone(),
                action_kind: action.action_kind.clone(),
                source_ref: resolution.source_ref.clone(),
                destination: action.requested_destination.clone(),
                bridge_ref: resolution.bridge_ref.clone(),
                policy_refs: vec![resolution.action_policy_ref.clone()],
                approval_policy_ref: resolution.approval_policy_ref.clone(),
                redaction_policy_id: resolution.redaction_policy_id.clone(),
                risk_class: action.risk_class.clone(),
                idempotency: action.idempotency.clone(),
                input_schema_ref: action.input_schema_ref.clone(),
                requires_approval: true,
            };
            sidecar.validate()?;
            package_sidecars.push(sidecar.package_sidecar_snapshot()?);
            action_capabilities.push(ExtensionPackageCapability::from_sidecar(&sidecar));
            action_sidecars.push(sidecar);
        }

        let candidates = action_capabilities
            .iter()
            .map(|capability| capability.capability_id.clone())
            .collect::<Vec<_>>();
        let catalog_snapshot = CapabilityCatalogSnapshot {
            catalog_id: resolution.catalog_id,
            source_kind: CapabilitySourceKind::Extension,
            source_ref: resolution.source_ref,
            version: Some(capabilities.version.as_str().to_string()),
            content_hash: Some(hash_json(&capabilities)?),
            trust_state: TrustClass::HostProvided,
            activation_policy_ref: resolution.activation_policy_ref,
            candidates,
        };

        Ok(Self {
            extension_id: capabilities.extension_id,
            version: capabilities.version,
            runtime_package_fingerprint: resolution.runtime_package_fingerprint,
            catalog_snapshot,
            action_capabilities,
            action_sidecars,
            package_sidecars,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the extension package capability portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ExtensionPackageCapability {
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
    /// Policy reference that must be resolved by the host or runtime before
    /// execution.
    pub policy_ref: PolicyRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// References to typed package sidecars needed by this capability.
    pub sidecar_refs: Vec<PackageSidecarRef>,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Readiness state for a capability or package feature.
    /// Launch and package validation use it to distinguish active, reserved, and blocked
    /// surfaces.
    pub readiness: CapabilityReadiness,
}

impl ExtensionPackageCapability {
    fn from_sidecar(sidecar: &ResolvedExtensionActionSidecar) -> Self {
        Self {
            capability_id: sidecar.action_ref.capability_id.clone(),
            kind: CapabilityKind::ExtensionAction,
            source: CapabilitySource {
                kind: CapabilitySourceKind::Extension,
                source_ref: sidecar.source_ref.clone(),
                adapter_ref: None,
            },
            namespace: CapabilityNamespace::new(format!(
                "extension.{}",
                sidecar.action_ref.extension_id.as_str()
            )),
            version: CapabilityVersion::new(EXTENSION_ACTION_SIDECAR_VERSION),
            visibility: CapabilityVisibility::Active,
            projection: ProjectionMode::NotProjected,
            policy_ref: sidecar.policy_refs[0].clone(),
            sidecar_refs: vec![sidecar.package_sidecar_ref()],
            privacy: PrivacyClass::ContentRefsOnly,
            readiness: extension_action_readiness(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the extension action ref portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ExtensionActionRef {
    /// Stable extension id used for typed lineage, lookup, or dedupe.
    pub extension_id: ExtensionId,
    /// Stable action id used for typed lineage, lookup, or dedupe.
    pub action_id: ExtensionActionId,
    /// Stable capability identifier used for package projection and
    /// executable routing.
    pub capability_id: CapabilityId,
}

impl ExtensionActionRef {
    /// Constructs this value from capability. Use it when adapting
    /// canonical SDK records without introducing a second behavior
    /// path.
    pub fn from_capability(
        extension_id: &ExtensionId,
        capability: &ExtensionActionCapability,
    ) -> Self {
        Self {
            extension_id: extension_id.clone(),
            action_id: capability.action_id.clone(),
            capability_id: CapabilityId::new(format!(
                "cap.{}.{}",
                extension_id.as_str(),
                capability.action_id.as_str()
            )),
        }
    }

    /// Returns subject id for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn subject_id(&self) -> String {
        format!("{}.{}", self.extension_id.as_str(), self.action_id.as_str())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the resolved extension action sidecar portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ResolvedExtensionActionSidecar {
    /// Identifier for the typed package sidecar.
    pub sidecar_id: String,
    /// Typed action ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub action_ref: ExtensionActionRef,
    /// Kind discriminator for action kind.
    /// Use it to route finite match arms without parsing display text.
    pub action_kind: ExtensionActionKind,
    /// Typed source reference that records where this item originated.
    pub source_ref: SourceRef,
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: DestinationRef,
    /// Typed bridge ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub bridge_ref: ExtensionBridgeRef,
    #[serde(default)]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Typed approval policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub approval_policy_ref: PolicyRef,
    /// Stable redaction policy id used for typed lineage, lookup, or dedupe.
    pub redaction_policy_id: String,
    /// Risk classification for the operation or capability.
    /// Policy uses it to decide whether approval, sandboxing, or denial is required.
    pub risk_class: RiskClass,
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency: ExtensionActionIdempotency,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed input schema ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub input_schema_ref: Option<PackageSidecarRef>,
    /// Boolean policy/capability flag for whether requires approval is
    /// enabled.
    pub requires_approval: bool,
}

impl ResolvedExtensionActionSidecar {
    /// Validates the package::extension invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
    pub fn validate(&self) -> Result<(), AgentError> {
        if self.policy_refs.is_empty() {
            return Err(AgentError::missing_required_field(
                "extension_action_sidecar.policy_refs",
            ));
        }
        if self.redaction_policy_id.is_empty() {
            return Err(AgentError::missing_required_field(
                "extension_action_sidecar.redaction_policy_id",
            ));
        }
        Ok(())
    }

    /// Builds the package sidecar ref value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn package_sidecar_ref(&self) -> PackageSidecarRef {
        let mut sidecar_ref = PackageSidecarRef::new(
            self.sidecar_id.clone(),
            EXTENSION_ACTION_SIDECAR_KIND,
            EXTENSION_ACTION_SIDECAR_VERSION,
        );
        sidecar_ref.content_hash = Some(hash_json(self).expect("extension sidecar hashes"));
        sidecar_ref
    }

    /// Builds the package sidecar snapshot value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn package_sidecar_snapshot(&self) -> Result<PackageSidecarSnapshot, AgentError> {
        self.validate()?;
        let mut policy_refs = self.policy_refs.clone();
        policy_refs.push(self.approval_policy_ref.clone());
        policy_refs.sort_by_key(|policy| policy.as_str().to_string());
        policy_refs.dedup_by(|left, right| left.as_str() == right.as_str());
        Ok(PackageSidecarSnapshot {
            sidecar_id: self.sidecar_id.clone(),
            kind: EXTENSION_ACTION_SIDECAR_KIND.to_string(),
            version: EXTENSION_ACTION_SIDECAR_VERSION.to_string(),
            refs: self
                .input_schema_ref
                .clone()
                .into_iter()
                .chain(std::iter::once(self.package_sidecar_ref()))
                .collect(),
            policy_refs,
            content_hash: hash_json(self)?,
            redacted_payload: None,
        })
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the extension manifest audit portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ExtensionManifestAudit {
    /// Forbidden fields or values found during audit.
    /// Use them as blockers before accepting package or manifest data.
    pub forbidden_fields: Vec<String>,
}

impl ExtensionManifestAudit {
    /// Returns has forbidden field for this package::extension value without
    /// performing external I/O.
    pub fn has_forbidden_field(&self, field: &str) -> bool {
        self.forbidden_fields
            .iter()
            .any(|candidate| candidate == field)
    }
}

/// Builds the audit core extension capabilities value.
/// This is data construction and performs no I/O, journal append, event publication, or process
pub fn audit_core_extension_capabilities(value: &Value) -> ExtensionManifestAudit {
    let forbidden = BTreeSet::from([
        "runtime",
        "app_event_subscriptions",
        "commands",
        "ui_surfaces",
        "action_permissions",
        "browser_safe_exports",
        "package_compatibility",
        "trust_state",
        "install_metadata",
        "marketplace",
        "transport",
        "process_runtime",
    ]);
    let mut found = BTreeSet::new();
    collect_forbidden_fields(value, &forbidden, &mut found);
    ExtensionManifestAudit {
        forbidden_fields: found.into_iter().collect(),
    }
}

fn collect_forbidden_fields(
    value: &Value,
    forbidden: &BTreeSet<&str>,
    found: &mut BTreeSet<String>,
) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                if forbidden.contains(key.as_str()) {
                    found.insert(key.clone());
                }
                collect_forbidden_fields(value, forbidden, found);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_forbidden_fields(item, forbidden, found);
            }
        }
        _ => {}
    }
}

fn extension_action_readiness() -> CapabilityReadiness {
    CapabilityReadiness {
        status: crate::capability::CapabilityReadinessStatus::Active,
        owner_role: "08-extension-sdk-packaging".to_string(),
        typed_sidecar_contract: Some("extension_action_sidecar".to_string()),
        fingerprint_fields: vec![
            "extension_id".to_string(),
            "extension_version".to_string(),
            "action_id".to_string(),
            "bridge_ref".to_string(),
            "policy_refs".to_string(),
            "source_ref".to_string(),
            "catalog_snapshot_ref".to_string(),
            "package_sidecar_ref".to_string(),
        ],
        emitted_events: vec![
            "ExtensionActionSubmitted".to_string(),
            "ExtensionActionStarted".to_string(),
            "ExtensionActionCompleted".to_string(),
            "ExtensionActionFailed".to_string(),
            "ExtensionActionDenied".to_string(),
        ],
        journal_records: vec![
            "package_catalog_snapshot".to_string(),
            "extension_action_intent".to_string(),
            "extension_action_result".to_string(),
            "approval_record".to_string(),
        ],
        acceptance_tests: vec![
            "extension_action_records_effect_intent_before_host_action".to_string(),
            "extension_cannot_self_approve".to_string(),
            "host_extension_manifest_never_enters_agent_sdk_core_as_authority".to_string(),
        ],
    }
}

fn hash_json(value: &impl Serialize) -> Result<String, AgentError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| AgentError::contract_violation(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}
