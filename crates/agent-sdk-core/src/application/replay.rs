//! Application-layer coordination over core primitives. Use these services to lower
//! helpers, drive runs, validate output, coordinate tools, approvals, delivery,
//! isolation, telemetry, and feature layers. Methods in this layer may call
//! configured ports, mutate in-memory stores, append journals, or publish events as
//! documented. This file contains the replay portion of that contract.
//!
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    content::MissingContentPolicy,
    domain::{
        AgentError, AgentErrorKind, ContentRef, DedupeKey, EffectId, JournalCursor,
        RetryClassification,
    },
    event::{EventCursor, EventStreamScope, cursor_compatible},
    journal::{
        JOURNAL_SCHEMA_VERSION, JournalRecord, JournalRecordKind, JournalRecordPayload,
        PendingSideEffect, RunCheckpoint,
    },
    output_delivery::{
        OutputDeliveryDedupeRecord, OutputDeliveryId, OutputDeliveryIntentRecord,
        OutputDeliveryReconciliationRecord, OutputDeliveryRecord, OutputDeliveryResultRecord,
        OutputDispatchStatus, ReplayRepairDecision, TerminalAppendStatus,
    },
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite replay mode cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ReplayMode {
    /// Use this variant when the contract needs to represent audit replay; selecting it has no side effect by itself.
    AuditReplay,
    /// Use this variant when the contract needs to represent resume replay; selecting it has no side effect by itself.
    ResumeReplay,
    /// Use this variant when the contract needs to represent repair replay; selecting it has no side effect by itself.
    RepairReplay,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite replay status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ReplayStatus {
    /// Use this variant when the contract needs to represent complete; selecting it has no side effect by itself.
    Complete,
    /// Use this variant when the contract needs to represent repair needed; selecting it has no side effect by itself.
    RepairNeeded,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite replay repair kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ReplayRepairKind {
    /// Use this variant when the contract needs to represent missing content ref; selecting it has no side effect by itself.
    MissingContentRef,
    /// Use this variant when the contract needs to represent unsafe pending side effect; selecting it has no side effect by itself.
    UnsafePendingSideEffect,
    /// Use this variant when the contract needs to represent non idempotent pending side effect; selecting it has no side effect by itself.
    NonIdempotentPendingSideEffect,
    /// Use this variant when the contract needs to represent output delivery reconciliation; selecting it has no side effect by itself.
    OutputDeliveryReconciliation,
    /// Use this variant when the contract needs to represent cursor scope mismatch; selecting it has no side effect by itself.
    CursorScopeMismatch,
    /// Use this variant when the contract needs to represent checkpoint invalid; selecting it has no side effect by itself.
    CheckpointInvalid,
    /// Use this variant when the contract needs to represent replay invariant violation; selecting it has no side effect by itself.
    ReplayInvariantViolation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds replay repair needed application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct ReplayRepairNeeded {
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: ReplayRepairKind,
    /// Stable record id used for typed lineage, lookup, or dedupe.
    pub record_id: String,
    /// Journal seq used by this record or request.
    pub journal_seq: u64,
    /// Redacted explanation for a denial, failure, status, or package delta.
    pub reason: String,
    /// Retry used by this record or request.
    pub retry: RetryClassification,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds replay pending side effect application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct ReplayPendingSideEffect {
    /// Stable effect id used for typed lineage, lookup, or dedupe.
    pub effect_id: EffectId,
    /// Stable intent record id used for typed lineage, lookup, or dedupe.
    pub intent_record_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency_key: Option<crate::domain::IdempotencyKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Dedupe policy or key for a side-effecting operation.
    /// Replay and repair use it to avoid sending or executing the same effect twice.
    pub dedupe_key: Option<DedupeKey>,
    /// Reason a pending side effect is unsafe to retry automatically.
    /// Recovery uses it to require repair or reconciliation before continuing.
    pub unsafe_pending_reason: String,
    /// Allowlist for this policy or contract.
    /// Validation uses it to reject undeclared or policy-denied values.
    pub retry_allowed: bool,
}

impl ReplayPendingSideEffect {
    /// Constructs this value from pending. Use it when adapting
    /// canonical SDK records without introducing a second behavior
    /// path.
    pub fn from_pending(pending: PendingSideEffect) -> Self {
        let retry_allowed = pending.idempotency_key.is_some() || pending.dedupe_key.is_some();
        Self {
            effect_id: pending.effect_id,
            intent_record_id: pending.intent_record_id,
            idempotency_key: pending.idempotency_key,
            dedupe_key: pending.dedupe_key,
            unsafe_pending_reason: pending.unsafe_pending_reason,
            retry_allowed,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds replay result application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct ReplayResult {
    /// Mode that selects how this operation or contract should behave.
    /// Callers use it to choose the explicit execution path instead of relying on hidden
    /// defaults.
    pub mode: ReplayMode,
    /// Finite status for this record or lifecycle stage.
    pub status: ReplayStatus,
    /// Allowlist for this policy or contract.
    /// Validation uses it to reject undeclared or policy-denied values.
    pub resume_allowed: bool,
    /// Latest journal seq used by this record or request.
    pub latest_journal_seq: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional terminal status value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub terminal_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional next loop state value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub next_loop_state: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Side effects found during replay that have intent evidence but no safe terminal result.
    /// Recovery must reconcile or repair these entries before the run can resume safely.
    pub unsafe_pending_side_effects: Vec<ReplayPendingSideEffect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed missing content refs references. Resolving them is separate from
    /// constructing this record.
    pub missing_content_refs: Vec<ContentRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Replay repairs required before the durable state can be resumed safely.
    /// Each entry names the repair category and evidence that must be reconciled.
    pub repair_needed: Vec<ReplayRepairNeeded>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Output delivery setting or policy.
    /// Delivery coordinators use it to decide sink mode, dedupe, and required evidence.
    pub output_delivery_repairs: Vec<OutputDeliveryReconciliationRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional latest checkpoint value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub latest_checkpoint: Option<RunCheckpoint>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite cursor compatibility cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum CursorCompatibility {
    /// Use this variant when the contract needs to represent compatible; selecting it has no side effect by itself.
    Compatible,
    /// Use this variant when the contract needs to represent scope mismatch; selecting it has no side effect by itself.
    ScopeMismatch,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite durable replay support cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum DurableReplaySupport {
    /// Use this variant when the contract needs to represent run journal; selecting it has no side effect by itself.
    RunJournal,
    /// Use this variant when the contract needs to represent host archive required; selecting it has no side effect by itself.
    HostArchiveRequired,
}

#[derive(Clone, Debug)]
/// Holds replay reducer application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct ReplayReducer {
    mode: ReplayMode,
    last_journal_seq: Option<u64>,
    seen_records: BTreeMap<String, JournalRecord>,
    available_content_refs: Option<BTreeSet<ContentRef>>,
    missing_content_policy: MissingContentPolicy,
    missing_content_refs: BTreeSet<ContentRef>,
    repair_needed: Vec<ReplayRepairNeeded>,
    unsafe_pending_side_effects: Vec<ReplayPendingSideEffect>,
    pending_effects: BTreeMap<EffectId, ReplayPendingSideEffect>,
    output_intents: BTreeMap<OutputDeliveryId, OutputIntentState>,
    output_results: BTreeMap<OutputDeliveryId, OutputDeliveryResultRecord>,
    output_dedupes: BTreeMap<DedupeKey, OutputDeliveryDedupeRecord>,
    output_reconciliations: BTreeMap<OutputDeliveryId, OutputDeliveryReconciliationRecord>,
    terminal_status: Option<String>,
    latest_checkpoint: Option<RunCheckpoint>,
}

impl ReplayReducer {
    /// Creates a new application::replay value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(mode: ReplayMode) -> Self {
        Self {
            mode,
            last_journal_seq: None,
            seen_records: BTreeMap::new(),
            available_content_refs: None,
            missing_content_policy: MissingContentPolicy::Fail,
            missing_content_refs: BTreeSet::new(),
            repair_needed: Vec::new(),
            unsafe_pending_side_effects: Vec::new(),
            pending_effects: BTreeMap::new(),
            output_intents: BTreeMap::new(),
            output_results: BTreeMap::new(),
            output_dedupes: BTreeMap::new(),
            output_reconciliations: BTreeMap::new(),
            terminal_status: None,
            latest_checkpoint: None,
        }
    }

    /// Returns this value with its available content refs setting
    /// replaced. The method follows builder-style data construction and
    /// does not execute external work.
    pub fn with_available_content_refs(
        mut self,
        refs: impl IntoIterator<Item = ContentRef>,
    ) -> Self {
        self.available_content_refs = Some(refs.into_iter().collect());
        self
    }

    /// Returns this value with its missing content policy setting
    /// replaced. The method follows builder-style data construction and
    /// does not execute external work.
    pub fn with_missing_content_policy(mut self, policy: MissingContentPolicy) -> Self {
        self.missing_content_policy = policy;
        self
    }

    /// Apply.
    /// This mutates only the replay projection with one journal record and never re-executes
    /// the recorded side effect.
    pub fn apply(&mut self, record: JournalRecord) -> Result<(), AgentError> {
        if self
            .seen_records
            .get(&record.record_id)
            .is_some_and(|seen| seen == &record && idempotent_duplicate_allowed(&record))
        {
            return Ok(());
        }
        self.validate_ordering(&record)?;
        self.validate_not_after_terminal(&record)?;
        self.observe_content_refs(&record.record_id, record.journal_seq, &record.content_refs);

        match &record.payload {
            JournalRecordPayload::Checkpoint(checkpoint) => {
                checkpoint
                    .validate_against_latest_seq(record.journal_seq)
                    .map_err(|error| {
                        self.repair(
                            ReplayRepairKind::CheckpointInvalid,
                            &record.record_id,
                            record.journal_seq,
                            error.context().message,
                            RetryClassification::RepairNeeded,
                        );
                        error
                    })?;
                self.observe_content_refs(
                    &record.record_id,
                    record.journal_seq,
                    &checkpoint.content_ref_manifest,
                );
                if checkpoint_is_newer(checkpoint, self.latest_checkpoint.as_ref()) {
                    self.latest_checkpoint = Some(checkpoint.clone());
                }
            }
            JournalRecordPayload::Recovery(recovery) => {
                for pending in recovery.unsafe_pending.iter().cloned() {
                    self.add_unsafe_pending(pending, &record.record_id, record.journal_seq);
                }
            }
            JournalRecordPayload::EffectIntent(intent) => {
                self.pending_effects.insert(
                    intent.effect_id.clone(),
                    ReplayPendingSideEffect {
                        effect_id: intent.effect_id.clone(),
                        intent_record_id: record.record_id.clone(),
                        idempotency_key: intent.idempotency_key.clone(),
                        dedupe_key: intent.dedupe_key.clone(),
                        unsafe_pending_reason: "effect intent has no terminal result in replay"
                            .to_string(),
                        retry_allowed: intent.idempotency_key.is_some()
                            || intent.dedupe_key.is_some(),
                    },
                );
            }
            JournalRecordPayload::EffectResult(result) => {
                self.pending_effects.remove(&result.effect_id);
            }
            JournalRecordPayload::OutputDelivery(output) => {
                self.apply_output_record(output, &record);
            }
            JournalRecordPayload::RunLifecycle(lifecycle) => {
                if is_terminal_lifecycle(&lifecycle.status) {
                    self.terminal_status = Some(lifecycle.status.clone());
                }
            }
            JournalRecordPayload::TerminalResult(marker) => {
                self.pending_effects.remove(&marker.effect_id);
                self.terminal_status = Some(marker.terminal_status.clone());
            }
            _ => {}
        }

        self.last_journal_seq = Some(record.journal_seq);
        self.seen_records.insert(record.record_id.clone(), record);
        Ok(())
    }

    /// Finish.
    /// This finalizes replay bookkeeping into a result and does not re-execute recorded
    /// effects.
    pub fn finish(mut self) -> Result<ReplayResult, AgentError> {
        self.finish_pending_effects();
        let output_delivery_repairs = self.finish_output_deliveries();
        let repair_needed = self.repair_needed;
        let missing_content_refs = self.missing_content_refs.into_iter().collect::<Vec<_>>();
        let unsafe_pending_side_effects = self.unsafe_pending_side_effects;
        let status = if repair_needed.is_empty()
            && missing_content_refs.is_empty()
            && unsafe_pending_side_effects
                .iter()
                .all(|pending| pending.retry_allowed)
        {
            ReplayStatus::Complete
        } else {
            ReplayStatus::RepairNeeded
        };
        let resume_allowed =
            self.mode != ReplayMode::ResumeReplay || status == ReplayStatus::Complete;

        Ok(ReplayResult {
            mode: self.mode,
            status,
            resume_allowed,
            latest_journal_seq: self.last_journal_seq.unwrap_or(0),
            terminal_status: self.terminal_status,
            next_loop_state: self
                .latest_checkpoint
                .as_ref()
                .map(|checkpoint| checkpoint.loop_state.clone()),
            unsafe_pending_side_effects,
            missing_content_refs,
            repair_needed,
            output_delivery_repairs,
            latest_checkpoint: self.latest_checkpoint,
        })
    }

    fn validate_ordering(&mut self, record: &JournalRecord) -> Result<(), AgentError> {
        if record.journal_schema_version != JOURNAL_SCHEMA_VERSION {
            return Err(AgentError::new(
                AgentErrorKind::RecoveryRepairNeeded,
                RetryClassification::RepairNeeded,
                "journal record schema version is not supported by replay reducer",
            ));
        }

        if self.seen_records.contains_key(&record.record_id) {
            return Err(AgentError::new(
                AgentErrorKind::InvalidStateTransition,
                RetryClassification::RepairNeeded,
                "duplicate non-idempotent journal record during replay",
            ));
        }

        if let Some(last_seq) = self.last_journal_seq {
            if record.journal_seq <= last_seq {
                return Err(AgentError::new(
                    AgentErrorKind::InvalidStateTransition,
                    RetryClassification::RepairNeeded,
                    "journal records must be strictly increasing during replay",
                ));
            }
        }
        Ok(())
    }

    fn validate_not_after_terminal(&self, record: &JournalRecord) -> Result<(), AgentError> {
        if self.terminal_status.is_none()
            || matches!(
                record.record_kind,
                JournalRecordKind::Checkpoint | JournalRecordKind::Recovery
            )
        {
            return Ok(());
        }
        Err(AgentError::new(
            AgentErrorKind::InvalidStateTransition,
            RetryClassification::RepairNeeded,
            "journal record appears after sealed terminal replay state",
        ))
    }

    fn observe_content_refs(&mut self, record_id: &str, journal_seq: u64, refs: &[ContentRef]) {
        let Some(available) = self.available_content_refs.as_ref() else {
            return;
        };
        let missing = refs
            .iter()
            .filter(|content_ref| {
                !available.contains(*content_ref)
                    && !self.missing_content_refs.contains(*content_ref)
            })
            .cloned()
            .collect::<Vec<_>>();
        for content_ref in missing {
            self.missing_content_refs.insert(content_ref.clone());
            if matches!(
                self.missing_content_policy,
                MissingContentPolicy::Fail
                    | MissingContentPolicy::RecoverableReplayGap
                    | MissingContentPolicy::RequestHostRepair
            ) {
                self.repair(
                    ReplayRepairKind::MissingContentRef,
                    record_id,
                    journal_seq,
                    format!("content ref {} is missing for replay", content_ref.as_str()),
                    RetryClassification::UserActionNeeded,
                );
            }
        }
    }

    fn add_unsafe_pending(
        &mut self,
        pending: PendingSideEffect,
        record_id: &str,
        journal_seq: u64,
    ) {
        let pending = ReplayPendingSideEffect::from_pending(pending);
        let repair_kind = if pending.retry_allowed {
            ReplayRepairKind::UnsafePendingSideEffect
        } else {
            ReplayRepairKind::NonIdempotentPendingSideEffect
        };
        let reason = pending.unsafe_pending_reason.clone();
        self.repair(
            repair_kind,
            record_id,
            journal_seq,
            reason,
            RetryClassification::RepairNeeded,
        );
        self.unsafe_pending_side_effects.push(pending);
    }

    fn apply_output_record(&mut self, output: &OutputDeliveryRecord, record: &JournalRecord) {
        match output {
            OutputDeliveryRecord::Intent(intent) => {
                self.output_intents.insert(
                    intent.delivery_id.clone(),
                    OutputIntentState {
                        record_id: record.record_id.clone(),
                        journal_seq: record.journal_seq,
                        intent: intent.clone(),
                    },
                );
            }
            OutputDeliveryRecord::Result(result) => {
                self.output_results
                    .insert(result.delivery_id.clone(), result.clone());
            }
            OutputDeliveryRecord::Dedupe(dedupe) => {
                self.output_dedupes
                    .insert(dedupe.dedupe_key.clone(), dedupe.clone());
            }
            OutputDeliveryRecord::Reconciliation(reconciliation) => {
                self.output_reconciliations
                    .insert(reconciliation.delivery_id.clone(), reconciliation.clone());
                self.repair(
                    ReplayRepairKind::OutputDeliveryReconciliation,
                    &record.record_id,
                    record.journal_seq,
                    reconciliation.unsafe_pending_reason.clone(),
                    RetryClassification::RepairNeeded,
                );
            }
            OutputDeliveryRecord::Event(_) => {}
        }
    }

    fn finish_pending_effects(&mut self) {
        let pending = self
            .pending_effects
            .values()
            .cloned()
            .collect::<Vec<ReplayPendingSideEffect>>();
        for pending in pending {
            let repair_kind = if pending.retry_allowed {
                ReplayRepairKind::UnsafePendingSideEffect
            } else {
                ReplayRepairKind::NonIdempotentPendingSideEffect
            };
            self.repair(
                repair_kind,
                &pending.intent_record_id,
                self.last_journal_seq.unwrap_or_default(),
                pending.unsafe_pending_reason.clone(),
                RetryClassification::RepairNeeded,
            );
            self.unsafe_pending_side_effects.push(pending);
        }
    }

    fn finish_output_deliveries(&mut self) -> Vec<OutputDeliveryReconciliationRecord> {
        let mut repairs = Vec::new();
        let intents = self
            .output_intents
            .values()
            .cloned()
            .collect::<Vec<OutputIntentState>>();
        for state in intents {
            if self.output_results.contains_key(&state.intent.delivery_id) {
                continue;
            }
            if let Some(reconciliation) = self
                .output_reconciliations
                .get(&state.intent.delivery_id)
                .cloned()
            {
                repairs.push(reconciliation);
                continue;
            }
            if let Some(dedupe) = self.output_dedupes.get(&state.intent.dedupe_key) {
                repairs.push(reconciliation_from_dedupe(&state, dedupe));
                continue;
            }

            let reconciliation = unsafe_output_reconciliation(&state);
            self.repair(
                ReplayRepairKind::OutputDeliveryReconciliation,
                &state.record_id,
                state.journal_seq,
                reconciliation.unsafe_pending_reason.clone(),
                RetryClassification::RepairNeeded,
            );
            repairs.push(reconciliation);
        }
        repairs
    }

    fn repair(
        &mut self,
        kind: ReplayRepairKind,
        record_id: &str,
        journal_seq: u64,
        reason: impl Into<String>,
        retry: RetryClassification,
    ) {
        self.repair_needed.push(ReplayRepairNeeded {
            kind,
            record_id: record_id.to_string(),
            journal_seq,
            reason: reason.into(),
            retry,
        });
    }
}

/// Check cursor compatibility.
/// This is replay bookkeeping over cursors or stream scopes and does not mutate runtime state
/// or re-execute effects.
pub fn check_cursor_compatibility(
    requested_scope: &EventStreamScope,
    cursor: Option<&EventCursor>,
) -> CursorCompatibility {
    match cursor_compatible(requested_scope, cursor) {
        Ok(()) => CursorCompatibility::Compatible,
        Err(_) => CursorCompatibility::ScopeMismatch,
    }
}

/// Returns durable replay support derived from the supplied state.
/// This derives SDK state locally and does not call host adapters.
pub fn durable_replay_support(scope: &EventStreamScope) -> DurableReplaySupport {
    match scope {
        EventStreamScope::Run(_) => DurableReplaySupport::RunJournal,
        EventStreamScope::All | EventStreamScope::Agent(_) | EventStreamScope::Filter { .. } => {
            DurableReplaySupport::HostArchiveRequired
        }
    }
}

#[derive(Clone, Debug)]
struct OutputIntentState {
    record_id: String,
    journal_seq: u64,
    intent: OutputDeliveryIntentRecord,
}

fn reconciliation_from_dedupe(
    state: &OutputIntentState,
    dedupe: &OutputDeliveryDedupeRecord,
) -> OutputDeliveryReconciliationRecord {
    OutputDeliveryReconciliationRecord {
        delivery_id: state.intent.delivery_id.clone(),
        intent_record_id: state.record_id.clone(),
        side_effect_kind: crate::effect::EffectKind::OutputDelivery,
        idempotency_key: state.intent.idempotency_key.clone(),
        dedupe_key: state.intent.dedupe_key.clone(),
        external_operation_id: dedupe.prior_external_operation_id.clone(),
        terminal_status: dedupe.prior_terminal_status,
        terminal_append_status: TerminalAppendStatus::NotAttempted,
        reconciliation_adapter: Some(state.intent.sink_ref.clone()),
        unsafe_pending_reason: "repair replay found completed dedupe proof".to_string(),
        replay_decision: ReplayRepairDecision::CompletedByDedupeProof,
        resend_allowed: false,
    }
}

fn unsafe_output_reconciliation(state: &OutputIntentState) -> OutputDeliveryReconciliationRecord {
    OutputDeliveryReconciliationRecord {
        delivery_id: state.intent.delivery_id.clone(),
        intent_record_id: state.record_id.clone(),
        side_effect_kind: crate::effect::EffectKind::OutputDelivery,
        idempotency_key: state.intent.idempotency_key.clone(),
        dedupe_key: state.intent.dedupe_key.clone(),
        external_operation_id: None,
        terminal_status: OutputDispatchStatus::ReconciliationNeeded,
        terminal_append_status: TerminalAppendStatus::NotAttempted,
        reconciliation_adapter: Some(state.intent.sink_ref.clone()),
        unsafe_pending_reason:
            "repair replay cannot resend output delivery without completed dedupe proof".to_string(),
        replay_decision: ReplayRepairDecision::UnsafePending,
        resend_allowed: false,
    }
}

fn checkpoint_is_newer(candidate: &RunCheckpoint, current: Option<&RunCheckpoint>) -> bool {
    current.is_none_or(|current| {
        (
            candidate.covers_journal_seq,
            candidate.checkpoint_seq,
            candidate.created_at_millis,
        ) > (
            current.covers_journal_seq,
            current.checkpoint_seq,
            current.created_at_millis,
        )
    })
}

fn is_terminal_lifecycle(status: &str) -> bool {
    matches!(
        status,
        "completed" | "failed" | "cancelled" | "run_completed" | "run_failed" | "run_cancelled"
    )
}

fn idempotent_duplicate_allowed(record: &JournalRecord) -> bool {
    record.idempotency_key.is_some()
        || record.dedupe_key.is_some()
        || matches!(
            record.record_kind,
            JournalRecordKind::Checkpoint | JournalRecordKind::Recovery
        )
}

/// Returns journal cursor for seq derived from the supplied state.
/// This derives SDK state locally and does not call host adapters.
pub fn journal_cursor_for_seq(seq: u64) -> JournalCursor {
    JournalCursor::new(format!("journal.{seq}"))
}
