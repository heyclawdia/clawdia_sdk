pub use crate::domain::JournalCursor;

use serde::{Deserialize, Serialize};

use crate::{
    approval_records::ApprovalRecord,
    domain::{
        AgentError, AgentId, AgentPoolId, AttemptId, ContentRef, ContextProjectionId,
        CorrelationEntry, DedupeKey, DestinationRef, EntityRef, IdempotencyKey, MessageId,
        PolicyRef, PrivacyClass, RunId, SourceRef, TopicId, TurnId, WakeConditionId,
    },
    effect::{EffectIntent, EffectResult},
    event::{EventCorrelation, EventFilterFingerprint},
    extension_records::ExtensionActionRecord,
    hook_records::HookRecord,
    output_delivery::OutputDeliveryRecord,
    provider::{ProviderStopReason, ProviderUsage},
    realtime_records::RealtimeSessionRecord,
    records_isolation::IsolationRecord,
    stream_records::StreamRuleRecord,
    structured_output::{
        RepairExhaustionRecord, RepairRecord, StructuredOutputLifecycleRecord, ValidationRecord,
    },
    subagent_records::{ChildLifecycleRecord, SubagentRecord},
    tool_records::ToolCallRecord,
    validated_output::{TypedResultPublicationRecord, ValidatedOutput, ValidationReportRecord},
};

pub const JOURNAL_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalRecordKind {
    Run,
    Turn,
    Context,
    Message,
    ModelAttempt,
    StructuredOutput,
    StreamRule,
    RealtimeSession,
    Hook,
    Approval,
    Tool,
    Isolation,
    ChildLifecycle,
    AgentPool,
    RunMessage,
    Wake,
    OutputDispatch,
    Subagent,
    ExtensionAction,
    Telemetry,
    Recovery,
    Checkpoint,
    EffectIntent,
    EffectResult,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventIndexProjection {
    pub run_id: RunId,
    pub agent_id: AgentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<TurnId>,
    pub event_family: String,
    pub event_kind: String,
    pub source: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<DestinationRef>,
    pub subject_ref: EntityRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_refs: Vec<EntityRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub correlation_keys: Vec<CorrelationEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub privacy_class: PrivacyClass,
    pub delivery_semantics: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct JournalRecord {
    pub journal_schema_version: u16,
    pub journal_seq: u64,
    pub record_id: String,
    pub record_kind: JournalRecordKind,
    pub run_id: RunId,
    pub agent_id: AgentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<TurnId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<AttemptId>,
    pub subject_ref: EntityRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_refs: Vec<EntityRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub causal_refs: Vec<String>,
    pub source: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<DestinationRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub correlation_keys: Vec<CorrelationEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub delivery_semantics: String,
    pub event_index: EventIndexProjection,
    pub timestamp_millis: u64,
    pub runtime_package_fingerprint: String,
    pub privacy: PrivacyClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_refs: Vec<ContentRef>,
    pub redaction_policy_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedupe_key: Option<DedupeKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_ref: Option<String>,
    pub payload: JournalRecordPayload,
}

impl JournalRecord {
    pub fn effect_intent(base: JournalRecordBase, intent: EffectIntent) -> Self {
        let mut event_index = base.event_index("effect", "intent");
        let effect_ref =
            EntityRef::new(crate::domain::EntityKind::Effect, intent.effect_id.clone());
        event_index.subject_ref = intent.subject_ref.clone();
        event_index.related_refs = vec![effect_ref.clone()];
        Self {
            journal_schema_version: JOURNAL_SCHEMA_VERSION,
            journal_seq: base.journal_seq,
            record_id: base.record_id,
            record_kind: JournalRecordKind::EffectIntent,
            run_id: base.run_id,
            agent_id: base.agent_id,
            turn_id: base.turn_id,
            attempt_id: base.attempt_id,
            subject_ref: intent.subject_ref.clone(),
            related_refs: vec![effect_ref],
            causal_refs: base.causal_refs,
            source: intent.source.clone(),
            destination: intent.destination.clone(),
            correlation_keys: Vec::new(),
            tags: base.tags,
            delivery_semantics: "journal_backed".to_string(),
            event_index,
            timestamp_millis: base.timestamp_millis,
            runtime_package_fingerprint: base.runtime_package_fingerprint,
            privacy: base.privacy,
            content_refs: intent.content_refs.clone(),
            redaction_policy_id: base.redaction_policy_id,
            idempotency_key: intent.idempotency_key.clone(),
            dedupe_key: intent.dedupe_key.clone(),
            checkpoint_ref: base.checkpoint_ref,
            payload: JournalRecordPayload::EffectIntent(intent),
        }
    }

    pub fn effect_result(base: JournalRecordBase, result: EffectResult) -> Self {
        let effect_ref =
            EntityRef::new(crate::domain::EntityKind::Effect, result.effect_id.clone());
        let mut event_index = base.event_index("effect", "result");
        event_index.subject_ref = effect_ref.clone();
        event_index.related_refs = vec![effect_ref.clone()];
        Self {
            journal_schema_version: JOURNAL_SCHEMA_VERSION,
            journal_seq: base.journal_seq,
            record_id: base.record_id,
            record_kind: JournalRecordKind::EffectResult,
            run_id: base.run_id,
            agent_id: base.agent_id,
            turn_id: base.turn_id,
            attempt_id: base.attempt_id,
            subject_ref: effect_ref.clone(),
            related_refs: vec![effect_ref],
            causal_refs: base.causal_refs,
            source: base.source,
            destination: base.destination,
            correlation_keys: Vec::new(),
            tags: base.tags,
            delivery_semantics: "journal_backed".to_string(),
            event_index,
            timestamp_millis: base.timestamp_millis,
            runtime_package_fingerprint: base.runtime_package_fingerprint,
            privacy: base.privacy,
            content_refs: result.content_refs.clone(),
            redaction_policy_id: base.redaction_policy_id,
            idempotency_key: None,
            dedupe_key: None,
            checkpoint_ref: base.checkpoint_ref,
            payload: JournalRecordPayload::EffectResult(result),
        }
    }

    pub fn checkpoint(base: JournalRecordBase, checkpoint: RunCheckpoint) -> Self {
        let event_index = base.event_index("checkpoint", "saved");
        Self {
            journal_schema_version: JOURNAL_SCHEMA_VERSION,
            journal_seq: base.journal_seq,
            record_id: base.record_id,
            record_kind: JournalRecordKind::Checkpoint,
            run_id: base.run_id.clone(),
            agent_id: base.agent_id,
            turn_id: base.turn_id,
            attempt_id: base.attempt_id,
            subject_ref: EntityRef::run(base.run_id),
            related_refs: Vec::new(),
            causal_refs: base.causal_refs,
            source: base.source,
            destination: base.destination,
            correlation_keys: Vec::new(),
            tags: base.tags,
            delivery_semantics: "journal_backed".to_string(),
            event_index,
            timestamp_millis: base.timestamp_millis,
            runtime_package_fingerprint: base.runtime_package_fingerprint,
            privacy: base.privacy,
            content_refs: checkpoint.content_ref_manifest.clone(),
            redaction_policy_id: base.redaction_policy_id,
            idempotency_key: None,
            dedupe_key: None,
            checkpoint_ref: Some(checkpoint.checkpoint_id.clone()),
            payload: JournalRecordPayload::Checkpoint(checkpoint),
        }
    }

    pub fn recovery(base: JournalRecordBase, recovery: RecoveryMarker) -> Self {
        let event_index = base.event_index("recovery", "marker");
        Self {
            journal_schema_version: JOURNAL_SCHEMA_VERSION,
            journal_seq: base.journal_seq,
            record_id: base.record_id,
            record_kind: JournalRecordKind::Recovery,
            run_id: base.run_id.clone(),
            agent_id: base.agent_id,
            turn_id: base.turn_id,
            attempt_id: base.attempt_id,
            subject_ref: EntityRef::run(base.run_id),
            related_refs: Vec::new(),
            causal_refs: base.causal_refs,
            source: base.source,
            destination: base.destination,
            correlation_keys: Vec::new(),
            tags: base.tags,
            delivery_semantics: "journal_backed".to_string(),
            event_index,
            timestamp_millis: base.timestamp_millis,
            runtime_package_fingerprint: base.runtime_package_fingerprint,
            privacy: base.privacy,
            content_refs: Vec::new(),
            redaction_policy_id: base.redaction_policy_id,
            idempotency_key: None,
            dedupe_key: None,
            checkpoint_ref: base.checkpoint_ref,
            payload: JournalRecordPayload::Recovery(recovery),
        }
    }

    pub fn feature_record(
        base: JournalRecordBase,
        record_kind: JournalRecordKind,
        event_family: impl Into<String>,
        event_kind: impl Into<String>,
        subject_ref: EntityRef,
        related_refs: Vec<EntityRef>,
        content_refs: Vec<ContentRef>,
        payload: JournalRecordPayload,
    ) -> Self {
        let mut event_index = base.event_index(event_family, event_kind);
        event_index.subject_ref = subject_ref.clone();
        event_index.related_refs = related_refs.clone();
        Self {
            journal_schema_version: JOURNAL_SCHEMA_VERSION,
            journal_seq: base.journal_seq,
            record_id: base.record_id,
            record_kind,
            run_id: base.run_id,
            agent_id: base.agent_id,
            turn_id: base.turn_id,
            attempt_id: base.attempt_id,
            subject_ref,
            related_refs,
            causal_refs: base.causal_refs,
            source: base.source,
            destination: base.destination,
            correlation_keys: Vec::new(),
            tags: base.tags,
            delivery_semantics: "journal_backed".to_string(),
            event_index,
            timestamp_millis: base.timestamp_millis,
            runtime_package_fingerprint: base.runtime_package_fingerprint,
            privacy: base.privacy,
            content_refs,
            redaction_policy_id: base.redaction_policy_id,
            idempotency_key: None,
            dedupe_key: None,
            checkpoint_ref: base.checkpoint_ref,
            payload,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct JournalRecordBase {
    pub journal_seq: u64,
    pub record_id: String,
    pub run_id: RunId,
    pub agent_id: AgentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<TurnId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<AttemptId>,
    pub source: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<DestinationRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub causal_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub timestamp_millis: u64,
    pub runtime_package_fingerprint: String,
    pub privacy: PrivacyClass,
    pub redaction_policy_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_ref: Option<String>,
}

impl JournalRecordBase {
    pub fn new(
        journal_seq: u64,
        record_id: impl Into<String>,
        run_id: RunId,
        agent_id: AgentId,
        source: SourceRef,
    ) -> Self {
        Self {
            journal_seq,
            record_id: record_id.into(),
            run_id,
            agent_id,
            turn_id: None,
            attempt_id: None,
            source,
            destination: None,
            causal_refs: Vec::new(),
            tags: Vec::new(),
            timestamp_millis: 0,
            runtime_package_fingerprint: "runtime.package.fingerprint.test".to_string(),
            privacy: PrivacyClass::ContentRefsOnly,
            redaction_policy_id: "redaction.default".to_string(),
            checkpoint_ref: None,
        }
    }

    fn event_index(
        &self,
        event_family: impl Into<String>,
        event_kind: impl Into<String>,
    ) -> EventIndexProjection {
        EventIndexProjection {
            run_id: self.run_id.clone(),
            agent_id: self.agent_id.clone(),
            turn_id: self.turn_id.clone(),
            event_family: event_family.into(),
            event_kind: event_kind.into(),
            source: self.source.clone(),
            destination: self.destination.clone(),
            subject_ref: EntityRef::run(self.run_id.clone()),
            related_refs: Vec::new(),
            correlation_keys: Vec::new(),
            tags: self.tags.clone(),
            privacy_class: self.privacy.clone(),
            delivery_semantics: "journal_backed".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JournalRecordPayload {
    RunLifecycle(RunLifecycleRecord),
    ContextProjection(ContextProjectionRecord),
    ModelAttempt(ModelAttemptRecord),
    Message(MessageRecord),
    StructuredOutput(StructuredOutputRecord),
    Approval(ApprovalRecord),
    Tool(ToolCallRecord),
    OutputDelivery(OutputDeliveryRecord),
    Hook(HookRecord),
    StreamRule(StreamRuleRecord),
    RealtimeSession(RealtimeSessionRecord),
    Isolation(IsolationRecord),
    ChildLifecycle(ChildLifecycleRecord),
    AgentPool(AgentPoolRecord),
    RunMessage(RunMessageRecord),
    Wake(WakeRecord),
    Subagent(SubagentRecord),
    ExtensionAction(ExtensionActionRecord),
    EffectIntent(EffectIntent),
    EffectResult(EffectResult),
    Checkpoint(RunCheckpoint),
    Recovery(RecoveryMarker),
    TerminalResult(TerminalResultMarker),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "record_type", content = "record", rename_all = "snake_case")]
pub enum StructuredOutputRecord {
    Lifecycle(StructuredOutputLifecycleRecord),
    Validation(ValidationRecord),
    Repair(RepairRecord),
    RepairExhaustion(RepairExhaustionRecord),
    ValidationReport(ValidationReportRecord),
    ValidatedOutput(ValidatedOutput),
    TypedResultPublication(TypedResultPublicationRecord),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RunLifecycleRecord {
    pub status: String,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContextProjectionRecord {
    pub projection_id: ContextProjectionId,
    pub selected_item_count: u32,
    pub provider_destination: DestinationRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ModelAttemptRecord {
    pub provider_route_id: String,
    pub provider_model_id: String,
    pub request_message_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<ProviderStopReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ProviderUsage>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MessageRecord {
    pub message_id: MessageId,
    pub role: String,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AgentPoolRecord {
    pub pool_id: AgentPoolId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub member_run_ids: Vec<RunId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<TopicId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub lifecycle_status: AgentPoolLifecycleStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentPoolLifecycleStatus {
    Created,
    RunJoined,
    RunLeft,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RunMessageRecord {
    pub message_id: MessageId,
    pub source_run_id: RunId,
    pub address_target: RunMessageAddressTargetRecord,
    pub content_ref: ContentRef,
    pub correlation: EventCorrelation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<MessageId>,
    pub delivery_status: RunMessageDeliveryStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delivered_to: Vec<RunId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub idempotency_key: IdempotencyKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_intent: Option<EffectIntent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_result: Option<EffectResult>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunMessageAddressTargetRecord {
    Run { run_id: RunId },
    Agent { agent_id: AgentId },
    Topic { topic_id: TopicId },
    Pool { pool_id: AgentPoolId },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunMessageDeliveryStatus {
    Accepted,
    Delivered,
    Responded,
    Failed,
    TimedOut,
    Expired,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WakeRecord {
    pub condition_id: WakeConditionId,
    pub run_id: RunId,
    pub event_filter_fingerprint: EventFilterFingerprint,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_millis: Option<u64>,
    pub resume_policy: WakeResumeInputPolicyRecord,
    pub trigger_status: WakeTriggerStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub idempotency_key: IdempotencyKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_event_id: Option<crate::domain::EventId>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WakeResumeInputPolicyRecord {
    MatchingEventRefs,
    RedactedSummary,
    None,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WakeTriggerStatus {
    Registered,
    Triggered,
    TimedOut,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RunCheckpoint {
    pub checkpoint_id: String,
    pub run_id: RunId,
    pub checkpoint_seq: u64,
    pub covers_journal_seq: u64,
    pub loop_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<TurnId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<AttemptId>,
    pub runtime_package_fingerprint: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_side_effects: Vec<PendingSideEffect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_approvals: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_ref_manifest: Vec<ContentRef>,
    pub state_hash: String,
    pub created_at_millis: u64,
    pub writer_id: String,
}

impl RunCheckpoint {
    pub fn validate_against_latest_seq(&self, latest_journal_seq: u64) -> Result<(), AgentError> {
        if self.covers_journal_seq > latest_journal_seq {
            return Err(AgentError::contract_violation(
                "checkpoint covers_journal_seq cannot point past latest committed journal record",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PendingSideEffect {
    pub effect_id: crate::domain::EffectId,
    pub intent_record_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedupe_key: Option<DedupeKey>,
    pub unsafe_pending_reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TerminalResultMarker {
    pub effect_id: crate::domain::EffectId,
    pub result_record_id: String,
    pub terminal_status: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecoveryMarker {
    pub unsafe_pending: Vec<PendingSideEffect>,
    pub recovery_reason: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
}
