//! Durable and observable SDK records. Use these DTOs for events, journals, effects,
//! context, output, and feature evidence. Constructing records is data-only;
//! persistence, publication, and external actions happen through ports or application
//! coordinators. This file contains the extension portion of that contract.
//!
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        ContentRef, DestinationRef, EffectId, EntityKind, EntityRef, IdempotencyKey, PolicyRef,
        PrivacyClass, RetentionClass, RunId, SourceRef, TurnId,
    },
    effect::{EffectIntent, EffectResult, EffectTerminalStatus},
    package_extension::{ExtensionActionKind, ExtensionActionRef, ExtensionActionRequestId},
    policy::RiskClass,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the extension action record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ExtensionActionRecord {
    /// Stable request id used for typed lineage, lookup, or dedupe.
    pub request_id: ExtensionActionRequestId,
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Turn identifier for one loop turn within a run.
    pub turn_id: Option<TurnId>,
    /// Typed action ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub action_ref: ExtensionActionRef,
    /// Kind discriminator for action kind.
    /// Use it to route finite match arms without parsing display text.
    pub action_kind: ExtensionActionKind,
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
    /// Risk classification for the operation or capability.
    /// Policy uses it to decide whether approval, sandboxing, or denial is required.
    pub risk_class: RiskClass,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention: RetentionClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed input refs references. Resolving them is separate from
    /// constructing this record.
    pub input_refs: Vec<ContentRef>,
    /// Safe summary of extension or tool input.
    /// It lets events and journals describe the request without exposing raw input.
    pub redacted_input_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency_key: Option<IdempotencyKey>,
    /// Finite status for this record or lifecycle stage.
    pub status: ExtensionActionRecordStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed approval request ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub approval_request_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional effect intent value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub effect_intent: Option<EffectIntent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional effect result value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub effect_result: Option<EffectResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Reason policy denied the requested operation.
    /// Expose it as redacted diagnostic text rather than raw private content.
    pub denied_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Reason a pending side effect is unsafe to retry automatically.
    /// Recovery uses it to require repair or reconciliation before continuing.
    pub unsafe_pending_reason: Option<String>,
}

impl ExtensionActionRecord {
    /// Builds the submitted value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn submitted(params: ExtensionActionRecordParams) -> Self {
        Self {
            request_id: params.request_id,
            run_id: params.run_id,
            turn_id: params.turn_id,
            action_ref: params.action_ref,
            action_kind: params.action_kind,
            source: params.source,
            destination: params.destination,
            policy_refs: params.policy_refs,
            risk_class: params.risk_class,
            privacy: params.privacy,
            retention: params.retention,
            input_refs: params.input_refs,
            redacted_input_summary: params.redacted_input_summary,
            idempotency_key: params.idempotency_key,
            status: ExtensionActionRecordStatus::Submitted,
            approval_request_ref: None,
            effect_intent: None,
            effect_result: None,
            denied_reason: None,
            unsafe_pending_reason: None,
        }
    }

    /// Returns an updated records::extension value with subject ref applied.
    /// This is data construction only and does not execute the configured
    /// behavior.
    pub fn subject_ref(&self) -> EntityRef {
        EntityRef::new(
            EntityKind::ExtensionAction,
            self.action_ref.subject_id().as_str(),
        )
    }

    /// Returns this value with its denial setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_denial(mut self, reason: impl Into<String>) -> Self {
        self.status = ExtensionActionRecordStatus::Denied;
        self.denied_reason = Some(reason.into());
        self
    }

    /// Returns this value with its intent setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_intent(mut self, intent: EffectIntent) -> Self {
        self.status = ExtensionActionRecordStatus::Started;
        self.effect_intent = Some(intent);
        self
    }

    /// Returns this value with its result setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_result(mut self, result: EffectResult) -> Self {
        self.status = ExtensionActionRecordStatus::from_terminal_status(&result.terminal_status);
        self.effect_result = Some(result);
        self
    }

    /// Returns this value with its recovery required setting replaced.
    /// The method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_recovery_required(
        mut self,
        result: EffectResult,
        unsafe_pending_reason: impl Into<String>,
    ) -> Self {
        self.status = ExtensionActionRecordStatus::RecoveryRequired;
        self.effect_result = Some(result);
        self.unsafe_pending_reason = Some(unsafe_pending_reason.into());
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the extension action record params record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ExtensionActionRecordParams {
    /// Stable request id used for typed lineage, lookup, or dedupe.
    pub request_id: ExtensionActionRequestId,
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Turn identifier for one loop turn within a run.
    pub turn_id: Option<TurnId>,
    /// Typed action ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub action_ref: ExtensionActionRef,
    /// Kind discriminator for action kind.
    /// Use it to route finite match arms without parsing display text.
    pub action_kind: ExtensionActionKind,
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
    /// Risk classification for the operation or capability.
    /// Policy uses it to decide whether approval, sandboxing, or denial is required.
    pub risk_class: RiskClass,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention: RetentionClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed input refs references. Resolving them is separate from
    /// constructing this record.
    pub input_refs: Vec<ContentRef>,
    /// Safe summary of extension or tool input.
    /// It lets events and journals describe the request without exposing raw input.
    pub redacted_input_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency_key: Option<IdempotencyKey>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite extension action record status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ExtensionActionRecordStatus {
    /// Use this variant when the contract needs to represent submitted; selecting it has no side effect by itself.
    Submitted,
    /// Use this variant when the contract needs to represent started; selecting it has no side effect by itself.
    Started,
    /// Use this variant when the contract needs to represent completed; selecting it has no side effect by itself.
    Completed,
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed,
    /// Use this variant when the contract needs to represent timed out; selecting it has no side effect by itself.
    TimedOut,
    /// Use this variant when the contract needs to represent cancelled; selecting it has no side effect by itself.
    Cancelled,
    /// Use this variant when the contract needs to represent denied; selecting it has no side effect by itself.
    Denied,
    /// Use this variant when the contract needs to represent unknown; selecting it has no side effect by itself.
    Unknown,
    /// Use this variant when the contract needs to represent recovery required; selecting it has no side effect by itself.
    RecoveryRequired,
}

impl ExtensionActionRecordStatus {
    fn from_terminal_status(status: &EffectTerminalStatus) -> Self {
        match status {
            EffectTerminalStatus::Completed => Self::Completed,
            EffectTerminalStatus::Failed => Self::Failed,
            EffectTerminalStatus::TimedOut => Self::TimedOut,
            EffectTerminalStatus::Cancelled => Self::Cancelled,
            EffectTerminalStatus::DeniedBeforeExecution => Self::Denied,
            EffectTerminalStatus::Unknown => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the extension action event record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ExtensionActionEvent {
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: ExtensionActionEventKind,
    /// Stable request id used for typed lineage, lookup, or dedupe.
    pub request_id: ExtensionActionRequestId,
    /// Typed action ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub action_ref: ExtensionActionRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable effect id used for typed lineage, lookup, or dedupe.
    pub effect_id: Option<EffectId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

impl ExtensionActionEvent {
    /// Constructs this value from record. Use it when adapting
    /// canonical SDK records without introducing a second behavior
    /// path.
    pub fn from_record(
        kind: ExtensionActionEventKind,
        record: &ExtensionActionRecord,
        runtime_package_fingerprint: impl Into<String>,
        redacted_summary: impl Into<String>,
    ) -> Self {
        let effect_id = record
            .effect_result
            .as_ref()
            .map(|result| result.effect_id.clone())
            .or_else(|| {
                record
                    .effect_intent
                    .as_ref()
                    .map(|intent| intent.effect_id.clone())
            });
        Self {
            kind,
            request_id: record.request_id.clone(),
            action_ref: record.action_ref.clone(),
            effect_id,
            policy_refs: record.policy_refs.clone(),
            runtime_package_fingerprint: runtime_package_fingerprint.into(),
            privacy: record.privacy,
            redacted_summary: redacted_summary.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite extension action event kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ExtensionActionEventKind {
    /// Use this variant when the contract needs to represent submitted; selecting it has no side effect by itself.
    Submitted,
    /// Use this variant when the contract needs to represent started; selecting it has no side effect by itself.
    Started,
    /// Use this variant when the contract needs to represent completed; selecting it has no side effect by itself.
    Completed,
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed,
    /// Use this variant when the contract needs to represent denied; selecting it has no side effect by itself.
    Denied,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the extension protocol recovery record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ExtensionProtocolRecoveryRecord {
    /// Stable protocol request id used for typed lineage, lookup, or dedupe.
    pub protocol_request_id: String,
    /// Machine-readable error kind reported by the adapter or protocol layer.
    /// Use it to classify recovery without parsing display text.
    pub error_kind: String,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
}
