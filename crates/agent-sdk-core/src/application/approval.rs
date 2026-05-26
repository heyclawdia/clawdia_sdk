//! Approval broker coordination. Use this module to turn policy decisions into
//! host-dispatched approval requests. Broker methods may call a dispatcher and record
//! cancellation or denial outcomes, but UI transport remains host-owned.
//!
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

use crate::{
    approval_ports::{ApprovalDispatchResponse, ApprovalDispatcher},
    approval_records::{
        ApprovalBrokerOutcome, ApprovalDecision, ApprovalLifecycleStatus, ApprovalRequest,
        ApprovalTerminalStatus, approval_dispatch_intent_record, approval_dispatch_result_record,
    },
    domain::{AgentError, AgentErrorKind, ApprovalRequestId, RetryClassification, SourceKind},
    effect::EffectTerminalStatus,
    journal::{JournalRecord, JournalRecordBase, PendingSideEffect, RecoveryMarker},
    journal_ports::RunJournal,
};

#[derive(Debug)]
/// Holds approval broker application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct ApprovalBroker {
    next_journal_seq: AtomicU64,
    cancelled_before_dispatch: Mutex<Vec<(ApprovalRequestId, String)>>,
}

impl Default for ApprovalBroker {
    fn default() -> Self {
        Self {
            next_journal_seq: AtomicU64::new(1),
            cancelled_before_dispatch: Mutex::new(Vec::new()),
        }
    }
}

impl ApprovalBroker {
    /// Coordinates cancel before dispatch for the application::approval
    /// contract. This may call configured ports and update
    /// runtime/journal/event state according to the surrounding module,
    /// without introducing a parallel behavior path.
    pub fn cancel_before_dispatch(
        &self,
        approval_request_id: ApprovalRequestId,
        reason: impl Into<String>,
    ) {
        self.cancelled_before_dispatch
            .lock()
            .expect("approval cancellation lock")
            .push((approval_request_id, reason.into()));
    }

    /// Coordinates request approval for the application::approval contract.
    /// This may call configured ports and update runtime/journal/event state
    /// according to the surrounding module, without introducing a parallel
    /// behavior path.
    pub fn request_approval(
        &self,
        request: ApprovalRequest,
        dispatcher: Option<&dyn ApprovalDispatcher>,
        journal: &dyn RunJournal,
    ) -> Result<ApprovalBrokerOutcome, AgentError> {
        request.validate()?;

        let intent_record =
            approval_dispatch_intent_record(self.base_for_request(&request, "intent"), &request);
        journal.append(intent_record).map_err(journal_failure)?;

        if self
            .take_cancelled_reason(&request.approval_request_id)
            .is_some()
        {
            return self.append_terminal_result(
                &request,
                journal,
                EffectTerminalStatus::Cancelled,
                ApprovalTerminalStatus::Cancelled,
                false,
                None,
                "approval.cancelled",
                "approval cancelled",
            );
        }

        let Some(dispatcher) = dispatcher else {
            return self.append_terminal_result(
                &request,
                journal,
                EffectTerminalStatus::Failed,
                ApprovalTerminalStatus::Denied,
                false,
                None,
                "missing.approval_dispatcher",
                "approval dispatcher unavailable",
            );
        };

        match dispatcher.dispatch(request.clone()) {
            Ok(ApprovalDispatchResponse::Decision(decision)) => {
                self.handle_decision(request, decision, journal)
            }
            Ok(ApprovalDispatchResponse::TimedOut) => self.append_terminal_result(
                &request,
                journal,
                EffectTerminalStatus::TimedOut,
                ApprovalTerminalStatus::TimedOut,
                true,
                None,
                "approval.timeout",
                "approval timed out",
            ),
            Ok(ApprovalDispatchResponse::Cancelled) => self.append_terminal_result(
                &request,
                journal,
                EffectTerminalStatus::Cancelled,
                ApprovalTerminalStatus::Cancelled,
                true,
                None,
                "approval.cancelled",
                "approval cancelled",
            ),
            Ok(ApprovalDispatchResponse::Unavailable { reason_code }) => self
                .append_terminal_result(
                    &request,
                    journal,
                    EffectTerminalStatus::Failed,
                    ApprovalTerminalStatus::DispatcherUnavailable,
                    true,
                    None,
                    reason_code,
                    "approval dispatcher unavailable",
                ),
            Err(error) => self.append_terminal_result(
                &request,
                journal,
                EffectTerminalStatus::Failed,
                ApprovalTerminalStatus::DispatcherUnavailable,
                true,
                None,
                "approval.dispatcher_error",
                format!("approval dispatcher failed: {}", error.context().message),
            ),
        }
    }

    fn handle_decision(
        &self,
        request: ApprovalRequest,
        decision: ApprovalDecision,
        journal: &dyn RunJournal,
    ) -> Result<ApprovalBrokerOutcome, AgentError> {
        if is_extension_self_response(&request, &decision) {
            return self.append_terminal_result(
                &request,
                journal,
                EffectTerminalStatus::DeniedBeforeExecution,
                ApprovalTerminalStatus::Denied,
                true,
                Some(ApprovalDecision::denied("approval.extension_self_response")),
                "approval.extension_self_response",
                "extension cannot approve its own action",
            );
        }

        if !request.allows_decision(decision.kind()) {
            return self.append_terminal_result(
                &request,
                journal,
                EffectTerminalStatus::DeniedBeforeExecution,
                ApprovalTerminalStatus::Denied,
                true,
                Some(ApprovalDecision::denied("approval.decision_not_allowed")),
                "approval.decision_not_allowed",
                "approval decision not allowed",
            );
        }

        match decision {
            ApprovalDecision::Approved { .. } => self.append_terminal_result(
                &request,
                journal,
                EffectTerminalStatus::Completed,
                ApprovalTerminalStatus::Approved,
                true,
                Some(decision),
                "approval.approved",
                "approval approved",
            ),
            ApprovalDecision::ApprovedForSession { .. } => self.append_terminal_result(
                &request,
                journal,
                EffectTerminalStatus::Completed,
                ApprovalTerminalStatus::ApprovedForSession,
                true,
                Some(decision),
                "approval.approved_for_session",
                "approval approved for session",
            ),
            ApprovalDecision::Denied { .. } => {
                let reason_code = match &decision {
                    ApprovalDecision::Denied { reason_code, .. } => reason_code.clone(),
                    _ => unreachable!("matched denied decision"),
                };
                self.append_terminal_result(
                    &request,
                    journal,
                    EffectTerminalStatus::DeniedBeforeExecution,
                    ApprovalTerminalStatus::Denied,
                    true,
                    Some(decision),
                    reason_code,
                    "approval denied",
                )
            }
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "approval terminal journaling needs explicit status, decision, and summary fields until this becomes a terminal-result command object"
    )]
    fn append_terminal_result(
        &self,
        request: &ApprovalRequest,
        journal: &dyn RunJournal,
        effect_status: EffectTerminalStatus,
        terminal_status: ApprovalTerminalStatus,
        dispatcher_contacted: bool,
        decision: Option<ApprovalDecision>,
        reason_code: impl Into<String>,
        redacted_summary: impl Into<String>,
    ) -> Result<ApprovalBrokerOutcome, AgentError> {
        let result_base = self.base_for_request(request, "result");
        let result_record = approval_dispatch_result_record(
            result_base.clone(),
            request,
            approval_lifecycle_status(&terminal_status),
            effect_status,
            redacted_summary,
        );
        if let Err(error) = journal.append(result_record) {
            if dispatcher_contacted {
                self.append_recovery_after_result_failure(request, journal, result_base, error)?;
            }
            return Err(AgentError::new(
                if dispatcher_contacted {
                    AgentErrorKind::RecoveryRepairNeeded
                } else {
                    AgentErrorKind::JournalFailure
                },
                RetryClassification::RepairNeeded,
                "approval dispatch terminal result append failed; tool execution remains blocked",
            )
            .with_subject(request.subject_ref()));
        }
        Ok(ApprovalBrokerOutcome {
            approval_request_id: request.approval_request_id.clone(),
            status: terminal_status,
            decision,
            reason_code: reason_code.into(),
        })
    }

    fn base_for_request(&self, request: &ApprovalRequest, suffix: &str) -> JournalRecordBase {
        let journal_seq = self.next_journal_seq.fetch_add(1, Ordering::SeqCst);
        let mut base = JournalRecordBase::new(
            journal_seq,
            format!(
                "journal.record.approval.{}.{}",
                request.approval_request_id.as_str(),
                suffix
            ),
            request.run_id.clone(),
            request.agent_id.clone(),
            request.source.clone(),
        );
        base.session_id = request.session_id.clone();
        base.turn_id = Some(request.turn_id.clone());
        base.destination = Some(request.destination.clone());
        base.timestamp_millis = request.created_at_millis + journal_seq;
        base.runtime_package_fingerprint = request.runtime_package_fingerprint.as_str().to_string();
        base
    }

    fn append_recovery_after_result_failure(
        &self,
        request: &ApprovalRequest,
        journal: &dyn RunJournal,
        mut base: JournalRecordBase,
        result_error: AgentError,
    ) -> Result<(), AgentError> {
        base.record_id = format!(
            "journal.record.approval.{}.recovery",
            request.approval_request_id.as_str()
        );
        let recovery = RecoveryMarker {
            unsafe_pending: vec![PendingSideEffect {
                effect_id: request.approval_dispatch_effect_id.clone(),
                intent_record_id: format!(
                    "journal.record.approval.{}.intent",
                    request.approval_request_id.as_str()
                ),
                idempotency_key: None,
                dedupe_key: None,
                unsafe_pending_reason: format!(
                    "approval dispatcher was contacted but terminal result append failed: {}",
                    result_error.context().message
                ),
            }],
            recovery_reason: "approval dispatch terminal result append failed".to_string(),
            policy_refs: request.policy_refs.clone(),
        };
        let recovery_record = JournalRecord::recovery(base, recovery);
        journal
            .append(recovery_record)
            .map(|_| ())
            .map_err(|error| {
                AgentError::new(
                    AgentErrorKind::RecoveryRepairNeeded,
                    RetryClassification::RepairNeeded,
                    format!(
                        "approval result append failed and recovery append failed: {}",
                        error.context().message
                    ),
                )
                .with_subject(request.subject_ref())
            })
    }

    fn take_cancelled_reason(&self, approval_request_id: &ApprovalRequestId) -> Option<String> {
        let mut cancelled = self
            .cancelled_before_dispatch
            .lock()
            .expect("approval cancellation lock");
        let index = cancelled
            .iter()
            .position(|(candidate, _)| candidate == approval_request_id)?;
        Some(cancelled.remove(index).1)
    }
}

fn approval_lifecycle_status(status: &ApprovalTerminalStatus) -> ApprovalLifecycleStatus {
    match status {
        ApprovalTerminalStatus::Approved => ApprovalLifecycleStatus::Approved,
        ApprovalTerminalStatus::ApprovedForSession => ApprovalLifecycleStatus::ApprovedForSession,
        ApprovalTerminalStatus::Denied => ApprovalLifecycleStatus::Denied,
        ApprovalTerminalStatus::TimedOut => ApprovalLifecycleStatus::TimedOut,
        ApprovalTerminalStatus::Cancelled => ApprovalLifecycleStatus::Cancelled,
        ApprovalTerminalStatus::DispatcherUnavailable => {
            ApprovalLifecycleStatus::DispatcherUnavailable
        }
        ApprovalTerminalStatus::RecoveryRequired => ApprovalLifecycleStatus::RecoveryRequired,
    }
}

fn journal_failure(error: AgentError) -> AgentError {
    AgentError::new(
        AgentErrorKind::JournalFailure,
        RetryClassification::RepairNeeded,
        error.context().message,
    )
}

fn is_extension_self_response(request: &ApprovalRequest, decision: &ApprovalDecision) -> bool {
    if request.source.kind != SourceKind::Extension {
        return false;
    }
    match decision {
        ApprovalDecision::Approved { actor_ref }
        | ApprovalDecision::ApprovedForSession { actor_ref } => actor_ref == &request.source,
        ApprovalDecision::Denied {
            actor_ref: Some(actor_ref),
            ..
        } => actor_ref == &request.source,
        ApprovalDecision::Denied {
            actor_ref: None, ..
        } => false,
    }
}
