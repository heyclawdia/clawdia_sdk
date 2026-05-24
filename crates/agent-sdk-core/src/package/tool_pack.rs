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

pub const TOOL_PACK_SIDECAR_KIND: &str = "tool_pack";
pub const TOOL_PACK_SIDECAR_VERSION: &str = "v1";

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ToolPackId(String);

impl ToolPackId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("ToolPackId must be valid")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, IdValidationError> {
        let value = value.into();
        validate_identifier(&value)?;
        Ok(Self(value))
    }

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
pub enum ToolPackKind {
    WorkspaceReadOnly,
    WorkspaceSearch,
    WorkspaceEdit,
    WorkspaceWrite,
    Shell,
    ResourceReaders,
    ToolDiscovery,
    External,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolPackSnapshot {
    pub pack_id: ToolPackId,
    pub kind: ToolPackKind,
    pub version: String,
    pub source: SourceRef,
    pub trust: TrustClass,
    #[serde(default)]
    pub tools: Vec<ToolPackToolSnapshot>,
    #[serde(default)]
    pub workspace_bounds: Option<WorkspaceBoundsSnapshot>,
    #[serde(default)]
    pub resource_routes: Vec<ResourceRouteSnapshot>,
    #[serde(default)]
    pub discovery: Option<ToolDiscoverySnapshot>,
}

impl ToolPackSnapshot {
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

    pub fn with_trust(mut self, trust: TrustClass) -> Self {
        self.trust = trust;
        self
    }

    pub fn with_tool(mut self, tool: ToolPackToolSnapshot) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn with_workspace_bounds(mut self, bounds: WorkspaceBoundsSnapshot) -> Self {
        self.workspace_bounds = Some(bounds);
        self
    }

    pub fn with_resource_route(mut self, route: ResourceRouteSnapshot) -> Self {
        self.resource_routes.push(route);
        self
    }

    pub fn with_discovery(mut self, discovery: ToolDiscoverySnapshot) -> Self {
        self.discovery = Some(discovery);
        self
    }

    pub fn sidecar_ref(&self) -> Result<PackageSidecarRef, AgentError> {
        let mut sidecar = PackageSidecarRef::new(
            self.pack_id.as_str(),
            TOOL_PACK_SIDECAR_KIND,
            TOOL_PACK_SIDECAR_VERSION,
        );
        sidecar.content_hash = Some(self.content_hash()?);
        Ok(sidecar)
    }

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
                    privacy: tool.privacy.clone(),
                    readiness: active_tool_pack_readiness(),
                })
            })
            .collect()
    }

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
pub struct ToolPackToolSnapshot {
    pub capability_id: CapabilityId,
    pub canonical_tool_name: CanonicalToolName,
    pub namespace: CapabilityNamespace,
    pub schema_ref: PackageSidecarRef,
    pub executor_ref: ExecutorRef,
    pub policy_refs: Vec<PolicyRef>,
    pub required_permissions: Vec<CapabilityPermission>,
    pub effect_class: EffectClass,
    pub risk_class: RiskClass,
    pub redaction_policy_ref: PolicyRef,
    pub timeout_ms: u64,
    pub cancellation: String,
    pub reconciliation: String,
    pub privacy: PrivacyClass,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceBoundsSnapshot {
    pub workspace_id: String,
    pub root_policy_ref: PolicyRef,
    pub max_file_bytes: u64,
    pub max_output_bytes: u64,
    pub max_matches: usize,
    pub follow_symlinks: bool,
    pub include_hidden: bool,
    pub anchor_validation: AnchorValidationRequirement,
    pub preview_apply: PreviewApplyRequirement,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnchorValidationRequirement {
    NotApplicable,
    HashLineRequired,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewApplyRequirement {
    NotApplicable,
    PreviewOnly,
    ApplyRequiresPreviewAndApproval,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResourceRouteSnapshot {
    pub scheme: String,
    pub source: SourceRef,
    pub permission_policy_ref: PolicyRef,
    pub parser_version: String,
    pub max_bytes: u64,
    pub privacy: PrivacyClass,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolDiscoverySnapshot {
    pub discovery_index_id: String,
    pub activation_policy_ref: PolicyRef,
    pub package_delta_required: bool,
}

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
