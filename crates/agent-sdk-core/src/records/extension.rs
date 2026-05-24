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
pub struct ExtensionActionRecord {
    pub request_id: ExtensionActionRequestId,
    pub run_id: RunId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<TurnId>,
    pub action_ref: ExtensionActionRef,
    pub action_kind: ExtensionActionKind,
    pub source: SourceRef,
    pub destination: DestinationRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub risk_class: RiskClass,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_refs: Vec<ContentRef>,
    pub redacted_input_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
    pub status: ExtensionActionRecordStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_request_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_intent: Option<EffectIntent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_result: Option<EffectResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub denied_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsafe_pending_reason: Option<String>,
}

impl ExtensionActionRecord {
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

    pub fn subject_ref(&self) -> EntityRef {
        EntityRef::new(
            EntityKind::ExtensionAction,
            self.action_ref.subject_id().as_str(),
        )
    }

    pub fn with_denial(mut self, reason: impl Into<String>) -> Self {
        self.status = ExtensionActionRecordStatus::Denied;
        self.denied_reason = Some(reason.into());
        self
    }

    pub fn with_intent(mut self, intent: EffectIntent) -> Self {
        self.status = ExtensionActionRecordStatus::Started;
        self.effect_intent = Some(intent);
        self
    }

    pub fn with_result(mut self, result: EffectResult) -> Self {
        self.status = ExtensionActionRecordStatus::from_terminal_status(&result.terminal_status);
        self.effect_result = Some(result);
        self
    }

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
pub struct ExtensionActionRecordParams {
    pub request_id: ExtensionActionRequestId,
    pub run_id: RunId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<TurnId>,
    pub action_ref: ExtensionActionRef,
    pub action_kind: ExtensionActionKind,
    pub source: SourceRef,
    pub destination: DestinationRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub risk_class: RiskClass,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_refs: Vec<ContentRef>,
    pub redacted_input_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionActionRecordStatus {
    Submitted,
    Started,
    Completed,
    Failed,
    TimedOut,
    Cancelled,
    Denied,
    Unknown,
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
pub struct ExtensionActionEvent {
    pub kind: ExtensionActionEventKind,
    pub request_id: ExtensionActionRequestId,
    pub action_ref: ExtensionActionRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_id: Option<EffectId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub runtime_package_fingerprint: String,
    pub privacy: PrivacyClass,
    pub redacted_summary: String,
}

impl ExtensionActionEvent {
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
            privacy: record.privacy.clone(),
            redacted_summary: redacted_summary.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionActionEventKind {
    Submitted,
    Started,
    Completed,
    Failed,
    Denied,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExtensionProtocolRecoveryRecord {
    pub protocol_request_id: String,
    pub error_kind: String,
    pub redacted_summary: String,
    pub runtime_package_fingerprint: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
}
