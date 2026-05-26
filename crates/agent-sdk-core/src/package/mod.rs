//! Runtime package authority for one run. Use this module to freeze provider routes,
//! capabilities, sidecars, catalogs, policies, output sinks, and fingerprints before
//! execution. Package builders are data-only; applying deltas returns a new snapshot
//! rather than mutating ambient state.
//!
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
    package_hooks::{HookSpec, validate_hook_specs},
};

/// Public realtime namespace. Use it for the documented realtime API
/// surface; prefer crate-root re-exports for common imports. Module
/// items must preserve the core ownership and side-effect boundaries
/// described in this file.
pub mod realtime;
/// Public stream namespace. Use it for the documented stream API
/// surface; prefer crate-root re-exports for common imports. Module
/// items must preserve the core ownership and side-effect boundaries
/// described in this file.
pub mod stream;
/// Public subagent namespace. Use it for the documented subagent API
/// surface; prefer crate-root re-exports for common imports. Module
/// items must preserve the core ownership and side-effect boundaries
/// described in this file.
pub mod subagent;
/// Public tool pack namespace. Use it for the documented tool pack API
/// surface; prefer crate-root re-exports for common imports. Module
/// items must preserve the core ownership and side-effect boundaries
/// described in this file.
pub mod tool_pack;

pub use crate::package_isolation::IsolationRequirementSnapshot;
pub use subagent::{
    ChildPackageStripManifest, ChildRuntimePackage, ChildRuntimePackagePolicy,
    ContextHandoffPolicy, DepthBudget, RouteInheritanceMode, SubagentRoutePolicy,
    SubagentToolPolicy, build_child_runtime_package,
};

/// Constant value for the package contract. Use it to keep SDK records
/// and tests aligned on the same stable value.
pub const RUNTIME_PACKAGE_SCHEMA_VERSION: u16 = 1;
/// Constant value for the package contract. Use it to keep SDK records
/// and tests aligned on the same stable value.
pub const RUNTIME_PACKAGE_FINGERPRINT_ALGORITHM: &str = "sha256:runtime-package-canonical-v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the runtime package portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct RuntimePackage {
    /// Wire schema version used for compatibility checks.
    pub schema_version: u16,
    /// Runtime package identifier for the immutable per-run package snapshot.
    pub package_id: RuntimePackageId,
    /// Agent snapshot frozen into this package or record.
    pub agent: AgentSnapshot,
    /// Provider route snapshot selected for this runtime package.
    pub provider_route: ProviderRouteSnapshot,
    /// Provider capability hints frozen into this package snapshot.
    pub provider_capabilities: ProviderCapabilitySnapshot,
    #[serde(default)]
    /// Output contracts frozen into this package or request.
    pub output_contracts: Vec<OutputContractSnapshot>,
    #[serde(default)]
    /// Output sink snapshots available to this run.
    pub output_sinks: Vec<OutputSinkSnapshot>,
    #[serde(default)]
    /// Capabilities frozen into the package or returned by an adapter health
    /// check.
    pub capabilities: Vec<CapabilitySpec>,
    #[serde(default)]
    /// Typed sidecar snapshots included in a package or delta.
    pub sidecars: Vec<PackageSidecarSnapshot>,
    #[serde(default)]
    /// Hook specs frozen into this package snapshot.
    pub hooks: Vec<HookSpec>,
    #[serde(default)]
    /// Isolation requirements frozen into the package snapshot.
    pub isolation_requirements: Vec<IsolationRequirementSnapshot>,
    #[serde(default)]
    /// Catalog snapshots contributed to or returned with a runtime package
    /// delta.
    pub catalogs: Vec<CapabilityCatalogSnapshot>,
    /// Child-run lifecycle policy frozen into the package snapshot.
    pub child_lifecycle: ChildLifecyclePolicySnapshot,
    /// Policies used by this record or request.
    pub policies: PolicySnapshot,
    /// Manifest describing which fields entered or were excluded from
    /// fingerprinting.
    pub fingerprint_manifest: FingerprintInputManifest,
    #[serde(default, skip_serializing_if = "VolatileRuntimeFields::is_empty")]
    /// Volatile used by this record or request.
    pub volatile: VolatileRuntimeFields,
}

impl RuntimePackage {
    /// Starts a builder for this package value. Building is data-only;
    /// runtime side effects occur only when a later coordinator or host
    /// port executes the built configuration.
    pub fn builder(package_id: RuntimePackageId) -> RuntimePackageBuilder {
        RuntimePackageBuilder::new(package_id)
    }

    /// Returns for agent for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
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

    /// Computes the stable canonical snapshot for this package value.
    /// The computation is deterministic and side-effect free so it can
    /// be used in package, journal, or test evidence.
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

    /// Computes the stable fingerprint for this package value. The
    /// computation is deterministic and side-effect free so it can be
    /// used in package, journal, or test evidence.
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

    /// Validates the package invariants and returns a typed error on
    /// failure. Validation is pure and does not perform I/O, dispatch,
    /// journal appends, or adapter calls.
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
        validate_hook_specs(&self.hooks)?;
        self.validate_hook_sidecars()?;
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

    /// Returns provider tool specs for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn provider_tool_specs(&self) -> Result<Vec<ProviderCapabilityProjection>, AgentError> {
        self.capabilities
            .iter()
            .filter_map(|capability| capability.project_for_provider().transpose())
            .collect()
    }

    /// Returns executable routes for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn executable_routes(&self) -> Result<Vec<ExecutableCapabilityRoute>, AgentError> {
        self.capabilities
            .iter()
            .filter_map(|capability| capability.executable_route().transpose())
            .collect()
    }

    /// Returns sidecar for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn sidecar(&self, sidecar_id: &str) -> Option<&PackageSidecarSnapshot> {
        self.sidecars
            .iter()
            .find(|sidecar| sidecar.sidecar_id == sidecar_id)
    }

    /// Returns this value with its output contract setting replaced.
    /// The method follows builder-style data construction and does not
    /// execute external work.
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

    /// Returns catalog for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn catalog(&self, catalog_id: &str) -> Option<&CapabilityCatalogSnapshot> {
        self.catalogs
            .iter()
            .find(|catalog| catalog.catalog_id == catalog_id)
    }

    /// Validates a package delta against this snapshot and returns a new
    /// runtime package. The method is pure with respect to the existing
    /// package and does not mutate ambient registries or execute activated
    /// capabilities.
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

    /// Returns conformance report for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
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

    fn validate_hook_sidecars(&self) -> Result<(), AgentError> {
        for spec in &self.hooks {
            let expected = canonical_sidecar(spec.sidecar_snapshot()?);
            let actual = self.sidecar(&expected.sidecar_id).ok_or_else(|| {
                AgentError::contract_violation(format!(
                    "hook sidecar {} is missing from runtime package",
                    expected.sidecar_id
                ))
            })?;
            if canonical_sidecar(actual.clone()) != expected {
                return Err(AgentError::contract_violation(format!(
                    "hook sidecar {} does not match hook spec {}",
                    actual.sidecar_id,
                    spec.hook_id.as_str()
                )));
            }
        }
        Ok(())
    }

    fn sync_hook_sidecars(&mut self) -> Result<(), AgentError> {
        for spec in &self.hooks {
            let sidecar = spec.sidecar_snapshot()?;
            self.sidecars
                .retain(|existing| existing.sidecar_id != sidecar.sidecar_id);
            self.sidecars.push(sidecar);
        }
        Ok(())
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
/// Describes the runtime package builder portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct RuntimePackageBuilder {
    package: RuntimePackage,
}

impl RuntimePackageBuilder {
    /// Creates a new package value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
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
            hooks: Vec::new(),
            isolation_requirements: Vec::new(),
            catalogs: Vec::new(),
            child_lifecycle: ChildLifecyclePolicySnapshot::safe_defaults(),
            policies: PolicySnapshot::default(),
            fingerprint_manifest: FingerprintInputManifest::p0_text(),
            volatile: VolatileRuntimeFields::default(),
        };
        Self { package }
    }

    /// Returns agent for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn agent(mut self, agent: AgentSnapshot) -> Self {
        self.package.agent = agent;
        self
    }

    /// Returns provider route for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn provider_route(mut self, provider_route: ProviderRouteSnapshot) -> Self {
        self.package.provider_route = provider_route;
        self
    }

    /// Returns output contract for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn output_contract(mut self, output_contract: OutputContractSnapshot) -> Self {
        self.package.output_contracts.push(output_contract);
        self
    }

    /// Returns output sink for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn output_sink(mut self, output_sink: OutputSinkSnapshot) -> Self {
        self.package.output_sinks.push(output_sink);
        self
    }

    /// Returns capability for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn capability(mut self, capability: CapabilitySpec) -> Self {
        self.package.capabilities.push(capability);
        self
    }

    /// Returns sidecar for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn sidecar(mut self, sidecar: PackageSidecarSnapshot) -> Self {
        self.package.sidecars.push(sidecar);
        self
    }

    /// Returns hook for the current value.
    /// This records a typed hook spec and derives its canonical sidecar during build; it does
    /// not resolve or invoke the hook executor.
    pub fn hook(mut self, hook: HookSpec) -> Self {
        self.package.hooks.push(hook);
        self
    }

    /// Returns catalog for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn catalog(mut self, catalog: CapabilityCatalogSnapshot) -> Self {
        self.package.catalogs.push(catalog);
        self
    }

    /// Returns isolation requirement for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn isolation_requirement(mut self, snapshot: IsolationRequirementSnapshot) -> Self {
        self.package.isolation_requirements.push(snapshot);
        self
    }

    /// Returns policy for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn policy(mut self, policy_ref: PolicyRef) -> Self {
        self.package.policies.policy_refs.push(policy_ref);
        self
    }

    /// Finishes builder validation and returns the configured value.
    /// This is data-only unless the surrounding builder explicitly
    /// documents adapter or store access.
    pub fn build(mut self) -> Result<RuntimePackage, AgentError> {
        self.package.sync_hook_sidecars()?;
        self.package.fingerprint_manifest = self.package.computed_fingerprint_manifest();
        self.package.validate()?;
        Ok(self.package)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the runtime package canonical v1 portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct RuntimePackageCanonicalV1 {
    /// Wire schema version used for compatibility checks.
    pub schema_version: u16,
    /// Runtime package identifier for the immutable per-run package snapshot.
    pub package_id: RuntimePackageId,
    /// Agent snapshot frozen into this package or record.
    pub agent: AgentSnapshot,
    /// Provider route snapshot selected for this runtime package.
    pub provider_route: ProviderRouteSnapshot,
    /// Provider capability hints frozen into this package snapshot.
    pub provider_capabilities: ProviderCapabilitySnapshot,
    /// Output contracts frozen into this package or request.
    pub output_contracts: Vec<OutputContractSnapshot>,
    /// Output sink snapshots available to this run.
    pub output_sinks: Vec<OutputSinkSnapshot>,
    /// Capabilities frozen into the package or returned by an adapter health
    /// check.
    pub capabilities: Vec<CapabilitySpec>,
    /// Typed sidecar snapshots included in a package or delta.
    pub sidecars: Vec<PackageSidecarSnapshot>,
    /// Isolation requirements frozen into the package snapshot.
    pub isolation_requirements: Vec<IsolationRequirementSnapshot>,
    /// Catalog snapshots contributed to or returned with a runtime package
    /// delta.
    pub catalogs: Vec<CapabilityCatalogSnapshot>,
    /// Child-run lifecycle policy frozen into the package snapshot.
    pub child_lifecycle: ChildLifecyclePolicySnapshot,
    /// Policies used by this record or request.
    pub policies: PolicySnapshot,
    /// Manifest describing which fields entered or were excluded from
    /// fingerprinting.
    pub fingerprint_manifest: FingerprintInputManifest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the runtime package fingerprint portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct RuntimePackageFingerprint(pub String);

impl RuntimePackageFingerprint {
    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the agent snapshot portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct AgentSnapshot {
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    /// Human-readable or protocol-visible name for this SDK item.
    pub name: String,
    #[serde(default)]
    /// Typed default behavior refs references. Resolving them is separate
    /// from constructing this record.
    pub default_behavior_refs: Vec<PolicyRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the provider route snapshot portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ProviderRouteSnapshot {
    /// Stable route id used for typed lineage, lookup, or dedupe.
    pub route_id: String,
    /// Stable model id used for typed lineage, lookup, or dedupe.
    pub model_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Policy reference governing provider projection or calls.
    pub provider_policy_ref: Option<PolicyRef>,
}

impl ProviderRouteSnapshot {
    /// Creates a new package value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new(route_id: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            route_id: route_id.into(),
            model_id: model_id.into(),
            provider_policy_ref: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the provider capability snapshot portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ProviderCapabilitySnapshot {
    /// Capability version advertised by the provider or package.
    /// Use it to match compatible feature contracts during package resolution.
    pub capability_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional realtime capability version advertised by the package.
    /// Package resolution can use it to match compatible realtime sidecars and adapters.
    pub realtime_capability_version: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the output contract snapshot portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct OutputContractSnapshot {
    /// Stable schema id used for typed lineage, lookup, or dedupe.
    pub schema_id: OutputSchemaId,
    /// Wire schema version used for compatibility checks.
    pub schema_version: SchemaVersion,
    /// Deterministic schema fingerprint used for stale checks, package
    /// evidence, or replay comparisons.
    pub schema_fingerprint: String,
    /// Schema dialect used to interpret the output schema.
    /// Validators use it to select the supported JSON-schema subset and compatibility rules.
    pub dialect: OutputSchemaDialect,
    /// Mode that selects how this operation or contract should behave.
    /// Callers use it to choose the explicit execution path instead of relying on hidden
    /// defaults.
    pub mode: OutputMode,
    /// Typed validation policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub validation_policy_ref: PolicyRef,
    /// Typed repair policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub repair_policy_ref: PolicyRef,
    /// Version of the local validator contract used for this output policy.
    /// Use it to keep validation and replay behavior stable across releases.
    pub local_validator_version: String,
    /// Policy for provider-side structured-output hints.
    /// Hints may guide prompting but cannot replace SDK-owned validation.
    pub provider_hint_policy: ProviderHintPolicy,
    /// Validation policy applied before output is accepted as typed data.
    /// It controls validator selection, bounds, failure visibility, and local validation
    /// behavior.
    pub validation: crate::output::ValidationPolicy,
    /// Repair policy used after structured output validation fails.
    /// It controls whether repair is attempted and which policy gates must approve it.
    pub repair: crate::output::RepairPolicy,
    /// Retry budget for validation, repair, or adapter attempts.
    /// Runtimes use it to stop bounded loops deterministically.
    pub retry_budget: crate::output::RetryBudget,
    /// Content-capture policy that governs raw content, summaries, redaction, and retention.
    /// Projection, telemetry, and delivery paths must honor it before exposing content.
    pub content_policy: crate::policy::ContentCapturePolicy,
    /// Provider-facing projection hint for structured output requests.
    /// It can guide model prompting but does not replace local validation policy.
    pub projection_hint: crate::output::OutputProjectionHint,
}

impl OutputContractSnapshot {
    /// Validates the package invariants and returns a typed error on
    /// failure. Validation is pure and does not perform I/O, dispatch,
    /// journal appends, or adapter calls.
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
/// Describes the output sink snapshot portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct OutputSinkSnapshot {
    /// Stable sink id used for typed lineage, lookup, or dedupe.
    pub sink_id: String,
    /// Typed delivery policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub delivery_policy_ref: PolicyRef,
    /// Dedupe policy or key for a side-effecting operation.
    /// Replay and repair use it to avoid sending or executing the same effect twice.
    pub dedupe_policy: String,
    /// Capability version advertised by an output sink.
    /// Delivery policy uses it to confirm that the sink can receive the requested payload
    /// shape.
    pub sink_capability_version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the package sidecar snapshot portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct PackageSidecarSnapshot {
    /// Identifier for the typed package sidecar.
    pub sidecar_id: String,
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: String,
    /// Version string for this capability, package, or protocol surface.
    /// Use it for compatibility checks during package or adapter resolution.
    pub version: String,
    /// References associated with refs.
    /// Resolve them through the appropriate registry or content store before using referenced
    /// data.
    pub refs: Vec<PackageSidecarRef>,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Stable hash for the bytes or canonical payload used for stale checks
    /// and fingerprints.
    pub content_hash: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the child lifecycle policy snapshot portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ChildLifecyclePolicySnapshot {
    /// Policy reference that must be resolved by the host or runtime before
    /// execution.
    pub policy_ref: PolicyRef,
    /// Manual cancellation policy for child or run lifecycle.
    /// Use it to decide whether cancellation cascades, detaches, or requires explicit cleanup.
    pub manual_cancel: String,
    /// Typed detach policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub detach_policy_ref: PolicyRef,
    /// cleanup timeout ms duration in milliseconds.
    pub cleanup_timeout_ms: u64,
}

impl ChildLifecyclePolicySnapshot {
    /// Returns an updated value with safe defaults configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
/// Describes the policy snapshot portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct PolicySnapshot {
    #[serde(default)]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
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
/// Describes the capability catalog snapshot portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct CapabilityCatalogSnapshot {
    /// Stable catalog id used for typed lineage, lookup, or dedupe.
    pub catalog_id: String,
    /// Kind discriminator for source kind.
    /// Use it to route finite match arms without parsing display text.
    pub source_kind: crate::capability::CapabilitySourceKind,
    /// Typed source reference that records where this item originated.
    pub source_ref: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Version string for this capability, package, or protocol surface.
    /// Use it for compatibility checks during package or adapter resolution.
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable hash for the bytes or canonical payload used for stale checks
    /// and fingerprints.
    pub content_hash: Option<String>,
    /// Trust state used by this record or request.
    pub trust_state: TrustClass,
    /// Typed activation policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub activation_policy_ref: PolicyRef,
    #[serde(default)]
    /// Candidate capabilities, tools, resources, or package entries exposed
    /// for host-approved selection.
    pub candidates: Vec<CapabilityId>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the package delta portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct PackageDelta {
    /// Fingerprint of the package snapshot that a delta was computed against.
    pub previous_fingerprint: RuntimePackageFingerprint,
    /// Source that requested this package delta, activation, or side effect.
    pub requested_by: SourceRef,
    /// Redacted explanation for a denial, failure, status, or package delta.
    pub reason: String,
    #[serde(default)]
    /// Capabilities that a package delta proposes to add to the next
    /// snapshot.
    pub activated_capabilities: Vec<CapabilitySpec>,
    #[serde(default)]
    /// Capability identifiers that a package delta proposes to remove from
    /// the next snapshot.
    pub deactivated_capability_ids: Vec<CapabilityId>,
    #[serde(default)]
    /// Catalog snapshots contributed to or returned with a runtime package
    /// delta.
    pub catalogs: Vec<CapabilityCatalogSnapshot>,
    #[serde(default)]
    /// Typed sidecar snapshots included in a package or delta.
    pub sidecars: Vec<PackageSidecarSnapshot>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the runtime package conformance report portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct RuntimePackageConformanceReport {
    /// Deterministic fingerprint for package, event, telemetry, or validation
    /// evidence.
    pub fingerprint: RuntimePackageFingerprint,
    /// Count of provider projection items observed or included in this
    /// record.
    pub provider_projection_count: usize,
    /// Count of executable route items observed or included in this record.
    pub executable_route_count: usize,
    /// Count of reserved inactive items observed or included in this record.
    pub reserved_inactive_count: usize,
    /// Count of catalog items observed or included in this record.
    pub catalog_count: usize,
    /// Count of sidecar items observed or included in this record.
    pub sidecar_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the fingerprint input manifest portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct FingerprintInputManifest {
    /// Algorithm name used for hashing or fingerprint generation.
    pub algorithm: String,
    /// Version of the canonical schema format used for fingerprinting.
    /// Use it to detect incompatible fingerprint inputs.
    pub canonical_schema_version: u16,
    /// Readiness state for a capability or package feature.
    /// Launch and package validation use it to distinguish active, reserved, and blocked
    /// surfaces.
    pub readiness_profile: ReadinessProfile,
    /// Fingerprint input groups included for this readiness profile.
    pub included_groups: Vec<FingerprintInputGroup>,
    /// Fingerprint input groups deliberately excluded for this readiness
    /// profile.
    pub excluded_groups: Vec<FingerprintExclusionGroup>,
    /// Readiness status for reserved feature capabilities.
    pub reserved_feature_status: Vec<ReservedFeatureFingerprintStatus>,
}

impl FingerprintInputManifest {
    /// Returns p0 text for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
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
/// Enumerates the finite readiness profile cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ReadinessProfile {
    /// Use this variant when the contract needs to represent p0 text; selecting it has no side effect by itself.
    P0Text,
    /// Use this variant when the contract needs to represent p1 typed output; selecting it has no side effect by itself.
    P1TypedOutput,
    /// Use this variant when the contract needs to represent p2 side effects; selecting it has no side effect by itself.
    P2SideEffects,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite fingerprint input group cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum FingerprintInputGroup {
    /// Use this variant when the contract needs to represent agent; selecting it has no side effect by itself.
    Agent,
    /// Use this variant when the contract needs to represent provider route; selecting it has no side effect by itself.
    ProviderRoute,
    /// Use this variant when the contract needs to represent output contracts; selecting it has no side effect by itself.
    OutputContracts,
    /// Use this variant when the contract needs to represent output sinks; selecting it has no side effect by itself.
    OutputSinks,
    /// Use this variant when the contract needs to represent capabilities; selecting it has no side effect by itself.
    Capabilities,
    /// Use this variant when the contract needs to represent sidecars; selecting it has no side effect by itself.
    Sidecars,
    /// Use this variant when the contract needs to represent isolation requirements; selecting it has no side effect by itself.
    IsolationRequirements,
    /// Use this variant when the contract needs to represent catalogs; selecting it has no side effect by itself.
    Catalogs,
    /// Use this variant when the contract needs to represent child lifecycle; selecting it has no side effect by itself.
    ChildLifecycle,
    /// Use this variant when the contract needs to represent policies; selecting it has no side effect by itself.
    Policies,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite fingerprint exclusion group cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum FingerprintExclusionGroup {
    /// Use this variant when the contract needs to represent run ids; selecting it has no side effect by itself.
    RunIds,
    /// Use this variant when the contract needs to represent event ids; selecting it has no side effect by itself.
    EventIds,
    /// Use this variant when the contract needs to represent timestamps; selecting it has no side effect by itself.
    Timestamps,
    /// Use this variant when the contract needs to represent adapter health; selecting it has no side effect by itself.
    AdapterHealth,
    /// Use this variant when the contract needs to represent temporary paths; selecting it has no side effect by itself.
    TemporaryPaths,
    /// Use this variant when the contract needs to represent telemetry sink health; selecting it has no side effect by itself.
    TelemetrySinkHealth,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the reserved feature fingerprint status portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ReservedFeatureFingerprintStatus {
    /// Stable capability identifier used for package projection and
    /// executable routing.
    pub capability_id: CapabilityId,
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: CapabilityKind,
    /// Implementation owner responsible for this capability surface.
    /// Use it to route follow-up work and validation ownership.
    pub owner_role: String,
    /// Finite status for this record or lifecycle stage.
    pub status: String,
    /// Redacted explanation for a denial, failure, status, or package delta.
    pub reason: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the volatile runtime fields portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct VolatileRuntimeFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Event identifier used to correlate live events with journal or replay
    /// evidence.
    pub event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// timestamp ms duration in milliseconds.
    pub timestamp_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Adapter health snapshot used to decide whether host support is
    /// available.
    pub adapter_health: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Temporary host path used only for diagnostics or adapter handoff; it
    /// should not become durable SDK authority.
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
