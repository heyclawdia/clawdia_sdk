use std::sync::{Arc, Mutex};

use crate::{
    domain::{AgentError, ContentRef},
    package_isolation::{
        IsolatedProcessRef, IsolationAdapterSessionRef, IsolationRuntimeRef, IsolationSessionRef,
        MountRef, NetworkNamespaceRef, PreparedEnvironmentRef, ProcessIoStreamRef,
        ProcessStatsSnapshotRef, RootfsRef, SecretMountRef,
    },
    ports_isolation::{
        CleanupRequest, CleanupResult, CleanupStatus, DetachTransferRequest, DetachTransferResult,
        EnvironmentPrepareRequest, ImageResolution, ImageResolveRequest, IsolationCapabilityReport,
        IsolationRuntime, MountPlan, MountResolveRequest, NetworkPrepareRequest, ProcessIoFrame,
        ProcessIoRequest, ProcessIoStream, ProcessSignalRequest, ProcessSignalResult,
        ProcessStartRequest, ProcessStartResult, ProcessStatsRequest, ProcessStatsSnapshot,
        ReclaimRequest, ReclaimResult, RootfsPrepareRequest, SecretMaterializationPlan,
        SecretPrepareRequest, SessionPrepareRequest,
    },
};

#[derive(Clone, Debug)]
pub struct FakeIsolationRuntime {
    report: IsolationCapabilityReport,
    calls: Arc<Mutex<Vec<String>>>,
    cleanup_status: CleanupStatus,
}

impl FakeIsolationRuntime {
    pub fn with_report(report: IsolationCapabilityReport) -> Self {
        Self {
            report,
            calls: Arc::new(Mutex::new(Vec::new())),
            cleanup_status: CleanupStatus::Completed,
        }
    }

    pub fn unsupported_host(
        adapter_ref: impl Into<IsolationRuntimeRef>,
        missing: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::with_report(IsolationCapabilityReport::unsupported(adapter_ref, missing))
    }

    pub fn host_process_only(adapter_ref: impl Into<IsolationRuntimeRef>) -> Self {
        Self::with_report(IsolationCapabilityReport::host_process(adapter_ref))
    }

    pub fn with_cleanup_status(mut self, cleanup_status: CleanupStatus) -> Self {
        self.cleanup_status = cleanup_status;
        self
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("fake isolation calls").clone()
    }

    pub fn call_count(&self) -> usize {
        self.calls().len()
    }

    pub fn start_process_call_count(&self) -> usize {
        self.calls()
            .into_iter()
            .filter(|call| call == "start_process")
            .count()
    }

    fn push_call(&self, call: impl Into<String>) {
        self.calls
            .lock()
            .expect("fake isolation calls")
            .push(call.into());
    }
}

impl IsolationRuntime for FakeIsolationRuntime {
    fn runtime_ref(&self) -> &IsolationRuntimeRef {
        &self.report.adapter_ref
    }

    fn capability_report(&self) -> Result<IsolationCapabilityReport, AgentError> {
        self.push_call("capability_report");
        Ok(self.report.clone())
    }

    fn prepare_session(
        &self,
        _request: SessionPrepareRequest,
    ) -> Result<IsolationSessionRef, AgentError> {
        self.push_call("prepare_session");
        Ok(IsolationSessionRef::new("session.fake.isolation"))
    }

    fn resolve_image(&self, request: ImageResolveRequest) -> Result<ImageResolution, AgentError> {
        self.push_call("resolve_image");
        Ok(ImageResolution {
            image_ref: request.image.image_ref,
            digest: "sha256:fake.image".to_string(),
            redacted_credential_alias: Some("credential.alias.redacted".to_string()),
        })
    }

    fn prepare_rootfs(&self, _request: RootfsPrepareRequest) -> Result<RootfsRef, AgentError> {
        self.push_call("prepare_rootfs");
        Ok(RootfsRef::new("rootfs.fake.isolation"))
    }

    fn resolve_mounts(&self, _request: MountResolveRequest) -> Result<MountPlan, AgentError> {
        self.push_call("resolve_mounts");
        Ok(MountPlan {
            mounts: vec![MountRef::new("mount.workspace.primary")],
            expanded_exposure_audits: vec!["workspace snapshot mounted by alias".to_string()],
        })
    }

    fn configure_network(
        &self,
        _request: NetworkPrepareRequest,
    ) -> Result<NetworkNamespaceRef, AgentError> {
        self.push_call("configure_network");
        Ok(NetworkNamespaceRef::new("network.fake.isolation"))
    }

    fn prepare_secrets(
        &self,
        request: SecretPrepareRequest,
    ) -> Result<SecretMaterializationPlan, AgentError> {
        self.push_call("prepare_secrets");
        Ok(SecretMaterializationPlan {
            secret_mount_refs: request
                .environment
                .spec
                .secrets
                .secret_mounts
                .iter()
                .map(|secret| secret.mount_ref.clone())
                .collect::<Vec<SecretMountRef>>(),
            teardown_required: true,
        })
    }

    fn prepare_environment(
        &self,
        _request: EnvironmentPrepareRequest,
    ) -> Result<PreparedEnvironmentRef, AgentError> {
        self.push_call("prepare_environment");
        Ok(PreparedEnvironmentRef::new("prepared.fake.isolation"))
    }

    fn start_process(
        &self,
        request: ProcessStartRequest,
    ) -> Result<ProcessStartResult, AgentError> {
        self.push_call("start_process");
        let process_ref = IsolatedProcessRef::new(format!(
            "process.ref.{}",
            request.process.process_id.as_str()
        ));
        Ok(ProcessStartResult {
            process_ref,
            adapter_session_ref: Some(IsolationAdapterSessionRef::new(
                "adapter.session.fake.isolation",
            )),
            terminal_status: crate::EffectTerminalStatus::Completed,
            external_operation_id: Some("external.fake.process.start".to_string()),
            io_frames: vec![
                ProcessIoFrame {
                    stream_ref: ProcessIoStreamRef::new("stream.stdout.fake"),
                    stream: ProcessIoStream::Stdout,
                    cursor: 1,
                    byte_count: 23,
                    content_hash: "sha256:fake.stdout".to_string(),
                    content_refs: vec![ContentRef::new("content.isolation.stdout")],
                    raw_content_present: false,
                    truncated: false,
                    redacted_summary: "stdout captured as content ref".to_string(),
                },
                ProcessIoFrame {
                    stream_ref: ProcessIoStreamRef::new("stream.stderr.fake"),
                    stream: ProcessIoStream::Stderr,
                    cursor: 1,
                    byte_count: 0,
                    content_hash: "sha256:fake.stderr.empty".to_string(),
                    content_refs: Vec::new(),
                    raw_content_present: false,
                    truncated: false,
                    redacted_summary: "stderr empty".to_string(),
                },
            ],
            redacted_summary: "isolated process started".to_string(),
        })
    }

    fn stream_io(&self, _request: ProcessIoRequest) -> Result<ProcessIoFrame, AgentError> {
        self.push_call("stream_io");
        Ok(ProcessIoFrame {
            stream_ref: ProcessIoStreamRef::new("stream.stdout.fake"),
            stream: ProcessIoStream::Stdout,
            cursor: 1,
            byte_count: 23,
            content_hash: "sha256:fake.stdout".to_string(),
            content_refs: vec![ContentRef::new("content.isolation.stdout")],
            raw_content_present: false,
            truncated: false,
            redacted_summary: "stdout captured as content ref".to_string(),
        })
    }

    fn signal_process(
        &self,
        request: ProcessSignalRequest,
    ) -> Result<ProcessSignalResult, AgentError> {
        self.push_call("signal_process");
        Ok(ProcessSignalResult {
            process_ref: request.process_ref,
            signal: request.signal,
            delivered: true,
            redacted_summary: "signal delivered to isolated process".to_string(),
        })
    }

    fn collect_stats(
        &self,
        request: ProcessStatsRequest,
    ) -> Result<ProcessStatsSnapshot, AgentError> {
        self.push_call("collect_stats");
        Ok(ProcessStatsSnapshot {
            snapshot_ref: ProcessStatsSnapshotRef::new("stats.fake.isolation"),
            process_ref: request.process_ref,
            cpu_millis: Some(10),
            memory_bytes: Some(1024),
            process_count: Some(1),
            filesystem_bytes: Some(2048),
            network_bytes: Some(0),
            exit_code: Some(0),
            redacted_summary: "fake process stats counters".to_string(),
        })
    }

    fn cleanup(&self, request: CleanupRequest) -> Result<CleanupResult, AgentError> {
        self.push_call("cleanup");
        let redacted_summary = match self.cleanup_status {
            CleanupStatus::Completed => "isolation cleanup completed",
            CleanupStatus::RepairNeeded => "isolation cleanup requires host repair",
        };
        Ok(CleanupResult {
            cleanup_plan_ref: request.cleanup_plan_ref,
            status: self.cleanup_status,
            external_operation_id: Some("external.fake.cleanup".to_string()),
            redacted_summary: redacted_summary.to_string(),
        })
    }

    fn detach(&self, _request: DetachTransferRequest) -> Result<DetachTransferResult, AgentError> {
        self.push_call("detach");
        Ok(DetachTransferResult {
            host_ack_ref: "host.ack.fake.detach".to_string(),
            redacted_summary: "detach acknowledged by fake host".to_string(),
        })
    }

    fn reclaim(&self, request: ReclaimRequest) -> Result<ReclaimResult, AgentError> {
        self.push_call("reclaim");
        Ok(ReclaimResult {
            ticket_ref: request.ticket_ref,
            status: CleanupStatus::Completed,
            redacted_summary: "fake reclaim completed".to_string(),
        })
    }
}
