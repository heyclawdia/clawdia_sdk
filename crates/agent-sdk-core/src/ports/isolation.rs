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

pub trait IsolationRuntime: Send + Sync {
    fn runtime_ref(&self) -> &IsolationRuntimeRef;

    fn capability_report(&self) -> Result<IsolationCapabilityReport, AgentError>;

    fn prepare_session(
        &self,
        request: SessionPrepareRequest,
    ) -> Result<IsolationSessionRef, AgentError>;

    fn resolve_image(&self, request: ImageResolveRequest) -> Result<ImageResolution, AgentError>;

    fn prepare_rootfs(&self, request: RootfsPrepareRequest) -> Result<RootfsRef, AgentError>;

    fn resolve_mounts(&self, request: MountResolveRequest) -> Result<MountPlan, AgentError>;

    fn configure_network(
        &self,
        request: NetworkPrepareRequest,
    ) -> Result<NetworkNamespaceRef, AgentError>;

    fn prepare_secrets(
        &self,
        request: SecretPrepareRequest,
    ) -> Result<SecretMaterializationPlan, AgentError>;

    fn prepare_environment(
        &self,
        request: EnvironmentPrepareRequest,
    ) -> Result<PreparedEnvironmentRef, AgentError>;

    fn start_process(&self, request: ProcessStartRequest)
    -> Result<ProcessStartResult, AgentError>;

    fn stream_io(&self, request: ProcessIoRequest) -> Result<ProcessIoFrame, AgentError>;

    fn signal_process(
        &self,
        request: ProcessSignalRequest,
    ) -> Result<ProcessSignalResult, AgentError>;

    fn collect_stats(
        &self,
        request: ProcessStatsRequest,
    ) -> Result<ProcessStatsSnapshot, AgentError>;

    fn cleanup(&self, request: CleanupRequest) -> Result<CleanupResult, AgentError>;

    fn detach(&self, request: DetachTransferRequest) -> Result<DetachTransferResult, AgentError>;

    fn reclaim(&self, request: ReclaimRequest) -> Result<ReclaimResult, AgentError>;
}

#[derive(Clone, Default)]
pub struct IsolationRuntimeRegistry {
    runtimes: BTreeMap<String, Arc<dyn IsolationRuntime>>,
}

impl IsolationRuntimeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

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

    pub fn get(&self, runtime_ref: &IsolationRuntimeRef) -> Option<Arc<dyn IsolationRuntime>> {
        self.runtimes.get(runtime_ref.as_str()).cloned()
    }

    pub fn first(&self) -> Option<Arc<dyn IsolationRuntime>> {
        self.runtimes.values().next().cloned()
    }

    pub fn is_empty(&self) -> bool {
        self.runtimes.is_empty()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationCapabilityReport {
    pub report_ref: IsolationCapabilityReportRef,
    pub adapter_ref: IsolationRuntimeRef,
    pub adapter_kind: IsolationRuntimeKind,
    pub adapter_version: String,
    pub capability_version: String,
    pub health: IsolationRuntimeHealth,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supported_classes: Vec<IsolationClass>,
    pub platform: PlatformReport,
    pub capabilities: IsolationCapabilitySet,
    pub trust: IsolationTrustRequirement,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unsupported_requirements: Vec<String>,
    pub log_retention: crate::domain::RetentionClass,
}

impl IsolationCapabilityReport {
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
pub enum IsolationRuntimeKind {
    HostProvided,
    HostProcess,
    OsSandbox,
    Container,
    LightweightVm,
    RemoteSandbox,
    TestOnlyFake,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationRuntimeHealth {
    Healthy,
    Degraded { reason: String },
    UnsupportedHost { missing_prerequisites: Vec<String> },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlatformReport {
    pub platform: String,
    pub os: String,
    pub cpu_architecture: String,
    pub emulation_supported: bool,
}

impl PlatformReport {
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
pub struct SessionPrepareRequest {
    pub environment: ExecutionEnvironment,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ImageResolveRequest {
    pub image: ImageRequest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ImageResolution {
    pub image_ref: ImageRef,
    pub digest: String,
    pub redacted_credential_alias: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootfsPrepareRequest {
    pub rootfs: RootfsRequest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MountResolveRequest {
    pub environment: ExecutionEnvironment,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MountPlan {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mounts: Vec<MountRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expanded_exposure_audits: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NetworkPrepareRequest {
    pub environment: ExecutionEnvironment,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SecretPrepareRequest {
    pub environment: ExecutionEnvironment,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SecretMaterializationPlan {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secret_mount_refs: Vec<SecretMountRef>,
    pub teardown_required: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentPrepareRequest {
    pub environment: ExecutionEnvironment,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProcessStartRequest {
    pub environment: ExecutionEnvironment,
    pub process: IsolatedProcessSpec,
    pub effect_intent: crate::EffectIntent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProcessStartResult {
    pub process_ref: IsolatedProcessRef,
    pub adapter_session_ref: Option<IsolationAdapterSessionRef>,
    pub terminal_status: crate::EffectTerminalStatus,
    pub external_operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub io_frames: Vec<ProcessIoFrame>,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProcessIoRequest {
    pub process_ref: IsolatedProcessRef,
    pub stream: ProcessIoStream,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProcessIoFrame {
    pub stream_ref: ProcessIoStreamRef,
    pub stream: ProcessIoStream,
    pub cursor: u64,
    pub byte_count: u64,
    pub content_hash: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_refs: Vec<ContentRef>,
    pub raw_content_present: bool,
    pub truncated: bool,
    pub redacted_summary: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessIoStream {
    Stdin,
    Stdout,
    Stderr,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProcessSignalRequest {
    pub process_ref: IsolatedProcessRef,
    pub signal: ProcessSignal,
    pub grace_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProcessSignalResult {
    pub process_ref: IsolatedProcessRef,
    pub signal: ProcessSignal,
    pub delivered: bool,
    pub redacted_summary: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessSignal {
    Interrupt,
    Terminate,
    Kill,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProcessStatsRequest {
    pub process_ref: IsolatedProcessRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProcessStatsSnapshot {
    pub snapshot_ref: ProcessStatsSnapshotRef,
    pub process_ref: IsolatedProcessRef,
    pub cpu_millis: Option<u64>,
    pub memory_bytes: Option<u64>,
    pub process_count: Option<u32>,
    pub filesystem_bytes: Option<u64>,
    pub network_bytes: Option<u64>,
    pub exit_code: Option<i32>,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CleanupRequest {
    pub environment: ExecutionEnvironment,
    pub cleanup_plan_ref: CleanupPlanRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CleanupResult {
    pub cleanup_plan_ref: CleanupPlanRef,
    pub status: CleanupStatus,
    pub external_operation_id: Option<String>,
    pub redacted_summary: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupStatus {
    Completed,
    RepairNeeded,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DetachTransferRequest {
    pub environment: ExecutionEnvironment,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DetachTransferResult {
    pub host_ack_ref: String,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReclaimRequest {
    pub ticket_ref: ReclaimTicketRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReclaimResult {
    pub ticket_ref: ReclaimTicketRef,
    pub status: CleanupStatus,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Default)]
pub struct IsolationRuntimeCallLog {
    calls: Arc<Mutex<Vec<String>>>,
}

impl IsolationRuntimeCallLog {
    pub fn push(&self, call: impl Into<String>) {
        self.calls
            .lock()
            .expect("isolation call log")
            .push(call.into());
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("isolation call log").clone()
    }

    pub fn count_matching(&self, call: &str) -> usize {
        self.calls()
            .into_iter()
            .filter(|entry| entry == call)
            .count()
    }
}

pub fn isolation_host_configuration_needed(message: impl Into<String>) -> AgentError {
    AgentError::new(
        AgentErrorKind::IsolationFailure,
        RetryClassification::HostConfigurationNeeded,
        message,
    )
}
