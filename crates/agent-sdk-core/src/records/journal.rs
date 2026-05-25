//! Durable run-journal records. Use these records as replayable truth for messages,
//! effects, checkpoints, output, and recovery. Record constructors are data-only;
//! append side effects happen through journal ports.
//!
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

/// Constant value for the records::journal contract. Use it to keep SDK
/// records and tests aligned on the same stable value.
pub const JOURNAL_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite journal record kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum JournalRecordKind {
    /// Use this variant when the contract needs to represent run; selecting it has no side effect by itself.
    Run,
    /// Use this variant when the contract needs to represent turn; selecting it has no side effect by itself.
    Turn,
    /// Use this variant when the contract needs to represent context; selecting it has no side effect by itself.
    Context,
    /// Use this variant when the contract needs to represent message; selecting it has no side effect by itself.
    Message,
    /// Use this variant when the contract needs to represent model attempt; selecting it has no side effect by itself.
    ModelAttempt,
    /// Use this variant when the contract needs to represent structured output; selecting it has no side effect by itself.
    StructuredOutput,
    /// Use this variant when the contract needs to represent stream rule; selecting it has no side effect by itself.
    StreamRule,
    /// Use this variant when the contract needs to represent realtime session; selecting it has no side effect by itself.
    RealtimeSession,
    /// Use this variant when the contract needs to represent hook; selecting it has no side effect by itself.
    Hook,
    /// Use this variant when the contract needs to represent approval; selecting it has no side effect by itself.
    Approval,
    /// Use this variant when the contract needs to represent tool; selecting it has no side effect by itself.
    Tool,
    /// Use this variant when the contract needs to represent isolation; selecting it has no side effect by itself.
    Isolation,
    /// Use this variant when the contract needs to represent child lifecycle; selecting it has no side effect by itself.
    ChildLifecycle,
    /// Use this variant when the contract needs to represent agent pool; selecting it has no side effect by itself.
    AgentPool,
    /// Use this variant when the contract needs to represent run message; selecting it has no side effect by itself.
    RunMessage,
    /// Use this variant when the contract needs to represent wake; selecting it has no side effect by itself.
    Wake,
    /// Use this variant when the contract needs to represent output dispatch; selecting it has no side effect by itself.
    OutputDispatch,
    /// Use this variant when the contract needs to represent subagent; selecting it has no side effect by itself.
    Subagent,
    /// Use this variant when the contract needs to represent extension action; selecting it has no side effect by itself.
    ExtensionAction,
    /// Use this variant when the contract needs to represent telemetry; selecting it has no side effect by itself.
    Telemetry,
    /// Use this variant when the contract needs to represent recovery; selecting it has no side effect by itself.
    Recovery,
    /// Use this variant when the contract needs to represent checkpoint; selecting it has no side effect by itself.
    Checkpoint,
    /// Use this variant when the contract needs to represent effect intent; selecting it has no side effect by itself.
    EffectIntent,
    /// Use this variant when the contract needs to represent effect result; selecting it has no side effect by itself.
    EffectResult,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the event index projection record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct EventIndexProjection {
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Turn identifier for one loop turn within a run.
    pub turn_id: Option<TurnId>,
    /// Event family used by this record or request.
    pub event_family: String,
    /// Kind discriminator for event kind.
    /// Use it to route finite match arms without parsing display text.
    pub event_kind: String,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: Option<DestinationRef>,
    /// Typed subject ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub subject_ref: EntityRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed related refs references. Resolving them is separate from
    /// constructing this record.
    pub related_refs: Vec<EntityRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Correlation-key selector for event filtering.
    /// `Any` leaves correlation keys unconstrained; `Include` restricts matches to listed keys.
    pub correlation_keys: Vec<CorrelationEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Tag selector for event filtering.
    /// `Any` leaves tags unconstrained; `Include` restricts matches to listed event tags.
    pub tags: Vec<String>,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy_class: PrivacyClass,
    /// Delivery-semantic selector for event filtering.
    /// `Any` leaves delivery semantics unconstrained; `Include` restricts matches to listed
    /// semantics.
    pub delivery_semantics: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the journal record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct JournalRecord {
    /// Wire schema version for this record shape.
    /// Use it for compatibility checks before deserializing or replaying stored data.
    pub journal_schema_version: u16,
    /// Journal seq used by this record or request.
    pub journal_seq: u64,
    /// Stable record id used for typed lineage, lookup, or dedupe.
    pub record_id: String,
    /// Kind discriminator for record kind.
    /// Use it to route finite match arms without parsing display text.
    pub record_kind: JournalRecordKind,
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Turn identifier for one loop turn within a run.
    pub turn_id: Option<TurnId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Attempt identifier for retry, repair, provider, or tool execution
    /// evidence.
    pub attempt_id: Option<AttemptId>,
    /// Typed subject ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub subject_ref: EntityRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed related refs references. Resolving them is separate from
    /// constructing this record.
    pub related_refs: Vec<EntityRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed causal refs references. Resolving them is separate from
    /// constructing this record.
    pub causal_refs: Vec<String>,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: Option<DestinationRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Correlation-key selector for event filtering.
    /// `Any` leaves correlation keys unconstrained; `Include` restricts matches to listed keys.
    pub correlation_keys: Vec<CorrelationEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Tag selector for event filtering.
    /// `Any` leaves tags unconstrained; `Include` restricts matches to listed event tags.
    pub tags: Vec<String>,
    /// Delivery-semantic selector for event filtering.
    /// `Any` leaves delivery semantics unconstrained; `Include` restricts matches to listed
    /// semantics.
    pub delivery_semantics: String,
    /// Event index used by this record or request.
    pub event_index: EventIndexProjection,
    /// Timestamp in milliseconds associated with this record.
    /// Use it for ordering and diagnostics; durable causality still comes from ids and cursors.
    pub timestamp_millis: u64,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Content references associated with this record; resolving them is a
    /// separate policy-gated step.
    pub content_refs: Vec<ContentRef>,
    /// Stable redaction policy id used for typed lineage, lookup, or dedupe.
    pub redaction_policy_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency_key: Option<IdempotencyKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Dedupe policy or key for a side-effecting operation.
    /// Replay and repair use it to avoid sending or executing the same effect twice.
    pub dedupe_key: Option<DedupeKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed checkpoint ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub checkpoint_ref: Option<String>,
    /// Payload carried by this record.
    /// Use the surrounding policy and redaction fields to decide whether it can be exposed.
    pub payload: JournalRecordPayload,
}

impl JournalRecord {
    /// Returns an updated records::journal value with effect intent applied.
    /// This is data construction only and does not execute the configured
    /// behavior.
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

    /// Builds the effect result record or result value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Builds the checkpoint record or result value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Recovery.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Builds the feature record record or result value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
/// Carries the journal record base record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct JournalRecordBase {
    /// Journal seq used by this record or request.
    pub journal_seq: u64,
    /// Stable record id used for typed lineage, lookup, or dedupe.
    pub record_id: String,
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Turn identifier for one loop turn within a run.
    pub turn_id: Option<TurnId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Attempt identifier for retry, repair, provider, or tool execution
    /// evidence.
    pub attempt_id: Option<AttemptId>,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: Option<DestinationRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed causal refs references. Resolving them is separate from
    /// constructing this record.
    pub causal_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Tag selector for event filtering.
    /// `Any` leaves tags unconstrained; `Include` restricts matches to listed event tags.
    pub tags: Vec<String>,
    /// Timestamp in milliseconds associated with this record.
    /// Use it for ordering and diagnostics; durable causality still comes from ids and cursors.
    pub timestamp_millis: u64,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Stable redaction policy id used for typed lineage, lookup, or dedupe.
    pub redaction_policy_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed checkpoint ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub checkpoint_ref: Option<String>,
}

impl JournalRecordBase {
    /// Creates a new records::journal value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
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
/// Enumerates the finite journal record payload cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum JournalRecordPayload {
    /// Use this variant when the contract needs to represent run lifecycle; selecting it has no side effect by itself.
    RunLifecycle(RunLifecycleRecord),
    /// Use this variant when the contract needs to represent context projection; selecting it has no side effect by itself.
    ContextProjection(ContextProjectionRecord),
    /// Use this variant when the contract needs to represent model attempt; selecting it has no side effect by itself.
    ModelAttempt(ModelAttemptRecord),
    /// Use this variant when the contract needs to represent message; selecting it has no side effect by itself.
    Message(MessageRecord),
    /// Use this variant when the contract needs to represent structured output; selecting it has no side effect by itself.
    StructuredOutput(StructuredOutputRecord),
    /// Use this variant when the contract needs to represent approval; selecting it has no side effect by itself.
    Approval(ApprovalRecord),
    /// Use this variant when the contract needs to represent tool; selecting it has no side effect by itself.
    Tool(ToolCallRecord),
    /// Use this variant when the contract needs to represent output delivery; selecting it has no side effect by itself.
    OutputDelivery(OutputDeliveryRecord),
    /// Use this variant when the contract needs to represent hook; selecting it has no side effect by itself.
    Hook(HookRecord),
    /// Use this variant when the contract needs to represent stream rule; selecting it has no side effect by itself.
    StreamRule(StreamRuleRecord),
    /// Use this variant when the contract needs to represent realtime session; selecting it has no side effect by itself.
    RealtimeSession(RealtimeSessionRecord),
    /// Use this variant when the contract needs to represent isolation; selecting it has no side effect by itself.
    Isolation(IsolationRecord),
    /// Use this variant when the contract needs to represent child lifecycle; selecting it has no side effect by itself.
    ChildLifecycle(ChildLifecycleRecord),
    /// Use this variant when the contract needs to represent agent pool; selecting it has no side effect by itself.
    AgentPool(AgentPoolRecord),
    /// Use this variant when the contract needs to represent run message; selecting it has no side effect by itself.
    RunMessage(RunMessageRecord),
    /// Use this variant when the contract needs to represent wake; selecting it has no side effect by itself.
    Wake(WakeRecord),
    /// Use this variant when the contract needs to represent subagent; selecting it has no side effect by itself.
    Subagent(SubagentRecord),
    /// Use this variant when the contract needs to represent extension action; selecting it has no side effect by itself.
    ExtensionAction(ExtensionActionRecord),
    /// Use this variant when the contract needs to represent effect intent; selecting it has no side effect by itself.
    EffectIntent(EffectIntent),
    /// Use this variant when the contract needs to represent effect result; selecting it has no side effect by itself.
    EffectResult(EffectResult),
    /// Use this variant when the contract needs to represent checkpoint; selecting it has no side effect by itself.
    Checkpoint(RunCheckpoint),
    /// Use this variant when the contract needs to represent recovery; selecting it has no side effect by itself.
    Recovery(RecoveryMarker),
    /// Use this variant when the contract needs to represent terminal result; selecting it has no side effect by itself.
    TerminalResult(TerminalResultMarker),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "record_type", content = "record", rename_all = "snake_case")]
/// Enumerates the finite structured output record cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum StructuredOutputRecord {
    /// Use this variant when the contract needs to represent lifecycle; selecting it has no side effect by itself.
    Lifecycle(StructuredOutputLifecycleRecord),
    /// Use this variant when the contract needs to represent validation; selecting it has no side effect by itself.
    Validation(ValidationRecord),
    /// Use this variant when the contract needs to represent repair; selecting it has no side effect by itself.
    Repair(RepairRecord),
    /// Use this variant when the contract needs to represent repair exhaustion; selecting it has no side effect by itself.
    RepairExhaustion(RepairExhaustionRecord),
    /// Use this variant when the contract needs to represent validation report; selecting it has no side effect by itself.
    ValidationReport(ValidationReportRecord),
    /// Use this variant when the contract needs to represent validated output; selecting it has no side effect by itself.
    ValidatedOutput(ValidatedOutput),
    /// Use this variant when the contract needs to represent typed result publication; selecting it has no side effect by itself.
    TypedResultPublication(TypedResultPublicationRecord),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the run lifecycle record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct RunLifecycleRecord {
    /// Finite status for this record or lifecycle stage.
    pub status: String,
    /// Redacted explanation for a denial, failure, status, or package delta.
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the context projection record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ContextProjectionRecord {
    /// Stable projection id used for typed lineage, lookup, or dedupe.
    pub projection_id: ContextProjectionId,
    /// Count of selected item items observed or included in this record.
    pub selected_item_count: u32,
    /// Provider destination used by this record or request.
    pub provider_destination: DestinationRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the model attempt record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ModelAttemptRecord {
    /// Stable provider route id used for typed lineage, lookup, or dedupe.
    pub provider_route_id: String,
    /// Stable provider model id used for typed lineage, lookup, or dedupe.
    pub provider_model_id: String,
    /// Count of request message items observed or included in this record.
    pub request_message_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional stop reason value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub stop_reason: Option<ProviderStopReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional usage value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub usage: Option<ProviderUsage>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the message record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct MessageRecord {
    /// Message identifier for transcript, projection, or provider-response
    /// lineage.
    pub message_id: MessageId,
    /// Role used by this record or request.
    pub role: String,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the agent pool record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct AgentPoolRecord {
    /// Stable pool id used for typed lineage, lookup, or dedupe.
    pub pool_id: AgentPoolId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Identifiers used to select or correlate member run values.
    /// Use them for typed lookup, filtering, or lineage instead of stringly typed matching.
    pub member_run_ids: Vec<RunId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of topics values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub topics: Vec<TopicId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Lifecycle status used by this record or request.
    pub lifecycle_status: AgentPoolLifecycleStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite agent pool lifecycle status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum AgentPoolLifecycleStatus {
    /// Use this variant when the contract needs to represent created; selecting it has no side effect by itself.
    Created,
    /// Use this variant when the contract needs to represent run joined; selecting it has no side effect by itself.
    RunJoined,
    /// Use this variant when the contract needs to represent run left; selecting it has no side effect by itself.
    RunLeft,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the run message record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct RunMessageRecord {
    /// Message identifier for transcript, projection, or provider-response
    /// lineage.
    pub message_id: MessageId,
    /// Stable source run id used for typed lineage, lookup, or dedupe.
    pub source_run_id: RunId,
    /// Address target used by this record or request.
    pub address_target: RunMessageAddressTargetRecord,
    /// Content reference where payload bytes or structured tool output are
    /// stored.
    pub content_ref: ContentRef,
    /// Correlation used by this record or request.
    pub correlation: EventCorrelation,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional reply to value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub reply_to: Option<MessageId>,
    /// Output delivery setting or policy.
    /// Delivery coordinators use it to decide sink mode, dedupe, and required evidence.
    pub delivery_status: RunMessageDeliveryStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of delivered to values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub delivered_to: Vec<RunId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency_key: IdempotencyKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional effect intent value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub effect_intent: Option<EffectIntent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional effect result value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub effect_result: Option<EffectResult>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Enumerates the finite run message address target record cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RunMessageAddressTargetRecord {
    /// Use this variant when the contract needs to represent run; selecting it has no side effect by itself.
    Run {
        /// Run identifier used for lineage, filtering, replay, and dedupe.
        run_id: RunId,
    },
    /// Use this variant when the contract needs to represent agent; selecting it has no side effect by itself.
    Agent {
        /// Agent identifier used for lineage, filtering, and ownership
        /// checks.
        agent_id: AgentId,
    },
    /// Use this variant when the contract needs to represent topic; selecting it has no side effect by itself.
    Topic {
        /// Stable topic id used for typed lineage, lookup, or dedupe.
        topic_id: TopicId,
    },
    /// Use this variant when the contract needs to represent pool; selecting it has no side effect by itself.
    Pool {
        /// Stable pool id used for typed lineage, lookup, or dedupe.
        pool_id: AgentPoolId,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite run message delivery status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RunMessageDeliveryStatus {
    /// Use this variant when the contract needs to represent accepted; selecting it has no side effect by itself.
    Accepted,
    /// Use this variant when the contract needs to represent delivered; selecting it has no side effect by itself.
    Delivered,
    /// Use this variant when the contract needs to represent responded; selecting it has no side effect by itself.
    Responded,
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed,
    /// Use this variant when the contract needs to represent timed out; selecting it has no side effect by itself.
    TimedOut,
    /// Use this variant when the contract needs to represent expired; selecting it has no side effect by itself.
    Expired,
    /// Use this variant when the contract needs to represent cancelled; selecting it has no side effect by itself.
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the wake record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct WakeRecord {
    /// Stable condition id used for typed lineage, lookup, or dedupe.
    pub condition_id: WakeConditionId,
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Deterministic event filter fingerprint used for stale checks, package
    /// evidence, or replay comparisons.
    pub event_filter_fingerprint: EventFilterFingerprint,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Time value in milliseconds for timeout millis.
    /// Use it for timeout, ordering, or diagnostic calculations.
    pub timeout_millis: Option<u64>,
    /// Resume policy used by this record or request.
    pub resume_policy: WakeResumeInputPolicyRecord,
    /// Trigger status used by this record or request.
    pub trigger_status: WakeTriggerStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency_key: IdempotencyKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable matched event id used for typed lineage, lookup, or dedupe.
    pub matched_event_id: Option<crate::domain::EventId>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite wake resume input policy record cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum WakeResumeInputPolicyRecord {
    /// Use this variant when the contract needs to represent matching event refs; selecting it has no side effect by itself.
    MatchingEventRefs,
    /// Use this variant when the contract needs to represent redacted summary; selecting it has no side effect by itself.
    RedactedSummary,
    /// Use this variant when the contract needs to represent none; selecting it has no side effect by itself.
    None,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite wake trigger status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum WakeTriggerStatus {
    /// Use this variant when the contract needs to represent registered; selecting it has no side effect by itself.
    Registered,
    /// Use this variant when the contract needs to represent triggered; selecting it has no side effect by itself.
    Triggered,
    /// Use this variant when the contract needs to represent timed out; selecting it has no side effect by itself.
    TimedOut,
    /// Use this variant when the contract needs to represent cancelled; selecting it has no side effect by itself.
    Cancelled,
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the run checkpoint record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct RunCheckpoint {
    /// Stable checkpoint id used for typed lineage, lookup, or dedupe.
    pub checkpoint_id: String,
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Checkpoint seq used by this record or request.
    pub checkpoint_seq: u64,
    /// Covers journal seq used by this record or request.
    pub covers_journal_seq: u64,
    /// Loop state used by this record or request.
    pub loop_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Turn identifier for one loop turn within a run.
    pub turn_id: Option<TurnId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Attempt identifier for retry, repair, provider, or tool execution
    /// evidence.
    pub attempt_id: Option<AttemptId>,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of pending side effects values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub pending_side_effects: Vec<PendingSideEffect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of pending approvals values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub pending_approvals: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Content reference associated with this value.
    /// Resolve it through policy-gated content stores instead of embedding raw content.
    pub content_ref_manifest: Vec<ContentRef>,
    /// Deterministic state hash used for stale checks, package evidence, or
    /// replay comparisons.
    pub state_hash: String,
    /// Time value in milliseconds for created at millis.
    /// Use it for timeout, ordering, or diagnostic calculations.
    pub created_at_millis: u64,
    /// Stable writer id used for typed lineage, lookup, or dedupe.
    pub writer_id: String,
}

impl RunCheckpoint {
    /// Validates the records::journal invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
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
/// Carries the pending side effect record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct PendingSideEffect {
    /// Stable effect id used for typed lineage, lookup, or dedupe.
    pub effect_id: crate::domain::EffectId,
    /// Stable intent record id used for typed lineage, lookup, or dedupe.
    pub intent_record_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency_key: Option<IdempotencyKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Dedupe policy or key for a side-effecting operation.
    /// Replay and repair use it to avoid sending or executing the same effect twice.
    pub dedupe_key: Option<DedupeKey>,
    /// Reason a pending side effect is unsafe to retry automatically.
    /// Recovery uses it to require repair or reconciliation before continuing.
    pub unsafe_pending_reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the terminal result marker record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct TerminalResultMarker {
    /// Stable effect id used for typed lineage, lookup, or dedupe.
    pub effect_id: crate::domain::EffectId,
    /// Stable result record id used for typed lineage, lookup, or dedupe.
    pub result_record_id: String,
    /// Terminal status used by this record or request.
    pub terminal_status: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the recovery marker record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct RecoveryMarker {
    /// Collection of unsafe pending values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub unsafe_pending: Vec<PendingSideEffect>,
    /// Recovery reason used by this record or request.
    pub recovery_reason: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
}
