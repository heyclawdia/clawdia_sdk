//! Application-layer coordination over core primitives. Use these services to lower
//! helpers, drive runs, validate output, coordinate tools, approvals, delivery,
//! isolation, telemetry, and feature layers. Methods in this layer may call
//! configured ports, mutate in-memory stores, append journals, or publish events as
//! documented. This file contains the checkpoint portion of that contract.
//!
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::{
    domain::{AgentError, JournalCursor, RunId},
    journal::RunCheckpoint,
};

/// Port or behavior contract for checkpoint store. Implementors should
/// preserve policy, redaction, idempotency, and replay expectations
/// from the surrounding module. Implementations may perform side
/// effects only as described by the trait methods.
pub trait CheckpointStore: Send + Sync {
    /// Saves checkpoint accelerator data for a run at a journal sequence.
    /// Implementations write or replace checkpoint accelerator data for the run without
    /// changing journal truth.
    fn save(
        &self,
        checkpoint: RunCheckpoint,
        latest_journal_seq: u64,
    ) -> Result<CheckpointSaveOutcome, AgentError>;

    /// Loads the latest checkpoint accelerator data for a run.
    /// Implementations read checkpoint storage for the requested run and return matching
    /// checkpoint data without altering durable journal truth.
    fn load_latest(&self, run_id: &RunId) -> Result<Option<RunCheckpoint>, AgentError>;

    /// Loads checkpoint accelerator data at or before the requested journal
    /// cursor.
    /// Implementations read checkpoint storage for the requested run and return matching
    /// checkpoint data without altering durable journal truth.
    fn load_at_or_before(
        &self,
        run_id: &RunId,
        cursor: &JournalCursor,
    ) -> Result<Option<RunCheckpoint>, AgentError>;

    /// Prunes checkpoint accelerator data according to retention policy.
    /// Implementations remove checkpoint accelerator entries according to retention policy
    /// without deleting journal truth.
    fn prune(
        &self,
        run_id: &RunId,
        policy: CheckpointPrunePolicy,
    ) -> Result<CheckpointPruneReport, AgentError>;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds checkpoint save outcome application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct CheckpointSaveOutcome {
    /// Typed checkpoint ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub checkpoint_ref: String,
    /// Covers journal seq used by this record or request.
    pub covers_journal_seq: u64,
    /// Whether terminal checkpoint is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub terminal_checkpoint: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds checkpoint prune policy application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct CheckpointPrunePolicy {
    /// Prune covered before used by this record or request.
    pub prune_covered_before: u64,
    /// Whether preserve latest terminal is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub preserve_latest_terminal: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds checkpoint prune report application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct CheckpointPruneReport {
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Count of pruned items observed or included in this record.
    pub pruned_count: usize,
    /// Count of retained items observed or included in this record.
    pub retained_count: usize,
    /// Terminal checkpoint retained even when pruning older checkpoint accelerators.
    /// Use it to keep terminal replay shortcuts available without treating checkpoints as
    /// durable truth.
    pub preserved_terminal_checkpoint: Option<String>,
}

#[derive(Clone, Debug, Default)]
/// Holds in memory checkpoint store application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct InMemoryCheckpointStore {
    checkpoints: Arc<Mutex<BTreeMap<RunId, Vec<RunCheckpoint>>>>,
}

impl InMemoryCheckpointStore {
    /// Creates a new application::checkpoint value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new() -> Self {
        Self::default()
    }

    /// Save.
    /// This stores checkpoint accelerator data in memory without changing journal truth.
    pub fn save(
        &self,
        checkpoint: RunCheckpoint,
        latest_journal_seq: u64,
    ) -> Result<CheckpointSaveOutcome, AgentError> {
        <Self as CheckpointStore>::save(self, checkpoint, latest_journal_seq)
    }

    /// Load latest.
    /// This reads the in-memory checkpoint store for the requested run and does not write
    /// checkpoints or journal records.
    pub fn load_latest(&self, run_id: &RunId) -> Result<Option<RunCheckpoint>, AgentError> {
        <Self as CheckpointStore>::load_latest(self, run_id)
    }

    /// Load at or before.
    /// This reads the in-memory checkpoint store for the requested run and does not write
    /// checkpoints or journal records.
    pub fn load_at_or_before(
        &self,
        run_id: &RunId,
        cursor: &JournalCursor,
    ) -> Result<Option<RunCheckpoint>, AgentError> {
        <Self as CheckpointStore>::load_at_or_before(self, run_id, cursor)
    }

    /// Prune.
    /// This prunes only the in-memory checkpoint accelerator and leaves durable journal records
    /// untouched.
    pub fn prune(
        &self,
        run_id: &RunId,
        policy: CheckpointPrunePolicy,
    ) -> Result<CheckpointPruneReport, AgentError> {
        <Self as CheckpointStore>::prune(self, run_id, policy)
    }

    /// List.
    /// This reads all in-memory checkpoint accelerator entries for one run.
    pub fn list(&self, run_id: &RunId) -> Result<Vec<RunCheckpoint>, AgentError> {
        Ok(self
            .checkpoints
            .lock()
            .map_err(|_| AgentError::contract_violation("checkpoint store lock poisoned"))?
            .get(run_id)
            .cloned()
            .unwrap_or_default())
    }
}

impl CheckpointStore for InMemoryCheckpointStore {
    fn save(
        &self,
        checkpoint: RunCheckpoint,
        latest_journal_seq: u64,
    ) -> Result<CheckpointSaveOutcome, AgentError> {
        checkpoint.validate_against_latest_seq(latest_journal_seq)?;
        let terminal_checkpoint = is_terminal_checkpoint(&checkpoint);
        let checkpoint_ref = checkpoint.checkpoint_id.clone();
        let covers_journal_seq = checkpoint.covers_journal_seq;
        let mut locked = self
            .checkpoints
            .lock()
            .map_err(|_| AgentError::contract_violation("checkpoint store lock poisoned"))?;
        let entries = locked.entry(checkpoint.run_id.clone()).or_default();
        entries.retain(|existing| existing.checkpoint_id != checkpoint.checkpoint_id);
        entries.push(checkpoint);
        entries.sort_by_key(|checkpoint| {
            (
                checkpoint.covers_journal_seq,
                checkpoint.checkpoint_seq,
                checkpoint.created_at_millis,
            )
        });

        Ok(CheckpointSaveOutcome {
            checkpoint_ref,
            covers_journal_seq,
            terminal_checkpoint,
        })
    }

    fn load_latest(&self, run_id: &RunId) -> Result<Option<RunCheckpoint>, AgentError> {
        Ok(self
            .checkpoints
            .lock()
            .map_err(|_| AgentError::contract_violation("checkpoint store lock poisoned"))?
            .get(run_id)
            .and_then(|checkpoints| checkpoints.iter().max_by_key(checkpoint_order).cloned()))
    }

    fn load_at_or_before(
        &self,
        run_id: &RunId,
        cursor: &JournalCursor,
    ) -> Result<Option<RunCheckpoint>, AgentError> {
        let cursor_seq = journal_cursor_seq(cursor);
        Ok(self
            .checkpoints
            .lock()
            .map_err(|_| AgentError::contract_violation("checkpoint store lock poisoned"))?
            .get(run_id)
            .and_then(|checkpoints| {
                checkpoints
                    .iter()
                    .filter(|checkpoint| checkpoint.covers_journal_seq <= cursor_seq)
                    .max_by_key(checkpoint_order)
                    .cloned()
            }))
    }

    fn prune(
        &self,
        run_id: &RunId,
        policy: CheckpointPrunePolicy,
    ) -> Result<CheckpointPruneReport, AgentError> {
        let mut locked = self
            .checkpoints
            .lock()
            .map_err(|_| AgentError::contract_violation("checkpoint store lock poisoned"))?;
        let Some(checkpoints) = locked.get_mut(run_id) else {
            return Ok(CheckpointPruneReport {
                run_id: run_id.clone(),
                pruned_count: 0,
                retained_count: 0,
                preserved_terminal_checkpoint: None,
            });
        };
        let terminal_to_preserve = policy
            .preserve_latest_terminal
            .then(|| {
                checkpoints
                    .iter()
                    .filter(|checkpoint| is_terminal_checkpoint(checkpoint))
                    .max_by_key(checkpoint_order)
                    .map(|checkpoint| checkpoint.checkpoint_id.clone())
            })
            .flatten();

        let before = checkpoints.len();
        checkpoints.retain(|checkpoint| {
            checkpoint.covers_journal_seq >= policy.prune_covered_before
                || terminal_to_preserve
                    .as_ref()
                    .is_some_and(|checkpoint_id| checkpoint_id == &checkpoint.checkpoint_id)
        });
        let retained_count = checkpoints.len();

        Ok(CheckpointPruneReport {
            run_id: run_id.clone(),
            pruned_count: before.saturating_sub(retained_count),
            retained_count,
            preserved_terminal_checkpoint: terminal_to_preserve,
        })
    }
}

fn checkpoint_order(checkpoint: &&RunCheckpoint) -> (u64, u64, u64) {
    (
        checkpoint.covers_journal_seq,
        checkpoint.checkpoint_seq,
        checkpoint.created_at_millis,
    )
}

fn is_terminal_checkpoint(checkpoint: &RunCheckpoint) -> bool {
    let state = checkpoint.loop_state.to_ascii_lowercase();
    state.starts_with("terminal:")
        || matches!(
            state.as_str(),
            "completed" | "failed" | "cancelled" | "terminal"
        )
}

fn journal_cursor_seq(cursor: &JournalCursor) -> u64 {
    cursor
        .as_str()
        .rsplit_once('.')
        .and_then(|(_, seq)| seq.parse::<u64>().ok())
        .unwrap_or(0)
}
