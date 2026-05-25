//! Host adapter boundaries for the SDK core. Use these traits and registries when
//! hosts provide providers, journals, sinks, tools, isolation, extensions, telemetry,
//! or subscriptions. Implementations may perform external side effects and must honor
//! policy, redaction, idempotency, and replay contracts. This file contains the
//! isolation portion of that contract.
//!
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::{
    domain::{AgentError, AgentErrorKind, ContentRef, RetryClassification},
    package_isolation::{
        CleanupPlanRef, ExecutionEnvironment, ImageRef, ImageRequest, IsolatedProcessRef,
        IsolatedProcessSpec, IsolationAdapterSessionRef, IsolationCapabilityReportRef,
        IsolationCapabilitySet, IsolationClass, IsolationRuntimeRef, IsolationSessionRef,
        IsolationTrustRequirement, MountRef, NetworkNamespaceRef, PreparedEnvironmentRef,
        ProcessIoStreamRef, ProcessStatsSnapshotRef, ReclaimTicketRef, RootfsRef, RootfsRequest,
        SecretMountRef,
    },
};

/// Port or behavior contract for isolation runtime. Implementors should
/// preserve policy, redaction, idempotency, and replay expectations
/// from the surrounding module. Implementations may perform side
/// effects only as described by the trait methods.
pub trait IsolationRuntime: Send + Sync {
    /// Returns runtime ref for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    fn runtime_ref(&self) -> &IsolationRuntimeRef;

    /// Returns capability report for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    fn capability_report(&self) -> Result<IsolationCapabilityReport, AgentError>;

    /// Prepares or selects an isolation session for the request.
    /// Implementations allocate or select an isolation session for the request; they may mutate
    /// adapter sandbox state but must not start the process.
    fn prepare_session(
        &self,
        request: SessionPrepareRequest,
    ) -> Result<IsolationSessionRef, AgentError>;

    /// Resolves an image request into adapter-specific image metadata.
    /// Implementations resolve isolation planning data; only process-start methods may launch a
    /// process.
    fn resolve_image(&self, request: ImageResolveRequest) -> Result<ImageResolution, AgentError>;

    /// Prepares the root filesystem for the isolation session.
    /// Implementations materialize or select the requested root filesystem and may touch host
    /// storage or image caches; they must not start the process.
    fn prepare_rootfs(&self, request: RootfsPrepareRequest) -> Result<RootfsRef, AgentError>;

    /// Resolves requested mounts into a mount plan for the environment.
    /// Implementations resolve isolation planning data; only process-start methods may launch a
    /// process.
    fn resolve_mounts(&self, request: MountResolveRequest) -> Result<MountPlan, AgentError>;

    /// Configures or selects the network namespace for the environment.
    /// Implementations resolve isolation planning data; only process-start methods may launch a
    /// process.
    fn configure_network(
        &self,
        request: NetworkPrepareRequest,
    ) -> Result<NetworkNamespaceRef, AgentError>;

    /// Prepares secret mounts or handles for the environment.
    /// Implementations materialize secret mounts or handles for the isolated environment and
    /// must not return raw secret values.
    fn prepare_secrets(
        &self,
        request: SecretPrepareRequest,
    ) -> Result<SecretMaterializationPlan, AgentError>;

    /// Combines prepared isolation pieces into an executable environment.
    /// Implementations combine prepared rootfs, mounts, network, and secrets into an executable
    /// environment handle; they must not start the process.
    fn prepare_environment(
        &self,
        request: EnvironmentPrepareRequest,
    ) -> Result<PreparedEnvironmentRef, AgentError>;

    /// Starts the prepared isolated process through the host adapter.
    /// Implementations may launch a process or container and return process
    /// handles, but journal intent/result recording stays with the runtime.
    fn start_process(&self, request: ProcessStartRequest)
    -> Result<ProcessStartResult, AgentError>;

    /// Reads or writes one bounded I/O frame for an isolated process.
    /// Implementations may touch host process streams, but must preserve
    /// redaction and return stream refs or bounded data according to policy.
    fn stream_io(&self, request: ProcessIoRequest) -> Result<ProcessIoFrame, AgentError>;

    /// Sends a control signal to an already-started isolated process.
    /// Implementations return the observed signal result and leave lifecycle
    /// journal evidence to the runtime.
    fn signal_process(
        &self,
        request: ProcessSignalRequest,
    ) -> Result<ProcessSignalResult, AgentError>;

    /// Collects statistics for an already-started isolated process.
    /// Implementations may query host process/container state and must return
    /// bounded metadata rather than raw process output.
    fn collect_stats(
        &self,
        request: ProcessStatsRequest,
    ) -> Result<ProcessStatsSnapshot, AgentError>;

    /// Cleans up adapter-owned isolation resources for a finished process.
    /// Implementations may remove sessions, mounts, namespaces, or reclaim
    /// tickets selected by the cleanup request.
    fn cleanup(&self, request: CleanupRequest) -> Result<CleanupResult, AgentError>;

    /// Transfers ownership of isolation resources according to a detach plan.
    /// Implementations may leave processes or resources running under a reclaim
    /// ticket, but must not silently discard cleanup responsibility.
    fn detach(&self, request: DetachTransferRequest) -> Result<DetachTransferResult, AgentError>;

    /// Reclaims resources that were previously detached from runtime ownership.
    /// Implementations may stop processes or remove resources referenced by the
    /// reclaim ticket and must report any cleanup failure for repair.
    fn reclaim(&self, request: ReclaimRequest) -> Result<ReclaimResult, AgentError>;
}

#[derive(Clone, Default)]
/// Carries isolation runtime registry data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct IsolationRuntimeRegistry {
    runtimes: BTreeMap<String, Arc<dyn IsolationRuntime>>,
}

impl IsolationRuntimeRegistry {
    /// Creates a new ports::isolation value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds data to this in-memory ports::isolation collection. It does not
    /// perform external I/O, execute tools, or append journals.
    pub fn register(&mut self, runtime: Arc<dyn IsolationRuntime>) -> Result<(), AgentError> {
        let runtime_ref = runtime.runtime_ref().as_str().to_string();
        if runtime_ref.is_empty() {
            return Err(AgentError::missing_required_field(
                "isolation_runtime.runtime_ref",
            ));
        }
        self.runtimes.insert(runtime_ref, runtime);
        Ok(())
    }

    /// Looks up an entry in this local store without registry or runtime work.
    /// This reads the in-memory isolation runtime registry and does not call the adapter.
    pub fn get(&self, runtime_ref: &IsolationRuntimeRef) -> Option<Arc<dyn IsolationRuntime>> {
        self.runtimes.get(runtime_ref.as_str()).cloned()
    }

    /// Returns first for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn first(&self) -> Option<Arc<dyn IsolationRuntime>> {
        self.runtimes.values().next().cloned()
    }

    /// Reports whether this value is empty. The check is pure and does
    /// not mutate SDK or host state.
    pub fn is_empty(&self) -> bool {
        self.runtimes.is_empty()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries isolation capability report data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct IsolationCapabilityReport {
    /// Typed report ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub report_ref: IsolationCapabilityReportRef,
    /// Typed adapter ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub adapter_ref: IsolationRuntimeRef,
    /// Kind discriminator for adapter kind.
    /// Use it to route finite match arms without parsing display text.
    pub adapter_kind: IsolationRuntimeKind,
    /// Version string for this capability, package, or protocol surface.
    /// Use it for compatibility checks during package or adapter resolution.
    pub adapter_version: String,
    /// Capability version advertised by the provider or package.
    /// Use it to match compatible feature contracts during package resolution.
    pub capability_version: String,
    /// Health used by this record or request.
    pub health: IsolationRuntimeHealth,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Classification selectors for supported classes.
    /// Policy and projection paths use them for finite routing decisions.
    pub supported_classes: Vec<IsolationClass>,
    /// Platform used by this record or request.
    pub platform: PlatformReport,
    /// Capabilities frozen into the package or returned by an adapter health
    /// check.
    pub capabilities: IsolationCapabilitySet,
    /// Trust class used when deciding whether context or capabilities may be
    /// admitted.
    pub trust: IsolationTrustRequirement,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of unsupported requirements values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub unsupported_requirements: Vec<String>,
    /// Retention class for referenced content or records.
    /// Stores and telemetry sinks use it to decide how long evidence may be kept.
    pub log_retention: crate::domain::RetentionClass,
}

impl IsolationCapabilityReport {
    /// Returns sandbox for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn sandbox(adapter_ref: impl Into<IsolationRuntimeRef>) -> Self {
        let adapter_ref = adapter_ref.into();
        Self {
            report_ref: IsolationCapabilityReportRef::new(format!(
                "report.{}",
                adapter_ref.as_str()
            )),
            adapter_ref,
            adapter_kind: IsolationRuntimeKind::HostProvided,
            adapter_version: "fake.v1".to_string(),
            capability_version: "isolation.capability.v1".to_string(),
            health: IsolationRuntimeHealth::Healthy,
            supported_classes: vec![IsolationClass::Sandbox],
            platform: PlatformReport::portable_fake(),
            capabilities: IsolationCapabilitySet::default().with_all([
                crate::IsolationCapability::ReadOnlyRoot,
                crate::IsolationCapability::NoNetworkGuarantee,
                crate::IsolationCapability::Cleanup,
                crate::IsolationCapability::ProcessTimeout,
                crate::IsolationCapability::ProcessSignal,
                crate::IsolationCapability::IoRedaction,
                crate::IsolationCapability::ContentRefIo,
                crate::IsolationCapability::ProcessStats,
                crate::IsolationCapability::SecretIsolation,
            ]),
            trust: IsolationTrustRequirement::local_dedicated(),
            unsupported_requirements: Vec::new(),
            log_retention: crate::domain::RetentionClass::RunScoped,
        }
    }

    /// Returns host process for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn host_process(adapter_ref: impl Into<IsolationRuntimeRef>) -> Self {
        let adapter_ref = adapter_ref.into();
        Self {
            report_ref: IsolationCapabilityReportRef::new(format!(
                "report.{}",
                adapter_ref.as_str()
            )),
            adapter_ref,
            adapter_kind: IsolationRuntimeKind::TestOnlyFake,
            adapter_version: "fake.host.v1".to_string(),
            capability_version: "isolation.capability.host.v1".to_string(),
            health: IsolationRuntimeHealth::Healthy,
            supported_classes: vec![IsolationClass::HostProcess],
            platform: PlatformReport::portable_fake(),
            capabilities: IsolationCapabilitySet::default().with_all([
                crate::IsolationCapability::ProcessTimeout,
                crate::IsolationCapability::IoRedaction,
            ]),
            trust: IsolationTrustRequirement::any(),
            unsupported_requirements: Vec::new(),
            log_retention: crate::domain::RetentionClass::RunScoped,
        }
    }

    /// Returns unsupported for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn unsupported(
        adapter_ref: impl Into<IsolationRuntimeRef>,
        missing: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let mut report = Self::sandbox(adapter_ref);
        let missing = missing.into_iter().map(Into::into).collect::<Vec<_>>();
        report.health = IsolationRuntimeHealth::UnsupportedHost {
            missing_prerequisites: missing.clone(),
        };
        report.unsupported_requirements = missing;
        report
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite isolation runtime kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum IsolationRuntimeKind {
    /// Use this variant when the contract needs to represent host provided; selecting it has no side effect by itself.
    HostProvided,
    /// Use this variant when the contract needs to represent host process; selecting it has no side effect by itself.
    HostProcess,
    /// Use this variant when the contract needs to represent os sandbox; selecting it has no side effect by itself.
    OsSandbox,
    /// Use this variant when the contract needs to represent container; selecting it has no side effect by itself.
    Container,
    /// Use this variant when the contract needs to represent lightweight vm; selecting it has no side effect by itself.
    LightweightVm,
    /// Use this variant when the contract needs to represent remote sandbox; selecting it has no side effect by itself.
    RemoteSandbox,
    /// Use this variant when the contract needs to represent test only fake; selecting it has no side effect by itself.
    TestOnlyFake,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite isolation runtime health cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum IsolationRuntimeHealth {
    /// Use this variant when the contract needs to represent healthy; selecting it has no side effect by itself.
    Healthy,
    /// Use this variant when the contract needs to represent degraded; selecting it has no side effect by itself.
    Degraded {
        /// Redacted explanation for a denial, failure, status, or package
        /// delta.
        reason: String,
    },
    /// Use this variant when the contract needs to represent unsupported host; selecting it has no side effect by itself.
    UnsupportedHost {
        /// Collection of missing prerequisites values.
        /// Ordering and membership should be treated as part of the serialized contract when
        /// relevant.
        missing_prerequisites: Vec<String>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries platform report data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct PlatformReport {
    /// Platform used by this record or request.
    pub platform: String,
    /// Os used by this record or request.
    pub os: String,
    /// Cpu architecture used by this record or request.
    pub cpu_architecture: String,
    /// Whether emulation supported is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub emulation_supported: bool,
}

impl PlatformReport {
    /// Returns portable fake for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn portable_fake() -> Self {
        Self {
            platform: "portable-test".to_string(),
            os: "test-os".to_string(),
            cpu_architecture: "test-arch".to_string(),
            emulation_supported: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries session prepare request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct SessionPrepareRequest {
    /// Environment used by this record or request.
    pub environment: ExecutionEnvironment,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries image resolve request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ImageResolveRequest {
    /// Image used by this record or request.
    pub image: ImageRequest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries image resolution data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ImageResolution {
    /// Typed image ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub image_ref: ImageRef,
    /// Digest used by this record or request.
    pub digest: String,
    /// Optional redacted credential alias value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub redacted_credential_alias: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries rootfs prepare request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct RootfsPrepareRequest {
    /// Rootfs used by this record or request.
    pub rootfs: RootfsRequest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries mount resolve request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct MountResolveRequest {
    /// Environment used by this record or request.
    pub environment: ExecutionEnvironment,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries mount plan data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct MountPlan {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of mounts values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub mounts: Vec<MountRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of expanded exposure audits values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub expanded_exposure_audits: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries network prepare request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct NetworkPrepareRequest {
    /// Environment used by this record or request.
    pub environment: ExecutionEnvironment,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries secret prepare request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct SecretPrepareRequest {
    /// Environment used by this record or request.
    pub environment: ExecutionEnvironment,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries secret materialization plan data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct SecretMaterializationPlan {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed secret mount refs references. Resolving them is separate from
    /// constructing this record.
    pub secret_mount_refs: Vec<SecretMountRef>,
    /// Whether teardown required is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub teardown_required: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries environment prepare request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct EnvironmentPrepareRequest {
    /// Environment used by this record or request.
    pub environment: ExecutionEnvironment,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries process start request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProcessStartRequest {
    /// Environment used by this record or request.
    pub environment: ExecutionEnvironment,
    /// Process used by this record or request.
    pub process: IsolatedProcessSpec,
    /// Effect intent used by this record or request.
    pub effect_intent: crate::EffectIntent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries process start result data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProcessStartResult {
    /// Typed process ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub process_ref: IsolatedProcessRef,
    /// Typed adapter session ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub adapter_session_ref: Option<IsolationAdapterSessionRef>,
    /// Terminal status used by this record or request.
    pub terminal_status: crate::EffectTerminalStatus,
    /// Stable external operation id used for typed lineage, lookup, or
    /// dedupe.
    pub external_operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of io frames values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub io_frames: Vec<ProcessIoFrame>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries process io request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProcessIoRequest {
    /// Typed process ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub process_ref: IsolatedProcessRef,
    /// Stream used by this record or request.
    pub stream: ProcessIoStream,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries process io frame data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProcessIoFrame {
    /// Typed stream ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub stream_ref: ProcessIoStreamRef,
    /// Stream used by this record or request.
    pub stream: ProcessIoStream,
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub cursor: u64,
    /// Count of byte items observed or included in this record.
    pub byte_count: u64,
    /// Stable hash for the bytes or canonical payload used for stale checks
    /// and fingerprints.
    pub content_hash: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Content references associated with this record; resolving them is a
    /// separate policy-gated step.
    pub content_refs: Vec<ContentRef>,
    /// Raw content or raw-content control for this value.
    /// Use it only when policy explicitly allows raw content capture or delivery.
    pub raw_content_present: bool,
    /// Whether output was shortened by byte, item, page, archive, or parser
    /// limits.
    pub truncated: bool,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite process io stream cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ProcessIoStream {
    /// Use this variant when the contract needs to represent stdin; selecting it has no side effect by itself.
    Stdin,
    /// Use this variant when the contract needs to represent stdout; selecting it has no side effect by itself.
    Stdout,
    /// Use this variant when the contract needs to represent stderr; selecting it has no side effect by itself.
    Stderr,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries process signal request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProcessSignalRequest {
    /// Typed process ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub process_ref: IsolatedProcessRef,
    /// Signal used by this record or request.
    pub signal: ProcessSignal,
    /// Grace period in milliseconds before termination or cleanup escalates.
    pub grace_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries process signal result data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProcessSignalResult {
    /// Typed process ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub process_ref: IsolatedProcessRef,
    /// Signal used by this record or request.
    pub signal: ProcessSignal,
    /// Whether delivered is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub delivered: bool,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite process signal cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ProcessSignal {
    /// Use this variant when the contract needs to represent interrupt; selecting it has no side effect by itself.
    Interrupt,
    /// Use this variant when the contract needs to represent terminate; selecting it has no side effect by itself.
    Terminate,
    /// Use this variant when the contract needs to represent kill; selecting it has no side effect by itself.
    Kill,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries process stats request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProcessStatsRequest {
    /// Typed process ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub process_ref: IsolatedProcessRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries process stats snapshot data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProcessStatsSnapshot {
    /// Typed snapshot ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub snapshot_ref: ProcessStatsSnapshotRef,
    /// Typed process ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub process_ref: IsolatedProcessRef,
    /// Time value in milliseconds for cpu millis.
    /// Use it for timeout, ordering, or diagnostic calculations.
    pub cpu_millis: Option<u64>,
    /// memory bytes used for bounds checks, summaries, or truncation
    /// evidence.
    pub memory_bytes: Option<u64>,
    /// Count of process items observed or included in this record.
    pub process_count: Option<u32>,
    /// filesystem bytes used for bounds checks, summaries, or truncation
    /// evidence.
    pub filesystem_bytes: Option<u64>,
    /// network bytes used for bounds checks, summaries, or truncation
    /// evidence.
    pub network_bytes: Option<u64>,
    /// Process exit status when the process reported one.
    pub exit_code: Option<i32>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries cleanup request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct CleanupRequest {
    /// Environment used by this record or request.
    pub environment: ExecutionEnvironment,
    /// Typed cleanup plan ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub cleanup_plan_ref: CleanupPlanRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries cleanup result data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct CleanupResult {
    /// Typed cleanup plan ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub cleanup_plan_ref: CleanupPlanRef,
    /// Finite status for this record or lifecycle stage.
    pub status: CleanupStatus,
    /// Stable external operation id used for typed lineage, lookup, or
    /// dedupe.
    pub external_operation_id: Option<String>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite cleanup status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum CleanupStatus {
    /// Use this variant when the contract needs to represent completed; selecting it has no side effect by itself.
    Completed,
    /// Use this variant when the contract needs to represent repair needed; selecting it has no side effect by itself.
    RepairNeeded,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries detach transfer request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct DetachTransferRequest {
    /// Environment used by this record or request.
    pub environment: ExecutionEnvironment,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries detach transfer result data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct DetachTransferResult {
    /// Typed host ack ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub host_ack_ref: String,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries reclaim request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ReclaimRequest {
    /// Typed ticket ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub ticket_ref: ReclaimTicketRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries reclaim result data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ReclaimResult {
    /// Typed ticket ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub ticket_ref: ReclaimTicketRef,
    /// Finite status for this record or lifecycle stage.
    pub status: CleanupStatus,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Default)]
/// Carries isolation runtime call log data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct IsolationRuntimeCallLog {
    calls: Arc<Mutex<Vec<String>>>,
}

impl IsolationRuntimeCallLog {
    /// Operates on in-memory or journal-derived ports::isolation state for
    /// diagnostics and repair evidence. It does not create a second run loop
    /// or product workflow owner.
    pub fn push(&self, call: impl Into<String>) {
        self.calls
            .lock()
            .expect("isolation call log")
            .push(call.into());
    }

    /// Operates on in-memory or journal-derived ports::isolation state for
    /// diagnostics and repair evidence. It does not create a second run loop
    /// or product workflow owner.
    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("isolation call log").clone()
    }

    /// Operates on in-memory or journal-derived ports::isolation state for
    /// diagnostics and repair evidence. It does not create a second run loop
    /// or product workflow owner.
    pub fn count_matching(&self, call: &str) -> usize {
        self.calls()
            .into_iter()
            .filter(|entry| entry == call)
            .count()
    }
}

/// Builds the isolation host configuration needed value.
/// This is data construction and performs no I/O, journal append, event publication, or process
pub fn isolation_host_configuration_needed(message: impl Into<String>) -> AgentError {
    AgentError::new(
        AgentErrorKind::IsolationFailure,
        RetryClassification::HostConfigurationNeeded,
        message,
    )
}
