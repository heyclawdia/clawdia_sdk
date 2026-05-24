use core::fmt;
use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize, de::Error as DeError};

use crate::{
    capability::PackageSidecarRef,
    domain::{
        AgentError, ContentRef, DedupeKey, DestinationKind, DestinationRef, EffectId, EntityKind,
        EntityRef, IdValidationError, IdempotencyKey, PolicyRef, RunId, SourceKind, SourceRef,
    },
    effect::{EffectIntent, EffectKind},
    ids::validate_identifier,
    package::RuntimePackageFingerprint,
};

pub const ISOLATION_REQUIREMENT_SCHEMA_VERSION: u16 = 1;

macro_rules! isolation_id {
    ($name:ident, $debug:literal) => {
        #[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self::try_new(value).expect(concat!(stringify!($name), " must be valid"))
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

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
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

isolation_id!(ExecutionEnvironmentId, "ExecutionEnvironmentId");
isolation_id!(IsolationRequirementRef, "IsolationRequirementRef");
isolation_id!(IsolationRuntimeRef, "IsolationRuntimeRef");
isolation_id!(IsolationSessionId, "IsolationSessionId");
isolation_id!(IsolationSessionRef, "IsolationSessionRef");
isolation_id!(IsolationAdapterSessionRef, "IsolationAdapterSessionRef");
isolation_id!(IsolationCapabilityReportRef, "IsolationCapabilityReportRef");
isolation_id!(ImageRef, "ImageRef");
isolation_id!(RootfsRef, "RootfsRef");
isolation_id!(MountRef, "MountRef");
isolation_id!(NetworkNamespaceRef, "NetworkNamespaceRef");
isolation_id!(SecretRef, "SecretRef");
isolation_id!(SecretMountRef, "SecretMountRef");
isolation_id!(PreparedEnvironmentRef, "PreparedEnvironmentRef");
isolation_id!(IsolatedProcessId, "IsolatedProcessId");
isolation_id!(IsolatedProcessRef, "IsolatedProcessRef");
isolation_id!(ProcessIoStreamRef, "ProcessIoStreamRef");
isolation_id!(ProcessStatsSnapshotRef, "ProcessStatsSnapshotRef");
isolation_id!(CleanupPlanRef, "CleanupPlanRef");
isolation_id!(ReclaimTicketRef, "ReclaimTicketRef");
isolation_id!(ChildArtifactId, "ChildArtifactId");
isolation_id!(RunChildLifecyclePolicyRef, "RunChildLifecyclePolicyRef");
isolation_id!(RuntimePackageSidecarId, "RuntimePackageSidecarId");
isolation_id!(PolicyDecisionRef, "PolicyDecisionRef");

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationClass {
    HostProcess,
    Sandbox,
    Container,
    LightweightVm,
    RemoteSandbox,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationRequirement {
    pub minimum_class: IsolationClass,
    pub trust: IsolationTrustRequirement,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preferred_adapters: Vec<IsolationRuntimeRef>,
    pub fallback: IsolationFallback,
    pub required_capabilities: IsolationCapabilitySet,
}

impl IsolationRequirement {
    pub fn at_least(minimum_class: IsolationClass) -> Self {
        Self {
            minimum_class,
            trust: IsolationTrustRequirement::any(),
            preferred_adapters: Vec::new(),
            fallback: IsolationFallback::Deny,
            required_capabilities: IsolationCapabilitySet::default(),
        }
    }

    pub fn prefer(mut self, adapter_ref: impl Into<IsolationRuntimeRef>) -> Self {
        self.preferred_adapters.push(adapter_ref.into());
        self
    }

    pub fn require_capabilities(
        mut self,
        capabilities: impl IntoIterator<Item = IsolationCapability>,
    ) -> Self {
        self.required_capabilities = self.required_capabilities.with_all(capabilities);
        self
    }

    pub fn require_locality(mut self) -> Self {
        self.trust.locality = LocalityRequirement::Local;
        self
    }

    pub fn require_secret_isolation(mut self) -> Self {
        self.trust.secret_isolation = SecretIsolationRequirement::Required;
        self
    }

    pub fn allow_downgrade(
        mut self,
        accepted_classes: impl IntoIterator<Item = IsolationClass>,
        accepted_capability_downgrades: impl IntoIterator<Item = IsolationCapability>,
        accepted_trust_downgrades: impl IntoIterator<Item = IsolationTrustField>,
        required_policy_refs: impl IntoIterator<Item = PolicyRef>,
    ) -> Self {
        self.fallback = IsolationFallback::AllowIfPackageAndPolicyApprove {
            accepted_classes: accepted_classes.into_iter().collect(),
            accepted_capability_downgrades: accepted_capability_downgrades.into_iter().collect(),
            accepted_trust_downgrades: accepted_trust_downgrades.into_iter().collect(),
            required_policy_refs: required_policy_refs.into_iter().collect(),
        };
        self
    }

    pub fn fallback_test_only_host_process(mut self) -> Self {
        self.fallback = IsolationFallback::TestOnlyHostProcess;
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationFallback {
    Deny,
    AllowIfPackageAndPolicyApprove {
        accepted_classes: Vec<IsolationClass>,
        accepted_capability_downgrades: Vec<IsolationCapability>,
        accepted_trust_downgrades: Vec<IsolationTrustField>,
        required_policy_refs: Vec<PolicyRef>,
    },
    TestOnlyHostProcess,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationCapabilitySet {
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub capabilities: BTreeSet<IsolationCapability>,
}

impl IsolationCapabilitySet {
    pub fn with_all(mut self, capabilities: impl IntoIterator<Item = IsolationCapability>) -> Self {
        self.capabilities.extend(capabilities);
        self
    }

    pub fn without(mut self, capability: IsolationCapability) -> Self {
        self.capabilities.remove(&capability);
        self
    }

    pub fn contains(&self, capability: &IsolationCapability) -> bool {
        self.capabilities.contains(capability)
    }

    pub fn missing_from(&self, available: &Self) -> Vec<IsolationCapability> {
        let mut missing = self
            .capabilities
            .iter()
            .filter(|capability| !available.contains(capability))
            .cloned()
            .collect::<Vec<_>>();
        missing.sort_by_key(|capability| format!("{capability:?}"));
        missing
    }
}

impl FromIterator<IsolationCapability> for IsolationCapabilitySet {
    fn from_iter<T: IntoIterator<Item = IsolationCapability>>(iter: T) -> Self {
        Self::default().with_all(iter)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationCapability {
    ReadOnlyRoot,
    WritableLayer,
    MountReadOnlyEnforcement,
    SingleFileMountExpansionAudit,
    NoNetworkGuarantee,
    EgressAllowlist,
    SecretIsolation,
    SecretMount,
    SecretRedaction,
    ProcessTimeout,
    ProcessSignal,
    ProcessRlimits,
    ContentRefIo,
    IoRedaction,
    ProcessStats,
    Cleanup,
    Detach,
    Reclaim,
    AuditLog,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationTrustRequirement {
    pub locality: LocalityRequirement,
    pub tenancy: TenantBoundaryRequirement,
    pub auditability: AuditabilityRequirement,
    pub cleanup: CleanupGuaranteeRequirement,
    pub data_residency: DataResidencyRequirement,
    pub secret_isolation: SecretIsolationRequirement,
}

impl IsolationTrustRequirement {
    pub fn any() -> Self {
        Self {
            locality: LocalityRequirement::Any,
            tenancy: TenantBoundaryRequirement::SharedAllowed,
            auditability: AuditabilityRequirement::None,
            cleanup: CleanupGuaranteeRequirement::BestEffort,
            data_residency: DataResidencyRequirement::Any,
            secret_isolation: SecretIsolationRequirement::None,
        }
    }

    pub fn local_dedicated() -> Self {
        Self {
            locality: LocalityRequirement::Local,
            tenancy: TenantBoundaryRequirement::Dedicated,
            auditability: AuditabilityRequirement::Required,
            cleanup: CleanupGuaranteeRequirement::Required,
            data_residency: DataResidencyRequirement::LocalOnly,
            secret_isolation: SecretIsolationRequirement::Required,
        }
    }

    pub fn best_effort_secret_isolation(mut self) -> Self {
        self.secret_isolation = SecretIsolationRequirement::BestEffort;
        self
    }

    pub fn gaps_against(&self, available: &Self) -> Vec<IsolationTrustField> {
        let mut gaps = Vec::new();
        if !locality_satisfies(&self.locality, &available.locality) {
            gaps.push(IsolationTrustField::Locality);
        }
        if !tenancy_satisfies(&self.tenancy, &available.tenancy) {
            gaps.push(IsolationTrustField::Tenancy);
        }
        if !auditability_satisfies(&self.auditability, &available.auditability) {
            gaps.push(IsolationTrustField::Auditability);
        }
        if !cleanup_satisfies(&self.cleanup, &available.cleanup) {
            gaps.push(IsolationTrustField::Cleanup);
        }
        if !data_residency_satisfies(&self.data_residency, &available.data_residency) {
            gaps.push(IsolationTrustField::DataResidency);
        }
        if !secret_satisfies(&self.secret_isolation, &available.secret_isolation) {
            gaps.push(IsolationTrustField::SecretIsolation);
        }
        gaps.sort_by_key(|gap| format!("{gap:?}"));
        gaps
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationTrustField {
    Locality,
    Tenancy,
    Auditability,
    Cleanup,
    DataResidency,
    SecretIsolation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalityRequirement {
    Any,
    Local,
    Remote,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TenantBoundaryRequirement {
    SharedAllowed,
    Dedicated,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditabilityRequirement {
    None,
    BestEffort,
    Required,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupGuaranteeRequirement {
    BestEffort,
    Required,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DataResidencyRequirement {
    Any,
    LocalOnly,
    Region(String),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretIsolationRequirement {
    None,
    BestEffort,
    Required,
}

fn locality_satisfies(required: &LocalityRequirement, available: &LocalityRequirement) -> bool {
    matches!(required, LocalityRequirement::Any) || required == available
}

fn tenancy_satisfies(
    required: &TenantBoundaryRequirement,
    available: &TenantBoundaryRequirement,
) -> bool {
    matches!(required, TenantBoundaryRequirement::SharedAllowed) || required == available
}

fn auditability_satisfies(
    required: &AuditabilityRequirement,
    available: &AuditabilityRequirement,
) -> bool {
    match required {
        AuditabilityRequirement::None => true,
        AuditabilityRequirement::BestEffort => !matches!(available, AuditabilityRequirement::None),
        AuditabilityRequirement::Required => matches!(available, AuditabilityRequirement::Required),
    }
}

fn cleanup_satisfies(
    required: &CleanupGuaranteeRequirement,
    available: &CleanupGuaranteeRequirement,
) -> bool {
    matches!(required, CleanupGuaranteeRequirement::BestEffort) || required == available
}

fn data_residency_satisfies(
    required: &DataResidencyRequirement,
    available: &DataResidencyRequirement,
) -> bool {
    matches!(required, DataResidencyRequirement::Any) || required == available
}

fn secret_satisfies(
    required: &SecretIsolationRequirement,
    available: &SecretIsolationRequirement,
) -> bool {
    match required {
        SecretIsolationRequirement::None => true,
        SecretIsolationRequirement::BestEffort => {
            !matches!(available, SecretIsolationRequirement::None)
        }
        SecretIsolationRequirement::Required => {
            matches!(available, SecretIsolationRequirement::Required)
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationRequirementSnapshot {
    pub requirement_ref: IsolationRequirementRef,
    pub sidecar_ref: PackageSidecarRef,
    pub schema_version: u16,
    pub requirement: IsolationRequirement,
    pub lifecycle_defaults: EnvironmentLifecyclePolicy,
    pub process_defaults: ProcessOwnershipPolicy,
    pub redaction_policy_ref: PolicyRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_downgrade_policy_refs: Vec<PolicyRef>,
    pub cleanup_policy_ref: PolicyRef,
    pub child_lifecycle_policy_ref: RunChildLifecyclePolicyRef,
    pub fingerprint_fields: IsolationFingerprintFields,
}

impl IsolationRequirementSnapshot {
    pub fn from_environment(
        sidecar_id: RuntimePackageSidecarId,
        environment: &ExecutionEnvironment,
        redaction_policy_ref: PolicyRef,
        cleanup_policy_ref: PolicyRef,
        child_lifecycle_policy_ref: PolicyRef,
    ) -> Self {
        let sidecar_ref =
            PackageSidecarRef::new(sidecar_id.as_str(), "isolation_requirement", "v1");
        let child_lifecycle_policy_ref =
            RunChildLifecyclePolicyRef::new(child_lifecycle_policy_ref.as_str());
        let fingerprint_fields = IsolationFingerprintFields::from_environment(
            environment,
            cleanup_policy_ref.clone(),
            child_lifecycle_policy_ref.clone(),
            redaction_policy_ref.clone(),
        );
        Self {
            requirement_ref: environment.requirement_ref.clone(),
            sidecar_ref,
            schema_version: ISOLATION_REQUIREMENT_SCHEMA_VERSION,
            requirement: environment.spec.requirement.clone(),
            lifecycle_defaults: environment.spec.lifecycle.clone(),
            process_defaults: environment.spec.ownership.clone(),
            redaction_policy_ref,
            allowed_downgrade_policy_refs: environment
                .spec
                .requirement
                .fallback
                .required_policy_refs(),
            cleanup_policy_ref,
            child_lifecycle_policy_ref,
            fingerprint_fields,
        }
    }

    pub fn validate(&self) -> Result<(), AgentError> {
        if self.schema_version != ISOLATION_REQUIREMENT_SCHEMA_VERSION {
            return Err(AgentError::contract_violation(
                "unsupported isolation requirement sidecar schema version",
            ));
        }
        if self.redaction_policy_ref.as_str().is_empty()
            || self.cleanup_policy_ref.as_str().is_empty()
            || self.child_lifecycle_policy_ref.as_str().is_empty()
        {
            return Err(AgentError::missing_required_field(
                "isolation_requirement.policy_refs",
            ));
        }
        Ok(())
    }
}

impl IsolationFallback {
    pub fn required_policy_refs(&self) -> Vec<PolicyRef> {
        match self {
            Self::AllowIfPackageAndPolicyApprove {
                required_policy_refs,
                ..
            } => required_policy_refs.clone(),
            Self::Deny | Self::TestOnlyHostProcess => Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationFingerprintFields {
    pub minimum_class: IsolationClass,
    pub trust: IsolationTrustRequirement,
    pub required_capabilities_hash: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preferred_adapter_refs: Vec<IsolationRuntimeRef>,
    pub fallback_policy: IsolationFallback,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_policy_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rootfs_policy_hash: Option<String>,
    pub mount_policy_hash: String,
    pub network_policy_hash: String,
    pub secret_policy_hash: String,
    pub resource_policy_hash: String,
    pub cleanup_policy_ref: PolicyRef,
    pub child_lifecycle_policy_ref: RunChildLifecyclePolicyRef,
    pub redaction_policy_ref: PolicyRef,
}

impl IsolationFingerprintFields {
    fn from_environment(
        environment: &ExecutionEnvironment,
        cleanup_policy_ref: PolicyRef,
        child_lifecycle_policy_ref: RunChildLifecyclePolicyRef,
        redaction_policy_ref: PolicyRef,
    ) -> Self {
        Self {
            minimum_class: environment.spec.requirement.minimum_class,
            trust: environment.spec.requirement.trust.clone(),
            required_capabilities_hash: stable_hash(
                &environment.spec.requirement.required_capabilities,
            ),
            preferred_adapter_refs: environment.spec.requirement.preferred_adapters.clone(),
            fallback_policy: environment.spec.requirement.fallback.clone(),
            image_policy_hash: environment.spec.image.as_ref().map(stable_hash),
            rootfs_policy_hash: environment.spec.rootfs.as_ref().map(stable_hash),
            mount_policy_hash: stable_hash(&environment.spec.filesystem),
            network_policy_hash: stable_hash(&environment.spec.network),
            secret_policy_hash: stable_hash(&environment.spec.secrets),
            resource_policy_hash: stable_hash(&environment.spec.resources),
            cleanup_policy_ref,
            child_lifecycle_policy_ref,
            redaction_policy_ref,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExecutionEnvironment {
    pub environment_id: ExecutionEnvironmentId,
    pub requirement_ref: IsolationRequirementRef,
    pub sidecar_ref: PackageSidecarRef,
    pub spec: EnvironmentSpec,
    pub source: SourceRef,
    pub destination: DestinationRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub runtime_package_fingerprint: RuntimePackageFingerprint,
}

impl ExecutionEnvironment {
    pub fn require(requirement: IsolationRequirement) -> ExecutionEnvironmentBuilder {
        ExecutionEnvironmentBuilder::new(requirement)
    }

    pub fn process(
        &self,
        argv: impl IntoIterator<Item = impl Into<String>>,
    ) -> IsolatedProcessSpecBuilder {
        IsolatedProcessSpecBuilder::new(self.spec.ownership.clone(), argv)
    }

    pub fn subject_ref(&self) -> EntityRef {
        EntityRef::new(
            EntityKind::ExecutionEnvironment,
            self.environment_id.as_str(),
        )
    }
}

#[derive(Clone, Debug)]
pub struct ExecutionEnvironmentBuilder {
    spec: EnvironmentSpec,
    source: SourceRef,
    destination: DestinationRef,
    policy_refs: Vec<PolicyRef>,
    runtime_package_fingerprint: RuntimePackageFingerprint,
    sidecar_ref: PackageSidecarRef,
}

impl ExecutionEnvironmentBuilder {
    fn new(requirement: IsolationRequirement) -> Self {
        let environment_id = ExecutionEnvironmentId::new("env.isolation.default");
        Self {
            sidecar_ref: PackageSidecarRef::new(
                "sidecar.isolation.default",
                "isolation_requirement",
                "v1",
            ),
            spec: EnvironmentSpec {
                environment_id: environment_id.clone(),
                kind: ExecutionEnvironmentKind::Sandbox,
                requirement,
                image: None,
                rootfs: None,
                resources: ResourceLimits::default(),
                filesystem: FilesystemIsolationPolicy::no_workspace(),
                network: NetworkIsolationPolicy::Disabled,
                secrets: SecretExposurePolicy::no_ambient(),
                lifecycle: EnvironmentLifecyclePolicy::EphemeralCleanupRequired {
                    cleanup_mode: CleanupMode::Always,
                },
                ownership: ProcessOwnershipPolicy::agent_owned(RunId::new("run.isolation.owner")),
                accepted_adapters: Vec::new(),
                io_policy: ProcessIoPolicy::refs_hashes_and_redacted_summary(),
                stats_policy: ProcessStatsPolicy::default_counters(),
            },
            source: SourceRef::with_kind(SourceKind::Sdk, "source.sdk.isolation"),
            destination: DestinationRef::with_kind(
                DestinationKind::ExternalRuntime,
                "destination.isolation.runtime",
            ),
            policy_refs: Vec::new(),
            runtime_package_fingerprint: RuntimePackageFingerprint(
                "runtime.package.fingerprint.pending".to_string(),
            ),
        }
    }

    pub fn environment_id(mut self, environment_id: impl Into<ExecutionEnvironmentId>) -> Self {
        let environment_id = environment_id.into();
        self.spec.environment_id = environment_id;
        self
    }

    pub fn filesystem(mut self, filesystem: FilesystemIsolationPolicy) -> Self {
        self.spec.filesystem = filesystem;
        self
    }

    pub fn workspace(mut self, workspace_ref: impl Into<String>, mode: WorkspaceMountMode) -> Self {
        self.spec.filesystem = FilesystemIsolationPolicy::workspace(workspace_ref, mode);
        self
    }

    pub fn network(mut self, network: NetworkIsolationPolicy) -> Self {
        self.spec.network = network;
        self
    }

    pub fn secrets(mut self, secrets: SecretExposurePolicy) -> Self {
        self.spec.secrets = secrets;
        self
    }

    pub fn ephemeral(mut self) -> Self {
        self.spec.lifecycle = EnvironmentLifecyclePolicy::EphemeralCleanupRequired {
            cleanup_mode: CleanupMode::Always,
        };
        self
    }

    pub fn source(mut self, source: SourceRef) -> Self {
        self.source = source;
        self
    }

    pub fn destination(mut self, destination: DestinationRef) -> Self {
        self.destination = destination;
        self
    }

    pub fn build(self) -> Result<ExecutionEnvironment, AgentError> {
        if self.spec.environment_id.as_str().is_empty() {
            return Err(AgentError::missing_required_field(
                "execution_environment.environment_id",
            ));
        }
        Ok(ExecutionEnvironment {
            requirement_ref: IsolationRequirementRef::new(format!(
                "isolation.requirement.{}",
                self.spec.environment_id.as_str()
            )),
            sidecar_ref: self.sidecar_ref,
            environment_id: self.spec.environment_id.clone(),
            spec: self.spec,
            source: self.source,
            destination: self.destination,
            policy_refs: self.policy_refs,
            runtime_package_fingerprint: self.runtime_package_fingerprint,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentSpec {
    pub environment_id: ExecutionEnvironmentId,
    pub kind: ExecutionEnvironmentKind,
    pub requirement: IsolationRequirement,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rootfs: Option<RootfsRequest>,
    pub resources: ResourceLimits,
    pub filesystem: FilesystemIsolationPolicy,
    pub network: NetworkIsolationPolicy,
    pub secrets: SecretExposurePolicy,
    pub lifecycle: EnvironmentLifecyclePolicy,
    pub ownership: ProcessOwnershipPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accepted_adapters: Vec<IsolationAdapterRequirement>,
    pub io_policy: ProcessIoPolicy,
    pub stats_policy: ProcessStatsPolicy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionEnvironmentKind {
    HostProcess,
    Sandbox,
    Container,
    LightweightVm,
    RemoteSandbox,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ImageRequest {
    pub image_ref: ImageRef,
    pub expected_digest: Option<String>,
    pub expected_architecture: Option<String>,
    pub credential_alias_ref: Option<SecretRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootfsRequest {
    pub rootfs_ref: RootfsRef,
    pub read_only: bool,
    pub writable_layer_ref: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResourceLimits {
    pub cpus: u16,
    pub memory_mb: u64,
    pub timeout_ms: u64,
    pub process_count: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpus: 1,
            memory_mb: 1024,
            timeout_ms: 120_000,
            process_count: 32,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FilesystemIsolationPolicy {
    pub root: RootFilesystemMode,
    pub workspace: WorkspaceMountPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mounts: Vec<MountPolicy>,
    pub symlink_policy: SymlinkPolicy,
    pub single_file_mount_expansion: SingleFileMountExpansionPolicy,
}

impl FilesystemIsolationPolicy {
    pub fn no_workspace() -> Self {
        Self {
            root: RootFilesystemMode::ReadOnly,
            workspace: WorkspaceMountPolicy {
                workspace_ref: None,
                mode: WorkspaceMountMode::None,
                mount_ref: None,
                destination_path: None,
            },
            mounts: Vec::new(),
            symlink_policy: SymlinkPolicy::DenyEscapes,
            single_file_mount_expansion: SingleFileMountExpansionPolicy::AuditExpandedParent,
        }
    }

    pub fn workspace(workspace_ref: impl Into<String>, mode: WorkspaceMountMode) -> Self {
        Self {
            workspace: WorkspaceMountPolicy {
                workspace_ref: Some(workspace_ref.into()),
                mode,
                mount_ref: Some(MountRef::new("mount.workspace.primary")),
                destination_path: Some("/workspace".to_string()),
            },
            ..Self::no_workspace()
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RootFilesystemMode {
    ReadOnly,
    WritableLayer,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceMountPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_ref: Option<String>,
    pub mode: WorkspaceMountMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mount_ref: Option<MountRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_path: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceMountMode {
    None,
    Snapshot,
    LiveReadOnly,
    LiveWritable,
    Scratch,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MountPolicy {
    pub mount_ref: MountRef,
    pub source_alias: String,
    pub destination_path: String,
    pub mode: MountMode,
    pub expansion_audit: MountExpansionAudit,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MountMode {
    ReadOnly,
    Writable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MountExpansionAudit {
    pub single_file_mount: bool,
    pub expanded_parent_alias: Option<String>,
    pub symlink_resolution: String,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SymlinkPolicy {
    DenyEscapes,
    FollowWithinAlias,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SingleFileMountExpansionPolicy {
    AuditExpandedParent,
    DenySingleFile,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkIsolationPolicy {
    Disabled,
    Allowlist { hosts: Vec<String> },
    EgressScoped { rules: Vec<String> },
    SocketRelay { relay_refs: Vec<String> },
    ExposedPorts { ports: Vec<u16> },
    AdapterDefinedDenied,
}

impl NetworkIsolationPolicy {
    pub fn summary_key(&self) -> String {
        match self {
            Self::Disabled => "disabled".to_string(),
            Self::Allowlist { hosts } => format!("allowlist:{}", hosts.join(",")),
            Self::EgressScoped { rules } => format!("egress:{}", rules.join(",")),
            Self::SocketRelay { relay_refs } => format!("socket_relay:{}", relay_refs.join(",")),
            Self::ExposedPorts { ports } => format!(
                "ports:{}",
                ports
                    .iter()
                    .map(u16::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::AdapterDefinedDenied => "adapter_defined_denied".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SecretExposurePolicy {
    pub ambient: AmbientSecretPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secret_mounts: Vec<SecretMountPolicy>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env_secrets: Vec<SecretEnvPolicy>,
    pub teardown_required: bool,
}

impl SecretExposurePolicy {
    pub fn no_ambient() -> Self {
        Self {
            ambient: AmbientSecretPolicy::Denied,
            secret_mounts: Vec::new(),
            env_secrets: Vec::new(),
            teardown_required: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AmbientSecretPolicy {
    Denied,
    ExplicitRefsOnly,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SecretMountPolicy {
    pub secret_ref: SecretRef,
    pub mount_ref: SecretMountRef,
    pub destination: DestinationRef,
    pub redaction_policy_ref: PolicyRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SecretEnvPolicy {
    pub secret_ref: SecretRef,
    pub env_key: String,
    pub destination: DestinationRef,
    pub redaction_policy_ref: PolicyRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum EnvironmentLifecyclePolicy {
    EphemeralCleanupRequired { cleanup_mode: CleanupMode },
    ReusableIfPolicyAllows { cleanup_mode: CleanupMode },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupMode {
    Always,
    OnSuccess,
    HostPolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProcessOwnershipPolicy {
    pub child_artifact_id: ChildArtifactId,
    pub owner_run_id: RunId,
    pub ownership_class: ProcessOwnershipClass,
    pub on_parent_cancel: ChildShutdownBehavior,
    pub on_parent_complete: ChildShutdownBehavior,
    pub detach_policy: DetachPolicy,
    pub reclaim_policy: ReclaimPolicy,
}

impl ProcessOwnershipPolicy {
    pub fn agent_owned(owner_run_id: RunId) -> Self {
        Self {
            child_artifact_id: ChildArtifactId::new("child.isolation.process"),
            owner_run_id,
            ownership_class: ProcessOwnershipClass::AgentOwned,
            on_parent_cancel: ChildShutdownBehavior::TerminateAfterGrace { grace_ms: 5_000 },
            on_parent_complete: ChildShutdownBehavior::RequireExitOrCleanup,
            detach_policy: DetachPolicy::deny(),
            reclaim_policy: ReclaimPolicy::host_required("policy.reclaim.host-required"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessOwnershipClass {
    AgentOwned,
    HostManaged,
    DetachedByIntent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildShutdownBehavior {
    TerminateAfterGrace { grace_ms: u64 },
    RequireExitOrCleanup,
    PreserveByPolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DetachPolicy {
    pub allowed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_policy_refs: Vec<PolicyRef>,
}

impl DetachPolicy {
    pub fn deny() -> Self {
        Self {
            allowed: false,
            required_policy_refs: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReclaimPolicy {
    pub required: bool,
    pub policy_ref: PolicyRef,
}

impl ReclaimPolicy {
    pub fn host_required(policy_id: impl Into<String>) -> Self {
        Self {
            required: true,
            policy_ref: PolicyRef::new(policy_id),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationAdapterRequirement {
    pub required_class: IsolationClass,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProcessIoPolicy {
    pub stdout: ProcessIoCapturePolicy,
    pub stderr: ProcessIoCapturePolicy,
}

impl ProcessIoPolicy {
    pub fn refs_hashes_and_redacted_summary() -> Self {
        Self {
            stdout: ProcessIoCapturePolicy::summary_hash_and_content_ref(),
            stderr: ProcessIoCapturePolicy::summary_hash_and_content_ref(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProcessIoCapturePolicy {
    pub default_capture: ProcessContentCaptureMode,
    pub max_bytes: u64,
    pub truncation_policy: TruncationPolicy,
    pub content_ref_mode: ContentRefMode,
    pub redaction_policy_ref: PolicyRef,
}

impl ProcessIoCapturePolicy {
    pub fn summary_hash_and_content_ref() -> Self {
        Self {
            default_capture: ProcessContentCaptureMode::RedactedSummary,
            max_bytes: 64 * 1024,
            truncation_policy: TruncationPolicy::TruncateWithMarker,
            content_ref_mode: ContentRefMode::ContentRefIfPolicyAllows,
            redaction_policy_ref: PolicyRef::new("policy.process_io.redacted"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessContentCaptureMode {
    Off,
    MetadataOnly,
    RedactedSummary,
    ContentRef,
    RawContentIfPolicyAllows,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TruncationPolicy {
    TruncateWithMarker,
    FailIfExceeded,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentRefMode {
    None,
    ContentRefIfPolicyAllows,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProcessStatsPolicy {
    pub collect_cpu: bool,
    pub collect_memory: bool,
    pub collect_process_count: bool,
    pub collect_filesystem_bytes: bool,
    pub collect_network_bytes: bool,
    pub collect_exit_status: bool,
}

impl ProcessStatsPolicy {
    pub fn default_counters() -> Self {
        Self {
            collect_cpu: true,
            collect_memory: true,
            collect_process_count: true,
            collect_filesystem_bytes: true,
            collect_network_bytes: true,
            collect_exit_status: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolatedProcessSpec {
    pub process_id: IsolatedProcessId,
    pub argv: Vec<String>,
    pub cwd: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<RedactedEnvVar>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    pub terminal_mode: TerminalMode,
    pub timeout_ms: u64,
    pub rlimits: ResourceLimits,
    pub stdin: StdinPolicy,
    pub stdout_policy: ProcessIoCapturePolicy,
    pub stderr_policy: ProcessIoCapturePolicy,
    pub stats_policy: ProcessStatsPolicy,
    pub ownership: ProcessOwnershipPolicy,
}

impl IsolatedProcessSpec {
    pub fn validate(&self) -> Result<(), AgentError> {
        if self.argv.is_empty() {
            return Err(AgentError::missing_required_field("isolated_process.argv"));
        }
        if self.env.iter().any(|entry| entry.raw_value_present) {
            return Err(AgentError::contract_violation(
                "isolated process environment cannot contain raw secret values",
            ));
        }
        Ok(())
    }

    pub fn redacted_summary(&self) -> String {
        format!(
            "argv_len={} env_keys={} cwd_alias={}",
            self.argv.len(),
            self.env.len(),
            self.cwd
        )
    }

    pub fn effect_intent(&self, environment: &ExecutionEnvironment) -> EffectIntent {
        let mut intent = EffectIntent::new(
            EffectId::new(format!("effect.{}", self.process_id.as_str())),
            EffectKind::IsolatedProcessStart,
            environment.subject_ref(),
            environment.source.clone(),
            "start isolated process with redacted argv and environment",
        );
        intent.destination = Some(environment.destination.clone());
        intent.policy_refs = environment.policy_refs.clone();
        intent.idempotency_key = Some(IdempotencyKey::new(format!(
            "idempotency.{}",
            self.process_id.as_str()
        )));
        intent.dedupe_key = Some(DedupeKey::new(format!(
            "dedupe.{}",
            self.process_id.as_str()
        )));
        intent
    }
}

#[derive(Clone, Debug)]
pub struct IsolatedProcessSpecBuilder {
    spec: IsolatedProcessSpec,
}

impl IsolatedProcessSpecBuilder {
    fn new(
        ownership: ProcessOwnershipPolicy,
        argv: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            spec: IsolatedProcessSpec {
                process_id: IsolatedProcessId::new("process.isolation.contract"),
                argv: argv.into_iter().map(Into::into).collect(),
                cwd: "/workspace".to_string(),
                env: Vec::new(),
                user: None,
                terminal_mode: TerminalMode::Pipe,
                timeout_ms: 120_000,
                rlimits: ResourceLimits::default(),
                stdin: StdinPolicy::Closed,
                stdout_policy: ProcessIoCapturePolicy::summary_hash_and_content_ref(),
                stderr_policy: ProcessIoCapturePolicy::summary_hash_and_content_ref(),
                stats_policy: ProcessStatsPolicy::default_counters(),
                ownership,
            },
        }
    }

    pub fn env_secret(mut self, name: impl Into<String>, secret_ref: impl Into<SecretRef>) -> Self {
        self.spec.env.push(RedactedEnvVar {
            name: name.into(),
            value_ref: Some(secret_ref.into()),
            redacted_value: "[secret-ref]".to_string(),
            raw_value_present: false,
        });
        self
    }

    pub fn build(self) -> Result<IsolatedProcessSpec, AgentError> {
        self.spec.validate()?;
        Ok(self.spec)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RedactedEnvVar {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_ref: Option<SecretRef>,
    pub redacted_value: String,
    pub raw_value_present: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalMode {
    Pipe,
    Pty,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StdinPolicy {
    Closed,
    ContentRef(ContentRef),
}

fn stable_hash(value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let digest = sha2::Sha256::digest(bytes);
    format!(
        "sha256:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

use sha2::Digest;
