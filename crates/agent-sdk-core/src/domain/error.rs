//! Domain primitives for stable SDK vocabulary. Use these items for IDs, refs,
//! policy, privacy, trust, and errors that cross crate or host boundaries. They are
//! data-only and must not perform provider, filesystem, network, or UI side effects.
//! This file contains the error portion of that contract.
//!
use crate::domain::{
    AttemptId, DestinationRef, EntityRef, EventId, PolicyRef, PrivacyClass, RunId, SourceRef,
    SpanId, ToolCallId, TurnId,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Error, Eq, PartialEq, Serialize)]
/// Enumerates the finite agent error cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum AgentError {
    #[error("missing required field: {field}")]
    /// Use this variant when the contract needs to represent missing required field; selecting it has no side effect by itself.
    MissingRequiredField {
        /// Field used by this record or request.
        field: String,
    },
    #[error("contract violation: {message}")]
    /// Use this variant when the contract needs to represent contract violation; selecting it has no side effect by itself.
    ContractViolation {
        /// Message used by this record or request.
        message: String,
    },
    #[error("host configuration needed: {message}")]
    /// Use this variant when the contract needs to represent host configuration needed; selecting it has no side effect by itself.
    HostConfigurationNeeded {
        /// Message used by this record or request.
        message: String,
    },
    #[error("{kind:?}: {}", context.message)]
    /// Use this variant when the contract needs to represent classified; selecting it has no side effect by itself.
    Classified {
        /// Kind/category for this record, capability, event, or detected
        /// resource.
        kind: AgentErrorKind,
        /// Retry used by this record or request.
        retry: RetryClassification,
        /// Context used by this record or request.
        context: ErrorContext,
        /// Identifiers used to select or correlate causal values.
        /// Use them for typed lookup, filtering, or lineage instead of stringly typed matching.
        causal_ids: CausalIds,
    },
}

impl AgentError {
    /// Creates a new domain::error value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new(
        kind: AgentErrorKind,
        retry: RetryClassification,
        message: impl Into<String>,
    ) -> Self {
        Self::Classified {
            kind,
            retry,
            context: ErrorContext::new(message),
            causal_ids: CausalIds::default(),
        }
    }

    /// Builds the kind value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn kind(&self) -> AgentErrorKind {
        match self {
            Self::MissingRequiredField { .. } => AgentErrorKind::InvalidPackage,
            Self::ContractViolation { .. } => AgentErrorKind::InvalidStateTransition,
            Self::HostConfigurationNeeded { .. } => AgentErrorKind::HostConfigurationNeeded,
            Self::Classified { kind, .. } => kind.clone(),
        }
    }

    /// Builds the retry value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn retry(&self) -> RetryClassification {
        match self {
            Self::MissingRequiredField { .. } | Self::HostConfigurationNeeded { .. } => {
                RetryClassification::HostConfigurationNeeded
            }
            Self::ContractViolation { .. } => RetryClassification::RepairNeeded,
            Self::Classified { retry, .. } => retry.clone(),
        }
    }

    /// Builds the context value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn context(&self) -> ErrorContext {
        match self {
            Self::MissingRequiredField { field } => {
                ErrorContext::new(format!("missing required field: {field}"))
            }
            Self::ContractViolation { message } | Self::HostConfigurationNeeded { message } => {
                ErrorContext::new(message.clone())
            }
            Self::Classified { context, .. } => context.clone(),
        }
    }

    /// Builds the causal ids value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn causal_ids(&self) -> CausalIds {
        match self {
            Self::Classified { causal_ids, .. } => causal_ids.clone(),
            _ => CausalIds::default(),
        }
    }

    /// Returns this value with its policy ref setting replaced. The
    /// method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_policy_ref(self, policy_ref: PolicyRef) -> Self {
        self.map_context(|context| context.policy_refs.push(policy_ref))
    }

    /// Returns this value with its source setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_source(self, source: SourceRef) -> Self {
        self.map_context(|context| context.source = Some(source))
    }

    /// Returns this value with its destination setting replaced. The
    /// method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_destination(self, destination: DestinationRef) -> Self {
        self.map_context(|context| context.destination = Some(destination))
    }

    /// Returns this value with its subject setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_subject(self, subject: EntityRef) -> Self {
        self.map_context(|context| context.subject = Some(subject))
    }

    /// Returns this value with its causal ids setting replaced. The
    /// method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_causal_ids(self, causal_ids: CausalIds) -> Self {
        match self {
            Self::Classified {
                kind,
                retry,
                context,
                ..
            } => Self::Classified {
                kind,
                retry,
                context,
                causal_ids,
            },
            other => Self::Classified {
                kind: other.kind(),
                retry: other.retry(),
                context: other.context(),
                causal_ids,
            },
        }
    }

    /// Builds the missing required field value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn missing_required_field(field: impl Into<String>) -> Self {
        Self::MissingRequiredField {
            field: field.into(),
        }
    }

    /// Builds the contract violation value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn contract_violation(message: impl Into<String>) -> Self {
        Self::ContractViolation {
            message: message.into(),
        }
    }

    /// Builds the host configuration needed value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn host_configuration_needed(message: impl Into<String>) -> Self {
        Self::HostConfigurationNeeded {
            message: message.into(),
        }
    }

    fn map_context(self, update: impl FnOnce(&mut ErrorContext)) -> Self {
        let kind = self.kind();
        let retry = self.retry();
        let mut context = self.context();
        let causal_ids = self.causal_ids();
        update(&mut context);
        Self::Classified {
            kind,
            retry,
            context,
            causal_ids,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Error, Eq, PartialEq, Serialize)]
/// Enumerates the finite agent error kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum AgentErrorKind {
    #[error("invalid package")]
    /// Use this variant when the contract needs to represent invalid package; selecting it has no side effect by itself.
    InvalidPackage,
    #[error("invalid state transition")]
    /// Use this variant when the contract needs to represent invalid state transition; selecting it has no side effect by itself.
    InvalidStateTransition,
    #[error("provider failure")]
    /// Use this variant when the contract needs to represent provider failure; selecting it has no side effect by itself.
    ProviderFailure,
    #[error("projection failure")]
    /// Use this variant when the contract needs to represent projection failure; selecting it has no side effect by itself.
    ProjectionFailure,
    #[error("tool failure")]
    /// Use this variant when the contract needs to represent tool failure; selecting it has no side effect by itself.
    ToolFailure,
    #[error("approval failure")]
    /// Use this variant when the contract needs to represent approval failure; selecting it has no side effect by itself.
    ApprovalFailure,
    #[error("policy denial")]
    /// Use this variant when the contract needs to represent policy denial; selecting it has no side effect by itself.
    PolicyDenial,
    #[error("journal failure")]
    /// Use this variant when the contract needs to represent journal failure; selecting it has no side effect by itself.
    JournalFailure,
    #[error("telemetry failure")]
    /// Use this variant when the contract needs to represent telemetry failure; selecting it has no side effect by itself.
    TelemetryFailure,
    #[error("isolation failure")]
    /// Use this variant when the contract needs to represent isolation failure; selecting it has no side effect by itself.
    IsolationFailure,
    #[error("structured output failure")]
    /// Use this variant when the contract needs to represent structured output failure; selecting it has no side effect by itself.
    StructuredOutputFailure,
    #[error("stream rule failure")]
    /// Use this variant when the contract needs to represent stream rule failure; selecting it has no side effect by itself.
    StreamRuleFailure,
    #[error("subagent failure")]
    /// Use this variant when the contract needs to represent subagent failure; selecting it has no side effect by itself.
    SubagentFailure,
    #[error("extension failure")]
    /// Use this variant when the contract needs to represent extension failure; selecting it has no side effect by itself.
    ExtensionFailure,
    #[error("cancellation")]
    /// Use this variant when the contract needs to represent cancellation; selecting it has no side effect by itself.
    Cancellation,
    #[error("child lifecycle failure")]
    /// Use this variant when the contract needs to represent child lifecycle failure; selecting it has no side effect by itself.
    ChildLifecycleFailure,
    #[error("hook failure")]
    /// Use this variant when the contract needs to represent hook failure; selecting it has no side effect by itself.
    HookFailure,
    #[error("timeout")]
    /// Use this variant when the contract needs to represent timeout; selecting it has no side effect by itself.
    Timeout,
    #[error("recovery or repair needed")]
    /// Use this variant when the contract needs to represent recovery repair needed; selecting it has no side effect by itself.
    RecoveryRepairNeeded,
    #[error("host configuration needed")]
    /// Use this variant when the contract needs to represent host configuration needed; selecting it has no side effect by itself.
    HostConfigurationNeeded,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Enumerates the finite retry classification cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RetryClassification {
    /// Use this variant when the contract needs to represent retryable; selecting it has no side effect by itself.
    Retryable,
    /// Use this variant when the contract needs to represent not retryable; selecting it has no side effect by itself.
    NotRetryable,
    /// Use this variant when the contract needs to represent repair needed; selecting it has no side effect by itself.
    RepairNeeded,
    /// Use this variant when the contract needs to represent user action needed; selecting it has no side effect by itself.
    UserActionNeeded,
    /// Use this variant when the contract needs to represent host configuration needed; selecting it has no side effect by itself.
    HostConfigurationNeeded,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Defines the error context SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct ErrorContext {
    /// Message used by this record or request.
    pub message: String,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: Option<SourceRef>,
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: Option<DestinationRef>,
    /// Optional subject value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub subject: Option<EntityRef>,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: Option<PrivacyClass>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: Option<String>,
}

impl ErrorContext {
    /// Creates a new domain::error value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            policy_refs: Vec::new(),
            source: None,
            destination: None,
            subject: None,
            privacy: None,
            redacted_summary: None,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Defines the causal ids SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct CausalIds {
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: Option<RunId>,
    /// Turn identifier for one loop turn within a run.
    pub turn_id: Option<TurnId>,
    /// Attempt identifier for retry, repair, provider, or tool execution
    /// evidence.
    pub attempt_id: Option<AttemptId>,
    /// Event identifier used to correlate live events with journal or replay
    /// evidence.
    pub event_id: Option<EventId>,
    /// Stable tool call id used for typed lineage, lookup, or dedupe.
    pub tool_call_id: Option<ToolCallId>,
    /// Stable span id used for typed lineage, lookup, or dedupe.
    pub span_id: Option<SpanId>,
    /// Collection of related values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub related: Vec<EntityRef>,
}
