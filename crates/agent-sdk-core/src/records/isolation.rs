//! Durable and observable SDK records. Use these DTOs for events, journals, effects,
//! context, output, and feature evidence. Constructing records is data-only;
//! persistence, publication, and external actions happen through ports or application
//! coordinators. This file contains the isolation portion of that contract.
//!
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

/// Constant value for the records::isolation contract. Use it to keep
/// SDK records and tests aligned on the same stable value.
pub const ISOLATION_RECORD_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "record_type", content = "record", rename_all = "snake_case")]
/// Enumerates the finite isolation record cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum IsolationRecord {
    /// Use this variant when the contract needs to represent requested; selecting it has no side effect by itself.
    Requested(IsolationRequestedRecord),
    /// Use this variant when the contract needs to represent adapter capability reported; selecting it has no side effect by itself.
    AdapterCapabilityReported(IsolationCapabilityReportedRecord),
    /// Use this variant when the contract needs to represent capability match; selecting it has no side effect by itself.
    CapabilityMatch(IsolationCapabilityMatchRecord),
    /// Use this variant when the contract needs to represent downgrade decision; selecting it has no side effect by itself.
    DowngradeDecision(IsolationDowngradeDecisionRecord),
    /// Use this variant when the contract needs to represent environment prepare intent; selecting it has no side effect by itself.
    EnvironmentPrepareIntent(IsolationEnvironmentPrepareIntentRecord),
    /// Use this variant when the contract needs to represent environment prepare result; selecting it has no side effect by itself.
    EnvironmentPrepareResult(IsolationEnvironmentPrepareResultRecord),
    /// Use this variant when the contract needs to represent process start intent; selecting it has no side effect by itself.
    ProcessStartIntent(IsolationProcessStartIntentRecord),
    /// Use this variant when the contract needs to represent process start result; selecting it has no side effect by itself.
    ProcessStartResult(IsolationProcessStartResultRecord),
    /// Use this variant when the contract needs to represent process io frame; selecting it has no side effect by itself.
    ProcessIoFrame(ProcessIoFrame),
    /// Use this variant when the contract needs to represent process stats snapshot; selecting it has no side effect by itself.
    ProcessStatsSnapshot(IsolationProcessStatsRecord),
    /// Use this variant when the contract needs to represent cleanup intent; selecting it has no side effect by itself.
    CleanupIntent(IsolationCleanupIntentRecord),
    /// Use this variant when the contract needs to represent cleanup result; selecting it has no side effect by itself.
    CleanupResult(IsolationCleanupResultRecord),
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed(IsolationFailureRecord),
}

impl IsolationRecord {
    /// Builds the event record record or result value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
/// Carries the isolation requested record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct IsolationRequestedRecord {
    /// Stable environment id used for typed lineage, lookup, or dedupe.
    pub environment_id: ExecutionEnvironmentId,
    /// Typed requirement ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub requirement_ref: IsolationRequirementRef,
    /// Classification value for requested class.
    /// Policy and projection paths use it for finite routing decisions.
    pub requested_class: IsolationClass,
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
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the isolation capability reported record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct IsolationCapabilityReportedRecord {
    /// Stable environment id used for typed lineage, lookup, or dedupe.
    pub environment_id: ExecutionEnvironmentId,
    /// Typed adapter ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub adapter_ref: IsolationRuntimeRef,
    /// Typed capability report ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub capability_report_ref: String,
    /// Health used by this record or request.
    pub health: String,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the isolation capability match record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct IsolationCapabilityMatchRecord {
    /// Stable environment id used for typed lineage, lookup, or dedupe.
    pub environment_id: ExecutionEnvironmentId,
    /// Typed adapter ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub adapter_ref: IsolationRuntimeRef,
    /// Classification value for requested class.
    /// Policy and projection paths use it for finite routing decisions.
    pub requested_class: IsolationClass,
    /// Classification value for selected class.
    /// Policy and projection paths use it for finite routing decisions.
    pub selected_class: IsolationClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of missing capabilities values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub missing_capabilities: Vec<IsolationCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of trust gaps values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub trust_gaps: Vec<IsolationTrustField>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the isolation downgrade decision record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct IsolationDowngradeDecisionRecord {
    /// Stable environment id used for typed lineage, lookup, or dedupe.
    pub environment_id: ExecutionEnvironmentId,
    /// Typed adapter ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub adapter_ref: IsolationRuntimeRef,
    /// Classification value for requested class.
    /// Policy and projection paths use it for finite routing decisions.
    pub requested_class: IsolationClass,
    /// Classification value for selected class.
    /// Policy and projection paths use it for finite routing decisions.
    pub selected_class: IsolationClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of capability gaps values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub capability_gaps: Vec<IsolationCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of trust gaps values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub trust_gaps: Vec<IsolationTrustField>,
    /// Whether approved is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub approved: bool,
    /// Optional policy decision scope value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub policy_decision_scope: Option<crate::application_isolation::PolicyDecisionScope>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed policy decision refs references. Resolving them is separate from
    /// constructing this record.
    pub policy_decision_refs: Vec<crate::package_isolation::PolicyDecisionRef>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the isolation environment prepare intent record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct IsolationEnvironmentPrepareIntentRecord {
    /// Stable environment id used for typed lineage, lookup, or dedupe.
    pub environment_id: ExecutionEnvironmentId,
    /// Effect intent used by this record or request.
    pub effect_intent: EffectIntent,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the isolation environment prepare result record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct IsolationEnvironmentPrepareResultRecord {
    /// Stable environment id used for typed lineage, lookup, or dedupe.
    pub environment_id: ExecutionEnvironmentId,
    /// Typed prepared environment ref reference. Resolving or executing it is
    /// a separate policy-gated step.
    pub prepared_environment_ref: PreparedEnvironmentRef,
    /// Effect result used by this record or request.
    pub effect_result: EffectResult,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the isolation process start intent record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct IsolationProcessStartIntentRecord {
    /// Stable environment id used for typed lineage, lookup, or dedupe.
    pub environment_id: ExecutionEnvironmentId,
    /// Typed process ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub process_ref: IsolatedProcessRef,
    /// Effect intent used by this record or request.
    pub effect_intent: EffectIntent,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the isolation process start result record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct IsolationProcessStartResultRecord {
    /// Stable environment id used for typed lineage, lookup, or dedupe.
    pub environment_id: ExecutionEnvironmentId,
    /// Typed process ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub process_ref: IsolatedProcessRef,
    /// Effect result used by this record or request.
    pub effect_result: EffectResult,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Content references associated with this record; resolving them is a
    /// separate policy-gated step.
    pub content_refs: Vec<ContentRef>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the isolation process stats record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct IsolationProcessStatsRecord {
    /// Stable environment id used for typed lineage, lookup, or dedupe.
    pub environment_id: ExecutionEnvironmentId,
    /// Typed snapshot ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub snapshot_ref: ProcessStatsSnapshotRef,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the isolation cleanup intent record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct IsolationCleanupIntentRecord {
    /// Stable environment id used for typed lineage, lookup, or dedupe.
    pub environment_id: ExecutionEnvironmentId,
    /// Typed cleanup plan ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub cleanup_plan_ref: CleanupPlanRef,
    /// Effect intent used by this record or request.
    pub effect_intent: EffectIntent,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the isolation cleanup result record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct IsolationCleanupResultRecord {
    /// Stable environment id used for typed lineage, lookup, or dedupe.
    pub environment_id: ExecutionEnvironmentId,
    /// Typed cleanup plan ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub cleanup_plan_ref: CleanupPlanRef,
    /// Finite status for this record or lifecycle stage.
    pub status: CleanupStatus,
    /// Effect result used by this record or request.
    pub effect_result: EffectResult,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the isolation failure record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct IsolationFailureRecord {
    /// Stable environment id used for typed lineage, lookup, or dedupe.
    pub environment_id: ExecutionEnvironmentId,
    /// Typed adapter ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub adapter_ref: Option<IsolationRuntimeRef>,
    /// Redacted explanation for a denial, failure, status, or package delta.
    pub reason: String,
    /// Retry classification used by this record or request.
    pub retry_classification: crate::RetryClassification,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the isolation event base record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct IsolationEventBase {
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    /// Typed subject ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub subject_ref: EntityRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed related refs references. Resolving them is separate from
    /// constructing this record.
    pub related_refs: Vec<EntityRef>,
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
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention: RetentionClass,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the isolation event record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct IsolationEventRecord {
    /// Wire schema version used for compatibility checks.
    pub schema_version: u16,
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    /// Kind discriminator for event kind.
    /// Use it to route finite match arms without parsing display text.
    pub event_kind: IsolationEventKind,
    /// Typed subject ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub subject_ref: EntityRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed related refs references. Resolving them is separate from
    /// constructing this record.
    pub related_refs: Vec<EntityRef>,
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
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention: RetentionClass,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite isolation event kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum IsolationEventKind {
    /// Use this variant when the contract needs to represent isolation requested; selecting it has no side effect by itself.
    IsolationRequested,
    /// Use this variant when the contract needs to represent isolation adapter health checked; selecting it has no side effect by itself.
    IsolationAdapterHealthChecked,
    /// Use this variant when the contract needs to represent isolation capability matched; selecting it has no side effect by itself.
    IsolationCapabilityMatched,
    /// Use this variant when the contract needs to represent isolation downgrade denied; selecting it has no side effect by itself.
    IsolationDowngradeDenied,
    /// Use this variant when the contract needs to represent isolation downgrade approved; selecting it has no side effect by itself.
    IsolationDowngradeApproved,
    /// Use this variant when the contract needs to represent isolation image resolved; selecting it has no side effect by itself.
    IsolationImageResolved,
    /// Use this variant when the contract needs to represent isolation rootfs prepared; selecting it has no side effect by itself.
    IsolationRootfsPrepared,
    /// Use this variant when the contract needs to represent isolation session prepared; selecting it has no side effect by itself.
    IsolationSessionPrepared,
    /// Use this variant when the contract needs to represent isolation mounts resolved; selecting it has no side effect by itself.
    IsolationMountsResolved,
    /// Use this variant when the contract needs to represent isolation network prepared; selecting it has no side effect by itself.
    IsolationNetworkPrepared,
    /// Use this variant when the contract needs to represent isolation secrets prepared; selecting it has no side effect by itself.
    IsolationSecretsPrepared,
    /// Use this variant when the contract needs to represent isolation environment prepared; selecting it has no side effect by itself.
    IsolationEnvironmentPrepared,
    /// Use this variant when the contract needs to represent isolation process started; selecting it has no side effect by itself.
    IsolationProcessStarted,
    /// Use this variant when the contract needs to represent isolation process io captured; selecting it has no side effect by itself.
    IsolationProcessIoCaptured,
    /// Use this variant when the contract needs to represent isolation process stats recorded; selecting it has no side effect by itself.
    IsolationProcessStatsRecorded,
    /// Use this variant when the contract needs to represent isolation process signalled; selecting it has no side effect by itself.
    IsolationProcessSignalled,
    /// Use this variant when the contract needs to represent isolation process exited; selecting it has no side effect by itself.
    IsolationProcessExited,
    /// Use this variant when the contract needs to represent isolation cleanup started; selecting it has no side effect by itself.
    IsolationCleanupStarted,
    /// Use this variant when the contract needs to represent isolation cleanup completed; selecting it has no side effect by itself.
    IsolationCleanupCompleted,
    /// Use this variant when the contract needs to represent isolation cleanup failed; selecting it has no side effect by itself.
    IsolationCleanupFailed,
    /// Use this variant when the contract needs to represent isolation failed; selecting it has no side effect by itself.
    IsolationFailed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the isolation network prepared record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct IsolationNetworkPreparedRecord {
    /// Stable environment id used for typed lineage, lookup, or dedupe.
    pub environment_id: ExecutionEnvironmentId,
    /// Typed network namespace ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub network_namespace_ref: NetworkNamespaceRef,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}
