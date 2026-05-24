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

pub const EXTENSION_ACTION_SIDECAR_KIND: &str = "extension_action";
pub const EXTENSION_ACTION_SIDECAR_VERSION: &str = "v1";

macro_rules! extension_id {
    ($name:ident, $debug:literal) => {
        #[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self::try_new(value).expect(concat!($debug, " must be valid"))
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
pub struct CoreExtensionCapabilities {
    pub extension_id: ExtensionId,
    pub version: ExtensionVersion,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ExtensionToolCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hooks: Vec<ExtensionHookCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub providers: Vec<ExtensionProviderCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subagents: Vec<ExtensionSubagentCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ExtensionActionCapability>,
}

impl CoreExtensionCapabilities {
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
pub struct CoreExtensionCapabilitiesBuilder {
    capabilities: CoreExtensionCapabilities,
}

impl CoreExtensionCapabilitiesBuilder {
    pub fn version(mut self, version: ExtensionVersion) -> Self {
        self.capabilities.version = version;
        self
    }

    pub fn tool(mut self, name: impl Into<String>) -> Self {
        self.capabilities
            .tools
            .push(ExtensionToolCapability::new(name));
        self
    }

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

    pub fn build(self) -> Result<CoreExtensionCapabilities, AgentError> {
        self.capabilities.validate()?;
        Ok(self.capabilities)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExtensionToolCapability {
    pub name: String,
}

impl ExtensionToolCapability {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExtensionHookCapability {
    pub hook_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExtensionProviderCapability {
    pub provider_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExtensionSubagentCapability {
    pub subagent_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExtensionActionCapability {
    pub action_id: ExtensionActionId,
    pub action_kind: ExtensionActionKind,
    pub requested_destination: DestinationRef,
    pub risk_class: RiskClass,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema_ref: Option<PackageSidecarRef>,
    pub idempotency: ExtensionActionIdempotency,
}

impl ExtensionActionCapability {
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
pub enum ExtensionActionKind {
    HostAction,
    OutputDelivery,
    ToolRequest,
}

impl ExtensionActionKind {
    pub fn effect_class(&self) -> crate::policy::EffectClass {
        match self {
            Self::HostAction | Self::OutputDelivery => crate::policy::EffectClass::Write,
            Self::ToolRequest => crate::policy::EffectClass::Network,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionActionIdempotency {
    IdempotentWhenKeyed,
    HostDedupeRequired,
    NonIdempotent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExtensionPackageResolution {
    pub source_ref: SourceRef,
    pub catalog_id: String,
    pub activation_policy_ref: PolicyRef,
    pub bridge_ref: ExtensionBridgeRef,
    pub action_policy_ref: PolicyRef,
    pub approval_policy_ref: PolicyRef,
    pub redaction_policy_id: String,
    pub runtime_package_fingerprint: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResolvedExtensionPackage {
    pub extension_id: ExtensionId,
    pub version: ExtensionVersion,
    pub runtime_package_fingerprint: String,
    pub catalog_snapshot: CapabilityCatalogSnapshot,
    pub action_capabilities: Vec<ExtensionPackageCapability>,
    pub action_sidecars: Vec<ResolvedExtensionActionSidecar>,
    pub package_sidecars: Vec<PackageSidecarSnapshot>,
}

impl ResolvedExtensionPackage {
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
pub struct ExtensionPackageCapability {
    pub capability_id: CapabilityId,
    pub kind: CapabilityKind,
    pub source: CapabilitySource,
    pub namespace: CapabilityNamespace,
    pub version: CapabilityVersion,
    pub visibility: CapabilityVisibility,
    pub projection: ProjectionMode,
    pub policy_ref: PolicyRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sidecar_refs: Vec<PackageSidecarRef>,
    pub privacy: PrivacyClass,
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
pub struct ExtensionActionRef {
    pub extension_id: ExtensionId,
    pub action_id: ExtensionActionId,
    pub capability_id: CapabilityId,
}

impl ExtensionActionRef {
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

    pub fn subject_id(&self) -> String {
        format!("{}.{}", self.extension_id.as_str(), self.action_id.as_str())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResolvedExtensionActionSidecar {
    pub sidecar_id: String,
    pub action_ref: ExtensionActionRef,
    pub action_kind: ExtensionActionKind,
    pub source_ref: SourceRef,
    pub destination: DestinationRef,
    pub bridge_ref: ExtensionBridgeRef,
    #[serde(default)]
    pub policy_refs: Vec<PolicyRef>,
    pub approval_policy_ref: PolicyRef,
    pub redaction_policy_id: String,
    pub risk_class: RiskClass,
    pub idempotency: ExtensionActionIdempotency,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema_ref: Option<PackageSidecarRef>,
    pub requires_approval: bool,
}

impl ResolvedExtensionActionSidecar {
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

    pub fn package_sidecar_ref(&self) -> PackageSidecarRef {
        let mut sidecar_ref = PackageSidecarRef::new(
            self.sidecar_id.clone(),
            EXTENSION_ACTION_SIDECAR_KIND,
            EXTENSION_ACTION_SIDECAR_VERSION,
        );
        sidecar_ref.content_hash = Some(hash_json(self).expect("extension sidecar hashes"));
        sidecar_ref
    }

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
        })
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExtensionManifestAudit {
    pub forbidden_fields: Vec<String>,
}

impl ExtensionManifestAudit {
    pub fn has_forbidden_field(&self, field: &str) -> bool {
        self.forbidden_fields
            .iter()
            .any(|candidate| candidate == field)
    }
}

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
