use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        AgentId, ContentRef, DestinationRef, EntityRef, PolicyRef, PrivacyClass, RetentionClass,
        RunId, SourceRef,
    },
    effect::{EffectIntent, EffectResult},
    package_isolation::{
        CleanupPlanRef, ExecutionEnvironmentId, IsolatedProcessRef, IsolationCapability,
        IsolationClass, IsolationRequirementRef, IsolationRuntimeRef, IsolationTrustField,
        NetworkNamespaceRef, PreparedEnvironmentRef, ProcessStatsSnapshotRef,
    },
    ports_isolation::{CleanupStatus, ProcessIoFrame},
};

pub const ISOLATION_RECORD_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "record_type", content = "record", rename_all = "snake_case")]
pub enum IsolationRecord {
    Requested(IsolationRequestedRecord),
    AdapterCapabilityReported(IsolationCapabilityReportedRecord),
    CapabilityMatch(IsolationCapabilityMatchRecord),
    DowngradeDecision(IsolationDowngradeDecisionRecord),
    EnvironmentPrepareIntent(IsolationEnvironmentPrepareIntentRecord),
    EnvironmentPrepareResult(IsolationEnvironmentPrepareResultRecord),
    ProcessStartIntent(IsolationProcessStartIntentRecord),
    ProcessStartResult(IsolationProcessStartResultRecord),
    ProcessIoFrame(ProcessIoFrame),
    ProcessStatsSnapshot(IsolationProcessStatsRecord),
    CleanupIntent(IsolationCleanupIntentRecord),
    CleanupResult(IsolationCleanupResultRecord),
    Failed(IsolationFailureRecord),
}

impl IsolationRecord {
    pub fn event_record(&self, base: IsolationEventBase) -> IsolationEventRecord {
        let (kind, summary) = match self {
            Self::Requested(record) => (
                IsolationEventKind::IsolationRequested,
                record.redacted_summary.clone(),
            ),
            Self::AdapterCapabilityReported(record) => (
                IsolationEventKind::IsolationAdapterHealthChecked,
                record.redacted_summary.clone(),
            ),
            Self::CapabilityMatch(record) => (
                IsolationEventKind::IsolationCapabilityMatched,
                record.redacted_summary.clone(),
            ),
            Self::DowngradeDecision(record) if record.approved => (
                IsolationEventKind::IsolationDowngradeApproved,
                record.redacted_summary.clone(),
            ),
            Self::DowngradeDecision(record) => (
                IsolationEventKind::IsolationDowngradeDenied,
                record.redacted_summary.clone(),
            ),
            Self::EnvironmentPrepareIntent(record) => (
                IsolationEventKind::IsolationEnvironmentPrepared,
                record.redacted_summary.clone(),
            ),
            Self::EnvironmentPrepareResult(record) => (
                IsolationEventKind::IsolationEnvironmentPrepared,
                record.redacted_summary.clone(),
            ),
            Self::ProcessStartIntent(record) => (
                IsolationEventKind::IsolationProcessStarted,
                record.redacted_summary.clone(),
            ),
            Self::ProcessStartResult(record) => (
                IsolationEventKind::IsolationProcessStarted,
                record.redacted_summary.clone(),
            ),
            Self::ProcessIoFrame(record) => (
                IsolationEventKind::IsolationProcessIoCaptured,
                record.redacted_summary.clone(),
            ),
            Self::ProcessStatsSnapshot(record) => (
                IsolationEventKind::IsolationProcessStatsRecorded,
                record.redacted_summary.clone(),
            ),
            Self::CleanupIntent(record) => (
                IsolationEventKind::IsolationCleanupStarted,
                record.redacted_summary.clone(),
            ),
            Self::CleanupResult(record) if record.status == CleanupStatus::Completed => (
                IsolationEventKind::IsolationCleanupCompleted,
                record.redacted_summary.clone(),
            ),
            Self::CleanupResult(record) => (
                IsolationEventKind::IsolationCleanupFailed,
                record.redacted_summary.clone(),
            ),
            Self::Failed(record) => (IsolationEventKind::IsolationFailed, record.reason.clone()),
        };
        IsolationEventRecord {
            schema_version: ISOLATION_RECORD_SCHEMA_VERSION,
            run_id: base.run_id,
            agent_id: base.agent_id,
            event_kind: kind,
            subject_ref: base.subject_ref,
            related_refs: base.related_refs,
            source: base.source,
            destination: base.destination,
            policy_refs: base.policy_refs,
            privacy: base.privacy,
            retention: base.retention,
            runtime_package_fingerprint: base.runtime_package_fingerprint,
            redacted_summary: summary,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationRequestedRecord {
    pub environment_id: ExecutionEnvironmentId,
    pub requirement_ref: IsolationRequirementRef,
    pub requested_class: IsolationClass,
    pub source: SourceRef,
    pub destination: DestinationRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationCapabilityReportedRecord {
    pub environment_id: ExecutionEnvironmentId,
    pub adapter_ref: IsolationRuntimeRef,
    pub capability_report_ref: String,
    pub health: String,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationCapabilityMatchRecord {
    pub environment_id: ExecutionEnvironmentId,
    pub adapter_ref: IsolationRuntimeRef,
    pub requested_class: IsolationClass,
    pub selected_class: IsolationClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_capabilities: Vec<IsolationCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trust_gaps: Vec<IsolationTrustField>,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationDowngradeDecisionRecord {
    pub environment_id: ExecutionEnvironmentId,
    pub adapter_ref: IsolationRuntimeRef,
    pub requested_class: IsolationClass,
    pub selected_class: IsolationClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capability_gaps: Vec<IsolationCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trust_gaps: Vec<IsolationTrustField>,
    pub approved: bool,
    pub policy_decision_scope: Option<crate::application_isolation::PolicyDecisionScope>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_decision_refs: Vec<crate::package_isolation::PolicyDecisionRef>,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationEnvironmentPrepareIntentRecord {
    pub environment_id: ExecutionEnvironmentId,
    pub effect_intent: EffectIntent,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationEnvironmentPrepareResultRecord {
    pub environment_id: ExecutionEnvironmentId,
    pub prepared_environment_ref: PreparedEnvironmentRef,
    pub effect_result: EffectResult,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationProcessStartIntentRecord {
    pub environment_id: ExecutionEnvironmentId,
    pub process_ref: IsolatedProcessRef,
    pub effect_intent: EffectIntent,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationProcessStartResultRecord {
    pub environment_id: ExecutionEnvironmentId,
    pub process_ref: IsolatedProcessRef,
    pub effect_result: EffectResult,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_refs: Vec<ContentRef>,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationProcessStatsRecord {
    pub environment_id: ExecutionEnvironmentId,
    pub snapshot_ref: ProcessStatsSnapshotRef,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationCleanupIntentRecord {
    pub environment_id: ExecutionEnvironmentId,
    pub cleanup_plan_ref: CleanupPlanRef,
    pub effect_intent: EffectIntent,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationCleanupResultRecord {
    pub environment_id: ExecutionEnvironmentId,
    pub cleanup_plan_ref: CleanupPlanRef,
    pub status: CleanupStatus,
    pub effect_result: EffectResult,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationFailureRecord {
    pub environment_id: ExecutionEnvironmentId,
    pub adapter_ref: Option<IsolationRuntimeRef>,
    pub reason: String,
    pub retry_classification: crate::RetryClassification,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationEventBase {
    pub run_id: RunId,
    pub agent_id: AgentId,
    pub subject_ref: EntityRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_refs: Vec<EntityRef>,
    pub source: SourceRef,
    pub destination: DestinationRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
    pub runtime_package_fingerprint: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationEventRecord {
    pub schema_version: u16,
    pub run_id: RunId,
    pub agent_id: AgentId,
    pub event_kind: IsolationEventKind,
    pub subject_ref: EntityRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_refs: Vec<EntityRef>,
    pub source: SourceRef,
    pub destination: DestinationRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
    pub runtime_package_fingerprint: String,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationEventKind {
    IsolationRequested,
    IsolationAdapterHealthChecked,
    IsolationCapabilityMatched,
    IsolationDowngradeDenied,
    IsolationDowngradeApproved,
    IsolationImageResolved,
    IsolationRootfsPrepared,
    IsolationSessionPrepared,
    IsolationMountsResolved,
    IsolationNetworkPrepared,
    IsolationSecretsPrepared,
    IsolationEnvironmentPrepared,
    IsolationProcessStarted,
    IsolationProcessIoCaptured,
    IsolationProcessStatsRecorded,
    IsolationProcessSignalled,
    IsolationProcessExited,
    IsolationCleanupStarted,
    IsolationCleanupCompleted,
    IsolationCleanupFailed,
    IsolationFailed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsolationNetworkPreparedRecord {
    pub environment_id: ExecutionEnvironmentId,
    pub network_namespace_ref: NetworkNamespaceRef,
    pub redacted_summary: String,
}
