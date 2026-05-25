//! Runtime-package records and builders. Use these items to describe the immutable
//! per-run package that freezes provider route, capabilities, policies, sidecars,
//! catalogs, and fingerprints. Builders are data-only and must not perform discovery
//! or execution side effects. This file contains the tool pack portion of that
//! contract.
//!
use core::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    capability::{
        CapabilityId, CapabilityKind, CapabilityNamespace, CapabilityReadiness, CapabilitySource,
        CapabilitySourceKind, CapabilitySpec, CapabilityVersion, CapabilityVisibility, ExecutorRef,
        PackageSidecarRef, ProjectionMode,
    },
    domain::{AgentError, IdValidationError, PolicyRef, PrivacyClass, SourceRef, TrustClass},
    ids::validate_identifier,
    package::PackageSidecarSnapshot,
    policy::{CapabilityPermission, EffectClass, RiskClass},
    tool_records::CanonicalToolName,
};

/// Constant value for the package::tool_pack contract. Use it to keep
/// SDK records and tests aligned on the same stable value.
pub const TOOL_PACK_SIDECAR_KIND: &str = "tool_pack";
/// Constant value for the package::tool_pack contract. Use it to keep
/// SDK records and tests aligned on the same stable value.
pub const TOOL_PACK_SIDECAR_VERSION: &str = "v1";

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Describes the tool pack id portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ToolPackId(String);

impl ToolPackId {
    /// Creates a new package::tool_pack value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    ///
    /// # Panics
    ///
    /// Panics if constructor invariants fail, such as invalid identifier
    /// text or constructor-specific bounds. Use a fallible constructor such as
    /// `try_new` when one is available for untrusted input.
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("ToolPackId must be valid")
    }

    /// Creates a new package::tool_pack value after validation. Returns
    /// an SDK error instead of panicking when the identifier or input
    /// does not satisfy the contract.
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

impl<'de> Deserialize<'de> for ToolPackId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(serde::de::Error::custom)
    }
}

impl fmt::Debug for ToolPackId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ToolPackId(redacted)")
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite tool pack kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ToolPackKind {
    /// Use this variant when the contract needs to represent workspace read only; selecting it has no side effect by itself.
    WorkspaceReadOnly,
    /// Use this variant when the contract needs to represent workspace search; selecting it has no side effect by itself.
    WorkspaceSearch,
    /// Use this variant when the contract needs to represent workspace edit; selecting it has no side effect by itself.
    WorkspaceEdit,
    /// Use this variant when the contract needs to represent workspace write; selecting it has no side effect by itself.
    WorkspaceWrite,
    /// Use this variant when the contract needs to represent shell; selecting it has no side effect by itself.
    Shell,
    /// Use this variant when the contract needs to represent resource readers; selecting it has no side effect by itself.
    ResourceReaders,
    /// Use this variant when the contract needs to represent tool discovery; selecting it has no side effect by itself.
    ToolDiscovery,
    /// Use this variant when the contract needs to represent external; selecting it has no side effect by itself.
    External,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the tool pack snapshot portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ToolPackSnapshot {
    /// Stable pack id used for typed lineage, lookup, or dedupe.
    pub pack_id: ToolPackId,
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: ToolPackKind,
    /// Version string for this capability, package, or protocol surface.
    /// Use it for compatibility checks during package or adapter resolution.
    pub version: String,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Trust class used when deciding whether context or capabilities may be
    /// admitted.
    pub trust: TrustClass,
    #[serde(default)]
    /// Collection of tools values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub tools: Vec<ToolPackToolSnapshot>,
    #[serde(default)]
    /// Optional workspace bounds value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub workspace_bounds: Option<WorkspaceBoundsSnapshot>,
    #[serde(default)]
    /// Collection of resource routes values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub resource_routes: Vec<ResourceRouteSnapshot>,
    #[serde(default)]
    /// Optional discovery value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub discovery: Option<ToolDiscoverySnapshot>,
}

impl ToolPackSnapshot {
    /// Creates a new package::tool_pack value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(
        pack_id: ToolPackId,
        kind: ToolPackKind,
        version: impl Into<String>,
        source: SourceRef,
    ) -> Self {
        Self {
            pack_id,
            kind,
            version: version.into(),
            source,
            trust: TrustClass::SdkGenerated,
            tools: Vec::new(),
            workspace_bounds: None,
            resource_routes: Vec::new(),
            discovery: None,
        }
    }

    /// Returns this value with its trust setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_trust(mut self, trust: TrustClass) -> Self {
        self.trust = trust;
        self
    }

    /// Returns this value with its tool setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_tool(mut self, tool: ToolPackToolSnapshot) -> Self {
        self.tools.push(tool);
        self
    }

    /// Returns this value with its workspace bounds setting replaced.
    /// The method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_workspace_bounds(mut self, bounds: WorkspaceBoundsSnapshot) -> Self {
        self.workspace_bounds = Some(bounds);
        self
    }

    /// Returns this value with its resource route setting replaced. The
    /// method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_resource_route(mut self, route: ResourceRouteSnapshot) -> Self {
        self.resource_routes.push(route);
        self
    }

    /// Returns this value with its discovery setting replaced. The
    /// method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_discovery(mut self, discovery: ToolDiscoverySnapshot) -> Self {
        self.discovery = Some(discovery);
        self
    }

    /// Builds the sidecar ref value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn sidecar_ref(&self) -> Result<PackageSidecarRef, AgentError> {
        let mut sidecar = PackageSidecarRef::new(
            self.pack_id.as_str(),
            TOOL_PACK_SIDECAR_KIND,
            TOOL_PACK_SIDECAR_VERSION,
        );
        sidecar.content_hash = Some(self.content_hash()?);
        Ok(sidecar)
    }

    /// Builds the package sidecar snapshot value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn package_sidecar_snapshot(&self) -> Result<PackageSidecarSnapshot, AgentError> {
        let refs = self
            .tools
            .iter()
            .map(|tool| tool.schema_ref.clone())
            .collect::<Vec<_>>();
        let mut policy_refs = self
            .tools
            .iter()
            .flat_map(|tool| {
                tool.policy_refs
                    .iter()
                    .chain(core::iter::once(&tool.redaction_policy_ref))
                    .cloned()
            })
            .collect::<Vec<_>>();
        policy_refs.extend(
            self.resource_routes
                .iter()
                .map(|route| route.permission_policy_ref.clone()),
        );
        if let Some(discovery) = &self.discovery {
            policy_refs.push(discovery.activation_policy_ref.clone());
        }
        Ok(PackageSidecarSnapshot {
            sidecar_id: self.pack_id.as_str().to_string(),
            kind: TOOL_PACK_SIDECAR_KIND.to_string(),
            version: self.version.clone(),
            refs,
            policy_refs,
            content_hash: self.content_hash()?,
        })
    }

    /// Builds the capability specs value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn capability_specs(&self) -> Result<Vec<CapabilitySpec>, AgentError> {
        let sidecar_ref = self.sidecar_ref()?;
        self.tools
            .iter()
            .map(|tool| {
                let policy_ref =
                    tool.policy_refs.first().cloned().ok_or_else(|| {
                        AgentError::missing_required_field("tool_pack.policy_ref")
                    })?;
                Ok(CapabilitySpec {
                    capability_id: tool.capability_id.clone(),
                    kind: CapabilityKind::Tool,
                    source: CapabilitySource {
                        kind: CapabilitySourceKind::ToolPack,
                        source_ref: self.source.clone(),
                        adapter_ref: None,
                    },
                    namespace: tool.namespace.clone(),
                    version: CapabilityVersion::new(self.version.clone()),
                    visibility: CapabilityVisibility::Active,
                    projection: ProjectionMode::ProviderToolSchema {
                        schema_ref: tool.schema_ref.clone(),
                    },
                    executor_ref: Some(tool.executor_ref.clone()),
                    policy_ref,
                    sidecar_refs: vec![sidecar_ref.clone()],
                    isolation_ref: None,
                    privacy: tool.privacy,
                    readiness: active_tool_pack_readiness(),
                })
            })
            .collect()
    }

    /// Computes the stable content hash for this package::tool_pack
    /// value. The computation is deterministic and side-effect free so
    /// it can be used in package, journal, or test evidence.
    pub fn content_hash(&self) -> Result<String, AgentError> {
        let bytes = serde_json::to_vec(&crate::domain::json::normalize_json_value(
            serde_json::to_value(self).map_err(|error| {
                AgentError::contract_violation(format!(
                    "tool pack sidecar serialization failed: {error}"
                ))
            })?,
        ))
        .map_err(|error| {
            AgentError::contract_violation(format!(
                "tool pack sidecar hash serialization failed: {error}"
            ))
        })?;
        Ok(format!("sha256:{}", hex_lower(&Sha256::digest(bytes))))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the tool pack tool snapshot portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ToolPackToolSnapshot {
    /// Stable capability identifier used for package projection and
    /// executable routing.
    pub capability_id: CapabilityId,
    /// Canonical tool name used by this record or request.
    pub canonical_tool_name: CanonicalToolName,
    /// Namespace that groups this capability or identifier.
    /// Use it to avoid collisions between packages, hosts, and extensions.
    pub namespace: CapabilityNamespace,
    /// Typed schema ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub schema_ref: PackageSidecarRef,
    /// Typed executor ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub executor_ref: ExecutorRef,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Collection of required permissions values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub required_permissions: Vec<CapabilityPermission>,
    /// Classification value for effect class.
    /// Policy and projection paths use it for finite routing decisions.
    pub effect_class: EffectClass,
    /// Risk classification for the operation or capability.
    /// Policy uses it to decide whether approval, sandboxing, or denial is required.
    pub risk_class: RiskClass,
    /// Typed redaction policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub redaction_policy_ref: PolicyRef,
    /// Timeout budget in milliseconds for the requested operation.
    pub timeout_ms: u64,
    /// Cancellation used by this record or request.
    pub cancellation: String,
    /// Reconciliation used by this record or request.
    pub reconciliation: String,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the workspace bounds snapshot portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct WorkspaceBoundsSnapshot {
    /// Stable workspace id used for typed lineage, lookup, or dedupe.
    pub workspace_id: String,
    /// Typed root policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub root_policy_ref: PolicyRef,
    /// max file bytes used for bounds checks, summaries, or truncation
    /// evidence.
    pub max_file_bytes: u64,
    /// max output bytes used for bounds checks, summaries, or truncation
    /// evidence.
    pub max_output_bytes: u64,
    /// Maximum number of matches to return.
    /// Use it to keep search output bounded for model context.
    pub max_matches: usize,
    /// Whether follow symlinks is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub follow_symlinks: bool,
    /// Whether include hidden is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub include_hidden: bool,
    /// Validation policy applied before output is accepted as typed data.
    /// It controls validator selection, bounds, failure visibility, and local validation
    /// behavior.
    pub anchor_validation: AnchorValidationRequirement,
    /// Preview apply used by this record or request.
    pub preview_apply: PreviewApplyRequirement,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite anchor validation requirement cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum AnchorValidationRequirement {
    /// Use this variant when the contract needs to represent not applicable; selecting it has no side effect by itself.
    NotApplicable,
    /// Use this variant when the contract needs to represent hash line required; selecting it has no side effect by itself.
    HashLineRequired,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite preview apply requirement cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum PreviewApplyRequirement {
    /// Use this variant when the contract needs to represent not applicable; selecting it has no side effect by itself.
    NotApplicable,
    /// Use this variant when the contract needs to represent preview only; selecting it has no side effect by itself.
    PreviewOnly,
    /// Use this variant when the contract needs to represent apply requires preview and approval; selecting it has no side effect by itself.
    ApplyRequiresPreviewAndApproval,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the resource route snapshot portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ResourceRouteSnapshot {
    /// URI scheme resolved by the resource reader.
    pub scheme: String,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Typed permission policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub permission_policy_ref: PolicyRef,
    /// Version string for this capability, package, or protocol surface.
    /// Use it for compatibility checks during package or adapter resolution.
    pub parser_version: String,
    /// Maximum byte budget the caller requested before truncation or summary
    /// behavior is applied.
    pub max_bytes: u64,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the tool discovery snapshot portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ToolDiscoverySnapshot {
    /// Stable discovery index id used for typed lineage, lookup, or dedupe.
    pub discovery_index_id: String,
    /// Typed activation policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub activation_policy_ref: PolicyRef,
    /// Whether activation must be applied as a package delta before use.
    pub package_delta_required: bool,
}

/// Builds the active tool pack readiness value with the documented defaults.
/// This is data-only and does not perform I/O, call host ports, append journals, publish
/// events, or start processes.
pub fn active_tool_pack_readiness() -> CapabilityReadiness {
    CapabilityReadiness {
        status: crate::capability::CapabilityReadinessStatus::Active,
        owner_role: "04-tools-approval-toolpacks".to_string(),
        typed_sidecar_contract: Some(
            "tool-pack-contract.md#runtimepackage-lowering-and-snapshot".to_string(),
        ),
        fingerprint_fields: vec![
            "tool_pack_id".to_string(),
            "version".to_string(),
            "source".to_string(),
            "trust".to_string(),
            "tool_specs".to_string(),
            "executor_ref".to_string(),
            "policy_refs".to_string(),
            "redaction_policy_ref".to_string(),
            "workspace_bounds".to_string(),
            "resource_routes".to_string(),
            "discovery_activation_policy".to_string(),
        ],
        emitted_events: vec![
            "tool_requested".to_string(),
            "tool_started".to_string(),
            "tool_completed".to_string(),
            "tool_failed".to_string(),
            "package_delta_requested".to_string(),
        ],
        journal_records: vec![
            "tool_record.requested".to_string(),
            "tool_record.intent".to_string(),
            "tool_record.result".to_string(),
            "package_delta".to_string(),
        ],
        acceptance_tests: vec![
            "tool_pack_snapshot_includes_capability_sidecar_policy_and_executor_refs".to_string(),
            "tool_pack_fingerprint_changes_when_executor_policy_or_sidecar_changes".to_string(),
            "agent_sdk_core_builds_without_toolkit_features".to_string(),
        ],
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}
