//! Extension action coordination and protocol recovery. Use this module when
//! extension-declared capabilities resolve into policy-checked SDK actions. Execution
//! may call extension ports and approval brokers, but extension hosts remain outside
//! core.
//!
use crate::{
    approval_ports::ApprovalDispatcher,
    approval_records::{ApprovalBrokerOutcome, ApprovalRequest},
    domain::{
        AgentError, AgentErrorKind, AgentId, DestinationRef, EffectId, EntityKind, EntityRef,
        JournalCursor, PolicyRef, PrivacyClass, RetentionClass, RetryClassification, RunId,
        SourceRef, ToolCallId, TurnId,
    },
    effect::{EffectIntent, EffectKind},
    extension_ports::{
        ExtensionActionExecutionOutput, ExtensionActionExecutionRequest,
        ExtensionActionExecutorRegistry, ExtensionActionRegistrySnapshot, ExtensionActionRequest,
        ExtensionActionRoute, ExtensionProtocolError,
    },
    extension_records::{
        ExtensionActionEvent, ExtensionActionEventKind, ExtensionActionRecord,
        ExtensionActionRecordParams, ExtensionActionRecordStatus,
    },
    journal::{
        JournalRecord, JournalRecordBase, JournalRecordKind, JournalRecordPayload,
        PendingSideEffect, RecoveryMarker,
    },
    journal_ports::RunJournal,
    package_extension::ExtensionActionRequestId,
    policy::{ApprovalDecisionKind, DispatcherScope, PolicyOutcome, PolicyStage},
};

/// Holds extension action coordinator application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct ExtensionActionCoordinator {
    snapshot: ExtensionActionRegistrySnapshot,
    executors: ExtensionActionExecutorRegistry,
    approval_broker: Option<crate::approval::ApprovalBroker>,
}

impl ExtensionActionCoordinator {
    /// Creates a new application::extension value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(
        snapshot: ExtensionActionRegistrySnapshot,
        executors: ExtensionActionExecutorRegistry,
    ) -> Self {
        Self {
            snapshot,
            executors,
            approval_broker: None,
        }
    }

    /// Returns this value with its approval broker setting replaced.
    /// The method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_approval_broker(
        mut self,
        approval_broker: crate::approval::ApprovalBroker,
    ) -> Self {
        self.approval_broker = Some(approval_broker);
        self
    }

    /// Executes one extension action with approval and journal gating.
    /// This appends intent/result records through the supplied journal and then
    /// calls the configured extension bridge only after policy allows it.
    pub fn execute<J>(
        &self,
        journal: &J,
        request: ExtensionActionRequest,
        context: ExtensionActionContext,
        dispatcher: Option<&dyn ApprovalDispatcher>,
    ) -> Result<ExtensionActionOutcome, AgentError>
    where
        J: RunJournal,
    {
        if request.runtime_package_fingerprint != self.snapshot.runtime_package_fingerprint {
            let record = self.denied_record(
                request,
                &context,
                "extension.action.stale_runtime_package_fingerprint",
            );
            self.append_extension_action_record(journal, &context, 0, "denied", record.clone())?;
            return Ok(ExtensionActionOutcome::denied(
                record,
                context.runtime_package_fingerprint,
                None,
            ));
        }

        let Some(route) = self
            .snapshot
            .find(&request.extension_id, &request.action_id)
            .cloned()
        else {
            let record = self.denied_record(request, &context, "extension.action.not_declared");
            self.append_extension_action_record(journal, &context, 0, "denied", record.clone())?;
            return Ok(ExtensionActionOutcome::denied(
                record,
                context.runtime_package_fingerprint,
                None,
            ));
        };

        if route.sidecar.policy_refs.is_empty() {
            let record = self.denied_record(request, &context, "extension.action.missing_policy");
            self.append_extension_action_record(journal, &context, 0, "denied", record.clone())?;
            return Ok(ExtensionActionOutcome::denied(
                record,
                context.runtime_package_fingerprint,
                None,
            ));
        }

        let mut record = self.submitted_record(&request, &route, &context);
        let mut events = vec![ExtensionActionEvent::from_record(
            ExtensionActionEventKind::Submitted,
            &record,
            context.runtime_package_fingerprint.clone(),
            "extension action submitted",
        )];

        let mut journal_offset = 0;
        let mut approval_outcome = None;
        if route.sidecar.requires_approval {
            let Some(broker) = &self.approval_broker else {
                let record = record.with_denial("extension.action.missing_approval_broker");
                self.append_extension_action_record(
                    journal,
                    &context,
                    journal_offset,
                    "denied",
                    record.clone(),
                )?;
                return Ok(ExtensionActionOutcome::denied(
                    record,
                    context.runtime_package_fingerprint,
                    None,
                ));
            };
            let approval_request = approval_request_for_action(&request, &route, &context)?;
            let outcome = broker.request_approval(approval_request, dispatcher, journal)?;
            journal_offset = 2;
            if !outcome.releases_tool_execution() {
                let reason = outcome.reason_code.clone();
                let denied_record = record.with_denial(reason);
                approval_outcome = Some(outcome);
                self.append_extension_action_record(
                    journal,
                    &context,
                    journal_offset,
                    "denied",
                    denied_record.clone(),
                )?;
                return Ok(ExtensionActionOutcome::denied(
                    denied_record,
                    context.runtime_package_fingerprint,
                    approval_outcome,
                ));
            }
            approval_outcome = Some(outcome);
        }

        let Some(executor) = self.executors.get(&route.sidecar.bridge_ref) else {
            let record = record.with_denial("extension.action.missing_bridge_executor");
            self.append_extension_action_record(
                journal,
                &context,
                journal_offset,
                "denied",
                record.clone(),
            )?;
            return Ok(ExtensionActionOutcome::denied(
                record,
                context.runtime_package_fingerprint,
                approval_outcome,
            ));
        };

        self.append_extension_action_record(
            journal,
            &context,
            journal_offset,
            "submitted",
            record.clone(),
        )?;
        journal_offset += 1;

        let effect_id = effect_id_for_request(&request.request_id);
        let intent = effect_intent(effect_id.clone(), &request, &route);
        record = record.with_intent(intent.clone());
        events.push(ExtensionActionEvent::from_record(
            ExtensionActionEventKind::Started,
            &record,
            context.runtime_package_fingerprint.clone(),
            "extension action started",
        ));

        let intent_record = JournalRecord::effect_intent(
            context.record_base(
                journal_offset,
                "intent",
                Some(route.sidecar.destination.clone()),
            ),
            intent.clone(),
        );
        let intent_cursor = journal.append(intent_record).map_err(|error| {
            AgentError::new(
                AgentErrorKind::JournalFailure,
                RetryClassification::RepairNeeded,
                error.context().message,
            )
            .with_subject(record.subject_ref())
        })?;
        journal_offset += 1;

        self.append_extension_action_record(
            journal,
            &context,
            journal_offset,
            "started",
            record.clone(),
        )?;
        journal_offset += 1;

        let execution_request = ExtensionActionExecutionRequest {
            action_request: request.clone(),
            route: route.clone(),
            effect_intent: intent,
        };
        let output = match executor.execute(&execution_request) {
            Ok(output) => output,
            Err(error) => ExtensionActionExecutionOutput::failed(
                "extension action bridge failed before terminal envelope",
                format!("{:?}", error.kind()),
            ),
        };
        let result = output.to_effect_result(effect_id.clone());
        let terminal_record = JournalRecord::effect_result(
            context.record_base(
                journal_offset,
                "result",
                Some(route.sidecar.destination.clone()),
            ),
            result.clone(),
        );

        match journal.append(terminal_record) {
            Ok(cursor) => {
                journal_offset += 1;
                record = record.with_result(result);
                self.append_extension_action_record(
                    journal,
                    &context,
                    journal_offset,
                    "terminal",
                    record.clone(),
                )?;
                let terminal_kind =
                    if matches!(record.status, ExtensionActionRecordStatus::Completed) {
                        ExtensionActionEventKind::Completed
                    } else {
                        ExtensionActionEventKind::Failed
                    };
                events.push(ExtensionActionEvent::from_record(
                    terminal_kind,
                    &record,
                    context.runtime_package_fingerprint,
                    "extension action terminal result recorded",
                ));
                Ok(ExtensionActionOutcome {
                    status: ExtensionActionOutcomeStatus::from_record_status(&record.status),
                    record,
                    intent_cursor: Some(intent_cursor),
                    terminal_cursor: Some(cursor),
                    approval_outcome,
                    events,
                    recovery_required: false,
                })
            }
            Err(result_error) => {
                let unsafe_pending_reason = format!(
                    "extension action terminal result append failed: {}",
                    result_error.context().message
                );
                let recovery = RecoveryMarker {
                    unsafe_pending: vec![PendingSideEffect {
                        effect_id,
                        intent_record_id: context.record_id("intent"),
                        idempotency_key: request.idempotency_key.clone(),
                        dedupe_key: request.dedupe_key.clone(),
                        unsafe_pending_reason: unsafe_pending_reason.clone(),
                    }],
                    recovery_reason: unsafe_pending_reason.clone(),
                    policy_refs: route.sidecar.policy_refs.clone(),
                };
                let recovery_record = JournalRecord::recovery(
                    context.record_base(
                        journal_offset,
                        "recovery",
                        Some(route.sidecar.destination),
                    ),
                    recovery,
                );
                let cursor = journal.append(recovery_record).map_err(|recovery_error| {
                    AgentError::new(
                        AgentErrorKind::RecoveryRepairNeeded,
                        RetryClassification::RepairNeeded,
                        format!(
                            "extension action result append failed and recovery append failed: {}; recovery: {}",
                            result_error.context().message,
                            recovery_error.context().message
                        ),
                    )
                    .with_subject(record.subject_ref())
                })?;
                record = record.with_recovery_required(result, unsafe_pending_reason);
                self.append_extension_action_record(
                    journal,
                    &context,
                    journal_offset + 1,
                    "terminal",
                    record.clone(),
                )?;
                events.push(ExtensionActionEvent::from_record(
                    ExtensionActionEventKind::Failed,
                    &record,
                    context.runtime_package_fingerprint,
                    "extension action recovery required",
                ));
                Ok(ExtensionActionOutcome {
                    status: ExtensionActionOutcomeStatus::RecoveryRequired,
                    record,
                    intent_cursor: Some(intent_cursor),
                    terminal_cursor: Some(cursor),
                    approval_outcome,
                    events,
                    recovery_required: true,
                })
            }
        }
    }

    fn submitted_record(
        &self,
        request: &ExtensionActionRequest,
        route: &ExtensionActionRoute,
        context: &ExtensionActionContext,
    ) -> ExtensionActionRecord {
        ExtensionActionRecord::submitted(ExtensionActionRecordParams {
            request_id: request.request_id.clone(),
            run_id: context.run_id.clone(),
            turn_id: context.turn_id.clone(),
            action_ref: route.action_ref.clone(),
            action_kind: route.sidecar.action_kind.clone(),
            source: request.source.clone(),
            destination: route.sidecar.destination.clone(),
            policy_refs: route.sidecar.policy_refs.clone(),
            risk_class: route.sidecar.risk_class.clone(),
            privacy: PrivacyClass::ContentRefsOnly,
            retention: RetentionClass::RunScoped,
            input_refs: request.input_refs.clone(),
            redacted_input_summary: request.redacted_input_summary.clone(),
            idempotency_key: request.idempotency_key.clone(),
        })
    }

    fn denied_record(
        &self,
        request: ExtensionActionRequest,
        context: &ExtensionActionContext,
        reason: impl Into<String>,
    ) -> ExtensionActionRecord {
        let action_ref = crate::package_extension::ExtensionActionRef {
            extension_id: request.extension_id.clone(),
            action_id: request.action_id.clone(),
            capability_id: crate::capability::CapabilityId::new(format!(
                "cap.{}.{}",
                request.extension_id.as_str(),
                request.action_id.as_str()
            )),
        };
        ExtensionActionRecord::submitted(ExtensionActionRecordParams {
            request_id: request.request_id,
            run_id: context.run_id.clone(),
            turn_id: context.turn_id.clone(),
            action_ref,
            action_kind: crate::package_extension::ExtensionActionKind::HostAction,
            source: request.source,
            destination: DestinationRef::with_kind(
                crate::domain::DestinationKind::Host,
                "destination.extension.action.unresolved",
            ),
            policy_refs: Vec::new(),
            risk_class: crate::policy::RiskClass::High,
            privacy: PrivacyClass::ContentRefsOnly,
            retention: RetentionClass::RunScoped,
            input_refs: request.input_refs,
            redacted_input_summary: request.redacted_input_summary,
            idempotency_key: request.idempotency_key,
        })
        .with_denial(reason)
    }

    fn append_extension_action_record<J>(
        &self,
        journal: &J,
        context: &ExtensionActionContext,
        offset: u64,
        suffix: &str,
        record: ExtensionActionRecord,
    ) -> Result<JournalCursor, AgentError>
    where
        J: RunJournal,
    {
        let event_kind = match record.status {
            ExtensionActionRecordStatus::Submitted => "extension_action_submitted",
            ExtensionActionRecordStatus::Started => "extension_action_started",
            ExtensionActionRecordStatus::Completed => "extension_action_completed",
            ExtensionActionRecordStatus::Denied => "extension_action_denied",
            ExtensionActionRecordStatus::Failed
            | ExtensionActionRecordStatus::TimedOut
            | ExtensionActionRecordStatus::Cancelled
            | ExtensionActionRecordStatus::Unknown
            | ExtensionActionRecordStatus::RecoveryRequired => "extension_action_failed",
        };
        let base = context.record_base(offset, suffix, Some(record.destination.clone()));
        journal
            .append(JournalRecord::feature_record(
                base,
                JournalRecordKind::ExtensionAction,
                "extension",
                event_kind,
                record.subject_ref(),
                Vec::new(),
                record.input_refs.clone(),
                JournalRecordPayload::ExtensionAction(record),
            ))
            .map_err(|error| {
                AgentError::new(
                    AgentErrorKind::JournalFailure,
                    RetryClassification::RepairNeeded,
                    error.context().message,
                )
            })
    }
}

#[derive(Clone, Debug)]
/// Holds extension action context application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct ExtensionActionContext {
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    /// Turn identifier for one loop turn within a run.
    pub turn_id: Option<TurnId>,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
    /// Next journal seq used by this record or request.
    pub next_journal_seq: u64,
    /// Timestamp in milliseconds associated with this record.
    /// Use it for ordering and diagnostics; durable causality still comes from ids and cursors.
    pub timestamp_millis: u64,
    /// Record id prefix used by this record or request.
    pub record_id_prefix: String,
    /// Stable redaction policy id used for typed lineage, lookup, or dedupe.
    pub redaction_policy_id: String,
}

impl ExtensionActionContext {
    fn record_id(&self, suffix: &str) -> String {
        format!("{}.{}", self.record_id_prefix, suffix)
    }

    fn record_base(
        &self,
        offset: u64,
        suffix: &str,
        destination: Option<DestinationRef>,
    ) -> JournalRecordBase {
        let mut base = JournalRecordBase::new(
            self.next_journal_seq + offset,
            self.record_id(suffix),
            self.run_id.clone(),
            self.agent_id.clone(),
            SourceRef::with_kind(
                crate::domain::SourceKind::Extension,
                "source.extension.action",
            ),
        );
        base.turn_id = self.turn_id.clone();
        base.destination = destination;
        base.timestamp_millis = self.timestamp_millis + offset;
        base.runtime_package_fingerprint = self.runtime_package_fingerprint.clone();
        base.privacy = PrivacyClass::ContentRefsOnly;
        base.redaction_policy_id = self.redaction_policy_id.clone();
        base.tags = vec!["extension_action".to_string()];
        base
    }
}

#[derive(Clone, Debug)]
/// Holds extension action outcome application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct ExtensionActionOutcome {
    /// Finite status for this record or lifecycle stage.
    pub status: ExtensionActionOutcomeStatus,
    /// Record used by this record or request.
    pub record: ExtensionActionRecord,
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub intent_cursor: Option<JournalCursor>,
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub terminal_cursor: Option<JournalCursor>,
    /// Optional approval outcome value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub approval_outcome: Option<ApprovalBrokerOutcome>,
    /// Bounded events included in this record. Limits and truncation are
    /// represented by companion metadata when applicable.
    pub events: Vec<ExtensionActionEvent>,
    /// Whether recovery required is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub recovery_required: bool,
}

impl ExtensionActionOutcome {
    fn denied(
        record: ExtensionActionRecord,
        runtime_package_fingerprint: String,
        approval_outcome: Option<ApprovalBrokerOutcome>,
    ) -> Self {
        let event = ExtensionActionEvent::from_record(
            ExtensionActionEventKind::Denied,
            &record,
            runtime_package_fingerprint,
            record
                .denied_reason
                .clone()
                .unwrap_or_else(|| "extension action denied".to_string()),
        );
        Self {
            status: ExtensionActionOutcomeStatus::Denied,
            record,
            intent_cursor: None,
            terminal_cursor: None,
            approval_outcome,
            events: vec![event],
            recovery_required: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Enumerates the finite extension action outcome status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ExtensionActionOutcomeStatus {
    /// Use this variant when the contract needs to represent completed; selecting it has no side effect by itself.
    Completed,
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed,
    /// Use this variant when the contract needs to represent denied; selecting it has no side effect by itself.
    Denied,
    /// Use this variant when the contract needs to represent recovery required; selecting it has no side effect by itself.
    RecoveryRequired,
}

impl ExtensionActionOutcomeStatus {
    fn from_record_status(status: &ExtensionActionRecordStatus) -> Self {
        match status {
            ExtensionActionRecordStatus::Completed => Self::Completed,
            ExtensionActionRecordStatus::RecoveryRequired => Self::RecoveryRequired,
            ExtensionActionRecordStatus::Denied => Self::Denied,
            _ => Self::Failed,
        }
    }
}

#[derive(Clone, Debug)]
/// Holds extension protocol recovery context application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct ExtensionProtocolRecoveryContext {
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
    /// Next journal seq used by this record or request.
    pub next_journal_seq: u64,
    /// Timestamp in milliseconds associated with this record.
    /// Use it for ordering and diagnostics; durable causality still comes from ids and cursors.
    pub timestamp_millis: u64,
    /// Stable record id used for typed lineage, lookup, or dedupe.
    pub record_id: String,
    /// Stable redaction policy id used for typed lineage, lookup, or dedupe.
    pub redaction_policy_id: String,
}

#[derive(Clone, Debug)]
/// Holds extension protocol recovery outcome application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct ExtensionProtocolRecoveryOutcome {
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub cursor: JournalCursor,
}

/// Recover extension protocol error.
/// This appends protocol-recovery evidence through the journal so extension transport errors
/// can be reconciled without executing another action.
pub fn recover_extension_protocol_error<J>(
    journal: &J,
    error: ExtensionProtocolError,
    context: ExtensionProtocolRecoveryContext,
    policy_refs: Vec<PolicyRef>,
) -> Result<ExtensionProtocolRecoveryOutcome, AgentError>
where
    J: RunJournal + ?Sized,
{
    let mut base = JournalRecordBase::new(
        context.next_journal_seq,
        context.record_id,
        context.run_id,
        context.agent_id,
        context.source,
    );
    base.timestamp_millis = context.timestamp_millis;
    base.runtime_package_fingerprint = context.runtime_package_fingerprint;
    base.privacy = PrivacyClass::ContentRefsOnly;
    base.redaction_policy_id = context.redaction_policy_id;
    base.tags = vec!["extension_protocol".to_string()];

    let recovery = RecoveryMarker {
        unsafe_pending: Vec::new(),
        recovery_reason: error.redacted_summary,
        policy_refs,
    };
    let cursor = journal
        .append(JournalRecord::recovery(base, recovery))
        .map_err(|append_error| {
            AgentError::new(
                AgentErrorKind::JournalFailure,
                RetryClassification::RepairNeeded,
                append_error.context().message,
            )
        })?;
    Ok(ExtensionProtocolRecoveryOutcome { cursor })
}

fn approval_request_for_action(
    request: &ExtensionActionRequest,
    route: &ExtensionActionRoute,
    context: &ExtensionActionContext,
) -> Result<ApprovalRequest, AgentError> {
    let requested_args_ref =
        request.input_refs.first().cloned().ok_or_else(|| {
            AgentError::missing_required_field("extension_action_request.input_refs")
        })?;
    Ok(ApprovalRequest {
        approval_request_id: crate::domain::ApprovalRequestId::new(format!(
            "approval.{}",
            request.request_id.as_str()
        )),
        approval_dispatch_effect_id: EffectId::new(format!(
            "effect.approval.{}",
            request.request_id.as_str()
        )),
        run_id: context.run_id.clone(),
        agent_id: context.agent_id.clone(),
        turn_id: context
            .turn_id
            .clone()
            .unwrap_or_else(|| TurnId::new("turn.extension.action")),
        tool_call_id: ToolCallId::new(format!("tool.call.{}", request.request_id.as_str())),
        source: request.source.clone(),
        destination: route.sidecar.destination.clone(),
        canonical_tool_name: format!("extension.action.{}", request.action_id.as_str()),
        tool_source: route.sidecar.source_ref.clone(),
        effect_class: route.sidecar.action_kind.effect_class(),
        risk_class: route.sidecar.risk_class.clone(),
        requested_args_ref,
        redacted_args_summary: request.redacted_input_summary.clone(),
        policy_refs: vec![route.sidecar.approval_policy_ref.clone()],
        dispatcher_scope: DispatcherScope::SourceScoped,
        timeout_ms: 120_000,
        allowed_decisions: vec![ApprovalDecisionKind::Approved, ApprovalDecisionKind::Denied],
        created_at_millis: context.timestamp_millis,
        runtime_package_fingerprint: crate::package::RuntimePackageFingerprint(
            context.runtime_package_fingerprint.clone(),
        ),
    })
}

fn effect_id_for_request(request_id: &ExtensionActionRequestId) -> EffectId {
    EffectId::new(format!("effect.{}", request_id.as_str()))
}

fn effect_intent(
    effect_id: EffectId,
    request: &ExtensionActionRequest,
    route: &ExtensionActionRoute,
) -> EffectIntent {
    let mut policy_refs = route.sidecar.policy_refs.clone();
    policy_refs.push(route.sidecar.approval_policy_ref.clone());
    policy_refs.sort_by_key(|policy| policy.as_str().to_string());
    policy_refs.dedup_by(|left, right| left.as_str() == right.as_str());
    let mut intent = EffectIntent::new(
        effect_id,
        EffectKind::ExtensionAction,
        EntityRef::new(
            EntityKind::ExtensionAction,
            route.action_ref.subject_id().as_str(),
        ),
        request.source.clone(),
        format!("extension action {}", route.action_ref.action_id.as_str()),
    );
    intent.destination = Some(route.sidecar.destination.clone());
    intent.policy_refs = policy_refs;
    intent.idempotency_key = request.idempotency_key.clone();
    intent.dedupe_key = request.dedupe_key.clone();
    intent.content_refs = request.input_refs.clone();
    intent
}

fn _policy_outcome_for_denial(policy_refs: Vec<PolicyRef>) -> PolicyOutcome {
    PolicyOutcome {
        stage: PolicyStage::PreTool,
        decision: crate::policy::PolicyDecision::deny("extension.action.denied"),
        subject: None,
        source: None,
        destination: None,
        policy_refs,
        privacy: PrivacyClass::Internal,
        retention: RetentionClass::RunScoped,
    }
}
