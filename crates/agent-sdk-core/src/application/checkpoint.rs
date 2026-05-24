use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::{
    domain::{AgentError, JournalCursor, RunId},
    journal::RunCheckpoint,
};

pub trait CheckpointStore: Send + Sync {
    fn save(
        &self,
        checkpoint: RunCheckpoint,
        latest_journal_seq: u64,
    ) -> Result<CheckpointSaveOutcome, AgentError>;

    fn load_latest(&self, run_id: &RunId) -> Result<Option<RunCheckpoint>, AgentError>;

    fn load_at_or_before(
        &self,
        run_id: &RunId,
        cursor: &JournalCursor,
    ) -> Result<Option<RunCheckpoint>, AgentError>;

    fn prune(
        &self,
        run_id: &RunId,
        policy: CheckpointPrunePolicy,
    ) -> Result<CheckpointPruneReport, AgentError>;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CheckpointSaveOutcome {
    pub checkpoint_ref: String,
    pub covers_journal_seq: u64,
    pub terminal_checkpoint: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CheckpointPrunePolicy {
    pub prune_covered_before: u64,
    pub preserve_latest_terminal: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CheckpointPruneReport {
    pub run_id: RunId,
    pub pruned_count: usize,
    pub retained_count: usize,
    pub preserved_terminal_checkpoint: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryCheckpointStore {
    checkpoints: Arc<Mutex<BTreeMap<RunId, Vec<RunCheckpoint>>>>,
}

impl InMemoryCheckpointStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn save(
        &self,
        checkpoint: RunCheckpoint,
        latest_journal_seq: u64,
    ) -> Result<CheckpointSaveOutcome, AgentError> {
        <Self as CheckpointStore>::save(self, checkpoint, latest_journal_seq)
    }

    pub fn load_latest(&self, run_id: &RunId) -> Result<Option<RunCheckpoint>, AgentError> {
        <Self as CheckpointStore>::load_latest(self, run_id)
    }

    pub fn load_at_or_before(
        &self,
        run_id: &RunId,
        cursor: &JournalCursor,
    ) -> Result<Option<RunCheckpoint>, AgentError> {
        <Self as CheckpointStore>::load_at_or_before(self, run_id, cursor)
    }

    pub fn prune(
        &self,
        run_id: &RunId,
        policy: CheckpointPrunePolicy,
    ) -> Result<CheckpointPruneReport, AgentError> {
        <Self as CheckpointStore>::prune(self, run_id, policy)
    }

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
