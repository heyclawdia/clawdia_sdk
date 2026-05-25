//! Output delivery coordination over destination refs and sinks. Use this module to
//! send final or streaming output through host-provided sinks with dedupe and journal
//! evidence. Dispatch may call sinks and append delivery intent/result records.
//!
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use crate::{
    domain::{
        AgentError, AgentErrorKind, AgentId, AttemptId, ContentRef, DestinationRef, EffectId,
        IdempotencyKey, MessageId, PolicyRef, PrivacyClass, RetentionClass, RetryClassification,
        RunId, SourceKind, SourceRef, TurnId, ValidatedOutputId,
    },
    effect::EffectKind,
    journal_ports::RunJournal,
    output_delivery::{
        OutputContentMode, OutputDeliveryDedupeRecord, OutputDeliveryId,
        OutputDeliveryIntentRecord, OutputDeliveryJournalBase, OutputDeliveryKind,
        OutputDeliveryPolicy, OutputDeliveryReceipt, OutputDeliveryReconciliationRecord,
        OutputDeliveryRequest, OutputDeliveryRequirement, OutputDeliveryResultRecord,
        OutputDispatchStatus, OutputSinkRef, ReplayRepairDecision, TerminalAppendStatus,
        build_output_delivery_dedupe_key,
    },
    output_delivery_port::{OutputSinkCapabilities, OutputSinkRegistry},
    package::RuntimePackageFingerprint,
};

#[derive(Clone)]
/// Holds output delivery service application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct OutputDeliveryService {
    journal: Arc<dyn RunJournal>,
    sinks: OutputSinkRegistry,
    dedupe_index: OutputDeliveryDedupeIndex,
    next_seq: Arc<AtomicU64>,
}

impl OutputDeliveryService {
    /// Creates a new application::output_delivery value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(journal: Arc<dyn RunJournal>, sinks: OutputSinkRegistry) -> Self {
        Self {
            journal,
            sinks,
            dedupe_index: OutputDeliveryDedupeIndex::default(),
            next_seq: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Returns this value with its dedupe index setting replaced. The
    /// method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_dedupe_index(mut self, dedupe_index: OutputDeliveryDedupeIndex) -> Self {
        self.dedupe_index = dedupe_index;
        self
    }

    /// Coordinates dispatch for the application::output_delivery contract.
    /// This may call configured ports and update runtime/journal/event state
    /// according to the surrounding module, without introducing a parallel
    /// behavior path.
    pub fn dispatch(
        &self,
        context: OutputDeliveryContext,
        candidate: OutputDeliveryCandidate,
    ) -> Result<OutputDeliveryOutcome, AgentError> {
        if candidate.policy.requirement == OutputDeliveryRequirement::Disabled {
            return Ok(OutputDeliveryOutcome::skipped(
                OutputDispatchStatus::SkippedOptional,
                "output delivery disabled by policy",
            ));
        }

        let sink_ref = match self.resolve_sink_ref(&candidate) {
            Ok(sink_ref) => sink_ref,
            Err(error) => {
                if candidate.policy.requirement == OutputDeliveryRequirement::Optional {
                    return Ok(OutputDeliveryOutcome::skipped(
                        OutputDispatchStatus::SkippedOptional,
                        "optional output delivery skipped because no sink was selected",
                    ));
                }
                return Err(error);
            }
        };

        let delivery_id = OutputDeliveryId::new(format!(
            "output.delivery.{}.{}",
            stable_fragment(context.run_id.as_str()),
            candidate.delivery_kind.dedupe_fragment().replace(':', ".")
        ));
        let effect_id = EffectId::new(format!("effect.{}", delivery_id.as_str()));
        let mut request = OutputDeliveryRequest {
            delivery_id: delivery_id.clone(),
            effect_id,
            run_id: context.run_id.clone(),
            agent_id: context.agent_id.clone(),
            turn_id: context.turn_id.clone(),
            attempt_id: context.attempt_id.clone(),
            source_message_id: candidate.source_message_id.clone(),
            validated_output_id: candidate.validated_output_id.clone(),
            destination: candidate.destination.clone(),
            sink_ref: sink_ref.clone(),
            delivery_kind: candidate.delivery_kind.clone(),
            content_mode: candidate
                .requested_content_mode
                .unwrap_or(candidate.policy.default_content_mode),
            content_refs: candidate.content_refs.clone(),
            redacted_summary: candidate.redacted_summary.clone(),
            raw_content: candidate.raw_content.clone(),
            privacy: candidate.privacy.clone(),
            retention: candidate.retention.clone(),
            policy_refs: candidate.policy.policy_refs(),
            idempotency_key: Some(IdempotencyKey::new(format!(
                "idempotency.{}",
                delivery_id.as_str()
            ))),
            dedupe_key: crate::domain::DedupeKey::new("dedupe.output_delivery.pending"),
            runtime_package_fingerprint: context.runtime_package_fingerprint.clone(),
        };
        request.dedupe_key = build_output_delivery_dedupe_key(&request);

        if let Some(proof) = self.dedupe_index.completed(&request.dedupe_key)? {
            let dedupe_record = OutputDeliveryDedupeRecord {
                delivery_id,
                dedupe_key: request.dedupe_key.clone(),
                prior_delivery_id: Some(proof.delivery_id),
                prior_external_operation_id: proof.external_operation_id,
                prior_terminal_status: proof.status,
                current_status: OutputDispatchStatus::Deduped,
                redacted_summary: "output delivery skipped by completed dedupe proof".to_string(),
                policy_refs: request.policy_refs.clone(),
            };
            self.journal
                .append(dedupe_record.to_journal_record(
                    self.journal_base(
                        &context,
                        &request.destination,
                        format!("journal.{}.dedupe", request.delivery_id.as_str()),
                    ),
                    request.destination.clone(),
                ))
                .map_err(journal_failure)?;
            return Ok(OutputDeliveryOutcome {
                status: OutputDispatchStatus::Deduped,
                request: Some(request.clone()),
                intent_record: None,
                result_record: None,
                dedupe_record: Some(dedupe_record),
                reconciliation_record: None,
                receipt: None,
                terminal_error: None,
            });
        }

        let Some(sink) = self.sinks.get(&sink_ref) else {
            if candidate.policy.requirement == OutputDeliveryRequirement::Optional {
                return Ok(OutputDeliveryOutcome::skipped(
                    OutputDispatchStatus::SkippedOptional,
                    "optional output delivery skipped because matching sink is missing",
                ));
            }
            return self.append_host_configuration_needed(
                context,
                request,
                "required output sink is missing",
            );
        };

        let capabilities = sink.capabilities();
        match resolve_content_mode(&candidate, &capabilities, &sink_ref) {
            Ok(content_mode) => {
                request.content_mode = content_mode;
                if content_mode != OutputContentMode::RawContentIfPolicyAllows {
                    request.raw_content = None;
                }
            }
            Err(error) => {
                if candidate.policy.requirement == OutputDeliveryRequirement::Optional {
                    return Ok(OutputDeliveryOutcome::skipped(
                        OutputDispatchStatus::SkippedOptional,
                        "optional output delivery skipped by sink capability or content policy",
                    ));
                }
                return self.append_host_configuration_needed(context, request, error);
            }
        }

        if !capabilities.supports_kind(&request.delivery_kind) {
            if candidate.policy.requirement == OutputDeliveryRequirement::Optional {
                return Ok(OutputDeliveryOutcome::skipped(
                    OutputDispatchStatus::SkippedOptional,
                    "optional output delivery skipped because sink cannot send this delivery kind",
                ));
            }
            return self.append_host_configuration_needed(
                context,
                request,
                "required output sink lacks delivery-kind capability",
            );
        }

        let intent = OutputDeliveryIntentRecord::from_request(&request);
        let intent_record_id = format!("journal.{}.intent", request.delivery_id.as_str());
        let intent_journal = intent.to_journal_record(self.journal_base(
            &context,
            &request.destination,
            intent_record_id.clone(),
        ));
        self.journal
            .append(intent_journal)
            .map_err(journal_failure)?;

        let sink_result = if request.delivery_kind.is_chunk() {
            sink.send_chunk(request.clone())
        } else {
            sink.send_final(request.clone())
        };

        match sink_result {
            Ok(receipt) if receipt.status == OutputDispatchStatus::Completed => {
                let result = OutputDeliveryResultRecord::completed(&request, &receipt);
                let result_base = self.journal_base(
                    &context,
                    &request.destination,
                    format!("journal.{}.result", request.delivery_id.as_str()),
                );
                if let Err(error) = self
                    .journal
                    .append(result.to_journal_record(result_base.clone()))
                {
                    return self.output_reconciliation_after_append_failure(
                        context,
                        request,
                        intent,
                        result,
                        Some(receipt),
                        intent_record_id,
                        result_base,
                        error,
                    );
                }
                self.dedupe_index.insert_completed(OutputDedupeProof {
                    dedupe_key: request.dedupe_key.clone(),
                    delivery_id: request.delivery_id.clone(),
                    external_operation_id: receipt.external_operation_id.clone(),
                    status: OutputDispatchStatus::Completed,
                })?;
                Ok(OutputDeliveryOutcome {
                    status: OutputDispatchStatus::Completed,
                    request: Some(request),
                    intent_record: Some(intent),
                    result_record: Some(result),
                    dedupe_record: None,
                    reconciliation_record: None,
                    receipt: Some(receipt),
                    terminal_error: None,
                })
            }
            Ok(receipt) => {
                let result = OutputDeliveryResultRecord::reconciliation_needed(&request, &receipt);
                let result_base = self.journal_base(
                    &context,
                    &request.destination,
                    format!("journal.{}.result", request.delivery_id.as_str()),
                );
                if let Err(error) = self
                    .journal
                    .append(result.to_journal_record(result_base.clone()))
                {
                    return self.output_reconciliation_after_append_failure(
                        context,
                        request,
                        intent,
                        result,
                        Some(receipt),
                        intent_record_id,
                        result_base,
                        error,
                    );
                }
                let reconciliation = OutputDeliveryReconciliationRecord {
                    delivery_id: request.delivery_id.clone(),
                    intent_record_id,
                    side_effect_kind: EffectKind::OutputDelivery,
                    idempotency_key: request.idempotency_key.clone(),
                    dedupe_key: request.dedupe_key.clone(),
                    external_operation_id: receipt.external_operation_id.clone(),
                    terminal_status: OutputDispatchStatus::ReconciliationNeeded,
                    terminal_append_status: TerminalAppendStatus::Appended,
                    reconciliation_adapter: Some(request.sink_ref.clone()),
                    unsafe_pending_reason: "sink returned unknown delivery outcome".to_string(),
                    replay_decision: ReplayRepairDecision::RequiresHostReconciliation,
                    resend_allowed: false,
                };
                Ok(OutputDeliveryOutcome {
                    status: OutputDispatchStatus::ReconciliationNeeded,
                    request: Some(request),
                    intent_record: Some(intent),
                    result_record: Some(result),
                    dedupe_record: None,
                    reconciliation_record: Some(reconciliation),
                    receipt: Some(receipt),
                    terminal_error: None,
                })
            }
            Err(error) => {
                let result = OutputDeliveryResultRecord::failed(
                    &request,
                    OutputDispatchStatus::Failed,
                    error.context().message,
                    error.retry(),
                );
                let result_base = self.journal_base(
                    &context,
                    &request.destination,
                    format!("journal.{}.result", request.delivery_id.as_str()),
                );
                if let Err(append_error) = self
                    .journal
                    .append(result.to_journal_record(result_base.clone()))
                {
                    return self.output_reconciliation_after_append_failure(
                        context,
                        request,
                        intent,
                        result,
                        None,
                        intent_record_id,
                        result_base,
                        append_error,
                    );
                }
                Ok(OutputDeliveryOutcome {
                    status: OutputDispatchStatus::Failed,
                    request: Some(request),
                    intent_record: Some(intent),
                    result_record: Some(result),
                    dedupe_record: None,
                    reconciliation_record: None,
                    receipt: None,
                    terminal_error: Some(error),
                })
            }
        }
    }

    /// Operates on in-memory or journal-derived application::output_delivery
    /// state for diagnostics and repair evidence. It does not create a second
    /// run loop or product workflow owner.
    pub fn repair_replay(
        &self,
        intent: &OutputDeliveryIntentRecord,
        terminal_result: Option<&OutputDeliveryResultRecord>,
    ) -> Result<OutputDeliveryReconciliationRecord, AgentError> {
        if let Some(result) = terminal_result {
            return Ok(OutputDeliveryReconciliationRecord {
                delivery_id: intent.delivery_id.clone(),
                intent_record_id: "journal.output_delivery.intent.replay".to_string(),
                side_effect_kind: EffectKind::OutputDelivery,
                idempotency_key: intent.idempotency_key.clone(),
                dedupe_key: intent.dedupe_key.clone(),
                external_operation_id: result.external_operation_id.clone(),
                terminal_status: result.dispatch_status,
                terminal_append_status: TerminalAppendStatus::Appended,
                reconciliation_adapter: Some(intent.sink_ref.clone()),
                unsafe_pending_reason: "terminal output delivery result already journaled"
                    .to_string(),
                replay_decision: ReplayRepairDecision::CompletedByDedupeProof,
                resend_allowed: false,
            });
        }

        if let Some(proof) = self.dedupe_index.completed(&intent.dedupe_key)? {
            return Ok(OutputDeliveryReconciliationRecord {
                delivery_id: intent.delivery_id.clone(),
                intent_record_id: "journal.output_delivery.intent.replay".to_string(),
                side_effect_kind: EffectKind::OutputDelivery,
                idempotency_key: intent.idempotency_key.clone(),
                dedupe_key: intent.dedupe_key.clone(),
                external_operation_id: proof.external_operation_id,
                terminal_status: proof.status,
                terminal_append_status: TerminalAppendStatus::NotAttempted,
                reconciliation_adapter: Some(intent.sink_ref.clone()),
                unsafe_pending_reason: "repair replay found completed dedupe proof".to_string(),
                replay_decision: ReplayRepairDecision::CompletedByDedupeProof,
                resend_allowed: false,
            });
        }

        Ok(OutputDeliveryReconciliationRecord {
            delivery_id: intent.delivery_id.clone(),
            intent_record_id: "journal.output_delivery.intent.replay".to_string(),
            side_effect_kind: EffectKind::OutputDelivery,
            idempotency_key: intent.idempotency_key.clone(),
            dedupe_key: intent.dedupe_key.clone(),
            external_operation_id: None,
            terminal_status: OutputDispatchStatus::ReconciliationNeeded,
            terminal_append_status: TerminalAppendStatus::NotAttempted,
            reconciliation_adapter: Some(intent.sink_ref.clone()),
            unsafe_pending_reason:
                "repair replay cannot resend output delivery without completed dedupe proof"
                    .to_string(),
            replay_decision: ReplayRepairDecision::UnsafePending,
            resend_allowed: false,
        })
    }

    fn resolve_sink_ref(
        &self,
        candidate: &OutputDeliveryCandidate,
    ) -> Result<OutputSinkRef, AgentError> {
        if let Some(sink_ref) = &candidate.policy.required_sink_ref {
            return Ok(sink_ref.clone());
        }
        if let Some(sink_ref) = &candidate.preferred_sink_ref {
            return Ok(sink_ref.clone());
        }
        self.sinks
            .first()
            .map(|sink| sink.sink_ref())
            .ok_or_else(|| {
                AgentError::new(
                    AgentErrorKind::HostConfigurationNeeded,
                    RetryClassification::HostConfigurationNeeded,
                    "no output sink is registered for output delivery",
                )
            })
    }

    fn output_reconciliation_after_append_failure(
        &self,
        _context: OutputDeliveryContext,
        request: OutputDeliveryRequest,
        intent: OutputDeliveryIntentRecord,
        result: OutputDeliveryResultRecord,
        receipt: Option<OutputDeliveryReceipt>,
        intent_record_id: String,
        mut result_base: OutputDeliveryJournalBase,
        append_error: AgentError,
    ) -> Result<OutputDeliveryOutcome, AgentError> {
        let reconciliation = OutputDeliveryReconciliationRecord {
            delivery_id: request.delivery_id.clone(),
            intent_record_id,
            side_effect_kind: EffectKind::OutputDelivery,
            idempotency_key: request.idempotency_key.clone(),
            dedupe_key: request.dedupe_key.clone(),
            external_operation_id: receipt
                .as_ref()
                .and_then(|receipt| receipt.external_operation_id.clone()),
            terminal_status: OutputDispatchStatus::ReconciliationNeeded,
            terminal_append_status: TerminalAppendStatus::AppendFailed,
            reconciliation_adapter: Some(request.sink_ref.clone()),
            unsafe_pending_reason: format!(
                "output sink was contacted but terminal result append failed: {}",
                append_error.context().message
            ),
            replay_decision: ReplayRepairDecision::RequiresHostReconciliation,
            resend_allowed: false,
        };
        result_base.record_id = format!("journal.{}.reconciliation", request.delivery_id.as_str());
        let recovery_record =
            reconciliation.to_journal_record(result_base, request.destination.clone());
        self.journal
            .append(recovery_record)
            .map_err(|recovery_error| {
                AgentError::new(
                    AgentErrorKind::RecoveryRepairNeeded,
                    RetryClassification::RepairNeeded,
                    format!(
                        "output delivery result append failed and reconciliation append failed: {}",
                        recovery_error.context().message
                    ),
                )
                .with_destination(request.destination.clone())
            })?;
        let destination = request.destination.clone();
        Ok(OutputDeliveryOutcome {
            status: OutputDispatchStatus::ReconciliationNeeded,
            request: Some(request),
            intent_record: Some(intent),
            result_record: Some(result),
            dedupe_record: None,
            reconciliation_record: Some(reconciliation),
            receipt,
            terminal_error: Some(
                AgentError::new(
                    AgentErrorKind::RecoveryRepairNeeded,
                    RetryClassification::RepairNeeded,
                    "output delivery terminal result append failed; replay requires reconciliation",
                )
                .with_destination(destination),
            ),
        })
    }

    fn append_host_configuration_needed(
        &self,
        context: OutputDeliveryContext,
        request: OutputDeliveryRequest,
        message: impl Into<String>,
    ) -> Result<OutputDeliveryOutcome, AgentError> {
        let message = message.into();
        let intent = OutputDeliveryIntentRecord::from_request(&request);
        self.journal
            .append(intent.to_journal_record(self.journal_base(
                &context,
                &request.destination,
                format!("journal.{}.intent", request.delivery_id.as_str()),
            )))
            .map_err(journal_failure)?;
        let result = OutputDeliveryResultRecord::failed(
            &request,
            OutputDispatchStatus::HostConfigurationNeeded,
            message.clone(),
            RetryClassification::HostConfigurationNeeded,
        );
        self.journal
            .append(result.to_journal_record(self.journal_base(
                &context,
                &request.destination,
                format!("journal.{}.result", request.delivery_id.as_str()),
            )))?;
        let error = AgentError::new(
            AgentErrorKind::HostConfigurationNeeded,
            RetryClassification::HostConfigurationNeeded,
            message,
        )
        .with_destination(request.destination.clone());
        Ok(OutputDeliveryOutcome {
            status: OutputDispatchStatus::HostConfigurationNeeded,
            request: Some(request),
            intent_record: Some(intent),
            result_record: Some(result),
            dedupe_record: None,
            reconciliation_record: None,
            receipt: None,
            terminal_error: Some(error),
        })
    }

    fn journal_base(
        &self,
        context: &OutputDeliveryContext,
        destination: &DestinationRef,
        record_id: String,
    ) -> OutputDeliveryJournalBase {
        OutputDeliveryJournalBase {
            journal_seq: self.next_seq.fetch_add(1, Ordering::SeqCst) + 1,
            record_id,
            run_id: context.run_id.clone(),
            agent_id: context.agent_id.clone(),
            turn_id: context.turn_id.clone(),
            attempt_id: context.attempt_id.clone(),
            source: context.source.clone(),
            destination: destination.clone(),
            timestamp_millis: 0,
            runtime_package_fingerprint: context.runtime_package_fingerprint.clone(),
            redaction_policy_id: "policy.redaction.default".to_string(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Holds output delivery context application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct OutputDeliveryContext {
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    /// Turn identifier for one loop turn within a run.
    pub turn_id: Option<TurnId>,
    /// Attempt identifier for retry, repair, provider, or tool execution
    /// evidence.
    pub attempt_id: Option<AttemptId>,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: RuntimePackageFingerprint,
}

impl OutputDeliveryContext {
    /// Creates a new application::output_delivery value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(
        run_id: RunId,
        agent_id: AgentId,
        runtime_package_fingerprint: RuntimePackageFingerprint,
    ) -> Self {
        Self {
            run_id,
            agent_id,
            turn_id: None,
            attempt_id: None,
            source: SourceRef::with_kind(SourceKind::Sdk, "source.sdk.output_delivery"),
            runtime_package_fingerprint,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Holds output delivery candidate application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct OutputDeliveryCandidate {
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: DestinationRef,
    /// Typed preferred sink ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub preferred_sink_ref: Option<OutputSinkRef>,
    /// Output delivery setting or policy.
    /// Delivery coordinators use it to decide sink mode, dedupe, and required evidence.
    pub delivery_kind: OutputDeliveryKind,
    /// Stable source message id used for typed lineage, lookup, or dedupe.
    pub source_message_id: Option<MessageId>,
    /// Stable validated output id used for typed lineage, lookup, or dedupe.
    pub validated_output_id: Option<ValidatedOutputId>,
    /// Content references associated with this record; resolving them is a
    /// separate policy-gated step.
    pub content_refs: Vec<ContentRef>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    /// Raw content or raw-content control for this value.
    /// Use it only when policy explicitly allows raw content capture or delivery.
    pub raw_content: Option<String>,
    /// Optional requested content mode value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub requested_content_mode: Option<OutputContentMode>,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention: RetentionClass,
    /// Policy used by this record or request.
    pub policy: OutputDeliveryPolicy,
}

impl OutputDeliveryCandidate {
    /// Builds the final message value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn final_message(
        destination: DestinationRef,
        sink_ref: OutputSinkRef,
        content_ref: ContentRef,
        policy_ref: PolicyRef,
    ) -> Self {
        Self {
            destination,
            preferred_sink_ref: Some(sink_ref.clone()),
            delivery_kind: OutputDeliveryKind::FinalMessage,
            source_message_id: Some(MessageId::new("message.output_delivery.final")),
            validated_output_id: None,
            content_refs: vec![content_ref],
            redacted_summary: "final assistant message ready for output delivery".to_string(),
            raw_content: None,
            requested_content_mode: None,
            privacy: PrivacyClass::ContentRefsOnly,
            retention: RetentionClass::RunScoped,
            policy: OutputDeliveryPolicy::required(policy_ref, sink_ref),
        }
    }
}

#[derive(Clone, Debug)]
/// Holds output delivery outcome application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct OutputDeliveryOutcome {
    /// Finite status for this record or lifecycle stage.
    pub status: OutputDispatchStatus,
    /// Request DTO or resolved call that triggered this operation.
    pub request: Option<OutputDeliveryRequest>,
    /// Optional intent record value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub intent_record: Option<OutputDeliveryIntentRecord>,
    /// Optional result record value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub result_record: Option<OutputDeliveryResultRecord>,
    /// Dedupe policy or key for a side-effecting operation.
    /// Replay and repair use it to avoid sending or executing the same effect twice.
    pub dedupe_record: Option<OutputDeliveryDedupeRecord>,
    /// Optional reconciliation record value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub reconciliation_record: Option<OutputDeliveryReconciliationRecord>,
    /// Optional receipt value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub receipt: Option<OutputDeliveryReceipt>,
    /// Optional terminal error value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub terminal_error: Option<AgentError>,
}

impl OutputDeliveryOutcome {
    fn skipped(status: OutputDispatchStatus, summary: impl Into<String>) -> Self {
        let _ = summary.into();
        Self {
            status,
            request: None,
            intent_record: None,
            result_record: None,
            dedupe_record: None,
            reconciliation_record: None,
            receipt: None,
            terminal_error: None,
        }
    }
}

#[derive(Clone, Debug, Default)]
/// Holds output delivery dedupe index application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct OutputDeliveryDedupeIndex {
    completed: Arc<Mutex<std::collections::BTreeMap<crate::domain::DedupeKey, OutputDedupeProof>>>,
}

impl OutputDeliveryDedupeIndex {
    /// Records a completed output-delivery dedupe proof in the in-memory index.
    /// This mutates only the local dedupe map and does not send output, append journals, or
    /// publish events.
    pub fn insert_completed(&self, proof: OutputDedupeProof) -> Result<(), AgentError> {
        self.completed
            .lock()
            .map_err(|_| AgentError::contract_violation("output dedupe index lock poisoned"))?
            .insert(proof.dedupe_key.clone(), proof);
        Ok(())
    }

    /// Returns an updated value with completed configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn completed(
        &self,
        dedupe_key: &crate::domain::DedupeKey,
    ) -> Result<Option<OutputDedupeProof>, AgentError> {
        Ok(self
            .completed
            .lock()
            .map_err(|_| AgentError::contract_violation("output dedupe index lock poisoned"))?
            .get(dedupe_key)
            .cloned())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Holds output dedupe proof application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct OutputDedupeProof {
    /// Dedupe policy or key for a side-effecting operation.
    /// Replay and repair use it to avoid sending or executing the same effect twice.
    pub dedupe_key: crate::domain::DedupeKey,
    /// Stable delivery id used for typed lineage, lookup, or dedupe.
    pub delivery_id: OutputDeliveryId,
    /// Stable external operation id used for typed lineage, lookup, or
    /// dedupe.
    pub external_operation_id: Option<String>,
    /// Finite status for this record or lifecycle stage.
    pub status: OutputDispatchStatus,
}

fn resolve_content_mode(
    candidate: &OutputDeliveryCandidate,
    capabilities: &OutputSinkCapabilities,
    sink_ref: &OutputSinkRef,
) -> Result<OutputContentMode, String> {
    let requested = candidate
        .requested_content_mode
        .unwrap_or(candidate.policy.default_content_mode);
    let fallback_modes = [
        requested,
        OutputContentMode::RedactedSummary,
        OutputContentMode::ContentRefsOnly,
    ];

    for mode in fallback_modes {
        if !candidate.policy.allows_mode(mode) || !capabilities.supports_content_mode(mode) {
            continue;
        }
        if mode == OutputContentMode::RawContentIfPolicyAllows {
            let Some(raw_content) = candidate.raw_content.as_ref() else {
                continue;
            };
            if !candidate
                .policy
                .raw_content_policy
                .allows_raw_for(sink_ref, raw_content.len())
            {
                continue;
            }
        }
        return Ok(mode);
    }

    Err(
        "output sink lacks required content-mode capability or policy denied raw content"
            .to_string(),
    )
}

fn journal_failure(error: AgentError) -> AgentError {
    AgentError::new(
        AgentErrorKind::JournalFailure,
        RetryClassification::RepairNeeded,
        error.context().message,
    )
}

fn stable_fragment(value: &str) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(value.as_bytes());
    digest[..6]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}
