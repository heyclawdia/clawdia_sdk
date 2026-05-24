use core::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    capability::{CapabilityId, CapabilityNamespace, ExecutorRef, PackageSidecarRef},
    domain::{
        ContentRef, DestinationRef, EffectId, EntityKind, EntityRef, IdValidationError,
        IdempotencyKey, PolicyRef, PrivacyClass, RetentionClass, RunId, SourceRef, ToolCallId,
        TurnId,
    },
    effect::{EffectIntent, EffectResult, EffectTerminalStatus},
    journal::{
        EventIndexProjection, JOURNAL_SCHEMA_VERSION, JournalRecord, JournalRecordBase,
        JournalRecordKind, JournalRecordPayload,
    },
    policy::{EffectClass, PolicyOutcome, RiskClass},
};

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CanonicalToolName(String);

impl CanonicalToolName {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("CanonicalToolName must be valid")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, IdValidationError> {
        let value = value.into();
        validate_tool_name(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for CanonicalToolName {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for CanonicalToolName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(serde::de::Error::custom)
    }
}

impl fmt::Debug for CanonicalToolName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CanonicalToolName(redacted)")
    }
}

impl fmt::Display for CanonicalToolName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CanonicalToolName(redacted)")
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolCallRecord {
    pub tool_call_id: ToolCallId,
    pub run_id: RunId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<TurnId>,
    pub capability_id: CapabilityId,
    pub canonical_tool_name: CanonicalToolName,
    pub namespace: CapabilityNamespace,
    pub source: SourceRef,
    pub destination: DestinationRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executor_ref: Option<ExecutorRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sidecar_refs: Vec<PackageSidecarRef>,
    pub effect_class: EffectClass,
    pub risk_class: RiskClass,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requested_args_refs: Vec<ContentRef>,
    pub redacted_args_summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub result_content_refs: Vec<ContentRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted_result_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
    pub status: ToolCallRecordStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_outcome: Option<PolicyOutcome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_intent: Option<EffectIntent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_result: Option<EffectResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsafe_pending_reason: Option<String>,
}

impl ToolCallRecord {
    pub fn requested(params: ToolCallRecordParams) -> Self {
        Self {
            tool_call_id: params.tool_call_id,
            run_id: params.run_id,
            turn_id: params.turn_id,
            capability_id: params.capability_id,
            canonical_tool_name: params.canonical_tool_name,
            namespace: params.namespace,
            source: params.source,
            destination: params.destination,
            executor_ref: params.executor_ref,
            policy_refs: params.policy_refs,
            sidecar_refs: params.sidecar_refs,
            effect_class: params.effect_class,
            risk_class: params.risk_class,
            privacy: params.privacy,
            retention: params.retention,
            requested_args_refs: params.requested_args_refs,
            redacted_args_summary: params.redacted_args_summary,
            result_content_refs: Vec::new(),
            redacted_result_summary: None,
            idempotency_key: params.idempotency_key,
            status: ToolCallRecordStatus::Requested,
            policy_outcome: None,
            effect_intent: None,
            effect_result: None,
            unsafe_pending_reason: None,
        }
    }

    pub fn subject_ref(&self) -> EntityRef {
        EntityRef::new(EntityKind::ToolCall, self.tool_call_id.clone())
    }

    pub fn with_denial(mut self, policy_outcome: PolicyOutcome) -> Self {
        self.status = ToolCallRecordStatus::DeniedBeforeExecution;
        self.policy_outcome = Some(policy_outcome);
        self
    }

    pub fn with_intent(mut self, intent: EffectIntent) -> Self {
        self.status = ToolCallRecordStatus::IntentRecorded;
        self.effect_intent = Some(intent);
        self
    }

    pub fn with_result(mut self, result: EffectResult, policy_outcome: PolicyOutcome) -> Self {
        self.status = ToolCallRecordStatus::from_terminal_status(&result.terminal_status);
        self.result_content_refs = result.content_refs.clone();
        self.redacted_result_summary = Some(result.redacted_summary.clone());
        self.policy_outcome = Some(policy_outcome);
        self.effect_result = Some(result);
        self
    }

    pub fn with_recovery_required(
        mut self,
        result: EffectResult,
        unsafe_pending_reason: impl Into<String>,
    ) -> Self {
        self.status = ToolCallRecordStatus::RecoveryRequired;
        self.result_content_refs = result.content_refs.clone();
        self.redacted_result_summary = Some(result.redacted_summary.clone());
        self.effect_result = Some(result);
        self.unsafe_pending_reason = Some(unsafe_pending_reason.into());
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolCallRecordParams {
    pub tool_call_id: ToolCallId,
    pub run_id: RunId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<TurnId>,
    pub capability_id: CapabilityId,
    pub canonical_tool_name: CanonicalToolName,
    pub namespace: CapabilityNamespace,
    pub source: SourceRef,
    pub destination: DestinationRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executor_ref: Option<ExecutorRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sidecar_refs: Vec<PackageSidecarRef>,
    pub effect_class: EffectClass,
    pub risk_class: RiskClass,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requested_args_refs: Vec<ContentRef>,
    pub redacted_args_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallRecordStatus {
    Requested,
    IntentRecorded,
    Completed,
    Failed,
    TimedOut,
    Cancelled,
    DeniedBeforeExecution,
    Unknown,
    RecoveryRequired,
}

impl ToolCallRecordStatus {
    fn from_terminal_status(status: &EffectTerminalStatus) -> Self {
        match status {
            EffectTerminalStatus::Completed => Self::Completed,
            EffectTerminalStatus::Failed => Self::Failed,
            EffectTerminalStatus::TimedOut => Self::TimedOut,
            EffectTerminalStatus::Cancelled => Self::Cancelled,
            EffectTerminalStatus::DeniedBeforeExecution => Self::DeniedBeforeExecution,
            EffectTerminalStatus::Unknown => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolResultRef {
    pub tool_call_id: ToolCallId,
    pub effect_id: EffectId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_refs: Vec<ContentRef>,
    pub redacted_summary: String,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
}

impl ToolResultRef {
    pub fn from_record(record: &ToolCallRecord) -> Option<Self> {
        let effect_result = record.effect_result.as_ref()?;
        Some(Self {
            tool_call_id: record.tool_call_id.clone(),
            effect_id: effect_result.effect_id.clone(),
            content_refs: record.result_content_refs.clone(),
            redacted_summary: record
                .redacted_result_summary
                .clone()
                .unwrap_or_else(|| "tool result redacted".to_string()),
            privacy: record.privacy.clone(),
            retention: record.retention.clone(),
        })
    }
}

fn validate_tool_name(value: &str) -> Result<(), IdValidationError> {
    if value.is_empty() {
        return Err(IdValidationError::Empty);
    }
    if value.len() > crate::domain::MAX_ID_LEN {
        return Err(IdValidationError::TooLong {
            max: crate::domain::MAX_ID_LEN,
            actual: value.len(),
        });
    }
    if let Some((index, _)) = value
        .char_indices()
        .find(|(_, character)| character.is_control())
    {
        return Err(IdValidationError::ControlCharacter { index });
    }
    Ok(())
}

pub fn tool_call_journal_record(
    base: JournalRecordBase,
    record: ToolCallRecord,
    event_kind: impl Into<String>,
) -> JournalRecord {
    let subject_ref = record.subject_ref();
    let mut related_refs = Vec::new();
    if let Some(intent) = &record.effect_intent {
        related_refs.push(EntityRef::new(EntityKind::Effect, intent.effect_id.clone()));
    }
    if let Some(result) = &record.effect_result {
        related_refs.push(EntityRef::new(EntityKind::Effect, result.effect_id.clone()));
    }
    related_refs.extend(
        record
            .requested_args_refs
            .iter()
            .chain(record.result_content_refs.iter())
            .map(|content_ref| EntityRef::new(EntityKind::Content, content_ref.as_str())),
    );
    let content_refs = record
        .requested_args_refs
        .iter()
        .chain(record.result_content_refs.iter())
        .cloned()
        .collect::<Vec<_>>();
    JournalRecord {
        journal_schema_version: JOURNAL_SCHEMA_VERSION,
        journal_seq: base.journal_seq,
        record_id: base.record_id,
        record_kind: JournalRecordKind::Tool,
        run_id: base.run_id.clone(),
        agent_id: base.agent_id.clone(),
        turn_id: base.turn_id.clone(),
        attempt_id: base.attempt_id.clone(),
        subject_ref: subject_ref.clone(),
        related_refs: related_refs.clone(),
        causal_refs: base.causal_refs,
        source: record.source.clone(),
        destination: Some(record.destination.clone()),
        correlation_keys: Vec::new(),
        tags: vec!["tool_execution".to_string()],
        delivery_semantics: "journal_backed".to_string(),
        event_index: EventIndexProjection {
            run_id: base.run_id,
            agent_id: base.agent_id,
            turn_id: base.turn_id,
            event_family: "tool".to_string(),
            event_kind: event_kind.into(),
            source: record.source.clone(),
            destination: Some(record.destination.clone()),
            subject_ref,
            related_refs,
            correlation_keys: Vec::new(),
            tags: vec!["tool_execution".to_string()],
            privacy_class: base.privacy.clone(),
            delivery_semantics: "journal_backed".to_string(),
        },
        timestamp_millis: base.timestamp_millis,
        runtime_package_fingerprint: base.runtime_package_fingerprint,
        privacy: base.privacy,
        content_refs,
        redaction_policy_id: base.redaction_policy_id,
        idempotency_key: record.idempotency_key.clone(),
        dedupe_key: None,
        checkpoint_ref: base.checkpoint_ref,
        payload: JournalRecordPayload::Tool(record),
    }
}
