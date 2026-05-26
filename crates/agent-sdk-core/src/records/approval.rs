//! Durable and observable SDK records. Use these DTOs for events, journals, effects,
//! context, output, and feature evidence. Constructing records is data-only;
//! persistence, publication, and external actions happen through ports or application
//! coordinators. This file contains the approval portion of that contract.
//!
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        AgentError, AgentId, ApprovalRequestId, ContentRef, DestinationRef, EffectId, EntityKind,
        EntityRef, PolicyRef, RunId, SessionId, SourceRef, ToolCallId, TurnId,
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
/// Carries the approval request record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ApprovalRequest {
    /// Stable approval request id used for typed lineage, lookup, or dedupe.
    pub approval_request_id: ApprovalRequestId,
    /// Effect id that links approval dispatch intent, response, and tool-release evidence.
    /// Use it to prove the approval decision belongs to the side effect being released.
    pub approval_dispatch_effect_id: EffectId,
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional host-provided session identifier for grouping related turns.
    pub session_id: Option<SessionId>,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    /// Turn identifier for one loop turn within a run.
    pub turn_id: TurnId,
    /// Stable tool call id used for typed lineage, lookup, or dedupe.
    pub tool_call_id: ToolCallId,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: DestinationRef,
    /// Canonical tool name used by this record or request.
    pub canonical_tool_name: String,
    /// Tool source used by this record or request.
    pub tool_source: SourceRef,
    /// Classification value for effect class.
    /// Policy and projection paths use it for finite routing decisions.
    pub effect_class: EffectClass,
    /// Risk classification for the operation or capability.
    /// Policy uses it to decide whether approval, sandboxing, or denial is required.
    pub risk_class: RiskClass,
    /// Typed requested args ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub requested_args_ref: ContentRef,
    /// Redacted summary for display, logs, events, or telemetry.
    /// It should describe the value without exposing raw private content.
    pub redacted_args_summary: String,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Dispatcher scope used by this record or request.
    pub dispatcher_scope: DispatcherScope,
    /// Timeout budget in milliseconds for the requested operation.
    pub timeout_ms: u64,
    /// Allowlist for this policy or contract.
    /// Validation uses it to reject undeclared or policy-denied values.
    pub allowed_decisions: Vec<ApprovalDecisionKind>,
    /// Time value in milliseconds for created at millis.
    /// Use it for timeout, ordering, or diagnostic calculations.
    pub created_at_millis: u64,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: RuntimePackageFingerprint,
}

impl ApprovalRequest {
    /// Validates the records::approval invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
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

    /// Returns an updated records::approval value with subject ref applied.
    /// This is data construction only and does not execute the configured
    /// behavior.
    pub fn subject_ref(&self) -> EntityRef {
        EntityRef::new(
            EntityKind::ApprovalRequest,
            self.approval_request_id.clone(),
        )
    }

    /// Returns whether allows decision applies for this state.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn allows_decision(&self, decision: ApprovalDecisionKind) -> bool {
        self.allowed_decisions.contains(&decision)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
/// Enumerates the finite approval decision cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ApprovalDecision {
    /// Use this variant when the contract needs to represent approved; selecting it has no side effect by itself.
    Approved {
        /// Typed actor ref reference. Resolving or executing it is a separate
        /// policy-gated step.
        actor_ref: SourceRef,
    },
    /// Use this variant when the contract needs to represent approved for session; selecting it has no side effect by itself.
    ApprovedForSession {
        /// Typed actor ref reference. Resolving or executing it is a separate
        /// policy-gated step.
        actor_ref: SourceRef,
    },
    /// Use this variant when the contract needs to represent denied; selecting it has no side effect by itself.
    Denied {
        /// Stable reason code for unavailable or degraded host behavior.
        reason_code: String,
        /// Typed actor ref reference. Resolving or executing it is a separate
        /// policy-gated step.
        actor_ref: Option<SourceRef>,
    },
}

impl ApprovalDecision {
    /// Builds the approved record or result value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn approved(actor_id: impl Into<String>) -> Self {
        Self::Approved {
            actor_ref: SourceRef::new(actor_id),
        }
    }

    /// Builds the approved for session value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn approved_for_session(actor_id: impl Into<String>) -> Self {
        Self::ApprovedForSession {
            actor_ref: SourceRef::new(actor_id),
        }
    }

    /// Builds the denied record or result value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn denied(reason_code: impl Into<String>) -> Self {
        Self::Denied {
            reason_code: reason_code.into(),
            actor_ref: None,
        }
    }

    /// Builds the kind value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn kind(&self) -> ApprovalDecisionKind {
        match self {
            Self::Approved { .. } => ApprovalDecisionKind::Approved,
            Self::ApprovedForSession { .. } => ApprovalDecisionKind::ApprovedForSession,
            Self::Denied { .. } => ApprovalDecisionKind::Denied,
        }
    }

    /// Constructs this value from finite token. Use it when adapting
    /// canonical SDK records without introducing a second behavior
    /// path.
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
/// Enumerates the finite approval lifecycle status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ApprovalLifecycleStatus {
    /// Use this variant when the contract needs to represent requested; selecting it has no side effect by itself.
    Requested,
    /// Use this variant when the contract needs to represent dispatch intent recorded; selecting it has no side effect by itself.
    DispatchIntentRecorded,
    /// Use this variant when the contract needs to represent dispatched; selecting it has no side effect by itself.
    Dispatched,
    /// Use this variant when the contract needs to represent approved; selecting it has no side effect by itself.
    Approved,
    /// Use this variant when the contract needs to represent approved for session; selecting it has no side effect by itself.
    ApprovedForSession,
    /// Use this variant when the contract needs to represent denied; selecting it has no side effect by itself.
    Denied,
    /// Use this variant when the contract needs to represent timed out; selecting it has no side effect by itself.
    TimedOut,
    /// Use this variant when the contract needs to represent cancelled; selecting it has no side effect by itself.
    Cancelled,
    /// Use this variant when the contract needs to represent dispatcher unavailable; selecting it has no side effect by itself.
    DispatcherUnavailable,
    /// Use this variant when the contract needs to represent recovery required; selecting it has no side effect by itself.
    RecoveryRequired,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite approval terminal status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ApprovalTerminalStatus {
    /// Use this variant when the contract needs to represent approved; selecting it has no side effect by itself.
    Approved,
    /// Use this variant when the contract needs to represent approved for session; selecting it has no side effect by itself.
    ApprovedForSession,
    /// Use this variant when the contract needs to represent denied; selecting it has no side effect by itself.
    Denied,
    /// Use this variant when the contract needs to represent timed out; selecting it has no side effect by itself.
    TimedOut,
    /// Use this variant when the contract needs to represent cancelled; selecting it has no side effect by itself.
    Cancelled,
    /// Use this variant when the contract needs to represent dispatcher unavailable; selecting it has no side effect by itself.
    DispatcherUnavailable,
    /// Use this variant when the contract needs to represent recovery required; selecting it has no side effect by itself.
    RecoveryRequired,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the approval broker outcome record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ApprovalBrokerOutcome {
    /// Stable approval request id used for typed lineage, lookup, or dedupe.
    pub approval_request_id: ApprovalRequestId,
    /// Finite status for this record or lifecycle stage.
    pub status: ApprovalTerminalStatus,
    /// Optional decision value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub decision: Option<ApprovalDecision>,
    /// Stable reason code for unavailable or degraded host behavior.
    pub reason_code: String,
}

impl ApprovalBrokerOutcome {
    /// Returns whether releases tool execution applies for this contract.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn releases_tool_execution(&self) -> bool {
        matches!(
            self.status,
            ApprovalTerminalStatus::Approved | ApprovalTerminalStatus::ApprovedForSession
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "record_type", content = "record", rename_all = "snake_case")]
/// Enumerates the finite approval record cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ApprovalRecord {
    /// Use this variant when the contract needs to represent requested; selecting it has no side effect by itself.
    Requested {
        /// Request DTO or resolved call that triggered this operation.
        request: ApprovalRequest,
    },
    /// Use this variant when the contract needs to represent dispatch intent; selecting it has no side effect by itself.
    DispatchIntent {
        /// Stable request id used for typed lineage, lookup, or dedupe.
        request_id: ApprovalRequestId,
        /// Effect intent used by this record or request.
        effect_intent: EffectIntent,
    },
    /// Use this variant when the contract needs to represent dispatch result; selecting it has no side effect by itself.
    DispatchResult {
        /// Stable request id used for typed lineage, lookup, or dedupe.
        request_id: ApprovalRequestId,
        /// Lifecycle status used by this record or request.
        lifecycle_status: ApprovalLifecycleStatus,
        /// Effect result used by this record or request.
        effect_result: EffectResult,
    },
    /// Use this variant when the contract needs to represent responded; selecting it has no side effect by itself.
    Responded {
        /// Stable request id used for typed lineage, lookup, or dedupe.
        request_id: ApprovalRequestId,
        /// Decision used by this record or request.
        decision: ApprovalDecision,
    },
    /// Use this variant when the contract needs to represent denied; selecting it has no side effect by itself.
    Denied {
        /// Stable request id used for typed lineage, lookup, or dedupe.
        request_id: ApprovalRequestId,
        /// Stable reason code for unavailable or degraded host behavior.
        reason_code: String,
    },
}

/// Builds the approval dispatch intent record record for this contract.
/// This is data-only and does not perform I/O, call host ports, append journals, publish
/// events, or start processes.
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

/// Builds the approval dispatch result record record for this contract.
/// This is data-only and does not perform I/O, call host ports, append journals, publish
/// events, or start processes.
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
        session_id: base.session_id.clone(),
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
            session_id: base.session_id,
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
            privacy_class: base.privacy,
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
