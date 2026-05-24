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
        #[derive(Clone, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

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

pub const EVENT_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventFamily {
    Run,
    Turn,
    Message,
    Model,
    Tool,
    Approval,
    Hook,
    Context,
    StreamRule,
    Realtime,
    Isolation,
    ChildLifecycle,
    AgentPool,
    Subagent,
    Extension,
    StructuredOutput,
    Output,
    OutputDelivery,
    Telemetry,
    Recovery,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    RunStarted,
    RunCompleted,
    RunFailed,
    RunCancelled,
    RunCheckpointed,
    RunCancelRequested,
    RunResumeRequested,
    RunResumeFailed,
    TurnStarted,
    TurnCompleted,
    TurnFailed,
    MessageAccepted,
    MessageCommitted,
    ProviderRequestProjected,
    ModelAttemptStarted,
    ModelStreamDelta,
    ModelMessageCompleted,
    ModelAttemptFailed,
    ToolRequested,
    ToolStarted,
    ToolCompleted,
    ToolFailed,
    ToolDenied,
    ToolRecoveryRequired,
    ApprovalRequested,
    ApprovalDispatched,
    ApprovalDispatchUnavailable,
    ApprovalResponded,
    ApprovalDenied,
    ApprovalTimedOut,
    ApprovalCancelled,
    HookRegistered,
    HookInvoked,
    HookCompleted,
    HookFailed,
    HookTimedOut,
    HookCancelled,
    HookResponseApplied,
    HookResponseRejected,
    ContextAssembled,
    StreamRuleRegistered,
    StreamRuleCompileFailed,
    StreamRuleMatched,
    StreamInterventionRequested,
    StreamInterventionApplied,
    StreamRuleRepeatStateRecorded,
    RealtimeConnectRequested,
    RealtimeConnected,
    RealtimeInputSendRequested,
    RealtimeInputSent,
    RealtimeOutputReceiveRequested,
    RealtimeOutputReceived,
    RealtimeInterrupted,
    RealtimeRestartRequested,
    RealtimeRestartStarted,
    RealtimeRestartCompleted,
    RealtimeRestartFailed,
    RealtimeCloseRequested,
    RealtimeClosed,
    RealtimeBackpressureApplied,
    IsolationRequested,
    IsolationAdapterHealthChecked,
    IsolationCapabilityMatched,
    IsolationDowngradeDenied,
    IsolationDowngradeApproved,
    IsolationEnvironmentPrepared,
    IsolationProcessStarted,
    IsolationProcessIoCaptured,
    IsolationProcessStatsRecorded,
    IsolationCleanupStarted,
    IsolationCleanupCompleted,
    IsolationCleanupFailed,
    IsolationFailed,
    ChildLifecycleRequested,
    ChildLifecycleCompleted,
    ChildLifecycleDetached,
    SubagentStarted,
    SubagentHandoff,
    SubagentEventWrapped,
    SubagentUsageRolledUp,
    SubagentCompleted,
    ExtensionActionSubmitted,
    ExtensionActionStarted,
    ExtensionActionCompleted,
    ExtensionActionFailed,
    ExtensionActionDenied,
    OutputDispatchRequested,
    OutputDispatchCompleted,
    OutputDispatchFailed,
    OutputDispatchDeduped,
    StructuredOutputRequested,
    StructuredOutputValidationStarted,
    StructuredOutputValidationFailed,
    StructuredOutputRepairRequested,
    StructuredOutputValidated,
    StructuredOutputFailed,
    TelemetrySinkFailed,
    TelemetrySinkRecovered,
    UsageRecorded,
    CostEstimated,
    CostCorrected,
    ReplayStarted,
    ReplayCompleted,
    ReplayFailed,
    AgentPoolCreated,
    AgentPoolRunJoined,
    AgentPoolRunLeft,
    RunMessageAccepted,
    RunMessageDelivered,
    RunMessageResponded,
    RunMessageFailed,
    RunMessageTimedOut,
    RunMessageExpired,
    RunMessageCancelled,
    WakeConditionRegistered,
    WakeConditionTriggered,
    WakeConditionTimedOut,
    WakeConditionCancelled,
    WakeConditionFailed,
}

impl EventKind {
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
pub struct AgentEvent {
    pub envelope: EventEnvelope,
    pub payload: EventPayload,
}

impl AgentEvent {
    pub fn envelope_only(envelope: EventEnvelope) -> Self {
        Self {
            envelope,
            payload: EventPayload::EnvelopeOnly,
        }
    }

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
pub struct EventEnvelope {
    pub schema_version: u16,
    pub event_id: EventId,
    pub event_seq: u64,
    pub event_family: EventFamily,
    pub event_kind: EventKind,
    pub payload_schema_version: u16,
    pub timestamp: String,
    pub recorded_at: String,
    pub run_id: RunId,
    pub agent_id: AgentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<TurnId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<AttemptId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<MessageId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_item_id: Option<ContextItemId>,
    pub trace_id: TraceId,
    pub span_id: SpanId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_event_id: Option<EventId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caused_by: Option<CausalRef>,
    pub subject_ref: EntityRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_refs: Vec<EntityRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub causal_refs: Vec<CausalRef>,
    pub correlation: EventCorrelation,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<EventTag>,
    pub source: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<DestinationRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journal_cursor: Option<JournalCursor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_after: Option<String>,
    pub delivery_semantics: EventDeliverySemantics,
    pub privacy: PrivacyClass,
    pub content_capture: ContentCaptureMode,
    pub redaction_policy_id: String,
    pub runtime_package_fingerprint: String,
}

impl EventEnvelope {
    pub fn cursor(&self, scope: EventStreamScope) -> EventCursor {
        EventCursor {
            scope,
            event_seq: self.event_seq,
            event_id: self.event_id.clone(),
            journal_cursor: self.journal_cursor.clone(),
        }
    }

    pub fn redacted_summary(&self) -> String {
        format!("{:?}/{:?}", self.event_family, self.event_kind)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum EventPayload {
    EnvelopeOnly,
    RedactedSummary {
        redacted_summary: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        payload_refs: Vec<EntityRef>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CausalRef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<EventId>,
    pub subject_ref: EntityRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventCorrelation {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<CorrelationEntry>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct EventTag(String);

impl EventTag {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventFrame {
    pub event: AgentEvent,
    pub cursor: EventCursor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_cursor: Option<ArchiveCursor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overflow: Option<EventOverflowNotice>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventCursor {
    pub scope: EventStreamScope,
    pub event_seq: u64,
    pub event_id: EventId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journal_cursor: Option<JournalCursor>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventStreamScope {
    All,
    Run(RunId),
    Agent(AgentId),
    Filter {
        filter_id: EventFilterId,
        filter_fingerprint: EventFilterFingerprint,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArchiveCursor {
    pub archive_id: ArchiveCursorId,
    pub position: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<EventId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watermark: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventOverflowNotice {
    pub policy: SubscriberOverflowPolicy,
    pub dropped_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap_start: Option<EventCursor>,
    pub gap_end: EventCursor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repair_from: Option<JournalCursor>,
    pub terminal_preserved: bool,
    pub reason: EventOverflowReason,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventOverflowReason {
    SubscriberQueueFull,
    SubscriberLagged,
    LiveBufferExpired,
    PolicyDroppedProgress,
    PolicyDroppedNonTerminal,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventDeliverySemantics {
    BestEffortLive,
    JournalBacked,
    DerivedReplay,
    DiagnosticOnly,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentCaptureMode {
    Off,
    MetadataOnly,
    RedactedSummary,
    PayloadRefs,
    RawContent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadAccessMode {
    EnvelopeOnly,
    RedactedSummary,
    PayloadRefs,
    FullPayloadIfPolicyAllows,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriberOverflowPolicy {
    DropNonTerminal,
    DropProgress,
    SummarizeAndContinue,
    BackpressureCaller,
    FailSubscriber,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubscriberQueueConfig {
    pub capacity: NonZeroUsize,
    pub terminal_reserve: NonZeroUsize,
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
pub enum EventFilterSet<T> {
    #[default]
    Any,
    Include(Vec<T>),
}

impl<T: PartialEq> EventFilterSet<T> {
    pub fn matches(&self, candidate: &T) -> bool {
        match self {
            Self::Any => true,
            Self::Include(values) => values.contains(candidate),
        }
    }

    pub fn is_any(&self) -> bool {
        matches!(self, Self::Any)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventFilter {
    pub run_ids: EventFilterSet<RunId>,
    pub agent_ids: EventFilterSet<AgentId>,
    pub turn_ids: EventFilterSet<TurnId>,
    pub families: EventFilterSet<EventFamily>,
    pub kinds: EventFilterSet<EventKind>,
    pub source_kinds: EventFilterSet<SourceKind>,
    pub destination_kinds: EventFilterSet<DestinationKind>,
    pub subject_kinds: EventFilterSet<EntityKind>,
    pub related_entity_kinds: EventFilterSet<EntityKind>,
    pub correlation_keys: EventFilterSet<crate::domain::CorrelationKey>,
    pub tags: EventFilterSet<EventTag>,
    pub privacy_classes: EventFilterSet<PrivacyClass>,
    pub delivery_semantics: EventFilterSet<EventDeliverySemantics>,
    pub terminal_only: bool,
    pub payload_access: PayloadAccessMode,
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
    pub fn terminal_run_events() -> Self {
        Self {
            families: EventFilterSet::Include(vec![EventFamily::Run]),
            terminal_only: true,
            ..Self::default()
        }
    }

    pub fn run(run_id: RunId) -> Self {
        Self {
            run_ids: EventFilterSet::Include(vec![run_id]),
            ..Self::default()
        }
    }

    pub fn agent(agent_id: AgentId) -> Self {
        Self {
            agent_ids: EventFilterSet::Include(vec![agent_id]),
            ..Self::default()
        }
    }

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
pub struct CompiledEventFilter {
    pub filter_id: EventFilterId,
    pub filter_fingerprint: EventFilterFingerprint,
    pub indexed_fields: Vec<EventIndexField>,
    pub payload_access: PayloadAccessMode,
    pub queue: SubscriberQueueConfig,
    criteria: EventFilter,
}

impl CompiledEventFilter {
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

    pub fn matches_envelope(&self, envelope: &EventEnvelope) -> bool {
        self.criteria.matches_envelope(envelope)
    }

    pub fn cursor_scope(&self) -> EventStreamScope {
        EventStreamScope::Filter {
            filter_id: self.filter_id.clone(),
            filter_fingerprint: self.filter_fingerprint.clone(),
        }
    }

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
pub enum EventIndexField {
    RunId,
    AgentId,
    TurnId,
    EventFamily,
    EventKind,
    Source,
    Destination,
    SubjectKind,
    RelatedEntityKind,
    CorrelationKey,
    Tag,
    Privacy,
    DeliverySemantics,
}

typed_string!(EventFilterId, "EventFilterId");
typed_string!(EventFilterFingerprint, "EventFilterFingerprint");

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubscriptionOptions {
    pub queue: SubscriberQueueConfig,
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
