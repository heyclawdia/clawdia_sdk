use crate::domain::{
    AttemptId, DestinationRef, EntityRef, EventId, PolicyRef, PrivacyClass, RunId, SourceRef,
    SpanId, ToolCallId, TurnId,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Error, Eq, PartialEq, Serialize)]
pub enum AgentError {
    #[error("missing required field: {field}")]
    MissingRequiredField { field: String },
    #[error("contract violation: {message}")]
    ContractViolation { message: String },
    #[error("host configuration needed: {message}")]
    HostConfigurationNeeded { message: String },
    #[error("{kind:?}: {}", context.message)]
    Classified {
        kind: AgentErrorKind,
        retry: RetryClassification,
        context: ErrorContext,
        causal_ids: CausalIds,
    },
}

impl AgentError {
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

    pub fn kind(&self) -> AgentErrorKind {
        match self {
            Self::MissingRequiredField { .. } => AgentErrorKind::InvalidPackage,
            Self::ContractViolation { .. } => AgentErrorKind::InvalidStateTransition,
            Self::HostConfigurationNeeded { .. } => AgentErrorKind::HostConfigurationNeeded,
            Self::Classified { kind, .. } => kind.clone(),
        }
    }

    pub fn retry(&self) -> RetryClassification {
        match self {
            Self::MissingRequiredField { .. } | Self::HostConfigurationNeeded { .. } => {
                RetryClassification::HostConfigurationNeeded
            }
            Self::ContractViolation { .. } => RetryClassification::RepairNeeded,
            Self::Classified { retry, .. } => retry.clone(),
        }
    }

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

    pub fn causal_ids(&self) -> CausalIds {
        match self {
            Self::Classified { causal_ids, .. } => causal_ids.clone(),
            _ => CausalIds::default(),
        }
    }

    pub fn with_policy_ref(self, policy_ref: PolicyRef) -> Self {
        self.map_context(|context| context.policy_refs.push(policy_ref))
    }

    pub fn with_source(self, source: SourceRef) -> Self {
        self.map_context(|context| context.source = Some(source))
    }

    pub fn with_destination(self, destination: DestinationRef) -> Self {
        self.map_context(|context| context.destination = Some(destination))
    }

    pub fn with_subject(self, subject: EntityRef) -> Self {
        self.map_context(|context| context.subject = Some(subject))
    }

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

    pub fn missing_required_field(field: impl Into<String>) -> Self {
        Self::MissingRequiredField {
            field: field.into(),
        }
    }

    pub fn contract_violation(message: impl Into<String>) -> Self {
        Self::ContractViolation {
            message: message.into(),
        }
    }

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
pub enum AgentErrorKind {
    #[error("invalid package")]
    InvalidPackage,
    #[error("invalid state transition")]
    InvalidStateTransition,
    #[error("provider failure")]
    ProviderFailure,
    #[error("projection failure")]
    ProjectionFailure,
    #[error("tool failure")]
    ToolFailure,
    #[error("approval failure")]
    ApprovalFailure,
    #[error("policy denial")]
    PolicyDenial,
    #[error("journal failure")]
    JournalFailure,
    #[error("telemetry failure")]
    TelemetryFailure,
    #[error("isolation failure")]
    IsolationFailure,
    #[error("structured output failure")]
    StructuredOutputFailure,
    #[error("stream rule failure")]
    StreamRuleFailure,
    #[error("subagent failure")]
    SubagentFailure,
    #[error("extension failure")]
    ExtensionFailure,
    #[error("cancellation")]
    Cancellation,
    #[error("child lifecycle failure")]
    ChildLifecycleFailure,
    #[error("hook failure")]
    HookFailure,
    #[error("timeout")]
    Timeout,
    #[error("recovery or repair needed")]
    RecoveryRepairNeeded,
    #[error("host configuration needed")]
    HostConfigurationNeeded,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RetryClassification {
    Retryable,
    NotRetryable,
    RepairNeeded,
    UserActionNeeded,
    HostConfigurationNeeded,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ErrorContext {
    pub message: String,
    pub policy_refs: Vec<PolicyRef>,
    pub source: Option<SourceRef>,
    pub destination: Option<DestinationRef>,
    pub subject: Option<EntityRef>,
    pub privacy: Option<PrivacyClass>,
    pub redacted_summary: Option<String>,
}

impl ErrorContext {
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
pub struct CausalIds {
    pub run_id: Option<RunId>,
    pub turn_id: Option<TurnId>,
    pub attempt_id: Option<AttemptId>,
    pub event_id: Option<EventId>,
    pub tool_call_id: Option<ToolCallId>,
    pub span_id: Option<SpanId>,
    pub related: Vec<EntityRef>,
}
