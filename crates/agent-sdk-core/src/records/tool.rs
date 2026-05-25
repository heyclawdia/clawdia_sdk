//! Durable and observable SDK records. Use these DTOs for events, journals, effects,
//! context, output, and feature evidence. Constructing records is data-only;
//! persistence, publication, and external actions happen through ports or application
//! coordinators. This file contains the tool portion of that contract.
//!
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
/// Carries the canonical tool name record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct CanonicalToolName(String);

impl CanonicalToolName {
    /// Creates a new records::tool value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    ///
    /// # Panics
    ///
    /// Panics if constructor invariants fail, such as invalid identifier
    /// text or constructor-specific bounds. Use a fallible constructor such as
    /// `try_new` when one is available for untrusted input.
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("CanonicalToolName must be valid")
    }

    /// Creates a new records::tool value after validation. Returns an
    /// SDK error instead of panicking when the identifier or input does
    /// not satisfy the contract.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdValidationError> {
        let value = value.into();
        validate_tool_name(&value)?;
        Ok(Self(value))
    }

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
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
/// Carries the tool call record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ToolCallRecord {
    /// Stable tool call id used for typed lineage, lookup, or dedupe.
    pub tool_call_id: ToolCallId,
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Turn identifier for one loop turn within a run.
    pub turn_id: Option<TurnId>,
    /// Stable capability identifier used for package projection and
    /// executable routing.
    pub capability_id: CapabilityId,
    /// Canonical tool name used by this record or request.
    pub canonical_tool_name: CanonicalToolName,
    /// Namespace that groups this capability or identifier.
    /// Use it to avoid collisions between packages, hosts, and extensions.
    pub namespace: CapabilityNamespace,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: DestinationRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed executor ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub executor_ref: Option<ExecutorRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// References to typed package sidecars needed by this capability.
    pub sidecar_refs: Vec<PackageSidecarRef>,
    /// Classification value for effect class.
    /// Policy and projection paths use it for finite routing decisions.
    pub effect_class: EffectClass,
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
    /// Typed requested args refs references. Resolving them is separate from
    /// constructing this record.
    pub requested_args_refs: Vec<ContentRef>,
    /// Redacted summary for display, logs, events, or telemetry.
    /// It should describe the value without exposing raw private content.
    pub redacted_args_summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed result content refs references. Resolving them is separate from
    /// constructing this record.
    pub result_content_refs: Vec<ContentRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Redacted summary for display, logs, events, or telemetry.
    /// It should describe the value without exposing raw private content.
    pub redacted_result_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency_key: Option<IdempotencyKey>,
    /// Finite status for this record or lifecycle stage.
    pub status: ToolCallRecordStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional policy outcome value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub policy_outcome: Option<PolicyOutcome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional effect intent value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub effect_intent: Option<EffectIntent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional effect result value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub effect_result: Option<EffectResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Reason a pending side effect is unsafe to retry automatically.
    /// Recovery uses it to require repair or reconciliation before continuing.
    pub unsafe_pending_reason: Option<String>,
}

impl ToolCallRecord {
    /// Requested.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Returns an updated records::tool value with subject ref applied. This
    /// is data construction only and does not execute the configured
    /// behavior.
    pub fn subject_ref(&self) -> EntityRef {
        EntityRef::new(EntityKind::ToolCall, self.tool_call_id.clone())
    }

    /// Returns this value with its denial setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_denial(mut self, policy_outcome: PolicyOutcome) -> Self {
        self.status = ToolCallRecordStatus::DeniedBeforeExecution;
        self.policy_outcome = Some(policy_outcome);
        self
    }

    /// Returns this value with its intent setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_intent(mut self, intent: EffectIntent) -> Self {
        self.status = ToolCallRecordStatus::IntentRecorded;
        self.effect_intent = Some(intent);
        self
    }

    /// Returns this value with its result setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_result(mut self, result: EffectResult, policy_outcome: PolicyOutcome) -> Self {
        self.status = ToolCallRecordStatus::from_terminal_status(&result.terminal_status);
        self.result_content_refs = result.content_refs.clone();
        self.redacted_result_summary = Some(result.redacted_summary.clone());
        self.policy_outcome = Some(policy_outcome);
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
        self.status = ToolCallRecordStatus::RecoveryRequired;
        self.result_content_refs = result.content_refs.clone();
        self.redacted_result_summary = Some(result.redacted_summary.clone());
        self.effect_result = Some(result);
        self.unsafe_pending_reason = Some(unsafe_pending_reason.into());
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the tool call record params record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ToolCallRecordParams {
    /// Stable tool call id used for typed lineage, lookup, or dedupe.
    pub tool_call_id: ToolCallId,
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Turn identifier for one loop turn within a run.
    pub turn_id: Option<TurnId>,
    /// Stable capability identifier used for package projection and
    /// executable routing.
    pub capability_id: CapabilityId,
    /// Canonical tool name used by this record or request.
    pub canonical_tool_name: CanonicalToolName,
    /// Namespace that groups this capability or identifier.
    /// Use it to avoid collisions between packages, hosts, and extensions.
    pub namespace: CapabilityNamespace,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: DestinationRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed executor ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub executor_ref: Option<ExecutorRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// References to typed package sidecars needed by this capability.
    pub sidecar_refs: Vec<PackageSidecarRef>,
    /// Classification value for effect class.
    /// Policy and projection paths use it for finite routing decisions.
    pub effect_class: EffectClass,
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
    /// Typed requested args refs references. Resolving them is separate from
    /// constructing this record.
    pub requested_args_refs: Vec<ContentRef>,
    /// Redacted summary for display, logs, events, or telemetry.
    /// It should describe the value without exposing raw private content.
    pub redacted_args_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency_key: Option<IdempotencyKey>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite tool call record status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ToolCallRecordStatus {
    /// Use this variant when the contract needs to represent requested; selecting it has no side effect by itself.
    Requested,
    /// Use this variant when the contract needs to represent intent recorded; selecting it has no side effect by itself.
    IntentRecorded,
    /// Use this variant when the contract needs to represent completed; selecting it has no side effect by itself.
    Completed,
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed,
    /// Use this variant when the contract needs to represent timed out; selecting it has no side effect by itself.
    TimedOut,
    /// Use this variant when the contract needs to represent cancelled; selecting it has no side effect by itself.
    Cancelled,
    /// Use this variant when the contract needs to represent denied before execution; selecting it has no side effect by itself.
    DeniedBeforeExecution,
    /// Use this variant when the contract needs to represent unknown; selecting it has no side effect by itself.
    Unknown,
    /// Use this variant when the contract needs to represent recovery required; selecting it has no side effect by itself.
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
/// Carries the tool result ref record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ToolResultRef {
    /// Stable tool call id used for typed lineage, lookup, or dedupe.
    pub tool_call_id: ToolCallId,
    /// Stable effect id used for typed lineage, lookup, or dedupe.
    pub effect_id: EffectId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Content references associated with this record; resolving them is a
    /// separate policy-gated step.
    pub content_refs: Vec<ContentRef>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention: RetentionClass,
}

impl ToolResultRef {
    /// Constructs this value from record. Use it when adapting
    /// canonical SDK records without introducing a second behavior
    /// path.
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
            privacy: record.privacy,
            retention: record.retention,
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

/// Builds the tool call journal record record or result value.
/// This is data-only and does not perform I/O, call host ports, append journals, publish
/// events, or start processes.
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
            privacy_class: base.privacy,
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
