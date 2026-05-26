//! Durable and observable SDK records. Use these DTOs for events, journals, effects,
//! context, output, and feature evidence. Constructing records is data-only;
//! persistence, publication, and external actions happen through ports or application
//! coordinators. This file contains the output delivery portion of that contract.
//!
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    domain::{
        AgentId, AttemptId, ContentRef, DedupeKey, DestinationRef, EffectId, EntityKind, EntityRef,
        IdempotencyKey, MessageId, PolicyRef, PrivacyClass, RetentionClass, RunId, SessionId,
        SourceRef, TurnId, ValidatedOutputId,
    },
    effect::{EffectIntent, EffectKind, EffectResult, EffectTerminalStatus},
    error::RetryClassification,
    journal::{
        EventIndexProjection, JOURNAL_SCHEMA_VERSION, JournalRecord, JournalRecordKind,
        JournalRecordPayload,
    },
    package::RuntimePackageFingerprint,
};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Carries the output delivery id record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct OutputDeliveryId(String);

impl OutputDeliveryId {
    /// Creates a new records::output_delivery value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(!value.is_empty(), "OutputDeliveryId must not be empty");
        Self(value)
    }

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Carries the output sink ref record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct OutputSinkRef(String);

impl OutputSinkRef {
    /// Creates a new records::output_delivery value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(!value.is_empty(), "OutputSinkRef must not be empty");
        Self(value)
    }

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite output delivery requirement cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum OutputDeliveryRequirement {
    /// Use this variant when the contract needs to represent disabled; selecting it has no side effect by itself.
    Disabled,
    /// Use this variant when the contract needs to represent optional; selecting it has no side effect by itself.
    Optional,
    /// Use this variant when the contract needs to represent required; selecting it has no side effect by itself.
    Required,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Enumerates the finite output delivery kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum OutputDeliveryKind {
    /// Use this variant when the contract needs to represent stream chunk; selecting it has no side effect by itself.
    StreamChunk {
        /// Cursor identifying a replay, export, or subscription position.
        /// Use it to resume without widening the original scope.
        stream_cursor: String,
        /// Chunk index used by this record or request.
        chunk_index: u64,
    },
    /// Use this variant when the contract needs to represent final message; selecting it has no side effect by itself.
    FinalMessage,
    /// Use this variant when the contract needs to represent final validated output; selecting it has no side effect by itself.
    FinalValidatedOutput,
}

impl OutputDeliveryKind {
    /// Reports whether this value is chunk. The check is pure and does
    /// not mutate SDK or host state.
    pub fn is_chunk(&self) -> bool {
        matches!(self, Self::StreamChunk { .. })
    }

    /// Returns dedupe fragment derived from the supplied state.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub(crate) fn dedupe_fragment(&self) -> String {
        match self {
            Self::StreamChunk {
                stream_cursor,
                chunk_index,
            } => format!("stream:{stream_cursor}:{chunk_index}"),
            Self::FinalMessage => "final:message".to_string(),
            Self::FinalValidatedOutput => "final:validated_output".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite output content mode cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum OutputContentMode {
    /// Use this variant when the contract needs to represent content refs only; selecting it has no side effect by itself.
    ContentRefsOnly,
    /// Use this variant when the contract needs to represent redacted summary; selecting it has no side effect by itself.
    RedactedSummary,
    /// Use this variant when the contract needs to represent raw content if policy allows; selecting it has no side effect by itself.
    RawContentIfPolicyAllows,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the output delivery policy record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct OutputDeliveryPolicy {
    /// Policy reference that must be resolved by the host or runtime before
    /// execution.
    pub policy_ref: PolicyRef,
    /// Requirement used by this record or request.
    pub requirement: OutputDeliveryRequirement,
    /// Default content mode used by this record or request.
    pub default_content_mode: OutputContentMode,
    #[serde(default)]
    /// Allowlist for this policy or contract.
    /// Validation uses it to reject undeclared or policy-denied values.
    pub allowed_content_modes: Vec<OutputContentMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed required sink ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub required_sink_ref: Option<OutputSinkRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed retry policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub retry_policy_ref: Option<PolicyRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed reconciliation policy ref reference. Resolving or executing it
    /// is a separate policy-gated step.
    pub reconciliation_policy_ref: Option<PolicyRef>,
    /// Raw content or raw-content control for this value.
    /// Use it only when policy explicitly allows raw content capture or delivery.
    pub raw_content_policy: RawOutputContentPolicy,
}

impl OutputDeliveryPolicy {
    /// Returns an updated value with required configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn required(policy_ref: PolicyRef, sink_ref: OutputSinkRef) -> Self {
        Self {
            policy_ref,
            requirement: OutputDeliveryRequirement::Required,
            default_content_mode: OutputContentMode::ContentRefsOnly,
            allowed_content_modes: vec![
                OutputContentMode::ContentRefsOnly,
                OutputContentMode::RedactedSummary,
            ],
            required_sink_ref: Some(sink_ref),
            retry_policy_ref: None,
            reconciliation_policy_ref: None,
            raw_content_policy: RawOutputContentPolicy::deny(),
        }
    }

    /// Returns an updated value with optional configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn optional(policy_ref: PolicyRef) -> Self {
        Self {
            policy_ref,
            requirement: OutputDeliveryRequirement::Optional,
            default_content_mode: OutputContentMode::RedactedSummary,
            allowed_content_modes: vec![OutputContentMode::RedactedSummary],
            required_sink_ref: None,
            retry_policy_ref: None,
            reconciliation_policy_ref: None,
            raw_content_policy: RawOutputContentPolicy::deny(),
        }
    }

    /// Returns an updated value with disabled configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn disabled(policy_ref: PolicyRef) -> Self {
        Self {
            policy_ref,
            requirement: OutputDeliveryRequirement::Disabled,
            default_content_mode: OutputContentMode::ContentRefsOnly,
            allowed_content_modes: Vec::new(),
            required_sink_ref: None,
            retry_policy_ref: None,
            reconciliation_policy_ref: None,
            raw_content_policy: RawOutputContentPolicy::deny(),
        }
    }

    /// Returns whether allows mode applies for this contract.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn allows_mode(&self, mode: OutputContentMode) -> bool {
        self.allowed_content_modes.contains(&mode)
    }

    /// Returns policy refs for callers that need to inspect the contract state.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn policy_refs(&self) -> Vec<PolicyRef> {
        let mut refs = vec![self.policy_ref.clone()];
        if let Some(policy_ref) = &self.retry_policy_ref {
            refs.push(policy_ref.clone());
        }
        if let Some(policy_ref) = &self.reconciliation_policy_ref {
            refs.push(policy_ref.clone());
        }
        refs
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the raw output content policy record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct RawOutputContentPolicy {
    /// Policy reference that must be resolved by the host or runtime before
    /// execution.
    pub policy_ref: PolicyRef,
    /// Boolean policy/capability flag for whether allow raw content is
    /// enabled.
    pub allow_raw_content: bool,
    /// Retention class for referenced content or records.
    /// Stores and telemetry sinks use it to decide how long evidence may be kept.
    pub retention_named: bool,
    /// Whether redaction policy named is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub redaction_policy_named: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed allowed sink ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub allowed_sink_ref: Option<OutputSinkRef>,
    /// Byte size or byte limit for byte limit.
    /// Use it to enforce bounded reads, writes, summaries, or parser output.
    pub byte_limit: u64,
}

impl RawOutputContentPolicy {
    /// Returns an updated records::output_delivery value with deny applied.
    /// This is data construction only and does not execute the configured
    /// behavior.
    pub fn deny() -> Self {
        Self {
            policy_ref: PolicyRef::new("policy.output_delivery.raw.deny"),
            allow_raw_content: false,
            retention_named: true,
            redaction_policy_named: true,
            allowed_sink_ref: None,
            byte_limit: 0,
        }
    }

    /// Returns an updated value with allow for sink configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn allow_for_sink(policy_ref: PolicyRef, sink_ref: OutputSinkRef, byte_limit: u64) -> Self {
        Self {
            policy_ref,
            allow_raw_content: true,
            retention_named: true,
            redaction_policy_named: true,
            allowed_sink_ref: Some(sink_ref),
            byte_limit,
        }
    }

    /// Returns whether allows raw for applies for this contract.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn allows_raw_for(&self, sink_ref: &OutputSinkRef, byte_len: usize) -> bool {
        self.allow_raw_content
            && self.retention_named
            && self.redaction_policy_named
            && self
                .allowed_sink_ref
                .as_ref()
                .is_some_and(|allowed| allowed == sink_ref)
            && self.byte_limit >= byte_len as u64
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the output delivery request record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct OutputDeliveryRequest {
    /// Stable delivery id used for typed lineage, lookup, or dedupe.
    pub delivery_id: OutputDeliveryId,
    /// Stable effect id used for typed lineage, lookup, or dedupe.
    pub effect_id: EffectId,
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional host-provided session identifier for grouping related turns.
    pub session_id: Option<SessionId>,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Turn identifier for one loop turn within a run.
    pub turn_id: Option<TurnId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Attempt identifier for retry, repair, provider, or tool execution
    /// evidence.
    pub attempt_id: Option<AttemptId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable source message id used for typed lineage, lookup, or dedupe.
    pub source_message_id: Option<MessageId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable validated output id used for typed lineage, lookup, or dedupe.
    pub validated_output_id: Option<ValidatedOutputId>,
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: DestinationRef,
    /// Typed sink ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub sink_ref: OutputSinkRef,
    /// Output delivery setting or policy.
    /// Delivery coordinators use it to decide sink mode, dedupe, and required evidence.
    pub delivery_kind: OutputDeliveryKind,
    /// Content mode used by this record or request.
    pub content_mode: OutputContentMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Content references associated with this record; resolving them is a
    /// separate policy-gated step.
    pub content_refs: Vec<ContentRef>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Raw content or raw-content control for this value.
    /// Use it only when policy explicitly allows raw content capture or delivery.
    pub raw_content: Option<String>,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention: RetentionClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency_key: Option<IdempotencyKey>,
    /// Dedupe policy or key for a side-effecting operation.
    /// Replay and repair use it to avoid sending or executing the same effect twice.
    pub dedupe_key: DedupeKey,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: RuntimePackageFingerprint,
}

impl OutputDeliveryRequest {
    /// Returns effect intent derived from the supplied state.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn effect_intent(&self) -> EffectIntent {
        let mut intent = EffectIntent::new(
            self.effect_id.clone(),
            EffectKind::OutputDelivery,
            EntityRef::new(EntityKind::OutputDelivery, self.delivery_id.as_str()),
            SourceRef::with_kind(crate::domain::SourceKind::Sdk, "source.sdk.output_delivery"),
            self.redacted_summary.clone(),
        );
        intent.destination = Some(self.destination.clone());
        intent.policy_refs = self.policy_refs.clone();
        intent.idempotency_key = self.idempotency_key.clone();
        intent.dedupe_key = Some(self.dedupe_key.clone());
        intent.content_refs = self.content_refs.clone();
        intent
    }

    /// Returns whether carries raw content applies for this contract.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn carries_raw_content(&self) -> bool {
        self.raw_content.is_some()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the output delivery receipt record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct OutputDeliveryReceipt {
    /// Stable delivery id used for typed lineage, lookup, or dedupe.
    pub delivery_id: OutputDeliveryId,
    /// Finite status for this record or lifecycle stage.
    pub status: OutputDispatchStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed ack ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub ack_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub destination_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable external operation id used for typed lineage, lookup, or
    /// dedupe.
    pub external_operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed reconciliation ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub reconciliation_ref: Option<String>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

impl OutputDeliveryReceipt {
    /// Returns an updated value with completed configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn completed(delivery_id: OutputDeliveryId, ack_ref: impl Into<String>) -> Self {
        Self {
            delivery_id,
            status: OutputDispatchStatus::Completed,
            ack_ref: Some(ack_ref.into()),
            destination_cursor: None,
            external_operation_id: None,
            reconciliation_ref: None,
            redacted_summary: "output delivery completed".to_string(),
        }
    }

    /// Builds the unknown record or result value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn unknown(delivery_id: OutputDeliveryId, reconciliation_ref: impl Into<String>) -> Self {
        Self {
            delivery_id,
            status: OutputDispatchStatus::ReconciliationNeeded,
            ack_ref: None,
            destination_cursor: None,
            external_operation_id: None,
            reconciliation_ref: Some(reconciliation_ref.into()),
            redacted_summary: "output delivery outcome unknown".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite output dispatch status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum OutputDispatchStatus {
    /// Use this variant when the contract needs to represent requested; selecting it has no side effect by itself.
    Requested,
    /// Use this variant when the contract needs to represent completed; selecting it has no side effect by itself.
    Completed,
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed,
    /// Use this variant when the contract needs to represent deduped; selecting it has no side effect by itself.
    Deduped,
    /// Use this variant when the contract needs to represent host configuration needed; selecting it has no side effect by itself.
    HostConfigurationNeeded,
    /// Use this variant when the contract needs to represent policy denied; selecting it has no side effect by itself.
    PolicyDenied,
    /// Use this variant when the contract needs to represent skipped optional; selecting it has no side effect by itself.
    SkippedOptional,
    /// Use this variant when the contract needs to represent reconciliation needed; selecting it has no side effect by itself.
    ReconciliationNeeded,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite output delivery event kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum OutputDeliveryEventKind {
    /// Use this variant when the contract needs to represent output dispatch requested; selecting it has no side effect by itself.
    OutputDispatchRequested,
    /// Use this variant when the contract needs to represent output dispatch completed; selecting it has no side effect by itself.
    OutputDispatchCompleted,
    /// Use this variant when the contract needs to represent output dispatch failed; selecting it has no side effect by itself.
    OutputDispatchFailed,
    /// Use this variant when the contract needs to represent output dispatch deduped; selecting it has no side effect by itself.
    OutputDispatchDeduped,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the output delivery intent record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct OutputDeliveryIntentRecord {
    /// Stable delivery id used for typed lineage, lookup, or dedupe.
    pub delivery_id: OutputDeliveryId,
    /// Effect intent used by this record or request.
    pub effect_intent: EffectIntent,
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: DestinationRef,
    /// Typed sink ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub sink_ref: OutputSinkRef,
    /// Typed desired sink ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub desired_sink_ref: OutputSinkRef,
    /// Output delivery setting or policy.
    /// Delivery coordinators use it to decide sink mode, dedupe, and required evidence.
    pub delivery_kind: OutputDeliveryKind,
    /// Content mode used by this record or request.
    pub content_mode: OutputContentMode,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency_key: Option<IdempotencyKey>,
    /// Dedupe policy or key for a side-effecting operation.
    /// Replay and repair use it to avoid sending or executing the same effect twice.
    pub dedupe_key: DedupeKey,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: RuntimePackageFingerprint,
}

impl OutputDeliveryIntentRecord {
    /// Constructs this value from request. Use it when adapting
    /// canonical SDK records without introducing a second behavior
    /// path.
    pub fn from_request(request: &OutputDeliveryRequest) -> Self {
        Self {
            delivery_id: request.delivery_id.clone(),
            effect_intent: request.effect_intent(),
            destination: request.destination.clone(),
            sink_ref: request.sink_ref.clone(),
            desired_sink_ref: request.sink_ref.clone(),
            delivery_kind: request.delivery_kind.clone(),
            content_mode: request.content_mode,
            content_refs: request.content_refs.clone(),
            redacted_summary: request.redacted_summary.clone(),
            privacy: request.privacy,
            retention: request.retention,
            policy_refs: request.policy_refs.clone(),
            idempotency_key: request.idempotency_key.clone(),
            dedupe_key: request.dedupe_key.clone(),
            runtime_package_fingerprint: request.runtime_package_fingerprint.clone(),
        }
    }

    /// Converts this value into journal record data.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn to_journal_record(&self, base: OutputDeliveryJournalBase) -> JournalRecord {
        output_delivery_effect_record(
            base,
            JournalRecordKind::OutputDispatch,
            "output_dispatch_requested",
            Some(self.idempotency_key.clone()).flatten(),
            Some(self.dedupe_key.clone()),
            self.effect_intent.content_refs.clone(),
            JournalRecordPayload::OutputDelivery(OutputDeliveryRecord::Intent(self.clone())),
            EntityRef::new(EntityKind::OutputDelivery, self.delivery_id.as_str()),
            self.destination.clone(),
            self.policy_refs.clone(),
            self.privacy,
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the output delivery result record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct OutputDeliveryResultRecord {
    /// Stable delivery id used for typed lineage, lookup, or dedupe.
    pub delivery_id: OutputDeliveryId,
    /// Effect result used by this record or request.
    pub effect_result: EffectResult,
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: DestinationRef,
    /// Typed sink ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub sink_ref: OutputSinkRef,
    /// Dispatch status used by this record or request.
    pub dispatch_status: OutputDispatchStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed ack ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub ack_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable external operation id used for typed lineage, lookup, or
    /// dedupe.
    pub external_operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed error ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub error_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Content references associated with this record; resolving them is a
    /// separate policy-gated step.
    pub content_refs: Vec<ContentRef>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    /// Retry classification used by this record or request.
    pub retry_classification: RetryClassification,
}

impl OutputDeliveryResultRecord {
    /// Returns an updated value with completed configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn completed(request: &OutputDeliveryRequest, receipt: &OutputDeliveryReceipt) -> Self {
        Self::from_status(
            request,
            OutputDispatchStatus::Completed,
            EffectTerminalStatus::Completed,
            receipt.ack_ref.clone(),
            receipt.external_operation_id.clone(),
            None,
            None,
            RetryClassification::NotRetryable,
            receipt.redacted_summary.clone(),
        )
    }

    /// Returns an updated value with failed configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn failed(
        request: &OutputDeliveryRequest,
        status: OutputDispatchStatus,
        error_ref: impl Into<String>,
        retry_classification: RetryClassification,
    ) -> Self {
        Self::from_status(
            request,
            status,
            EffectTerminalStatus::Failed,
            None,
            None,
            None,
            Some(error_ref.into()),
            retry_classification,
            "output delivery failed before host send or at sink boundary",
        )
    }

    /// Builds the reconciliation needed record or result value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn reconciliation_needed(
        request: &OutputDeliveryRequest,
        receipt: &OutputDeliveryReceipt,
    ) -> Self {
        Self::from_status(
            request,
            OutputDispatchStatus::ReconciliationNeeded,
            EffectTerminalStatus::Unknown,
            receipt.ack_ref.clone(),
            receipt.external_operation_id.clone(),
            receipt.reconciliation_ref.clone(),
            None,
            RetryClassification::RepairNeeded,
            receipt.redacted_summary.clone(),
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "status projection mirrors output-delivery effect fields; grouping belongs with a dedicated result-builder API"
    )]
    fn from_status(
        request: &OutputDeliveryRequest,
        dispatch_status: OutputDispatchStatus,
        terminal_status: EffectTerminalStatus,
        ack_ref: Option<String>,
        external_operation_id: Option<String>,
        reconciliation_ref: Option<String>,
        error_ref: Option<String>,
        retry_classification: RetryClassification,
        redacted_summary: impl Into<String>,
    ) -> Self {
        let redacted_summary = redacted_summary.into();
        Self {
            delivery_id: request.delivery_id.clone(),
            effect_result: EffectResult {
                effect_id: request.effect_id.clone(),
                terminal_status,
                external_operation_id: external_operation_id.clone(),
                reconciliation_ref,
                error_ref: error_ref.clone(),
                content_refs: request.content_refs.clone(),
                redacted_summary: redacted_summary.clone(),
            },
            destination: request.destination.clone(),
            sink_ref: request.sink_ref.clone(),
            dispatch_status,
            ack_ref,
            external_operation_id,
            error_ref,
            content_refs: request.content_refs.clone(),
            redacted_summary,
            retry_classification,
        }
    }

    /// Converts this value into journal record data.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn to_journal_record(&self, base: OutputDeliveryJournalBase) -> JournalRecord {
        let event_kind = match self.dispatch_status {
            OutputDispatchStatus::Completed => "output_dispatch_completed",
            OutputDispatchStatus::HostConfigurationNeeded
            | OutputDispatchStatus::PolicyDenied
            | OutputDispatchStatus::Failed => "output_dispatch_failed",
            OutputDispatchStatus::ReconciliationNeeded => "output_dispatch_reconciliation_needed",
            OutputDispatchStatus::Deduped => "output_dispatch_deduped",
            OutputDispatchStatus::Requested | OutputDispatchStatus::SkippedOptional => {
                "output_dispatch_status"
            }
        };
        output_delivery_effect_record(
            base,
            JournalRecordKind::OutputDispatch,
            event_kind,
            None,
            None,
            self.effect_result.content_refs.clone(),
            JournalRecordPayload::OutputDelivery(OutputDeliveryRecord::Result(self.clone())),
            EntityRef::new(EntityKind::OutputDelivery, self.delivery_id.as_str()),
            self.destination.clone(),
            Vec::new(),
            PrivacyClass::ContentRefsOnly,
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the output delivery dedupe record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct OutputDeliveryDedupeRecord {
    /// Stable delivery id used for typed lineage, lookup, or dedupe.
    pub delivery_id: OutputDeliveryId,
    /// Dedupe policy or key for a side-effecting operation.
    /// Replay and repair use it to avoid sending or executing the same effect twice.
    pub dedupe_key: DedupeKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable prior delivery id used for typed lineage, lookup, or dedupe.
    pub prior_delivery_id: Option<OutputDeliveryId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// External sink operation id from a previous delivery attempt, when known.
    /// Reconciliation uses it to avoid duplicate sends and to connect repaired evidence to the
    /// prior external operation.
    pub prior_external_operation_id: Option<String>,
    /// Prior terminal status used by this record or request.
    pub prior_terminal_status: OutputDispatchStatus,
    /// Current status used by this record or request.
    pub current_status: OutputDispatchStatus,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
}

impl OutputDeliveryDedupeRecord {
    /// Converts this value into journal record data.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn to_journal_record(
        &self,
        base: OutputDeliveryJournalBase,
        destination: DestinationRef,
    ) -> JournalRecord {
        output_delivery_effect_record(
            base,
            JournalRecordKind::OutputDispatch,
            "output_dispatch_deduped",
            None,
            Some(self.dedupe_key.clone()),
            Vec::new(),
            JournalRecordPayload::OutputDelivery(OutputDeliveryRecord::Dedupe(self.clone())),
            EntityRef::new(EntityKind::OutputDelivery, self.delivery_id.as_str()),
            destination,
            self.policy_refs.clone(),
            PrivacyClass::ContentRefsOnly,
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the output delivery reconciliation record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct OutputDeliveryReconciliationRecord {
    /// Stable delivery id used for typed lineage, lookup, or dedupe.
    pub delivery_id: OutputDeliveryId,
    /// Stable intent record id used for typed lineage, lookup, or dedupe.
    pub intent_record_id: String,
    /// Kind discriminator for side effect kind.
    /// Use it to route finite match arms without parsing display text.
    pub side_effect_kind: EffectKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency_key: Option<IdempotencyKey>,
    /// Dedupe policy or key for a side-effecting operation.
    /// Replay and repair use it to avoid sending or executing the same effect twice.
    pub dedupe_key: DedupeKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable external operation id used for typed lineage, lookup, or
    /// dedupe.
    pub external_operation_id: Option<String>,
    /// Terminal status used by this record or request.
    pub terminal_status: OutputDispatchStatus,
    /// Terminal append status used by this record or request.
    pub terminal_append_status: TerminalAppendStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional reconciliation adapter value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub reconciliation_adapter: Option<OutputSinkRef>,
    /// Reason a pending side effect is unsafe to retry automatically.
    /// Recovery uses it to require repair or reconciliation before continuing.
    pub unsafe_pending_reason: String,
    /// Replay decision used by this record or request.
    pub replay_decision: ReplayRepairDecision,
    /// Allowlist for this policy or contract.
    /// Validation uses it to reject undeclared or policy-denied values.
    pub resend_allowed: bool,
}

impl OutputDeliveryReconciliationRecord {
    /// Converts this value into journal record data.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn to_journal_record(
        &self,
        base: OutputDeliveryJournalBase,
        destination: DestinationRef,
    ) -> JournalRecord {
        output_delivery_effect_record(
            base,
            JournalRecordKind::OutputDispatch,
            "output_dispatch_reconciliation_needed",
            self.idempotency_key.clone(),
            Some(self.dedupe_key.clone()),
            Vec::new(),
            JournalRecordPayload::OutputDelivery(OutputDeliveryRecord::Reconciliation(
                self.clone(),
            )),
            EntityRef::new(EntityKind::OutputDelivery, self.delivery_id.as_str()),
            destination,
            Vec::new(),
            PrivacyClass::ContentRefsOnly,
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite terminal append status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum TerminalAppendStatus {
    /// Use this variant when the contract needs to represent not attempted; selecting it has no side effect by itself.
    NotAttempted,
    /// Use this variant when the contract needs to represent appended; selecting it has no side effect by itself.
    Appended,
    /// Use this variant when the contract needs to represent append failed; selecting it has no side effect by itself.
    AppendFailed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite replay repair decision cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ReplayRepairDecision {
    /// Use this variant when the contract needs to represent completed by dedupe proof; selecting it has no side effect by itself.
    CompletedByDedupeProof,
    /// Use this variant when the contract needs to represent requires host reconciliation; selecting it has no side effect by itself.
    RequiresHostReconciliation,
    /// Use this variant when the contract needs to represent unsafe pending; selecting it has no side effect by itself.
    UnsafePending,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the output delivery event record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct OutputDeliveryEventRecord {
    /// Kind discriminator for event kind.
    /// Use it to route finite match arms without parsing display text.
    pub event_kind: OutputDeliveryEventKind,
    /// Stable delivery id used for typed lineage, lookup, or dedupe.
    pub delivery_id: OutputDeliveryId,
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: DestinationRef,
    /// Typed sink ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub sink_ref: OutputSinkRef,
    /// Dedupe policy or key for a side-effecting operation.
    /// Replay and repair use it to avoid sending or executing the same effect twice.
    pub dedupe_key: DedupeKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable source message id used for typed lineage, lookup, or dedupe.
    pub source_message_id: Option<MessageId>,
    /// Dispatch status used by this record or request.
    pub dispatch_status: OutputDispatchStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed ack ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub ack_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional reconciliation status value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub reconciliation_status: Option<ReplayRepairDecision>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "record_type", content = "record", rename_all = "snake_case")]
/// Enumerates the finite output delivery record cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
#[expect(
    clippy::large_enum_variant,
    reason = "output-delivery records are durable serde payloads; direct variants stay explicit until a fixture-reviewed envelope migration"
)]
pub enum OutputDeliveryRecord {
    /// Use this variant when the contract needs to represent intent; selecting it has no side effect by itself.
    Intent(OutputDeliveryIntentRecord),
    /// Use this variant when the contract needs to represent result; selecting it has no side effect by itself.
    Result(OutputDeliveryResultRecord),
    /// Use this variant when the contract needs to represent dedupe; selecting it has no side effect by itself.
    Dedupe(OutputDeliveryDedupeRecord),
    /// Use this variant when the contract needs to represent reconciliation; selecting it has no side effect by itself.
    Reconciliation(OutputDeliveryReconciliationRecord),
    /// Use this variant when the contract needs to represent event; selecting it has no side effect by itself.
    Event(OutputDeliveryEventRecord),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the output delivery journal base record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct OutputDeliveryJournalBase {
    /// Journal seq used by this record or request.
    pub journal_seq: u64,
    /// Stable record id used for typed lineage, lookup, or dedupe.
    pub record_id: String,
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional host-provided session identifier for grouping related turns.
    pub session_id: Option<SessionId>,
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
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: DestinationRef,
    /// Timestamp in milliseconds associated with this record.
    /// Use it for ordering and diagnostics; durable causality still comes from ids and cursors.
    pub timestamp_millis: u64,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: RuntimePackageFingerprint,
    /// Stable redaction policy id used for typed lineage, lookup, or dedupe.
    pub redaction_policy_id: String,
}

/// Builds the build output delivery dedupe key value.
/// This is data construction and performs no I/O, journal append, event publication, or process
pub fn build_output_delivery_dedupe_key(request: &OutputDeliveryRequest) -> DedupeKey {
    let content_refs = request
        .content_refs
        .iter()
        .map(|content_ref| content_ref.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let policy_refs = request
        .policy_refs
        .iter()
        .map(|policy_ref| {
            format!(
                "{}:{}",
                policy_ref.as_str(),
                policy_ref.version.as_deref().unwrap_or("unversioned")
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let preimage = format!(
        "run={}|destination_kind={:?}|destination={}|sink={}|kind={}|message={}|validated={}|content={}|policies={}|package={}",
        request.run_id.as_str(),
        request.destination.kind,
        request.destination.as_str(),
        request.sink_ref.as_str(),
        request.delivery_kind.dedupe_fragment(),
        request
            .source_message_id
            .as_ref()
            .map(|id| id.as_str())
            .unwrap_or("none"),
        request
            .validated_output_id
            .as_ref()
            .map(|id| id.as_str())
            .unwrap_or("none"),
        content_refs,
        policy_refs,
        request.runtime_package_fingerprint.as_str(),
    );
    let digest = Sha256::digest(preimage.as_bytes());
    DedupeKey::new(format!("dedupe.output_delivery.{digest:x}"))
}

#[expect(
    clippy::too_many_arguments,
    reason = "private output-delivery journal constructor mirrors effect and event lineage fields for auditability"
)]
fn output_delivery_effect_record(
    base: OutputDeliveryJournalBase,
    record_kind: JournalRecordKind,
    event_kind: &str,
    idempotency_key: Option<IdempotencyKey>,
    dedupe_key: Option<DedupeKey>,
    content_refs: Vec<ContentRef>,
    payload: JournalRecordPayload,
    subject_ref: EntityRef,
    destination: DestinationRef,
    _policy_refs: Vec<PolicyRef>,
    privacy: PrivacyClass,
) -> JournalRecord {
    let related_refs = content_refs
        .iter()
        .map(|content_ref| EntityRef::new(EntityKind::Content, content_ref.as_str()))
        .collect::<Vec<_>>();
    JournalRecord {
        journal_schema_version: JOURNAL_SCHEMA_VERSION,
        journal_seq: base.journal_seq,
        record_id: base.record_id,
        record_kind,
        run_id: base.run_id.clone(),
        session_id: base.session_id.clone(),
        agent_id: base.agent_id.clone(),
        turn_id: base.turn_id.clone(),
        attempt_id: base.attempt_id.clone(),
        subject_ref: subject_ref.clone(),
        related_refs: related_refs.clone(),
        causal_refs: Vec::new(),
        source: base.source.clone(),
        destination: Some(destination.clone()),
        correlation_keys: Vec::new(),
        tags: vec!["output_delivery".to_string()],
        delivery_semantics: "journal_backed".to_string(),
        event_index: EventIndexProjection {
            run_id: base.run_id,
            session_id: base.session_id,
            agent_id: base.agent_id,
            turn_id: base.turn_id,
            event_family: "output_delivery".to_string(),
            event_kind: event_kind.to_string(),
            source: base.source,
            destination: Some(destination),
            subject_ref,
            related_refs,
            correlation_keys: Vec::new(),
            tags: vec!["output_delivery".to_string()],
            privacy_class: privacy,
            delivery_semantics: "journal_backed".to_string(),
        },
        timestamp_millis: base.timestamp_millis,
        runtime_package_fingerprint: base.runtime_package_fingerprint.as_str().to_string(),
        privacy,
        content_refs,
        redaction_policy_id: base.redaction_policy_id,
        idempotency_key,
        dedupe_key,
        checkpoint_ref: None,
        payload,
    }
}
