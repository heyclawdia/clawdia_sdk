use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        AgentError, AgentId, ApprovalRequestId, ContentRef, DestinationRef, EffectId, EntityKind,
        EntityRef, PolicyRef, RunId, SourceRef, ToolCallId, TurnId,
    },
    effect::{EffectIntent, EffectKind, EffectResult, EffectTerminalStatus},
    journal::{
        EventIndexProjection, JOURNAL_SCHEMA_VERSION, JournalRecord, JournalRecordBase,
        JournalRecordKind, JournalRecordPayload,
    },
    package::RuntimePackageFingerprint,
    policy::{ApprovalDecisionKind, DispatcherScope, EffectClass, RiskClass},
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApprovalRequest {
    pub approval_request_id: ApprovalRequestId,
    pub approval_dispatch_effect_id: EffectId,
    pub run_id: RunId,
    pub agent_id: AgentId,
    pub turn_id: TurnId,
    pub tool_call_id: ToolCallId,
    pub source: SourceRef,
    pub destination: DestinationRef,
    pub canonical_tool_name: String,
    pub tool_source: SourceRef,
    pub effect_class: EffectClass,
    pub risk_class: RiskClass,
    pub requested_args_ref: ContentRef,
    pub redacted_args_summary: String,
    pub policy_refs: Vec<PolicyRef>,
    pub dispatcher_scope: DispatcherScope,
    pub timeout_ms: u64,
    pub allowed_decisions: Vec<ApprovalDecisionKind>,
    pub created_at_millis: u64,
    pub runtime_package_fingerprint: RuntimePackageFingerprint,
}

impl ApprovalRequest {
    pub fn validate(&self) -> Result<(), AgentError> {
        if self.canonical_tool_name.is_empty() {
            return Err(AgentError::missing_required_field(
                "approval_request.canonical_tool_name",
            ));
        }
        if self.redacted_args_summary.is_empty() {
            return Err(AgentError::missing_required_field(
                "approval_request.redacted_args_summary",
            ));
        }
        if self.policy_refs.is_empty() {
            return Err(AgentError::missing_required_field(
                "approval_request.policy_refs",
            ));
        }
        if self.allowed_decisions.is_empty() {
            return Err(AgentError::missing_required_field(
                "approval_request.allowed_decisions",
            ));
        }
        if self.timeout_ms == 0 {
            return Err(AgentError::contract_violation(
                "approval_request.timeout_ms must be greater than zero",
            ));
        }
        Ok(())
    }

    pub fn subject_ref(&self) -> EntityRef {
        EntityRef::new(
            EntityKind::ApprovalRequest,
            self.approval_request_id.clone(),
        )
    }

    pub fn allows_decision(&self, decision: ApprovalDecisionKind) -> bool {
        self.allowed_decisions.contains(&decision)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approved {
        actor_ref: SourceRef,
    },
    ApprovedForSession {
        actor_ref: SourceRef,
    },
    Denied {
        reason_code: String,
        actor_ref: Option<SourceRef>,
    },
}

impl ApprovalDecision {
    pub fn approved(actor_id: impl Into<String>) -> Self {
        Self::Approved {
            actor_ref: SourceRef::new(actor_id),
        }
    }

    pub fn approved_for_session(actor_id: impl Into<String>) -> Self {
        Self::ApprovedForSession {
            actor_ref: SourceRef::new(actor_id),
        }
    }

    pub fn denied(reason_code: impl Into<String>) -> Self {
        Self::Denied {
            reason_code: reason_code.into(),
            actor_ref: None,
        }
    }

    pub fn kind(&self) -> ApprovalDecisionKind {
        match self {
            Self::Approved { .. } => ApprovalDecisionKind::Approved,
            Self::ApprovedForSession { .. } => ApprovalDecisionKind::ApprovedForSession,
            Self::Denied { .. } => ApprovalDecisionKind::Denied,
        }
    }

    pub fn from_finite_token(token: &str, actor_ref: SourceRef) -> Option<Self> {
        match token {
            "approved" => Some(Self::Approved { actor_ref }),
            "approved_for_session" => Some(Self::ApprovedForSession { actor_ref }),
            "denied" => Some(Self::Denied {
                reason_code: "approval.denied".to_string(),
                actor_ref: Some(actor_ref),
            }),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalLifecycleStatus {
    Requested,
    DispatchIntentRecorded,
    Dispatched,
    Approved,
    ApprovedForSession,
    Denied,
    TimedOut,
    Cancelled,
    DispatcherUnavailable,
    RecoveryRequired,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalTerminalStatus {
    Approved,
    ApprovedForSession,
    Denied,
    TimedOut,
    Cancelled,
    DispatcherUnavailable,
    RecoveryRequired,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApprovalBrokerOutcome {
    pub approval_request_id: ApprovalRequestId,
    pub status: ApprovalTerminalStatus,
    pub decision: Option<ApprovalDecision>,
    pub reason_code: String,
}

impl ApprovalBrokerOutcome {
    pub fn releases_tool_execution(&self) -> bool {
        matches!(
            self.status,
            ApprovalTerminalStatus::Approved | ApprovalTerminalStatus::ApprovedForSession
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "record_type", content = "record", rename_all = "snake_case")]
pub enum ApprovalRecord {
    Requested {
        request: ApprovalRequest,
    },
    DispatchIntent {
        request_id: ApprovalRequestId,
        effect_intent: EffectIntent,
    },
    DispatchResult {
        request_id: ApprovalRequestId,
        lifecycle_status: ApprovalLifecycleStatus,
        effect_result: EffectResult,
    },
    Responded {
        request_id: ApprovalRequestId,
        decision: ApprovalDecision,
    },
    Denied {
        request_id: ApprovalRequestId,
        reason_code: String,
    },
}

pub fn approval_dispatch_intent_record(
    base: JournalRecordBase,
    request: &ApprovalRequest,
) -> JournalRecord {
    let effect_intent = approval_dispatch_intent(request);
    approval_journal_record(
        base,
        request,
        "approval_dispatch_intent",
        effect_intent.content_refs.clone(),
        JournalRecordPayload::Approval(ApprovalRecord::DispatchIntent {
            request_id: request.approval_request_id.clone(),
            effect_intent,
        }),
    )
}

pub fn approval_dispatch_result_record(
    base: JournalRecordBase,
    request: &ApprovalRequest,
    lifecycle_status: ApprovalLifecycleStatus,
    status: EffectTerminalStatus,
    redacted_summary: impl Into<String>,
) -> JournalRecord {
    let effect_result = EffectResult {
        effect_id: request.approval_dispatch_effect_id.clone(),
        terminal_status: status,
        external_operation_id: None,
        reconciliation_ref: None,
        error_ref: None,
        content_refs: Vec::new(),
        redacted_summary: redacted_summary.into(),
    };
    approval_journal_record(
        base,
        request,
        "approval_dispatch_result",
        effect_result.content_refs.clone(),
        JournalRecordPayload::Approval(ApprovalRecord::DispatchResult {
            request_id: request.approval_request_id.clone(),
            lifecycle_status,
            effect_result,
        }),
    )
}

fn approval_dispatch_intent(request: &ApprovalRequest) -> EffectIntent {
    let mut intent = EffectIntent::new(
        request.approval_dispatch_effect_id.clone(),
        EffectKind::ApprovalDispatch,
        request.subject_ref(),
        request.source.clone(),
        format!(
            "approval dispatch for {}: {}",
            request.canonical_tool_name, request.redacted_args_summary
        ),
    );
    intent.destination = Some(request.destination.clone());
    intent.policy_refs = request.policy_refs.clone();
    intent.content_refs = vec![request.requested_args_ref.clone()];
    intent
}

fn approval_journal_record(
    base: JournalRecordBase,
    request: &ApprovalRequest,
    event_kind: &str,
    content_refs: Vec<ContentRef>,
    payload: JournalRecordPayload,
) -> JournalRecord {
    let subject_ref = request.subject_ref();
    let effect_ref = EntityRef::new(
        EntityKind::Effect,
        request.approval_dispatch_effect_id.clone(),
    );
    JournalRecord {
        journal_schema_version: JOURNAL_SCHEMA_VERSION,
        journal_seq: base.journal_seq,
        record_id: base.record_id,
        record_kind: JournalRecordKind::Approval,
        run_id: base.run_id.clone(),
        agent_id: base.agent_id.clone(),
        turn_id: base.turn_id.clone(),
        attempt_id: base.attempt_id.clone(),
        subject_ref: subject_ref.clone(),
        related_refs: vec![effect_ref],
        causal_refs: base.causal_refs,
        source: request.source.clone(),
        destination: Some(request.destination.clone()),
        correlation_keys: Vec::new(),
        tags: vec!["approval".to_string()],
        delivery_semantics: "journal_backed".to_string(),
        event_index: EventIndexProjection {
            run_id: base.run_id,
            agent_id: base.agent_id,
            turn_id: base.turn_id,
            event_family: "approval".to_string(),
            event_kind: event_kind.to_string(),
            source: request.source.clone(),
            destination: Some(request.destination.clone()),
            subject_ref,
            related_refs: Vec::new(),
            correlation_keys: Vec::new(),
            tags: vec!["approval".to_string()],
            privacy_class: base.privacy.clone(),
            delivery_semantics: "journal_backed".to_string(),
        },
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
