//! Runtime-package records and builders. Use these items to describe the immutable
//! per-run package that freezes provider route, capabilities, policies, sidecars,
//! catalogs, and fingerprints. Builders are data-only and must not perform discovery
//! or execution side effects. This file contains the isolation portion of that
//! contract.
//!
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

/// Constant value for the package::isolation contract. Use it to keep
/// SDK records and tests aligned on the same stable value.
pub const ISOLATION_REQUIREMENT_SCHEMA_VERSION: u16 = 1;

macro_rules! isolation_id {
    ($name:ident, $debug:literal) => {
        #[doc = concat!(
            "Typed isolation/package identifier for `",
            stringify!($name),
            "`. Use it to refer to isolation resources without granting ambient runtime power; ",
            "constructing it is data-only and performs no side effects."
        )]
        #[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Creates a new package::isolation value with explicit
            /// caller-provided inputs. This constructor is data-only
            /// and performs no I/O or external side effects.
            ///
            /// # Panics
            ///
            /// Panics if constructor invariants fail, such as invalid identifier
            /// text or constructor-specific bounds. Use a fallible constructor such as
            /// `try_new` when one is available for untrusted input.
            pub fn new(value: impl Into<String>) -> Self {
                Self::try_new(value).expect(concat!(stringify!($name), " must be valid"))
            }

            /// Creates a new package::isolation value after validation.
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
/// Enumerates the finite isolation class cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum IsolationClass {
    /// Use this variant when the contract needs to represent host process; selecting it has no side effect by itself.
    HostProcess,
    /// Use this variant when the contract needs to represent sandbox; selecting it has no side effect by itself.
    Sandbox,
    /// Use this variant when the contract needs to represent container; selecting it has no side effect by itself.
    Container,
    /// Use this variant when the contract needs to represent lightweight vm; selecting it has no side effect by itself.
    LightweightVm,
    /// Use this variant when the contract needs to represent remote sandbox; selecting it has no side effect by itself.
    RemoteSandbox,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the isolation requirement portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct IsolationRequirement {
    /// Classification value for minimum class.
    /// Policy and projection paths use it for finite routing decisions.
    pub minimum_class: IsolationClass,
    /// Trust class used when deciding whether context or capabilities may be
    /// admitted.
    pub trust: IsolationTrustRequirement,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of preferred adapters values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub preferred_adapters: Vec<IsolationRuntimeRef>,
    /// Fallback used by this record or request.
    pub fallback: IsolationFallback,
    /// Required capabilities used by this record or request.
    pub required_capabilities: IsolationCapabilitySet,
}

impl IsolationRequirement {
    /// Returns an updated package::isolation value with at least applied.
    /// This is data construction only and does not execute the configured
    /// behavior.
    pub fn at_least(minimum_class: IsolationClass) -> Self {
        Self {
            minimum_class,
            trust: IsolationTrustRequirement::any(),
            preferred_adapters: Vec::new(),
            fallback: IsolationFallback::Deny,
            required_capabilities: IsolationCapabilitySet::default(),
        }
    }

    /// Returns an updated package::isolation value with prefer applied. This
    /// is data construction only and does not execute the configured
    /// behavior.
    pub fn prefer(mut self, adapter_ref: impl Into<IsolationRuntimeRef>) -> Self {
        self.preferred_adapters.push(adapter_ref.into());
        self
    }

    /// Returns an updated value with require capabilities configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn require_capabilities(
        mut self,
        capabilities: impl IntoIterator<Item = IsolationCapability>,
    ) -> Self {
        self.required_capabilities = self.required_capabilities.with_all(capabilities);
        self
    }

    /// Returns an updated value with require locality configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn require_locality(mut self) -> Self {
        self.trust.locality = LocalityRequirement::Local;
        self
    }

    /// Returns an updated value with require secret isolation configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn require_secret_isolation(mut self) -> Self {
        self.trust.secret_isolation = SecretIsolationRequirement::Required;
        self
    }

    /// Returns an updated package::isolation value with allow downgrade
    /// applied. This is data construction only and does not execute the
    /// configured behavior.
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

    /// Returns an updated value with fallback test only host process configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn fallback_test_only_host_process(mut self) -> Self {
        self.fallback = IsolationFallback::TestOnlyHostProcess;
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite isolation fallback cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum IsolationFallback {
    /// Use this variant when the contract needs to represent deny; selecting it has no side effect by itself.
    Deny,
    /// Use this variant when the contract needs to represent allow if package and policy approve; selecting it has no side effect by itself.
    AllowIfPackageAndPolicyApprove {
        /// Classification selectors for accepted classes.
        /// Policy and projection paths use them for finite routing decisions.
        accepted_classes: Vec<IsolationClass>,
        /// Isolation capabilities the package is willing to downgrade when the ideal
        /// environment is unavailable.
        /// Policy resolution must still approve these downgrades before an adapter is selected.
        accepted_capability_downgrades: Vec<IsolationCapability>,
        /// Isolation trust fields the package is willing to relax when the ideal environment is
        /// unavailable.
        /// Policy resolution must still approve these downgrades before an adapter is selected.
        accepted_trust_downgrades: Vec<IsolationTrustField>,
        /// Typed required policy refs references. Resolving them is separate
        /// from constructing this record.
        required_policy_refs: Vec<PolicyRef>,
    },
    /// Use this variant when the contract needs to represent test only host process; selecting it has no side effect by itself.
    TestOnlyHostProcess,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the isolation capability set portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct IsolationCapabilitySet {
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    /// Capabilities frozen into the package or returned by an adapter health
    /// check.
    pub capabilities: BTreeSet<IsolationCapability>,
}

impl IsolationCapabilitySet {
    /// Returns this value with its all setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_all(mut self, capabilities: impl IntoIterator<Item = IsolationCapability>) -> Self {
        self.capabilities.extend(capabilities);
        self
    }

    /// Returns an updated package::isolation value with without applied. This
    /// is data construction only and does not execute the configured
    /// behavior.
    pub fn without(mut self, capability: IsolationCapability) -> Self {
        self.capabilities.remove(&capability);
        self
    }

    /// Reads the stored contains without registry or runtime work.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn contains(&self, capability: &IsolationCapability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Returns an updated package::isolation value with missing from applied.
    /// This is data construction only and does not execute the configured
    /// behavior.
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
/// Enumerates the finite isolation capability cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum IsolationCapability {
    /// Use this variant when the contract needs to represent read only root; selecting it has no side effect by itself.
    ReadOnlyRoot,
    /// Use this variant when the contract needs to represent writable layer; selecting it has no side effect by itself.
    WritableLayer,
    /// Use this variant when the contract needs to represent mount read only enforcement; selecting it has no side effect by itself.
    MountReadOnlyEnforcement,
    /// Use this variant when the contract needs to represent single file mount expansion audit; selecting it has no side effect by itself.
    SingleFileMountExpansionAudit,
    /// Use this variant when the contract needs to represent no network guarantee; selecting it has no side effect by itself.
    NoNetworkGuarantee,
    /// Use this variant when the contract needs to represent egress allowlist; selecting it has no side effect by itself.
    EgressAllowlist,
    /// Use this variant when the contract needs to represent secret isolation; selecting it has no side effect by itself.
    SecretIsolation,
    /// Use this variant when the contract needs to represent secret mount; selecting it has no side effect by itself.
    SecretMount,
    /// Use this variant when the contract needs to represent secret redaction; selecting it has no side effect by itself.
    SecretRedaction,
    /// Use this variant when the contract needs to represent process timeout; selecting it has no side effect by itself.
    ProcessTimeout,
    /// Use this variant when the contract needs to represent process signal; selecting it has no side effect by itself.
    ProcessSignal,
    /// Use this variant when the contract needs to represent process rlimits; selecting it has no side effect by itself.
    ProcessRlimits,
    /// Use this variant when the contract needs to represent content ref io; selecting it has no side effect by itself.
    ContentRefIo,
    /// Use this variant when the contract needs to represent io redaction; selecting it has no side effect by itself.
    IoRedaction,
    /// Use this variant when the contract needs to represent process stats; selecting it has no side effect by itself.
    ProcessStats,
    /// Use this variant when the contract needs to represent cleanup; selecting it has no side effect by itself.
    Cleanup,
    /// Use this variant when the contract needs to represent detach; selecting it has no side effect by itself.
    Detach,
    /// Use this variant when the contract needs to represent reclaim; selecting it has no side effect by itself.
    Reclaim,
    /// Use this variant when the contract needs to represent audit log; selecting it has no side effect by itself.
    AuditLog,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the isolation trust requirement portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct IsolationTrustRequirement {
    /// Locality used by this record or request.
    pub locality: LocalityRequirement,
    /// Tenancy used by this record or request.
    pub tenancy: TenantBoundaryRequirement,
    /// Auditability used by this record or request.
    pub auditability: AuditabilityRequirement,
    /// Cleanup used by this record or request.
    pub cleanup: CleanupGuaranteeRequirement,
    /// Data residency used by this record or request.
    pub data_residency: DataResidencyRequirement,
    /// Secret isolation used by this record or request.
    pub secret_isolation: SecretIsolationRequirement,
}

impl IsolationTrustRequirement {
    /// Returns any for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
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

    /// Returns local dedicated for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
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

    /// Returns an updated value with best effort secret isolation configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn best_effort_secret_isolation(mut self) -> Self {
        self.secret_isolation = SecretIsolationRequirement::BestEffort;
        self
    }

    /// Returns the gaps against currently held by this value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
/// Enumerates the finite isolation trust field cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum IsolationTrustField {
    /// Use this variant when the contract needs to represent locality; selecting it has no side effect by itself.
    Locality,
    /// Use this variant when the contract needs to represent tenancy; selecting it has no side effect by itself.
    Tenancy,
    /// Use this variant when the contract needs to represent auditability; selecting it has no side effect by itself.
    Auditability,
    /// Use this variant when the contract needs to represent cleanup; selecting it has no side effect by itself.
    Cleanup,
    /// Use this variant when the contract needs to represent data residency; selecting it has no side effect by itself.
    DataResidency,
    /// Use this variant when the contract needs to represent secret isolation; selecting it has no side effect by itself.
    SecretIsolation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite locality requirement cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum LocalityRequirement {
    /// Use this variant when the contract needs to represent any; selecting it has no side effect by itself.
    Any,
    /// Use this variant when the contract needs to represent local; selecting it has no side effect by itself.
    Local,
    /// Use this variant when the contract needs to represent remote; selecting it has no side effect by itself.
    Remote,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite tenant boundary requirement cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum TenantBoundaryRequirement {
    /// Use this variant when the contract needs to represent shared allowed; selecting it has no side effect by itself.
    SharedAllowed,
    /// Use this variant when the contract needs to represent dedicated; selecting it has no side effect by itself.
    Dedicated,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite auditability requirement cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum AuditabilityRequirement {
    /// Use this variant when the contract needs to represent none; selecting it has no side effect by itself.
    None,
    /// Use this variant when the contract needs to represent best effort; selecting it has no side effect by itself.
    BestEffort,
    /// Use this variant when the contract needs to represent required; selecting it has no side effect by itself.
    Required,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite cleanup guarantee requirement cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum CleanupGuaranteeRequirement {
    /// Use this variant when the contract needs to represent best effort; selecting it has no side effect by itself.
    BestEffort,
    /// Use this variant when the contract needs to represent required; selecting it has no side effect by itself.
    Required,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite data residency requirement cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum DataResidencyRequirement {
    /// Use this variant when the contract needs to represent any; selecting it has no side effect by itself.
    Any,
    /// Use this variant when the contract needs to represent local only; selecting it has no side effect by itself.
    LocalOnly,
    /// Use this variant when the contract needs to represent region; selecting it has no side effect by itself.
    Region(String),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite secret isolation requirement cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum SecretIsolationRequirement {
    /// Use this variant when the contract needs to represent none; selecting it has no side effect by itself.
    None,
    /// Use this variant when the contract needs to represent best effort; selecting it has no side effect by itself.
    BestEffort,
    /// Use this variant when the contract needs to represent required; selecting it has no side effect by itself.
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
/// Describes the isolation requirement snapshot portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct IsolationRequirementSnapshot {
    /// Typed requirement ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub requirement_ref: IsolationRequirementRef,
    /// Typed sidecar ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub sidecar_ref: PackageSidecarRef,
    /// Wire schema version used for compatibility checks.
    pub schema_version: u16,
    /// Requirement used by this record or request.
    pub requirement: IsolationRequirement,
    /// Lifecycle defaults used by this record or request.
    pub lifecycle_defaults: EnvironmentLifecyclePolicy,
    /// Process defaults used by this record or request.
    pub process_defaults: ProcessOwnershipPolicy,
    /// Typed redaction policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub redaction_policy_ref: PolicyRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy decisions that authorize isolation downgrade choices for this package.
    /// Use them as audit evidence when an adapter starts with weaker capabilities or trust than
    /// requested.
    pub allowed_downgrade_policy_refs: Vec<PolicyRef>,
    /// Typed cleanup policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub cleanup_policy_ref: PolicyRef,
    /// Typed child lifecycle policy ref reference. Resolving or executing it
    /// is a separate policy-gated step.
    pub child_lifecycle_policy_ref: RunChildLifecyclePolicyRef,
    /// Deterministic fingerprint fields used for stale checks, package
    /// evidence, or replay comparisons.
    pub fingerprint_fields: IsolationFingerprintFields,
}

impl IsolationRequirementSnapshot {
    /// Constructs this value from environment. Use it when adapting
    /// canonical SDK records without introducing a second behavior
    /// path.
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

    /// Validates the package::isolation invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
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
    /// Returns required policy refs for callers that need to inspect the contract state.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
/// Describes the isolation fingerprint fields portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct IsolationFingerprintFields {
    /// Classification value for minimum class.
    /// Policy and projection paths use it for finite routing decisions.
    pub minimum_class: IsolationClass,
    /// Trust class used when deciding whether context or capabilities may be
    /// admitted.
    pub trust: IsolationTrustRequirement,
    /// Deterministic required capabilities hash used for stale checks,
    /// package evidence, or replay comparisons.
    pub required_capabilities_hash: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed preferred adapter refs references. Resolving them is separate
    /// from constructing this record.
    pub preferred_adapter_refs: Vec<IsolationRuntimeRef>,
    /// Fallback policy used by this record or request.
    pub fallback_policy: IsolationFallback,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Deterministic image policy hash used for stale checks, package
    /// evidence, or replay comparisons.
    pub image_policy_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Deterministic rootfs policy hash used for stale checks, package
    /// evidence, or replay comparisons.
    pub rootfs_policy_hash: Option<String>,
    /// Deterministic mount policy hash used for stale checks, package
    /// evidence, or replay comparisons.
    pub mount_policy_hash: String,
    /// Deterministic network policy hash used for stale checks, package
    /// evidence, or replay comparisons.
    pub network_policy_hash: String,
    /// Deterministic secret policy hash used for stale checks, package
    /// evidence, or replay comparisons.
    pub secret_policy_hash: String,
    /// Deterministic resource policy hash used for stale checks, package
    /// evidence, or replay comparisons.
    pub resource_policy_hash: String,
    /// Typed cleanup policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub cleanup_policy_ref: PolicyRef,
    /// Typed child lifecycle policy ref reference. Resolving or executing it
    /// is a separate policy-gated step.
    pub child_lifecycle_policy_ref: RunChildLifecyclePolicyRef,
    /// Typed redaction policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
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
/// Describes the execution environment portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ExecutionEnvironment {
    /// Stable environment id used for typed lineage, lookup, or dedupe.
    pub environment_id: ExecutionEnvironmentId,
    /// Typed requirement ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub requirement_ref: IsolationRequirementRef,
    /// Typed sidecar ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub sidecar_ref: PackageSidecarRef,
    /// Spec used by this record or request.
    pub spec: EnvironmentSpec,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: DestinationRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: RuntimePackageFingerprint,
}

impl ExecutionEnvironment {
    /// Returns an updated package::isolation value with require applied. This
    /// is data construction only and does not execute the configured
    /// behavior.
    pub fn require(requirement: IsolationRequirement) -> ExecutionEnvironmentBuilder {
        ExecutionEnvironmentBuilder::new(requirement)
    }

    /// Returns an updated package::isolation value with process applied. This
    /// is data construction only and does not execute the configured
    /// behavior.
    pub fn process(
        &self,
        argv: impl IntoIterator<Item = impl Into<String>>,
    ) -> IsolatedProcessSpecBuilder {
        IsolatedProcessSpecBuilder::new(self.spec.ownership.clone(), argv)
    }

    /// Returns an updated package::isolation value with subject ref applied.
    /// This is data construction only and does not execute the configured
    /// behavior.
    pub fn subject_ref(&self) -> EntityRef {
        EntityRef::new(
            EntityKind::ExecutionEnvironment,
            self.environment_id.as_str(),
        )
    }
}

#[derive(Clone, Debug)]
/// Describes the execution environment builder portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
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

    /// Returns an updated package::isolation value with environment id
    /// applied. This is data construction only and does not execute the
    /// configured behavior.
    pub fn environment_id(mut self, environment_id: impl Into<ExecutionEnvironmentId>) -> Self {
        let environment_id = environment_id.into();
        self.spec.environment_id = environment_id;
        self
    }

    /// Returns an updated package::isolation value with filesystem applied.
    /// This is data construction only and does not execute the configured
    /// behavior.
    pub fn filesystem(mut self, filesystem: FilesystemIsolationPolicy) -> Self {
        self.spec.filesystem = filesystem;
        self
    }

    /// Returns an updated package::isolation value with workspace applied.
    /// This is data construction only and does not execute the configured
    /// behavior.
    pub fn workspace(mut self, workspace_ref: impl Into<String>, mode: WorkspaceMountMode) -> Self {
        self.spec.filesystem = FilesystemIsolationPolicy::workspace(workspace_ref, mode);
        self
    }

    /// Returns an updated package::isolation value with network applied. This
    /// is data construction only and does not execute the configured
    /// behavior.
    pub fn network(mut self, network: NetworkIsolationPolicy) -> Self {
        self.spec.network = network;
        self
    }

    /// Returns an updated package::isolation value with secrets applied. This
    /// is data construction only and does not execute the configured
    /// behavior.
    pub fn secrets(mut self, secrets: SecretExposurePolicy) -> Self {
        self.spec.secrets = secrets;
        self
    }

    /// Returns an updated package::isolation value with ephemeral applied.
    /// This is data construction only and does not execute the configured
    /// behavior.
    pub fn ephemeral(mut self) -> Self {
        self.spec.lifecycle = EnvironmentLifecyclePolicy::EphemeralCleanupRequired {
            cleanup_mode: CleanupMode::Always,
        };
        self
    }

    /// Returns an updated package::isolation value with source applied. This
    /// is data construction only and does not execute the configured
    /// behavior.
    pub fn source(mut self, source: SourceRef) -> Self {
        self.source = source;
        self
    }

    /// Returns an updated package::isolation value with destination applied.
    /// This is data construction only and does not execute the configured
    /// behavior.
    pub fn destination(mut self, destination: DestinationRef) -> Self {
        self.destination = destination;
        self
    }

    /// Finishes builder validation and returns the configured value.
    /// This is data-only unless the surrounding builder explicitly
    /// documents adapter or store access.
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
/// Describes the environment spec portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct EnvironmentSpec {
    /// Stable environment id used for typed lineage, lookup, or dedupe.
    pub environment_id: ExecutionEnvironmentId,
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: ExecutionEnvironmentKind,
    /// Requirement used by this record or request.
    pub requirement: IsolationRequirement,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional image value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub image: Option<ImageRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional rootfs value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub rootfs: Option<RootfsRequest>,
    /// Resources used by this record or request.
    pub resources: ResourceLimits,
    /// Filesystem used by this record or request.
    pub filesystem: FilesystemIsolationPolicy,
    /// Whether the request asks for network access. Host sandbox policy is
    /// still authoritative.
    pub network: NetworkIsolationPolicy,
    /// Secrets used by this record or request.
    pub secrets: SecretExposurePolicy,
    /// Lifecycle used by this record or request.
    pub lifecycle: EnvironmentLifecyclePolicy,
    /// Ownership used by this record or request.
    pub ownership: ProcessOwnershipPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of accepted adapters values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub accepted_adapters: Vec<IsolationAdapterRequirement>,
    /// Io policy used by this record or request.
    pub io_policy: ProcessIoPolicy,
    /// Stats policy used by this record or request.
    pub stats_policy: ProcessStatsPolicy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite execution environment kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ExecutionEnvironmentKind {
    /// Use this variant when the contract needs to represent host process; selecting it has no side effect by itself.
    HostProcess,
    /// Use this variant when the contract needs to represent sandbox; selecting it has no side effect by itself.
    Sandbox,
    /// Use this variant when the contract needs to represent container; selecting it has no side effect by itself.
    Container,
    /// Use this variant when the contract needs to represent lightweight vm; selecting it has no side effect by itself.
    LightweightVm,
    /// Use this variant when the contract needs to represent remote sandbox; selecting it has no side effect by itself.
    RemoteSandbox,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the image request portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ImageRequest {
    /// Typed image ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub image_ref: ImageRef,
    /// Optional expected digest value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub expected_digest: Option<String>,
    /// Optional expected architecture value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub expected_architecture: Option<String>,
    /// Typed credential alias ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub credential_alias_ref: Option<SecretRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the rootfs request portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct RootfsRequest {
    /// Typed rootfs ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub rootfs_ref: RootfsRef,
    /// Whether read only is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub read_only: bool,
    /// Typed writable layer ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub writable_layer_ref: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the resource limits portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ResourceLimits {
    /// Cpus used by this record or request.
    pub cpus: u16,
    /// Memory mb used by this record or request.
    pub memory_mb: u64,
    /// Timeout budget in milliseconds for the requested operation.
    pub timeout_ms: u64,
    /// Count of process items observed or included in this record.
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
/// Describes the filesystem isolation policy portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct FilesystemIsolationPolicy {
    /// Root used by this record or request.
    pub root: RootFilesystemMode,
    /// Workspace used by this record or request.
    pub workspace: WorkspaceMountPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of mounts values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub mounts: Vec<MountPolicy>,
    /// Symlink policy used by this record or request.
    pub symlink_policy: SymlinkPolicy,
    /// Policy for expanding or denying single-file mounts while preparing isolation mount
    /// plans.
    /// Use it to keep file mounts deterministic and to avoid surprising directory exposure.
    pub single_file_mount_expansion: SingleFileMountExpansionPolicy,
}

impl FilesystemIsolationPolicy {
    /// Returns no workspace for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
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

    /// Returns an updated package::isolation value with workspace applied.
    /// This is data construction only and does not execute the configured
    /// behavior.
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
/// Enumerates the finite root filesystem mode cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RootFilesystemMode {
    /// Use this variant when the contract needs to represent read only; selecting it has no side effect by itself.
    ReadOnly,
    /// Use this variant when the contract needs to represent writable layer; selecting it has no side effect by itself.
    WritableLayer,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the workspace mount policy portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct WorkspaceMountPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed workspace ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub workspace_ref: Option<String>,
    /// Mode that selects how this operation or contract should behave.
    /// Callers use it to choose the explicit execution path instead of relying on hidden
    /// defaults.
    pub mode: WorkspaceMountMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed mount ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub mount_ref: Option<MountRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional destination path value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub destination_path: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite workspace mount mode cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum WorkspaceMountMode {
    /// Use this variant when the contract needs to represent none; selecting it has no side effect by itself.
    None,
    /// Use this variant when the contract needs to represent snapshot; selecting it has no side effect by itself.
    Snapshot,
    /// Use this variant when the contract needs to represent live read only; selecting it has no side effect by itself.
    LiveReadOnly,
    /// Use this variant when the contract needs to represent live writable; selecting it has no side effect by itself.
    LiveWritable,
    /// Use this variant when the contract needs to represent scratch; selecting it has no side effect by itself.
    Scratch,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the mount policy portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct MountPolicy {
    /// Typed mount ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub mount_ref: MountRef,
    /// Source alias used by this record or request.
    pub source_alias: String,
    /// Destination path used by this record or request.
    pub destination_path: String,
    /// Mode that selects how this operation or contract should behave.
    /// Callers use it to choose the explicit execution path instead of relying on hidden
    /// defaults.
    pub mode: MountMode,
    /// Expansion audit used by this record or request.
    pub expansion_audit: MountExpansionAudit,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite mount mode cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum MountMode {
    /// Use this variant when the contract needs to represent read only; selecting it has no side effect by itself.
    ReadOnly,
    /// Use this variant when the contract needs to represent writable; selecting it has no side effect by itself.
    Writable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the mount expansion audit portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct MountExpansionAudit {
    /// Whether single file mount is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub single_file_mount: bool,
    /// Optional expanded parent alias value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub expanded_parent_alias: Option<String>,
    /// Symlink resolution used by this record or request.
    pub symlink_resolution: String,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite symlink policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum SymlinkPolicy {
    /// Use this variant when the contract needs to represent deny escapes; selecting it has no side effect by itself.
    DenyEscapes,
    /// Use this variant when the contract needs to represent follow within alias; selecting it has no side effect by itself.
    FollowWithinAlias,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite single file mount expansion policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum SingleFileMountExpansionPolicy {
    /// Use this variant when the contract needs to represent audit expanded parent; selecting it has no side effect by itself.
    AuditExpandedParent,
    /// Use this variant when the contract needs to represent deny single file; selecting it has no side effect by itself.
    DenySingleFile,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite network isolation policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum NetworkIsolationPolicy {
    /// Use this variant when the contract needs to represent disabled; selecting it has no side effect by itself.
    Disabled,
    /// Use this variant when the contract needs to represent allowlist; selecting it has no side effect by itself.
    Allowlist {
        /// Network host allowlist entries requested by policy.
        hosts: Vec<String>,
    },
    /// Use this variant when the contract needs to represent egress scoped; selecting it has no side effect by itself.
    EgressScoped {
        /// Network or stream-rule entries requested by policy.
        rules: Vec<String>,
    },
    /// Use this variant when the contract needs to represent socket relay; selecting it has no side effect by itself.
    SocketRelay {
        /// Socket relay references requested by isolation policy.
        relay_refs: Vec<String>,
    },
    /// Use this variant when the contract needs to represent exposed ports; selecting it has no side effect by itself.
    ExposedPorts {
        /// Network ports requested by isolation policy.
        ports: Vec<u16>,
    },
    /// Use this variant when the contract needs to represent adapter defined denied; selecting it has no side effect by itself.
    AdapterDefinedDenied,
}

impl NetworkIsolationPolicy {
    /// Returns summary key for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
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
/// Describes the secret exposure policy portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct SecretExposurePolicy {
    /// Ambient used by this record or request.
    pub ambient: AmbientSecretPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of secret mounts values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub secret_mounts: Vec<SecretMountPolicy>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of env secrets values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub env_secrets: Vec<SecretEnvPolicy>,
    /// Whether teardown required is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub teardown_required: bool,
}

impl SecretExposurePolicy {
    /// Returns an updated package::isolation value with no ambient applied.
    /// This is data construction only and does not execute the configured
    /// behavior.
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
/// Enumerates the finite ambient secret policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum AmbientSecretPolicy {
    /// Use this variant when the contract needs to represent denied; selecting it has no side effect by itself.
    Denied,
    /// Use this variant when the contract needs to represent explicit refs only; selecting it has no side effect by itself.
    ExplicitRefsOnly,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the secret mount policy portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct SecretMountPolicy {
    /// Typed secret ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub secret_ref: SecretRef,
    /// Typed mount ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub mount_ref: SecretMountRef,
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: DestinationRef,
    /// Typed redaction policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub redaction_policy_ref: PolicyRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the secret env policy portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct SecretEnvPolicy {
    /// Typed secret ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub secret_ref: SecretRef,
    /// Env key used by this record or request.
    pub env_key: String,
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: DestinationRef,
    /// Typed redaction policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub redaction_policy_ref: PolicyRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Enumerates the finite environment lifecycle policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum EnvironmentLifecyclePolicy {
    /// Use this variant when the contract needs to represent ephemeral cleanup required; selecting it has no side effect by itself.
    EphemeralCleanupRequired {
        /// Cleanup mode required after an isolated workload or child process
        /// lifecycle step.
        cleanup_mode: CleanupMode,
    },
    /// Use this variant when the contract needs to represent reusable if policy allows; selecting it has no side effect by itself.
    ReusableIfPolicyAllows {
        /// Cleanup mode required after an isolated workload or child process
        /// lifecycle step.
        cleanup_mode: CleanupMode,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite cleanup mode cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum CleanupMode {
    /// Use this variant when the contract needs to represent always; selecting it has no side effect by itself.
    Always,
    /// Use this variant when the contract needs to represent on success; selecting it has no side effect by itself.
    OnSuccess,
    /// Use this variant when the contract needs to represent host policy; selecting it has no side effect by itself.
    HostPolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the process ownership policy portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ProcessOwnershipPolicy {
    /// Stable child artifact id used for typed lineage, lookup, or dedupe.
    pub child_artifact_id: ChildArtifactId,
    /// Stable owner run id used for typed lineage, lookup, or dedupe.
    pub owner_run_id: RunId,
    /// Classification value for ownership class.
    /// Policy and projection paths use it for finite routing decisions.
    pub ownership_class: ProcessOwnershipClass,
    /// On parent cancel used by this record or request.
    pub on_parent_cancel: ChildShutdownBehavior,
    /// On parent complete used by this record or request.
    pub on_parent_complete: ChildShutdownBehavior,
    /// Detach policy used by this record or request.
    pub detach_policy: DetachPolicy,
    /// Reclaim policy used by this record or request.
    pub reclaim_policy: ReclaimPolicy,
}

impl ProcessOwnershipPolicy {
    /// Returns an updated package::isolation value with agent owned applied.
    /// This is data construction only and does not execute the configured
    /// behavior.
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
/// Enumerates the finite process ownership class cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ProcessOwnershipClass {
    /// Use this variant when the contract needs to represent agent owned; selecting it has no side effect by itself.
    AgentOwned,
    /// Use this variant when the contract needs to represent host managed; selecting it has no side effect by itself.
    HostManaged,
    /// Use this variant when the contract needs to represent detached by intent; selecting it has no side effect by itself.
    DetachedByIntent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite child shutdown behavior cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ChildShutdownBehavior {
    /// Use this variant when the contract needs to represent terminate after grace; selecting it has no side effect by itself.
    TerminateAfterGrace {
        /// Grace period in milliseconds before termination or cleanup
        /// escalates.
        grace_ms: u64,
    },
    /// Use this variant when the contract needs to represent require exit or cleanup; selecting it has no side effect by itself.
    RequireExitOrCleanup,
    /// Use this variant when the contract needs to represent preserve by policy; selecting it has no side effect by itself.
    PreserveByPolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the detach policy portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct DetachPolicy {
    /// Allowlist for this policy or contract.
    /// Validation uses it to reject undeclared or policy-denied values.
    pub allowed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed required policy refs references. Resolving them is separate from
    /// constructing this record.
    pub required_policy_refs: Vec<PolicyRef>,
}

impl DetachPolicy {
    /// Returns an updated package::isolation value with deny applied. This is
    /// data construction only and does not execute the configured behavior.
    pub fn deny() -> Self {
        Self {
            allowed: false,
            required_policy_refs: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the reclaim policy portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ReclaimPolicy {
    /// Whether required is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub required: bool,
    /// Policy reference that must be resolved by the host or runtime before
    /// execution.
    pub policy_ref: PolicyRef,
}

impl ReclaimPolicy {
    /// Returns an updated package::isolation value with host required
    /// applied. This is data construction only and does not execute the
    /// configured behavior.
    pub fn host_required(policy_id: impl Into<String>) -> Self {
        Self {
            required: true,
            policy_ref: PolicyRef::new(policy_id),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the isolation adapter requirement portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct IsolationAdapterRequirement {
    /// Classification value for required class.
    /// Policy and projection paths use it for finite routing decisions.
    pub required_class: IsolationClass,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the process io policy portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ProcessIoPolicy {
    /// Captured standard output. Current shell execution captures the full
    /// buffered stream; hosts should add bounds before using it with
    /// untrusted commands.
    pub stdout: ProcessIoCapturePolicy,
    /// Captured standard error. Current shell execution captures the full
    /// buffered stream; hosts should add bounds before using it with
    /// untrusted commands.
    pub stderr: ProcessIoCapturePolicy,
}

impl ProcessIoPolicy {
    /// Computes the stable refs hashes and redacted summary for this
    /// package::isolation value. The computation is deterministic and
    /// side-effect free so it can be used in package, journal, or test
    /// evidence.
    pub fn refs_hashes_and_redacted_summary() -> Self {
        Self {
            stdout: ProcessIoCapturePolicy::summary_hash_and_content_ref(),
            stderr: ProcessIoCapturePolicy::summary_hash_and_content_ref(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the process io capture policy portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ProcessIoCapturePolicy {
    /// Default capture used by this record or request.
    pub default_capture: ProcessContentCaptureMode,
    /// Maximum byte budget the caller requested before truncation or summary
    /// behavior is applied.
    pub max_bytes: u64,
    /// Truncation policy used by this record or request.
    pub truncation_policy: TruncationPolicy,
    /// Content reference associated with this value.
    /// Resolve it through policy-gated content stores instead of embedding raw content.
    pub content_ref_mode: ContentRefMode,
    /// Typed redaction policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub redaction_policy_ref: PolicyRef,
}

impl ProcessIoCapturePolicy {
    /// Computes the stable summary hash and content ref for this
    /// package::isolation value. The computation is deterministic and
    /// side-effect free so it can be used in package, journal, or test
    /// evidence.
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
/// Enumerates the finite process content capture mode cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ProcessContentCaptureMode {
    /// Use this variant when the contract needs to represent off; selecting it has no side effect by itself.
    Off,
    /// Use this variant when the contract needs to represent metadata only; selecting it has no side effect by itself.
    MetadataOnly,
    /// Use this variant when the contract needs to represent redacted summary; selecting it has no side effect by itself.
    RedactedSummary,
    /// Use this variant when the contract needs to represent content ref; selecting it has no side effect by itself.
    ContentRef,
    /// Use this variant when the contract needs to represent raw content if policy allows; selecting it has no side effect by itself.
    RawContentIfPolicyAllows,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite truncation policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum TruncationPolicy {
    /// Use this variant when the contract needs to represent truncate with marker; selecting it has no side effect by itself.
    TruncateWithMarker,
    /// Use this variant when the contract needs to represent fail if exceeded; selecting it has no side effect by itself.
    FailIfExceeded,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite content ref mode cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ContentRefMode {
    /// Use this variant when the contract needs to represent none; selecting it has no side effect by itself.
    None,
    /// Use this variant when the contract needs to represent content ref if policy allows; selecting it has no side effect by itself.
    ContentRefIfPolicyAllows,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the process stats policy portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ProcessStatsPolicy {
    /// Whether collect cpu is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub collect_cpu: bool,
    /// Whether collect memory is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub collect_memory: bool,
    /// Count of collect process items observed or included in this record.
    pub collect_process_count: bool,
    /// collect filesystem bytes used for bounds checks, summaries, or
    /// truncation evidence.
    pub collect_filesystem_bytes: bool,
    /// collect network bytes used for bounds checks, summaries, or truncation
    /// evidence.
    pub collect_network_bytes: bool,
    /// Whether collect exit status is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub collect_exit_status: bool,
}

impl ProcessStatsPolicy {
    /// Builds the default counters value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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
/// Describes the isolated process spec portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct IsolatedProcessSpec {
    /// Stable process id used for typed lineage, lookup, or dedupe.
    pub process_id: IsolatedProcessId,
    /// Command and arguments requested for shell execution. The first element
    /// is the executable path/name.
    pub argv: Vec<String>,
    /// Working directory requested for command execution; hosts must keep it
    /// inside approved bounds.
    pub cwd: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Environment overrides requested for shell execution. Hosts should
    /// treat values as sensitive unless policy says otherwise.
    pub env: Vec<RedactedEnvVar>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional user value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub user: Option<String>,
    /// Terminal mode used by this record or request.
    pub terminal_mode: TerminalMode,
    /// Timeout budget in milliseconds for the requested operation.
    pub timeout_ms: u64,
    /// Rlimits used by this record or request.
    pub rlimits: ResourceLimits,
    /// Stdin used by this record or request.
    pub stdin: StdinPolicy,
    /// Stdout policy used by this record or request.
    pub stdout_policy: ProcessIoCapturePolicy,
    /// Stderr policy used by this record or request.
    pub stderr_policy: ProcessIoCapturePolicy,
    /// Stats policy used by this record or request.
    pub stats_policy: ProcessStatsPolicy,
    /// Ownership used by this record or request.
    pub ownership: ProcessOwnershipPolicy,
}

impl IsolatedProcessSpec {
    /// Validates the package::isolation invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
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

    /// Builds the redacted summary value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn redacted_summary(&self) -> String {
        format!(
            "argv_len={} env_keys={} cwd_alias={}",
            self.argv.len(),
            self.env.len(),
            self.cwd
        )
    }

    /// Returns an updated package::isolation value with effect intent
    /// applied. This is data construction only and does not execute the
    /// configured behavior.
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
/// Describes the isolated process spec builder portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
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

    /// Returns an updated package::isolation value with env secret applied.
    /// This is data construction only and does not execute the configured
    /// behavior.
    pub fn env_secret(mut self, name: impl Into<String>, secret_ref: impl Into<SecretRef>) -> Self {
        self.spec.env.push(RedactedEnvVar {
            name: name.into(),
            value_ref: Some(secret_ref.into()),
            redacted_value: "[secret-ref]".to_string(),
            raw_value_present: false,
        });
        self
    }

    /// Finishes builder validation and returns the configured value.
    /// This is data-only unless the surrounding builder explicitly
    /// documents adapter or store access.
    pub fn build(self) -> Result<IsolatedProcessSpec, AgentError> {
        self.spec.validate()?;
        Ok(self.spec)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the redacted env var portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct RedactedEnvVar {
    /// Human-readable or protocol-visible name for this SDK item.
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed value ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub value_ref: Option<SecretRef>,
    /// Redacted value used by this record or request.
    pub redacted_value: String,
    /// Raw content or raw-content control for this value.
    /// Use it only when policy explicitly allows raw content capture or delivery.
    pub raw_value_present: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite terminal mode cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum TerminalMode {
    /// Use this variant when the contract needs to represent pipe; selecting it has no side effect by itself.
    Pipe,
    /// Use this variant when the contract needs to represent pty; selecting it has no side effect by itself.
    Pty,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite stdin policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum StdinPolicy {
    /// Use this variant when the contract needs to represent closed; selecting it has no side effect by itself.
    Closed,
    /// Use this variant when the contract needs to represent content ref; selecting it has no side effect by itself.
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
