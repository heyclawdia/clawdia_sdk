use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    capability::{
        CapabilityId, CapabilityKind, CapabilitySpec, ExecutableCapabilityRoute, PackageSidecarRef,
        ProviderCapabilityProjection,
    },
    domain::{
        AgentError, AgentId, OutputSchemaId, PolicyRef, RuntimePackageId, SourceRef, TrustClass,
    },
    output::{OutputContract, OutputMode, OutputSchemaDialect, ProviderHintPolicy, SchemaVersion},
};

pub mod realtime;
pub mod stream;
pub mod subagent;
pub mod tool_pack;

pub use crate::package_isolation::IsolationRequirementSnapshot;
pub use subagent::{
    ChildPackageStripManifest, ChildRuntimePackage, ChildRuntimePackagePolicy,
    ContextHandoffPolicy, DepthBudget, RouteInheritanceMode, SubagentRoutePolicy,
    SubagentToolPolicy, build_child_runtime_package,
};

pub const RUNTIME_PACKAGE_SCHEMA_VERSION: u16 = 1;
pub const RUNTIME_PACKAGE_FINGERPRINT_ALGORITHM: &str = "sha256:runtime-package-canonical-v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimePackage {
    pub schema_version: u16,
    pub package_id: RuntimePackageId,
    pub agent: AgentSnapshot,
    pub provider_route: ProviderRouteSnapshot,
    pub provider_capabilities: ProviderCapabilitySnapshot,
    #[serde(default)]
    pub output_contracts: Vec<OutputContractSnapshot>,
    #[serde(default)]
    pub output_sinks: Vec<OutputSinkSnapshot>,
    #[serde(default)]
    pub capabilities: Vec<CapabilitySpec>,
    #[serde(default)]
    pub sidecars: Vec<PackageSidecarSnapshot>,
    #[serde(default)]
    pub isolation_requirements: Vec<IsolationRequirementSnapshot>,
    #[serde(default)]
    pub catalogs: Vec<CapabilityCatalogSnapshot>,
    pub child_lifecycle: ChildLifecyclePolicySnapshot,
    pub policies: PolicySnapshot,
    pub fingerprint_manifest: FingerprintInputManifest,
    #[serde(default, skip_serializing_if = "VolatileRuntimeFields::is_empty")]
    pub volatile: VolatileRuntimeFields,
}

impl RuntimePackage {
    pub fn builder(package_id: RuntimePackageId) -> RuntimePackageBuilder {
        RuntimePackageBuilder::new(package_id)
    }

    pub fn for_agent(agent_id: AgentId, agent_name: impl Into<String>) -> RuntimePackageBuilder {
        RuntimePackageBuilder::new(RuntimePackageId::new(format!(
            "package.{}",
            agent_id.as_str()
        )))
        .agent(AgentSnapshot {
            agent_id,
            name: agent_name.into(),
            default_behavior_refs: Vec::new(),
        })
    }

    pub fn canonical_snapshot(&self) -> Result<RuntimePackageCanonicalV1, AgentError> {
        self.validate()?;
        Ok(RuntimePackageCanonicalV1 {
            schema_version: self.schema_version,
            package_id: self.package_id.clone(),
            agent: canonical_agent(self.agent.clone()),
            provider_route: self.provider_route.clone(),
            provider_capabilities: self.provider_capabilities.clone(),
            output_contracts: sorted_by_key(self.output_contracts.clone(), |item| {
                item.schema_id.as_str().to_string()
            }),
            output_sinks: sorted_by_key(self.output_sinks.clone(), |item| item.sink_id.clone()),
            capabilities: sorted_by_key(
                self.capabilities
                    .clone()
                    .into_iter()
                    .map(canonical_capability)
                    .collect(),
                |item| item.capability_id.as_str().to_string(),
            ),
            sidecars: sorted_by_key(
                self.sidecars
                    .clone()
                    .into_iter()
                    .map(canonical_sidecar)
                    .collect(),
                |item| item.sidecar_id.clone(),
            ),
            isolation_requirements: sorted_by_key(self.isolation_requirements.clone(), |item| {
                item.requirement_ref.as_str().to_string()
            }),
            catalogs: sorted_by_key(
                self.catalogs
                    .clone()
                    .into_iter()
                    .map(canonical_catalog)
                    .collect(),
                |item| item.catalog_id.clone(),
            ),
            child_lifecycle: self.child_lifecycle.clone(),
            policies: self.policies.clone_sorted(),
            fingerprint_manifest: self.computed_fingerprint_manifest(),
        })
    }

    pub fn fingerprint(&self) -> Result<RuntimePackageFingerprint, AgentError> {
        let canonical = self.canonical_snapshot()?;
        let preimage = serde_json::json!({
            "algorithm": RUNTIME_PACKAGE_FINGERPRINT_ALGORITHM,
            "canonical_schema_version": RUNTIME_PACKAGE_SCHEMA_VERSION,
            "snapshot": canonical,
        });
        let bytes = serde_json::to_vec(&crate::domain::json::normalize_json_value(preimage))
            .map_err(|error| {
                AgentError::contract_violation(format!(
                    "package fingerprint serialization failed: {error}"
                ))
            })?;
        let digest = Sha256::digest(bytes);
        Ok(RuntimePackageFingerprint(format!(
            "{RUNTIME_PACKAGE_FINGERPRINT_ALGORITHM}:{}",
            hex_lower(&digest)
        )))
    }

    pub fn validate(&self) -> Result<(), AgentError> {
        if self.schema_version != RUNTIME_PACKAGE_SCHEMA_VERSION {
            return Err(AgentError::contract_violation(format!(
                "unsupported runtime package schema version {}",
                self.schema_version
            )));
        }
        if self.provider_route.route_id.is_empty() {
            return Err(AgentError::missing_required_field(
                "provider_route.route_id",
            ));
        }
        if self.provider_route.model_id.is_empty() {
            return Err(AgentError::missing_required_field(
                "provider_route.model_id",
            ));
        }

        for capability in &self.capabilities {
            capability.validate()?;
        }
        for output_contract in &self.output_contracts {
            output_contract.validate()?;
        }
        for isolation_requirement in &self.isolation_requirements {
            isolation_requirement.validate()?;
        }

        let projected_ids = self
            .provider_tool_specs()?
            .into_iter()
            .map(|projection| projection.capability_id)
            .collect::<Vec<_>>();
        let executable_ids = self
            .executable_routes()?
            .into_iter()
            .map(|route| route.capability_id)
            .collect::<Vec<_>>();
        for projected_id in projected_ids {
            if !executable_ids.contains(&projected_id) {
                return Err(AgentError::contract_violation(format!(
                    "projected capability {} has no executable route in the same runtime package",
                    projected_id.as_str()
                )));
            }
        }

        Ok(())
    }

    pub fn provider_tool_specs(&self) -> Result<Vec<ProviderCapabilityProjection>, AgentError> {
        self.capabilities
            .iter()
            .filter_map(|capability| capability.project_for_provider().transpose())
            .collect()
    }

    pub fn executable_routes(&self) -> Result<Vec<ExecutableCapabilityRoute>, AgentError> {
        self.capabilities
            .iter()
            .filter_map(|capability| capability.executable_route().transpose())
            .collect()
    }

    pub fn sidecar(&self, sidecar_id: &str) -> Option<&PackageSidecarSnapshot> {
        self.sidecars
            .iter()
            .find(|sidecar| sidecar.sidecar_id == sidecar_id)
    }

    pub fn with_output_contract(
        mut self,
        output_contract: &OutputContract,
    ) -> Result<Self, AgentError> {
        output_contract.validate_shape()?;
        let snapshot = OutputContractSnapshot::from(output_contract);
        self.output_contracts
            .retain(|existing| existing.schema_id != snapshot.schema_id);
        self.output_contracts.push(snapshot);
        self.fingerprint_manifest = self.computed_fingerprint_manifest();
        self.validate()?;
        Ok(self)
    }

    pub fn catalog(&self, catalog_id: &str) -> Option<&CapabilityCatalogSnapshot> {
        self.catalogs
            .iter()
            .find(|catalog| catalog.catalog_id == catalog_id)
    }

    pub fn apply_delta(&self, delta: PackageDelta) -> Result<Self, AgentError> {
        if delta.previous_fingerprint != self.fingerprint()? {
            return Err(AgentError::contract_violation(
                "package delta previous fingerprint does not match current package",
            ));
        }

        let mut next = self.clone();
        for capability_id in delta.deactivated_capability_ids {
            next.capabilities
                .retain(|capability| capability.capability_id != capability_id);
        }
        next.capabilities.extend(delta.activated_capabilities);
        next.catalogs.extend(delta.catalogs);
        next.sidecars.extend(delta.sidecars);
        next.fingerprint_manifest = next.computed_fingerprint_manifest();
        next.validate()?;
        Ok(next)
    }

    pub fn conformance_report(&self) -> Result<RuntimePackageConformanceReport, AgentError> {
        let fingerprint = self.fingerprint()?;
        Ok(RuntimePackageConformanceReport {
            fingerprint,
            provider_projection_count: self.provider_tool_specs()?.len(),
            executable_route_count: self.executable_routes()?.len(),
            reserved_inactive_count: self
                .capabilities
                .iter()
                .filter(|capability| capability.kind.is_reserved())
                .count(),
            catalog_count: self.catalogs.len(),
            sidecar_count: self.sidecars.len(),
        })
    }

    fn computed_fingerprint_manifest(&self) -> FingerprintInputManifest {
        FingerprintInputManifest {
            algorithm: RUNTIME_PACKAGE_FINGERPRINT_ALGORITHM.to_string(),
            canonical_schema_version: RUNTIME_PACKAGE_SCHEMA_VERSION,
            readiness_profile: self.fingerprint_manifest.readiness_profile.clone(),
            included_groups: vec![
                FingerprintInputGroup::Agent,
                FingerprintInputGroup::ProviderRoute,
                FingerprintInputGroup::OutputContracts,
                FingerprintInputGroup::OutputSinks,
                FingerprintInputGroup::Capabilities,
                FingerprintInputGroup::Sidecars,
                FingerprintInputGroup::IsolationRequirements,
                FingerprintInputGroup::Catalogs,
                FingerprintInputGroup::ChildLifecycle,
                FingerprintInputGroup::Policies,
            ],
            excluded_groups: vec![
                FingerprintExclusionGroup::RunIds,
                FingerprintExclusionGroup::EventIds,
                FingerprintExclusionGroup::Timestamps,
                FingerprintExclusionGroup::AdapterHealth,
                FingerprintExclusionGroup::TemporaryPaths,
                FingerprintExclusionGroup::TelemetrySinkHealth,
            ],
            reserved_feature_status: self
                .capabilities
                .iter()
                .filter(|capability| capability.kind.is_reserved())
                .map(|capability| ReservedFeatureFingerprintStatus {
                    capability_id: capability.capability_id.clone(),
                    kind: capability.kind.clone(),
                    owner_role: capability.readiness.owner_role.clone(),
                    status: "reserved_inactive".to_string(),
                    reason: "owner workstream has not supplied sidecar, fingerprint, event, journal, and acceptance-test evidence".to_string(),
                })
                .collect::<Vec<_>>()
                .canonicalized_reserved_status(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RuntimePackageBuilder {
    package: RuntimePackage,
}

impl RuntimePackageBuilder {
    pub fn new(package_id: RuntimePackageId) -> Self {
        let package = RuntimePackage {
            schema_version: RUNTIME_PACKAGE_SCHEMA_VERSION,
            package_id,
            agent: AgentSnapshot {
                agent_id: AgentId::new("agent.default"),
                name: "agent".to_string(),
                default_behavior_refs: Vec::new(),
            },
            provider_route: ProviderRouteSnapshot::new("provider.fake", "model.fake"),
            provider_capabilities: ProviderCapabilitySnapshot {
                capability_version: "provider.capabilities.v1".to_string(),
                realtime_capability_version: None,
            },
            output_contracts: Vec::new(),
            output_sinks: Vec::new(),
            capabilities: Vec::new(),
            sidecars: Vec::new(),
            isolation_requirements: Vec::new(),
            catalogs: Vec::new(),
            child_lifecycle: ChildLifecyclePolicySnapshot::safe_defaults(),
            policies: PolicySnapshot::default(),
            fingerprint_manifest: FingerprintInputManifest::p0_text(),
            volatile: VolatileRuntimeFields::default(),
        };
        Self { package }
    }

    pub fn agent(mut self, agent: AgentSnapshot) -> Self {
        self.package.agent = agent;
        self
    }

    pub fn provider_route(mut self, provider_route: ProviderRouteSnapshot) -> Self {
        self.package.provider_route = provider_route;
        self
    }

    pub fn output_contract(mut self, output_contract: OutputContractSnapshot) -> Self {
        self.package.output_contracts.push(output_contract);
        self
    }

    pub fn output_sink(mut self, output_sink: OutputSinkSnapshot) -> Self {
        self.package.output_sinks.push(output_sink);
        self
    }

    pub fn capability(mut self, capability: CapabilitySpec) -> Self {
        self.package.capabilities.push(capability);
        self
    }

    pub fn sidecar(mut self, sidecar: PackageSidecarSnapshot) -> Self {
        self.package.sidecars.push(sidecar);
        self
    }

    pub fn catalog(mut self, catalog: CapabilityCatalogSnapshot) -> Self {
        self.package.catalogs.push(catalog);
        self
    }

    pub fn isolation_requirement(mut self, snapshot: IsolationRequirementSnapshot) -> Self {
        self.package.isolation_requirements.push(snapshot);
        self
    }

    pub fn policy(mut self, policy_ref: PolicyRef) -> Self {
        self.package.policies.policy_refs.push(policy_ref);
        self
    }

    pub fn build(mut self) -> Result<RuntimePackage, AgentError> {
        self.package.fingerprint_manifest = self.package.computed_fingerprint_manifest();
        self.package.validate()?;
        Ok(self.package)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimePackageCanonicalV1 {
    pub schema_version: u16,
    pub package_id: RuntimePackageId,
    pub agent: AgentSnapshot,
    pub provider_route: ProviderRouteSnapshot,
    pub provider_capabilities: ProviderCapabilitySnapshot,
    pub output_contracts: Vec<OutputContractSnapshot>,
    pub output_sinks: Vec<OutputSinkSnapshot>,
    pub capabilities: Vec<CapabilitySpec>,
    pub sidecars: Vec<PackageSidecarSnapshot>,
    pub isolation_requirements: Vec<IsolationRequirementSnapshot>,
    pub catalogs: Vec<CapabilityCatalogSnapshot>,
    pub child_lifecycle: ChildLifecyclePolicySnapshot,
    pub policies: PolicySnapshot,
    pub fingerprint_manifest: FingerprintInputManifest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimePackageFingerprint(pub String);

impl RuntimePackageFingerprint {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AgentSnapshot {
    pub agent_id: AgentId,
    pub name: String,
    #[serde(default)]
    pub default_behavior_refs: Vec<PolicyRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderRouteSnapshot {
    pub route_id: String,
    pub model_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_policy_ref: Option<PolicyRef>,
}

impl ProviderRouteSnapshot {
    pub fn new(route_id: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            route_id: route_id.into(),
            model_id: model_id.into(),
            provider_policy_ref: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderCapabilitySnapshot {
    pub capability_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realtime_capability_version: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutputContractSnapshot {
    pub schema_id: OutputSchemaId,
    pub schema_version: SchemaVersion,
    pub schema_fingerprint: String,
    pub dialect: OutputSchemaDialect,
    pub mode: OutputMode,
    pub validation_policy_ref: PolicyRef,
    pub repair_policy_ref: PolicyRef,
    pub local_validator_version: String,
    pub provider_hint_policy: ProviderHintPolicy,
    pub validation: crate::output::ValidationPolicy,
    pub repair: crate::output::RepairPolicy,
    pub retry_budget: crate::output::RetryBudget,
    pub content_policy: crate::policy::ContentCapturePolicy,
    pub projection_hint: crate::output::OutputProjectionHint,
}

impl OutputContractSnapshot {
    pub fn validate(&self) -> Result<(), AgentError> {
        if !self.schema_fingerprint.starts_with("sha256:") {
            return Err(AgentError::contract_violation(
                "output contract snapshot schema_fingerprint must be sha256-prefixed",
            ));
        }
        Ok(())
    }
}

impl From<&OutputContract> for OutputContractSnapshot {
    fn from(contract: &OutputContract) -> Self {
        Self {
            schema_id: contract.schema_id.clone(),
            schema_version: contract.schema_version,
            schema_fingerprint: contract.schema_fingerprint().as_str().to_string(),
            dialect: contract.dialect.clone(),
            mode: contract.mode.clone(),
            validation_policy_ref: contract.validation.validator_ref_policy(),
            repair_policy_ref: contract.repair.repair_adapter_ref_policy(),
            local_validator_version: contract.validation.validator_ref.as_str().to_string(),
            provider_hint_policy: contract.projection_hint.provider_hint_policy.clone(),
            validation: contract.validation.clone(),
            repair: contract.repair.clone(),
            retry_budget: contract.retry_budget.clone(),
            content_policy: contract.content_policy.clone(),
            projection_hint: contract.projection_hint.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutputSinkSnapshot {
    pub sink_id: String,
    pub delivery_policy_ref: PolicyRef,
    pub dedupe_policy: String,
    pub sink_capability_version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PackageSidecarSnapshot {
    pub sidecar_id: String,
    pub kind: String,
    pub version: String,
    pub refs: Vec<PackageSidecarRef>,
    pub policy_refs: Vec<PolicyRef>,
    pub content_hash: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChildLifecyclePolicySnapshot {
    pub policy_ref: PolicyRef,
    pub manual_cancel: String,
    pub detach_policy_ref: PolicyRef,
    pub cleanup_timeout_ms: u64,
}

impl ChildLifecyclePolicySnapshot {
    pub fn safe_defaults() -> Self {
        Self {
            policy_ref: PolicyRef::with_kind(
                crate::domain::PolicyKind::RuntimePackage,
                "policy.child.safe-defaults",
            ),
            manual_cancel: "cancel_agent_owned_children".to_string(),
            detach_policy_ref: PolicyRef::with_kind(
                crate::domain::PolicyKind::RuntimePackage,
                "policy.detach.deny-by-default",
            ),
            cleanup_timeout_ms: 30_000,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PolicySnapshot {
    #[serde(default)]
    pub policy_refs: Vec<PolicyRef>,
}

impl PolicySnapshot {
    fn clone_sorted(&self) -> Self {
        Self {
            policy_refs: sorted_policy_refs(self.policy_refs.clone()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilityCatalogSnapshot {
    pub catalog_id: String,
    pub source_kind: crate::capability::CapabilitySourceKind,
    pub source_ref: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    pub trust_state: TrustClass,
    pub activation_policy_ref: PolicyRef,
    #[serde(default)]
    pub candidates: Vec<CapabilityId>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PackageDelta {
    pub previous_fingerprint: RuntimePackageFingerprint,
    pub requested_by: SourceRef,
    pub reason: String,
    #[serde(default)]
    pub activated_capabilities: Vec<CapabilitySpec>,
    #[serde(default)]
    pub deactivated_capability_ids: Vec<CapabilityId>,
    #[serde(default)]
    pub catalogs: Vec<CapabilityCatalogSnapshot>,
    #[serde(default)]
    pub sidecars: Vec<PackageSidecarSnapshot>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimePackageConformanceReport {
    pub fingerprint: RuntimePackageFingerprint,
    pub provider_projection_count: usize,
    pub executable_route_count: usize,
    pub reserved_inactive_count: usize,
    pub catalog_count: usize,
    pub sidecar_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FingerprintInputManifest {
    pub algorithm: String,
    pub canonical_schema_version: u16,
    pub readiness_profile: ReadinessProfile,
    pub included_groups: Vec<FingerprintInputGroup>,
    pub excluded_groups: Vec<FingerprintExclusionGroup>,
    pub reserved_feature_status: Vec<ReservedFeatureFingerprintStatus>,
}

impl FingerprintInputManifest {
    pub fn p0_text() -> Self {
        Self {
            algorithm: RUNTIME_PACKAGE_FINGERPRINT_ALGORITHM.to_string(),
            canonical_schema_version: RUNTIME_PACKAGE_SCHEMA_VERSION,
            readiness_profile: ReadinessProfile::P0Text,
            included_groups: Vec::new(),
            excluded_groups: Vec::new(),
            reserved_feature_status: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessProfile {
    P0Text,
    P1TypedOutput,
    P2SideEffects,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FingerprintInputGroup {
    Agent,
    ProviderRoute,
    OutputContracts,
    OutputSinks,
    Capabilities,
    Sidecars,
    IsolationRequirements,
    Catalogs,
    ChildLifecycle,
    Policies,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FingerprintExclusionGroup {
    RunIds,
    EventIds,
    Timestamps,
    AdapterHealth,
    TemporaryPaths,
    TelemetrySinkHealth,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReservedFeatureFingerprintStatus {
    pub capability_id: CapabilityId,
    pub kind: CapabilityKind,
    pub owner_role: String,
    pub status: String,
    pub reason: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct VolatileRuntimeFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_health: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporary_path: Option<String>,
}

impl VolatileRuntimeFields {
    fn is_empty(&self) -> bool {
        self.run_id.is_none()
            && self.event_id.is_none()
            && self.timestamp_ms.is_none()
            && self.adapter_health.is_none()
            && self.temporary_path.is_none()
    }
}

fn sorted_by_key<T, F>(mut items: Vec<T>, mut key: F) -> Vec<T>
where
    F: FnMut(&T) -> String,
{
    items.sort_by_key(|item| key(item));
    items
}

fn canonical_agent(mut agent: AgentSnapshot) -> AgentSnapshot {
    agent.default_behavior_refs = sorted_policy_refs(agent.default_behavior_refs);
    agent
}

fn canonical_capability(mut capability: CapabilitySpec) -> CapabilitySpec {
    capability.projection = canonical_projection(capability.projection);
    capability.sidecar_refs = sorted_sidecar_refs(capability.sidecar_refs);
    capability
}

fn canonical_projection(
    projection: crate::capability::ProjectionMode,
) -> crate::capability::ProjectionMode {
    match projection {
        crate::capability::ProjectionMode::ProducesContextItems { mut allowed_kinds } => {
            allowed_kinds.sort();
            crate::capability::ProjectionMode::ProducesContextItems { allowed_kinds }
        }
        crate::capability::ProjectionMode::ProjectsContextRefs {
            mut allowed_ref_kinds,
        } => {
            allowed_ref_kinds.sort();
            crate::capability::ProjectionMode::ProjectsContextRefs { allowed_ref_kinds }
        }
        other => other,
    }
}

fn canonical_sidecar(mut sidecar: PackageSidecarSnapshot) -> PackageSidecarSnapshot {
    sidecar.refs = sorted_sidecar_refs(sidecar.refs);
    sidecar.policy_refs = sorted_policy_refs(sidecar.policy_refs);
    sidecar
}

fn canonical_catalog(mut catalog: CapabilityCatalogSnapshot) -> CapabilityCatalogSnapshot {
    catalog.candidates = sorted_by_key(catalog.candidates, |capability| {
        capability.as_str().to_string()
    });
    catalog
}

fn sorted_policy_refs(policy_refs: Vec<PolicyRef>) -> Vec<PolicyRef> {
    sorted_by_key(policy_refs, policy_ref_key)
}

fn sorted_sidecar_refs(sidecar_refs: Vec<PackageSidecarRef>) -> Vec<PackageSidecarRef> {
    sorted_by_key(sidecar_refs, sidecar_ref_key)
}

fn policy_ref_key(policy: &PolicyRef) -> String {
    format!(
        "{:?}:{}:{}",
        policy.kind,
        policy.as_str(),
        policy.version.as_deref().unwrap_or("")
    )
}

fn sidecar_ref_key(sidecar: &PackageSidecarRef) -> String {
    format!(
        "{}:{}:{}:{}",
        sidecar.sidecar_id,
        sidecar.kind,
        sidecar.version,
        sidecar.content_hash.as_deref().unwrap_or("")
    )
}

trait CanonicalReservedStatus {
    fn canonicalized_reserved_status(self) -> Self;
}

impl CanonicalReservedStatus for Vec<ReservedFeatureFingerprintStatus> {
    fn canonicalized_reserved_status(mut self) -> Self {
        self.sort_by_key(|status| status.capability_id.as_str().to_string());
        self
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}
