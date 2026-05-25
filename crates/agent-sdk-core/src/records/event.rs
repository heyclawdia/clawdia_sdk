//! Canonical event records for live observation and telemetry projection. Use these
//! records to describe what happened without requiring raw content capture.
//! Constructors are data-only; delivery belongs to event buses and sinks.
//!
use std::num::NonZeroUsize;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    domain::{
        AgentError, AgentId, ArchiveCursorId, ContextItemId, CorrelationEntry, DestinationKind,
        DestinationRef, EntityKind, EntityRef, EventId, JournalCursor, MessageId, PolicyRef,
        PrivacyClass, RunId, SourceKind, SourceRef, SpanId, TraceId, TurnId,
    },
    ids::AttemptId,
};

macro_rules! typed_string {
    ($name:ident, $debug:literal) => {
        #[doc = concat!(
                            "Typed event-string wrapper for `",
                            stringify!($name),
                            "`. Use it for stable event filter, cursor, or fingerprint fields; ",
                            "constructing it is data-only and performs no side effects."
                        )]
        #[derive(Clone, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Creates a new records::event value with explicit
            /// caller-provided inputs. This constructor is data-only
            /// and performs no I/O or external side effects.
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Returns this value as str. The accessor is side-effect
            /// free and keeps ownership with the caller.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str(concat!($debug, "(redacted)"))
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str(concat!($debug, "(redacted)"))
            }
        }
    };
}

/// Constant value for the records::event contract. Use it to keep SDK
/// records and tests aligned on the same stable value.
pub const EVENT_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite event family cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum EventFamily {
    /// Use this variant when the contract needs to represent run; selecting it has no side effect by itself.
    Run,
    /// Use this variant when the contract needs to represent turn; selecting it has no side effect by itself.
    Turn,
    /// Use this variant when the contract needs to represent message; selecting it has no side effect by itself.
    Message,
    /// Use this variant when the contract needs to represent model; selecting it has no side effect by itself.
    Model,
    /// Use this variant when the contract needs to represent tool; selecting it has no side effect by itself.
    Tool,
    /// Use this variant when the contract needs to represent approval; selecting it has no side effect by itself.
    Approval,
    /// Use this variant when the contract needs to represent hook; selecting it has no side effect by itself.
    Hook,
    /// Use this variant when the contract needs to represent context; selecting it has no side effect by itself.
    Context,
    /// Use this variant when the contract needs to represent stream rule; selecting it has no side effect by itself.
    StreamRule,
    /// Use this variant when the contract needs to represent realtime; selecting it has no side effect by itself.
    Realtime,
    /// Use this variant when the contract needs to represent isolation; selecting it has no side effect by itself.
    Isolation,
    /// Use this variant when the contract needs to represent child lifecycle; selecting it has no side effect by itself.
    ChildLifecycle,
    /// Use this variant when the contract needs to represent agent pool; selecting it has no side effect by itself.
    AgentPool,
    /// Use this variant when the contract needs to represent subagent; selecting it has no side effect by itself.
    Subagent,
    /// Use this variant when the contract needs to represent extension; selecting it has no side effect by itself.
    Extension,
    /// Use this variant when the contract needs to represent structured output; selecting it has no side effect by itself.
    StructuredOutput,
    /// Use this variant when the contract needs to represent output; selecting it has no side effect by itself.
    Output,
    /// Use this variant when the contract needs to represent output delivery; selecting it has no side effect by itself.
    OutputDelivery,
    /// Use this variant when the contract needs to represent telemetry; selecting it has no side effect by itself.
    Telemetry,
    /// Use this variant when the contract needs to represent recovery; selecting it has no side effect by itself.
    Recovery,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite event kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum EventKind {
    /// Use this variant when the contract needs to represent run started; selecting it has no side effect by itself.
    RunStarted,
    /// Use this variant when the contract needs to represent run completed; selecting it has no side effect by itself.
    RunCompleted,
    /// Use this variant when the contract needs to represent run failed; selecting it has no side effect by itself.
    RunFailed,
    /// Use this variant when the contract needs to represent run cancelled; selecting it has no side effect by itself.
    RunCancelled,
    /// Use this variant when the contract needs to represent run checkpointed; selecting it has no side effect by itself.
    RunCheckpointed,
    /// Use this variant when the contract needs to represent run cancel requested; selecting it has no side effect by itself.
    RunCancelRequested,
    /// Use this variant when the contract needs to represent run resume requested; selecting it has no side effect by itself.
    RunResumeRequested,
    /// Use this variant when the contract needs to represent run resume failed; selecting it has no side effect by itself.
    RunResumeFailed,
    /// Use this variant when the contract needs to represent turn started; selecting it has no side effect by itself.
    TurnStarted,
    /// Use this variant when the contract needs to represent turn completed; selecting it has no side effect by itself.
    TurnCompleted,
    /// Use this variant when the contract needs to represent turn failed; selecting it has no side effect by itself.
    TurnFailed,
    /// Use this variant when the contract needs to represent message accepted; selecting it has no side effect by itself.
    MessageAccepted,
    /// Use this variant when the contract needs to represent message committed; selecting it has no side effect by itself.
    MessageCommitted,
    /// Use this variant when the contract needs to represent provider request projected; selecting it has no side effect by itself.
    ProviderRequestProjected,
    /// Use this variant when the contract needs to represent model attempt started; selecting it has no side effect by itself.
    ModelAttemptStarted,
    /// Use this variant when the contract needs to represent model stream delta; selecting it has no side effect by itself.
    ModelStreamDelta,
    /// Use this variant when the contract needs to represent model message completed; selecting it has no side effect by itself.
    ModelMessageCompleted,
    /// Use this variant when the contract needs to represent model attempt failed; selecting it has no side effect by itself.
    ModelAttemptFailed,
    /// Use this variant when the contract needs to represent tool requested; selecting it has no side effect by itself.
    ToolRequested,
    /// Use this variant when the contract needs to represent tool started; selecting it has no side effect by itself.
    ToolStarted,
    /// Use this variant when the contract needs to represent tool completed; selecting it has no side effect by itself.
    ToolCompleted,
    /// Use this variant when the contract needs to represent tool failed; selecting it has no side effect by itself.
    ToolFailed,
    /// Use this variant when the contract needs to represent tool denied; selecting it has no side effect by itself.
    ToolDenied,
    /// Use this variant when the contract needs to represent tool recovery required; selecting it has no side effect by itself.
    ToolRecoveryRequired,
    /// Use this variant when the contract needs to represent approval requested; selecting it has no side effect by itself.
    ApprovalRequested,
    /// Use this variant when the contract needs to represent approval dispatched; selecting it has no side effect by itself.
    ApprovalDispatched,
    /// Use this variant when the contract needs to represent approval dispatch unavailable; selecting it has no side effect by itself.
    ApprovalDispatchUnavailable,
    /// Use this variant when the contract needs to represent approval responded; selecting it has no side effect by itself.
    ApprovalResponded,
    /// Use this variant when the contract needs to represent approval denied; selecting it has no side effect by itself.
    ApprovalDenied,
    /// Use this variant when the contract needs to represent approval timed out; selecting it has no side effect by itself.
    ApprovalTimedOut,
    /// Use this variant when the contract needs to represent approval cancelled; selecting it has no side effect by itself.
    ApprovalCancelled,
    /// Use this variant when the contract needs to represent hook registered; selecting it has no side effect by itself.
    HookRegistered,
    /// Use this variant when the contract needs to represent hook invoked; selecting it has no side effect by itself.
    HookInvoked,
    /// Use this variant when the contract needs to represent hook completed; selecting it has no side effect by itself.
    HookCompleted,
    /// Use this variant when the contract needs to represent hook failed; selecting it has no side effect by itself.
    HookFailed,
    /// Use this variant when the contract needs to represent hook timed out; selecting it has no side effect by itself.
    HookTimedOut,
    /// Use this variant when the contract needs to represent hook cancelled; selecting it has no side effect by itself.
    HookCancelled,
    /// Use this variant when the contract needs to represent hook response applied; selecting it has no side effect by itself.
    HookResponseApplied,
    /// Use this variant when the contract needs to represent hook response rejected; selecting it has no side effect by itself.
    HookResponseRejected,
    /// Use this variant when the contract needs to represent context assembled; selecting it has no side effect by itself.
    ContextAssembled,
    /// Use this variant when the contract needs to represent stream rule registered; selecting it has no side effect by itself.
    StreamRuleRegistered,
    /// Use this variant when the contract needs to represent stream rule compile failed; selecting it has no side effect by itself.
    StreamRuleCompileFailed,
    /// Use this variant when the contract needs to represent stream rule matched; selecting it has no side effect by itself.
    StreamRuleMatched,
    /// Use this variant when the contract needs to represent stream intervention requested; selecting it has no side effect by itself.
    StreamInterventionRequested,
    /// Use this variant when the contract needs to represent stream intervention applied; selecting it has no side effect by itself.
    StreamInterventionApplied,
    /// Use this variant when the contract needs to represent stream rule repeat state recorded; selecting it has no side effect by itself.
    StreamRuleRepeatStateRecorded,
    /// Use this variant when the contract needs to represent realtime connect requested; selecting it has no side effect by itself.
    RealtimeConnectRequested,
    /// Use this variant when the contract needs to represent realtime connected; selecting it has no side effect by itself.
    RealtimeConnected,
    /// Use this variant when the contract needs to represent realtime input send requested; selecting it has no side effect by itself.
    RealtimeInputSendRequested,
    /// Use this variant when the contract needs to represent realtime input sent; selecting it has no side effect by itself.
    RealtimeInputSent,
    /// Use this variant when the contract needs to represent realtime output receive requested; selecting it has no side effect by itself.
    RealtimeOutputReceiveRequested,
    /// Use this variant when the contract needs to represent realtime output received; selecting it has no side effect by itself.
    RealtimeOutputReceived,
    /// Use this variant when the contract needs to represent realtime interrupted; selecting it has no side effect by itself.
    RealtimeInterrupted,
    /// Use this variant when the contract needs to represent realtime restart requested; selecting it has no side effect by itself.
    RealtimeRestartRequested,
    /// Use this variant when the contract needs to represent realtime restart started; selecting it has no side effect by itself.
    RealtimeRestartStarted,
    /// Use this variant when the contract needs to represent realtime restart completed; selecting it has no side effect by itself.
    RealtimeRestartCompleted,
    /// Use this variant when the contract needs to represent realtime restart failed; selecting it has no side effect by itself.
    RealtimeRestartFailed,
    /// Use this variant when the contract needs to represent realtime close requested; selecting it has no side effect by itself.
    RealtimeCloseRequested,
    /// Use this variant when the contract needs to represent realtime closed; selecting it has no side effect by itself.
    RealtimeClosed,
    /// Use this variant when the contract needs to represent realtime backpressure applied; selecting it has no side effect by itself.
    RealtimeBackpressureApplied,
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
    /// Use this variant when the contract needs to represent isolation environment prepared; selecting it has no side effect by itself.
    IsolationEnvironmentPrepared,
    /// Use this variant when the contract needs to represent isolation process started; selecting it has no side effect by itself.
    IsolationProcessStarted,
    /// Use this variant when the contract needs to represent isolation process io captured; selecting it has no side effect by itself.
    IsolationProcessIoCaptured,
    /// Use this variant when the contract needs to represent isolation process stats recorded; selecting it has no side effect by itself.
    IsolationProcessStatsRecorded,
    /// Use this variant when the contract needs to represent isolation cleanup started; selecting it has no side effect by itself.
    IsolationCleanupStarted,
    /// Use this variant when the contract needs to represent isolation cleanup completed; selecting it has no side effect by itself.
    IsolationCleanupCompleted,
    /// Use this variant when the contract needs to represent isolation cleanup failed; selecting it has no side effect by itself.
    IsolationCleanupFailed,
    /// Use this variant when the contract needs to represent isolation failed; selecting it has no side effect by itself.
    IsolationFailed,
    /// Use this variant when the contract needs to represent child lifecycle requested; selecting it has no side effect by itself.
    ChildLifecycleRequested,
    /// Use this variant when the contract needs to represent child lifecycle completed; selecting it has no side effect by itself.
    ChildLifecycleCompleted,
    /// Use this variant when the contract needs to represent child lifecycle detached; selecting it has no side effect by itself.
    ChildLifecycleDetached,
    /// Use this variant when the contract needs to represent subagent started; selecting it has no side effect by itself.
    SubagentStarted,
    /// Use this variant when the contract needs to represent subagent handoff; selecting it has no side effect by itself.
    SubagentHandoff,
    /// Use this variant when the contract needs to represent subagent event wrapped; selecting it has no side effect by itself.
    SubagentEventWrapped,
    /// Use this variant when the contract needs to represent subagent usage rolled up; selecting it has no side effect by itself.
    SubagentUsageRolledUp,
    /// Use this variant when the contract needs to represent subagent completed; selecting it has no side effect by itself.
    SubagentCompleted,
    /// Use this variant when the contract needs to represent extension action submitted; selecting it has no side effect by itself.
    ExtensionActionSubmitted,
    /// Use this variant when the contract needs to represent extension action started; selecting it has no side effect by itself.
    ExtensionActionStarted,
    /// Use this variant when the contract needs to represent extension action completed; selecting it has no side effect by itself.
    ExtensionActionCompleted,
    /// Use this variant when the contract needs to represent extension action failed; selecting it has no side effect by itself.
    ExtensionActionFailed,
    /// Use this variant when the contract needs to represent extension action denied; selecting it has no side effect by itself.
    ExtensionActionDenied,
    /// Use this variant when the contract needs to represent output dispatch requested; selecting it has no side effect by itself.
    OutputDispatchRequested,
    /// Use this variant when the contract needs to represent output dispatch completed; selecting it has no side effect by itself.
    OutputDispatchCompleted,
    /// Use this variant when the contract needs to represent output dispatch failed; selecting it has no side effect by itself.
    OutputDispatchFailed,
    /// Use this variant when the contract needs to represent output dispatch deduped; selecting it has no side effect by itself.
    OutputDispatchDeduped,
    /// Use this variant when the contract needs to represent structured output requested; selecting it has no side effect by itself.
    StructuredOutputRequested,
    /// Use this variant when the contract needs to represent structured output validation started; selecting it has no side effect by itself.
    StructuredOutputValidationStarted,
    /// Use this variant when the contract needs to represent structured output validation failed; selecting it has no side effect by itself.
    StructuredOutputValidationFailed,
    /// Use this variant when the contract needs to represent structured output repair requested; selecting it has no side effect by itself.
    StructuredOutputRepairRequested,
    /// Use this variant when the contract needs to represent structured output validated; selecting it has no side effect by itself.
    StructuredOutputValidated,
    /// Use this variant when the contract needs to represent structured output failed; selecting it has no side effect by itself.
    StructuredOutputFailed,
    /// Use this variant when the contract needs to represent telemetry sink failed; selecting it has no side effect by itself.
    TelemetrySinkFailed,
    /// Use this variant when the contract needs to represent telemetry sink recovered; selecting it has no side effect by itself.
    TelemetrySinkRecovered,
    /// Use this variant when the contract needs to represent usage recorded; selecting it has no side effect by itself.
    UsageRecorded,
    /// Use this variant when the contract needs to represent cost estimated; selecting it has no side effect by itself.
    CostEstimated,
    /// Use this variant when the contract needs to represent cost corrected; selecting it has no side effect by itself.
    CostCorrected,
    /// Use this variant when the contract needs to represent replay started; selecting it has no side effect by itself.
    ReplayStarted,
    /// Use this variant when the contract needs to represent replay completed; selecting it has no side effect by itself.
    ReplayCompleted,
    /// Use this variant when the contract needs to represent replay failed; selecting it has no side effect by itself.
    ReplayFailed,
    /// Use this variant when the contract needs to represent agent pool created; selecting it has no side effect by itself.
    AgentPoolCreated,
    /// Use this variant when the contract needs to represent agent pool run joined; selecting it has no side effect by itself.
    AgentPoolRunJoined,
    /// Use this variant when the contract needs to represent agent pool run left; selecting it has no side effect by itself.
    AgentPoolRunLeft,
    /// Use this variant when the contract needs to represent run message accepted; selecting it has no side effect by itself.
    RunMessageAccepted,
    /// Use this variant when the contract needs to represent run message delivered; selecting it has no side effect by itself.
    RunMessageDelivered,
    /// Use this variant when the contract needs to represent run message responded; selecting it has no side effect by itself.
    RunMessageResponded,
    /// Use this variant when the contract needs to represent run message failed; selecting it has no side effect by itself.
    RunMessageFailed,
    /// Use this variant when the contract needs to represent run message timed out; selecting it has no side effect by itself.
    RunMessageTimedOut,
    /// Use this variant when the contract needs to represent run message expired; selecting it has no side effect by itself.
    RunMessageExpired,
    /// Use this variant when the contract needs to represent run message cancelled; selecting it has no side effect by itself.
    RunMessageCancelled,
    /// Use this variant when the contract needs to represent wake condition registered; selecting it has no side effect by itself.
    WakeConditionRegistered,
    /// Use this variant when the contract needs to represent wake condition triggered; selecting it has no side effect by itself.
    WakeConditionTriggered,
    /// Use this variant when the contract needs to represent wake condition timed out; selecting it has no side effect by itself.
    WakeConditionTimedOut,
    /// Use this variant when the contract needs to represent wake condition cancelled; selecting it has no side effect by itself.
    WakeConditionCancelled,
    /// Use this variant when the contract needs to represent wake condition failed; selecting it has no side effect by itself.
    WakeConditionFailed,
}

impl EventKind {
    /// Reports whether this value is terminal. The check is pure and
    /// does not mutate SDK or host state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::RunCompleted
                | Self::RunFailed
                | Self::RunCancelled
                | Self::TurnCompleted
                | Self::TurnFailed
                | Self::ModelMessageCompleted
                | Self::ModelAttemptFailed
                | Self::ToolCompleted
                | Self::ToolFailed
                | Self::ToolDenied
                | Self::ToolRecoveryRequired
                | Self::ApprovalResponded
                | Self::OutputDispatchCompleted
                | Self::OutputDispatchFailed
                | Self::OutputDispatchDeduped
                | Self::ApprovalDenied
                | Self::ApprovalTimedOut
                | Self::ApprovalCancelled
                | Self::ApprovalDispatchUnavailable
                | Self::HookCompleted
                | Self::HookFailed
                | Self::HookTimedOut
                | Self::HookCancelled
                | Self::HookResponseApplied
                | Self::HookResponseRejected
                | Self::RealtimeRestartFailed
                | Self::RealtimeClosed
                | Self::IsolationCleanupCompleted
                | Self::IsolationCleanupFailed
                | Self::IsolationFailed
                | Self::ChildLifecycleCompleted
                | Self::ChildLifecycleDetached
                | Self::SubagentCompleted
                | Self::ExtensionActionCompleted
                | Self::ExtensionActionFailed
                | Self::ExtensionActionDenied
                | Self::StructuredOutputValidated
                | Self::StructuredOutputFailed
                | Self::ReplayCompleted
                | Self::ReplayFailed
                | Self::RunMessageResponded
                | Self::RunMessageFailed
                | Self::RunMessageTimedOut
                | Self::RunMessageExpired
                | Self::RunMessageCancelled
                | Self::WakeConditionTriggered
                | Self::WakeConditionTimedOut
                | Self::WakeConditionCancelled
                | Self::WakeConditionFailed
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the agent event record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct AgentEvent {
    /// Envelope used by this record or request.
    pub envelope: EventEnvelope,
    /// Payload carried by this record.
    /// Use the surrounding policy and redaction fields to decide whether it can be exposed.
    pub payload: EventPayload,
}

impl AgentEvent {
    /// Builds the envelope only value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn envelope_only(envelope: EventEnvelope) -> Self {
        Self {
            envelope,
            payload: EventPayload::EnvelopeOnly,
        }
    }

    /// Returns this value with its redacted summary setting replaced.
    /// The method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_redacted_summary(
        envelope: EventEnvelope,
        redacted_summary: impl Into<String>,
    ) -> Self {
        Self {
            envelope,
            payload: EventPayload::RedactedSummary {
                redacted_summary: redacted_summary.into(),
                payload_refs: Vec::new(),
            },
        }
    }

    /// Reads the stored redacted summary without registry or runtime work.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn redacted_summary(&self) -> Option<&str> {
        match &self.payload {
            EventPayload::EnvelopeOnly => None,
            EventPayload::RedactedSummary {
                redacted_summary, ..
            } => Some(redacted_summary.as_str()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the event envelope record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct EventEnvelope {
    /// Wire schema version used for compatibility checks.
    pub schema_version: u16,
    /// Event identifier used to correlate live events with journal or replay
    /// evidence.
    pub event_id: EventId,
    /// Event seq used by this record or request.
    pub event_seq: u64,
    /// Event family used by this record or request.
    pub event_family: EventFamily,
    /// Kind discriminator for event kind.
    /// Use it to route finite match arms without parsing display text.
    pub event_kind: EventKind,
    /// Wire schema version for this record shape.
    /// Use it for compatibility checks before deserializing or replaying stored data.
    pub payload_schema_version: u16,
    /// Timestamp in milliseconds associated with this record.
    /// Use it for ordering and diagnostics; durable causality still comes from ids and cursors.
    pub timestamp: String,
    /// Recorded at used by this record or request.
    pub recorded_at: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Message identifier for transcript, projection, or provider-response
    /// lineage.
    pub message_id: Option<MessageId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable context item id used for typed lineage, lookup, or dedupe.
    pub context_item_id: Option<ContextItemId>,
    /// Stable trace id used for typed lineage, lookup, or dedupe.
    pub trace_id: TraceId,
    /// Stable span id used for typed lineage, lookup, or dedupe.
    pub span_id: SpanId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable parent event id used for typed lineage, lookup, or dedupe.
    pub parent_event_id: Option<EventId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional caused by value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub caused_by: Option<CausalRef>,
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
    pub causal_refs: Vec<CausalRef>,
    /// Correlation used by this record or request.
    pub correlation: EventCorrelation,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Tag selector for event filtering.
    /// `Any` leaves tags unconstrained; `Include` restricts matches to listed event tags.
    pub tags: Vec<EventTag>,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: Option<DestinationRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub journal_cursor: Option<JournalCursor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional state before value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub state_before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional state after value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub state_after: Option<String>,
    /// Delivery-semantic selector for event filtering.
    /// `Any` leaves delivery semantics unconstrained; `Include` restricts matches to listed
    /// semantics.
    pub delivery_semantics: EventDeliverySemantics,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Content capture used by this record or request.
    pub content_capture: ContentCaptureMode,
    /// Stable redaction policy id used for typed lineage, lookup, or dedupe.
    pub redaction_policy_id: String,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
}

impl EventEnvelope {
    /// Builds the cursor value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn cursor(&self, scope: EventStreamScope) -> EventCursor {
        EventCursor {
            scope,
            event_seq: self.event_seq,
            event_id: self.event_id.clone(),
            journal_cursor: self.journal_cursor.clone(),
        }
    }

    /// Builds the redacted summary value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn redacted_summary(&self) -> String {
        format!("{:?}/{:?}", self.event_family, self.event_kind)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
/// Enumerates the finite event payload cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum EventPayload {
    /// Use this variant when the contract needs to represent envelope only; selecting it has no side effect by itself.
    EnvelopeOnly,
    /// Use this variant when the contract needs to represent redacted summary; selecting it has no side effect by itself.
    RedactedSummary {
        /// Redacted human-readable summary safe for events, telemetry, and
        /// logs.
        redacted_summary: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        /// Typed payload refs references. Resolving them is separate from
        /// constructing this record.
        payload_refs: Vec<EntityRef>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the causal ref record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct CausalRef {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Event identifier used to correlate live events with journal or replay
    /// evidence.
    pub event_id: Option<EventId>,
    /// Typed subject ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub subject_ref: EntityRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Redacted explanation for a denial, failure, status, or package delta.
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the event correlation record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct EventCorrelation {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Bounded entries included in this record. Limits and truncation are
    /// represented by companion metadata when applicable.
    pub entries: Vec<CorrelationEntry>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Carries the event tag record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct EventTag(String);

impl EventTag {
    /// Creates a new records::event value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the event frame record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct EventFrame {
    /// Event used by this record or request.
    pub event: AgentEvent,
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub cursor: EventCursor,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub archive_cursor: Option<ArchiveCursor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Overflow policy applied when a subscriber queue reaches capacity.
    /// It decides whether to drop, summarize, backpressure, or fail the subscriber.
    pub overflow: Option<EventOverflowNotice>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the event cursor record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct EventCursor {
    /// Scope used by this record or request.
    pub scope: EventStreamScope,
    /// Event seq used by this record or request.
    pub event_seq: u64,
    /// Event identifier used to correlate live events with journal or replay
    /// evidence.
    pub event_id: EventId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub journal_cursor: Option<JournalCursor>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite event stream scope cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum EventStreamScope {
    /// Use this variant when the contract needs to represent all; selecting it has no side effect by itself.
    All,
    /// Use this variant when the contract needs to represent run; selecting it has no side effect by itself.
    Run(RunId),
    /// Use this variant when the contract needs to represent agent; selecting it has no side effect by itself.
    Agent(AgentId),
    /// Use this variant when the contract needs to represent filter; selecting it has no side effect by itself.
    Filter {
        /// Stable filter id used for typed lineage, lookup, or dedupe.
        filter_id: EventFilterId,
        /// Deterministic filter fingerprint used for stale checks, package
        /// evidence, or replay comparisons.
        filter_fingerprint: EventFilterFingerprint,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the archive cursor record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ArchiveCursor {
    /// Stable archive id used for typed lineage, lookup, or dedupe.
    pub archive_id: ArchiveCursorId,
    /// Position used by this record or request.
    pub position: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Event identifier used to correlate live events with journal or replay
    /// evidence.
    pub event_id: Option<EventId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional watermark value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub watermark: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the event overflow notice record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct EventOverflowNotice {
    /// Policy used by this record or request.
    pub policy: SubscriberOverflowPolicy,
    /// Count of dropped items observed or included in this record.
    pub dropped_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional gap start value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub gap_start: Option<EventCursor>,
    /// Gap end used by this record or request.
    pub gap_end: EventCursor,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Repair policy used after structured output validation fails.
    /// It controls whether repair is attempted and which policy gates must approve it.
    pub repair_from: Option<JournalCursor>,
    /// Whether terminal preserved is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub terminal_preserved: bool,
    /// Redacted explanation for a denial, failure, status, or package delta.
    pub reason: EventOverflowReason,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite event overflow reason cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum EventOverflowReason {
    /// Use this variant when the contract needs to represent subscriber queue full; selecting it has no side effect by itself.
    SubscriberQueueFull,
    /// Use this variant when the contract needs to represent subscriber lagged; selecting it has no side effect by itself.
    SubscriberLagged,
    /// Use this variant when the contract needs to represent live buffer expired; selecting it has no side effect by itself.
    LiveBufferExpired,
    /// Use this variant when the contract needs to represent policy dropped progress; selecting it has no side effect by itself.
    PolicyDroppedProgress,
    /// Use this variant when the contract needs to represent policy dropped non terminal; selecting it has no side effect by itself.
    PolicyDroppedNonTerminal,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite event delivery semantics cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum EventDeliverySemantics {
    /// Use this variant when the contract needs to represent best effort live; selecting it has no side effect by itself.
    BestEffortLive,
    /// Use this variant when the contract needs to represent journal backed; selecting it has no side effect by itself.
    JournalBacked,
    /// Use this variant when the contract needs to represent derived replay; selecting it has no side effect by itself.
    DerivedReplay,
    /// Use this variant when the contract needs to represent diagnostic only; selecting it has no side effect by itself.
    DiagnosticOnly,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite content capture mode cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ContentCaptureMode {
    /// Use this variant when the contract needs to represent off; selecting it has no side effect by itself.
    Off,
    /// Use this variant when the contract needs to represent metadata only; selecting it has no side effect by itself.
    MetadataOnly,
    /// Use this variant when the contract needs to represent redacted summary; selecting it has no side effect by itself.
    RedactedSummary,
    /// Use this variant when the contract needs to represent payload refs; selecting it has no side effect by itself.
    PayloadRefs,
    /// Use this variant when the contract needs to represent raw content; selecting it has no side effect by itself.
    RawContent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite payload access mode cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum PayloadAccessMode {
    /// Use this variant when the contract needs to represent envelope only; selecting it has no side effect by itself.
    EnvelopeOnly,
    /// Use this variant when the contract needs to represent redacted summary; selecting it has no side effect by itself.
    RedactedSummary,
    /// Use this variant when the contract needs to represent payload refs; selecting it has no side effect by itself.
    PayloadRefs,
    /// Use this variant when the contract needs to represent full payload if policy allows; selecting it has no side effect by itself.
    FullPayloadIfPolicyAllows,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite subscriber overflow policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum SubscriberOverflowPolicy {
    /// Use this variant when the contract needs to represent drop non terminal; selecting it has no side effect by itself.
    DropNonTerminal,
    /// Use this variant when the contract needs to represent drop progress; selecting it has no side effect by itself.
    DropProgress,
    /// Use this variant when the contract needs to represent summarize and continue; selecting it has no side effect by itself.
    SummarizeAndContinue,
    /// Use this variant when the contract needs to represent backpressure caller; selecting it has no side effect by itself.
    BackpressureCaller,
    /// Use this variant when the contract needs to represent fail subscriber; selecting it has no side effect by itself.
    FailSubscriber,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the subscriber queue config record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct SubscriberQueueConfig {
    /// Total subscriber queue capacity.
    /// This bounds buffered event frames for a live subscriber.
    pub capacity: NonZeroUsize,
    /// Queue slots reserved for terminal frames.
    /// This keeps important terminal events available even when non-terminal frames overflow.
    pub terminal_reserve: NonZeroUsize,
    /// Overflow policy applied when a subscriber queue reaches capacity.
    /// It decides whether to drop, summarize, backpressure, or fail the subscriber.
    pub overflow: SubscriberOverflowPolicy,
}

impl Default for SubscriberQueueConfig {
    fn default() -> Self {
        Self {
            capacity: NonZeroUsize::new(64).expect("nonzero default capacity"),
            terminal_reserve: NonZeroUsize::new(1).expect("nonzero terminal reserve"),
            overflow: SubscriberOverflowPolicy::DropNonTerminal,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite event filter set cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum EventFilterSet<T> {
    #[default]
    /// Use this variant when the contract needs to represent any; selecting it has no side effect by itself.
    Any,
    /// Use this variant when the contract needs to represent include; selecting it has no side effect by itself.
    Include(Vec<T>),
}

impl<T: PartialEq> EventFilterSet<T> {
    /// Returns matches for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn matches(&self, candidate: &T) -> bool {
        match self {
            Self::Any => true,
            Self::Include(values) => values.contains(candidate),
        }
    }

    /// Reports whether this value is any. The check is pure and does
    /// not mutate SDK or host state.
    pub fn is_any(&self) -> bool {
        matches!(self, Self::Any)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the event filter record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct EventFilter {
    /// Run-id selector for event filtering.
    /// `Any` leaves run ids unconstrained; `Include` restricts matches to the listed runs.
    pub run_ids: EventFilterSet<RunId>,
    /// Agent-id selector for event filtering.
    /// `Any` leaves agent ids unconstrained; `Include` restricts matches to the listed agents.
    pub agent_ids: EventFilterSet<AgentId>,
    /// Turn-id selector for event filtering.
    /// `Any` leaves turn ids unconstrained; `Include` restricts matches to the listed turns.
    pub turn_ids: EventFilterSet<TurnId>,
    /// Event-family selector for event filtering.
    /// `Any` leaves event families unconstrained; `Include` restricts matches to listed
    /// families.
    pub families: EventFilterSet<EventFamily>,
    /// Event-kind selector for event filtering.
    /// `Any` leaves event kinds unconstrained; `Include` restricts matches to listed event
    /// kinds.
    pub kinds: EventFilterSet<EventKind>,
    /// Source-kind selector for event filtering.
    /// `Any` leaves source kinds unconstrained; `Include` restricts matches to listed source
    /// kinds.
    pub source_kinds: EventFilterSet<SourceKind>,
    /// Destination-kind selector for event filtering.
    /// `Any` leaves destination kinds unconstrained; `Include` restricts matches to listed
    /// destination kinds.
    pub destination_kinds: EventFilterSet<DestinationKind>,
    /// Subject entity-kind selector for event filtering.
    /// `Any` leaves subject kinds unconstrained; `Include` restricts matches to listed entity
    /// kinds.
    pub subject_kinds: EventFilterSet<EntityKind>,
    /// Related-entity kind selector for event filtering.
    /// `Any` leaves related kinds unconstrained; `Include` restricts matches to listed entity
    /// kinds.
    pub related_entity_kinds: EventFilterSet<EntityKind>,
    /// Correlation-key selector for event filtering.
    /// `Any` leaves correlation keys unconstrained; `Include` restricts matches to listed keys.
    pub correlation_keys: EventFilterSet<crate::domain::CorrelationKey>,
    /// Tag selector for event filtering.
    /// `Any` leaves tags unconstrained; `Include` restricts matches to listed event tags.
    pub tags: EventFilterSet<EventTag>,
    /// Privacy-class selector for event filtering.
    /// `Any` leaves privacy classes unconstrained; `Include` restricts matches to listed
    /// classes.
    pub privacy_classes: EventFilterSet<PrivacyClass>,
    /// Delivery-semantic selector for event filtering.
    /// `Any` leaves delivery semantics unconstrained; `Include` restricts matches to listed
    /// semantics.
    pub delivery_semantics: EventFilterSet<EventDeliverySemantics>,
    /// Whether the filter should match only terminal event frames.
    /// When true, non-terminal frames are excluded even if all other selectors match.
    pub terminal_only: bool,
    /// Payload access mode allowed for matching event frames.
    /// Use it to keep subscriptions envelope-only unless payload access is explicitly
    /// requested.
    pub payload_access: PayloadAccessMode,
    /// Subscriber queue settings used for streams created with this filter.
    /// It controls capacity, terminal reserve, and overflow behavior for the subscriber.
    pub queue: SubscriberQueueConfig,
}

impl Default for EventFilter {
    fn default() -> Self {
        Self {
            run_ids: EventFilterSet::Any,
            agent_ids: EventFilterSet::Any,
            turn_ids: EventFilterSet::Any,
            families: EventFilterSet::Any,
            kinds: EventFilterSet::Any,
            source_kinds: EventFilterSet::Any,
            destination_kinds: EventFilterSet::Any,
            subject_kinds: EventFilterSet::Any,
            related_entity_kinds: EventFilterSet::Any,
            correlation_keys: EventFilterSet::Any,
            tags: EventFilterSet::Any,
            privacy_classes: EventFilterSet::Any,
            delivery_semantics: EventFilterSet::Any,
            terminal_only: false,
            payload_access: PayloadAccessMode::EnvelopeOnly,
            queue: SubscriberQueueConfig::default(),
        }
    }
}

impl EventFilter {
    /// Builds the terminal run events value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn terminal_run_events() -> Self {
        Self {
            families: EventFilterSet::Include(vec![EventFamily::Run]),
            terminal_only: true,
            ..Self::default()
        }
    }

    /// Builds the run value with the documented defaults.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn run(run_id: RunId) -> Self {
        Self {
            run_ids: EventFilterSet::Include(vec![run_id]),
            ..Self::default()
        }
    }

    /// Returns agent for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn agent(agent_id: AgentId) -> Self {
        Self {
            agent_ids: EventFilterSet::Include(vec![agent_id]),
            ..Self::default()
        }
    }

    /// Returns compile for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn compile(self) -> Result<CompiledEventFilter, AgentError> {
        CompiledEventFilter::new(self)
    }

    fn indexed_fields(&self) -> Vec<EventIndexField> {
        let mut fields = Vec::new();
        if !self.run_ids.is_any() {
            fields.push(EventIndexField::RunId);
        }
        if !self.agent_ids.is_any() {
            fields.push(EventIndexField::AgentId);
        }
        if !self.turn_ids.is_any() {
            fields.push(EventIndexField::TurnId);
        }
        if !self.families.is_any() {
            fields.push(EventIndexField::EventFamily);
        }
        if !self.kinds.is_any() {
            fields.push(EventIndexField::EventKind);
        }
        if !self.source_kinds.is_any() {
            fields.push(EventIndexField::Source);
        }
        if !self.destination_kinds.is_any() {
            fields.push(EventIndexField::Destination);
        }
        if !self.subject_kinds.is_any() {
            fields.push(EventIndexField::SubjectKind);
        }
        if !self.related_entity_kinds.is_any() {
            fields.push(EventIndexField::RelatedEntityKind);
        }
        if !self.correlation_keys.is_any() {
            fields.push(EventIndexField::CorrelationKey);
        }
        if !self.tags.is_any() {
            fields.push(EventIndexField::Tag);
        }
        if !self.privacy_classes.is_any() {
            fields.push(EventIndexField::Privacy);
        }
        if !self.delivery_semantics.is_any() {
            fields.push(EventIndexField::DeliverySemantics);
        }
        fields
    }

    fn matches_envelope(&self, envelope: &EventEnvelope) -> bool {
        self.run_ids.matches(&envelope.run_id)
            && self.agent_ids.matches(&envelope.agent_id)
            && option_matches(&self.turn_ids, envelope.turn_id.as_ref())
            && self.families.matches(&envelope.event_family)
            && self.kinds.matches(&envelope.event_kind)
            && self.source_kinds.matches(&envelope.source.kind)
            && option_matches(
                &self.destination_kinds,
                envelope
                    .destination
                    .as_ref()
                    .map(|destination| &destination.kind),
            )
            && self.subject_kinds.matches(&envelope.subject_ref.kind)
            && any_matches(
                &self.related_entity_kinds,
                envelope.related_refs.iter().map(|entity| &entity.kind),
            )
            && any_matches(
                &self.correlation_keys,
                envelope.correlation.entries.iter().map(|entry| &entry.key),
            )
            && any_matches(&self.tags, envelope.tags.iter())
            && self.privacy_classes.matches(&envelope.privacy)
            && self
                .delivery_semantics
                .matches(&envelope.delivery_semantics)
            && (!self.terminal_only || envelope.event_kind.is_terminal())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the compiled event filter record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct CompiledEventFilter {
    /// Stable filter id used for typed lineage, lookup, or dedupe.
    pub filter_id: EventFilterId,
    /// Deterministic filter fingerprint used for stale checks, package
    /// evidence, or replay comparisons.
    pub filter_fingerprint: EventFilterFingerprint,
    /// Collection of indexed fields values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub indexed_fields: Vec<EventIndexField>,
    /// Payload access mode allowed for matching event frames.
    /// Use it to keep subscriptions envelope-only unless payload access is explicitly
    /// requested.
    pub payload_access: PayloadAccessMode,
    /// Subscriber queue settings used for streams created with this filter.
    /// It controls capacity, terminal reserve, and overflow behavior for the subscriber.
    pub queue: SubscriberQueueConfig,
    criteria: EventFilter,
}

impl CompiledEventFilter {
    /// Creates a new records::event value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new(criteria: EventFilter) -> Result<Self, AgentError> {
        let encoded = serde_json::to_vec(&criteria)
            .map_err(|error| AgentError::contract_violation(error.to_string()))?;
        let fingerprint = format!("sha256:{:x}", Sha256::digest(encoded));
        let filter_id = EventFilterId::new(format!("filter.{:x}", Sha256::digest(&fingerprint)));
        let indexed_fields = criteria.indexed_fields();

        Ok(Self {
            filter_id,
            filter_fingerprint: EventFilterFingerprint::new(fingerprint),
            indexed_fields,
            payload_access: criteria.payload_access.clone(),
            queue: criteria.queue.clone(),
            criteria,
        })
    }

    /// Returns matches envelope for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn matches_envelope(&self, envelope: &EventEnvelope) -> bool {
        self.criteria.matches_envelope(envelope)
    }

    /// Returns cursor scope derived from the supplied state.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn cursor_scope(&self) -> EventStreamScope {
        EventStreamScope::Filter {
            filter_id: self.filter_id.clone(),
            filter_fingerprint: self.filter_fingerprint.clone(),
        }
    }

    /// Returns the criteria currently held by this value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn criteria(&self) -> &EventFilter {
        &self.criteria
    }
}

fn option_matches<T: PartialEq>(filter: &EventFilterSet<T>, candidate: Option<&T>) -> bool {
    match filter {
        EventFilterSet::Any => true,
        EventFilterSet::Include(_) => candidate.is_some_and(|candidate| filter.matches(candidate)),
    }
}

fn any_matches<'a, T: PartialEq + 'a>(
    filter: &EventFilterSet<T>,
    candidates: impl Iterator<Item = &'a T>,
) -> bool {
    match filter {
        EventFilterSet::Any => true,
        EventFilterSet::Include(_) => candidates
            .into_iter()
            .any(|candidate| filter.matches(candidate)),
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite event index field cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum EventIndexField {
    /// Use this variant when the contract needs to represent run id; selecting it has no side effect by itself.
    RunId,
    /// Use this variant when the contract needs to represent agent id; selecting it has no side effect by itself.
    AgentId,
    /// Use this variant when the contract needs to represent turn id; selecting it has no side effect by itself.
    TurnId,
    /// Use this variant when the contract needs to represent event family; selecting it has no side effect by itself.
    EventFamily,
    /// Use this variant when the contract needs to represent event kind; selecting it has no side effect by itself.
    EventKind,
    /// Use this variant when the contract needs to represent source; selecting it has no side effect by itself.
    Source,
    /// Use this variant when the contract needs to represent destination; selecting it has no side effect by itself.
    Destination,
    /// Use this variant when the contract needs to represent subject kind; selecting it has no side effect by itself.
    SubjectKind,
    /// Use this variant when the contract needs to represent related entity kind; selecting it has no side effect by itself.
    RelatedEntityKind,
    /// Use this variant when the contract needs to represent correlation key; selecting it has no side effect by itself.
    CorrelationKey,
    /// Use this variant when the contract needs to represent tag; selecting it has no side effect by itself.
    Tag,
    /// Use this variant when the contract needs to represent privacy; selecting it has no side effect by itself.
    Privacy,
    /// Use this variant when the contract needs to represent delivery semantics; selecting it has no side effect by itself.
    DeliverySemantics,
}

typed_string!(EventFilterId, "EventFilterId");
typed_string!(EventFilterFingerprint, "EventFilterFingerprint");

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the subscription options record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct SubscriptionOptions {
    /// Subscriber queue settings used for streams created with this filter.
    /// It controls capacity, terminal reserve, and overflow behavior for the subscriber.
    pub queue: SubscriberQueueConfig,
    /// Payload access mode allowed for matching event frames.
    /// Use it to keep subscriptions envelope-only unless payload access is explicitly
    /// requested.
    pub payload_access: PayloadAccessMode,
}

impl Default for SubscriptionOptions {
    fn default() -> Self {
        Self {
            queue: SubscriberQueueConfig::default(),
            payload_access: PayloadAccessMode::EnvelopeOnly,
        }
    }
}

/// Returns cursor compatible derived from the supplied state.
/// This is data-only and does not perform I/O, call host ports, append journals, publish
/// events, or start processes.
pub fn cursor_compatible(
    requested_scope: &EventStreamScope,
    cursor: Option<&EventCursor>,
) -> Result<(), AgentError> {
    let Some(cursor) = cursor else {
        return Ok(());
    };

    let compatible = match (requested_scope, &cursor.scope) {
        (EventStreamScope::All, EventStreamScope::All) => true,
        (EventStreamScope::Run(requested), EventStreamScope::Run(cursor_run)) => {
            requested == cursor_run
        }
        (EventStreamScope::Agent(requested), EventStreamScope::Agent(cursor_agent)) => {
            requested == cursor_agent
        }
        (
            EventStreamScope::Filter {
                filter_fingerprint: requested,
                ..
            },
            EventStreamScope::Filter {
                filter_fingerprint: cursor,
                ..
            },
        ) => requested == cursor,
        _ => false,
    };

    if compatible {
        Ok(())
    } else {
        Err(AgentError::contract_violation(
            "cursor scope mismatch for requested event stream",
        ))
    }
}
