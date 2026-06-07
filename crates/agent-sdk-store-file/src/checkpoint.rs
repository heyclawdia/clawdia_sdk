use std::path::PathBuf;

use agent_sdk_core::{
    AgentError, CheckpointPrunePolicy, CheckpointPruneReport, CheckpointSaveOutcome,
    CheckpointStore, JournalCursor, RunCheckpoint, RunId,
};

use crate::util::{read_json, remove_file_if_exists, root_join, safe_segment, write_json};

#[derive(Clone, Debug)]
/// Filesystem-backed checkpoint accelerator store.
pub struct FileCheckpointStore {
    root: PathBuf,
}

impl FileCheckpointStore {
    /// Creates a checkpoint store rooted under the provided directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn checkpoint_path(&self, checkpoint: &RunCheckpoint) -> PathBuf {
        root_join(
            &self.root,
            &[
                "runs".to_string(),
                safe_segment(checkpoint.run_id.as_str()),
                "checkpoints".to_string(),
                format!(
                    "{:020}-{}.json",
                    checkpoint.covers_journal_seq,
                    safe_segment(&checkpoint.checkpoint_id)
                ),
            ],
        )
    }

    fn checkpoint_dir(&self, run_id: &RunId) -> PathBuf {
        root_join(
            &self.root,
            &[
                "runs".to_string(),
                safe_segment(run_id.as_str()),
                "checkpoints".to_string(),
            ],
        )
    }

    fn list(&self, run_id: &RunId) -> Result<Vec<(PathBuf, RunCheckpoint)>, AgentError> {
        let dir = self.checkpoint_dir(run_id);
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(dir).map_err(|error| {
            AgentError::new(
                agent_sdk_core::AgentErrorKind::RecoveryRepairNeeded,
                agent_sdk_core::RetryClassification::Retryable,
                error.to_string(),
            )
        })? {
            let path = entry.map_err(|error| {
                AgentError::new(
                    agent_sdk_core::AgentErrorKind::RecoveryRepairNeeded,
                    agent_sdk_core::RetryClassification::Retryable,
                    error.to_string(),
                )
            })?;
            let path = path.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            if let Some(checkpoint) = read_json::<RunCheckpoint>(&path)? {
                entries.push((path, checkpoint));
            }
        }
        entries.sort_by_key(|(_, checkpoint)| {
            (
                checkpoint.covers_journal_seq,
                checkpoint.checkpoint_seq,
                checkpoint.created_at_millis,
            )
        });
        Ok(entries)
    }
}

impl CheckpointStore for FileCheckpointStore {
    fn save(
        &self,
        checkpoint: RunCheckpoint,
        latest_journal_seq: u64,
    ) -> Result<CheckpointSaveOutcome, AgentError> {
        checkpoint.validate_against_latest_seq(latest_journal_seq)?;
        let checkpoint_ref = checkpoint.checkpoint_id.clone();
        let covers_journal_seq = checkpoint.covers_journal_seq;
        let terminal_checkpoint = checkpoint.pending_side_effects.is_empty()
            && checkpoint.pending_approvals.is_empty()
            && checkpoint.loop_state == "terminal";
        write_json(&self.checkpoint_path(&checkpoint), &checkpoint)?;
        Ok(CheckpointSaveOutcome {
            checkpoint_ref,
            covers_journal_seq,
            terminal_checkpoint,
        })
    }

    fn load_latest(&self, run_id: &RunId) -> Result<Option<RunCheckpoint>, AgentError> {
        Ok(self
            .list(run_id)?
            .into_iter()
            .map(|(_, checkpoint)| checkpoint)
            .max_by_key(|checkpoint| {
                (
                    checkpoint.covers_journal_seq,
                    checkpoint.checkpoint_seq,
                    checkpoint.created_at_millis,
                )
            }))
    }

    fn load_at_or_before(
        &self,
        run_id: &RunId,
        cursor: &JournalCursor,
    ) -> Result<Option<RunCheckpoint>, AgentError> {
        let cursor_seq = cursor
            .as_str()
            .strip_prefix("journal.")
            .unwrap_or(cursor.as_str())
            .parse::<u64>()
            .unwrap_or(0);
        Ok(self
            .list(run_id)?
            .into_iter()
            .map(|(_, checkpoint)| checkpoint)
            .filter(|checkpoint| checkpoint.covers_journal_seq <= cursor_seq)
            .max_by_key(|checkpoint| {
                (
                    checkpoint.covers_journal_seq,
                    checkpoint.checkpoint_seq,
                    checkpoint.created_at_millis,
                )
            }))
    }

    fn prune(
        &self,
        run_id: &RunId,
        policy: CheckpointPrunePolicy,
    ) -> Result<CheckpointPruneReport, AgentError> {
        let entries = self.list(run_id)?;
        let terminal_to_preserve = policy.preserve_latest_terminal.then(|| {
            entries
                .iter()
                .filter(|(_, checkpoint)| {
                    checkpoint.pending_side_effects.is_empty()
                        && checkpoint.pending_approvals.is_empty()
                        && checkpoint.loop_state == "terminal"
                })
                .max_by_key(|(_, checkpoint)| {
                    (
                        checkpoint.covers_journal_seq,
                        checkpoint.checkpoint_seq,
                        checkpoint.created_at_millis,
                    )
                })
                .map(|(_, checkpoint)| checkpoint.checkpoint_id.clone())
        });
        let terminal_to_preserve = terminal_to_preserve.flatten();

        let mut pruned_count = 0;
        let mut retained_count = 0;
        for (path, checkpoint) in entries {
            let preserve_terminal = terminal_to_preserve
                .as_ref()
                .is_some_and(|id| id == &checkpoint.checkpoint_id);
            if checkpoint.covers_journal_seq < policy.prune_covered_before && !preserve_terminal {
                remove_file_if_exists(&path)?;
                pruned_count += 1;
            } else {
                retained_count += 1;
            }
        }

        Ok(CheckpointPruneReport {
            run_id: run_id.clone(),
            pruned_count,
            retained_count,
            preserved_terminal_checkpoint: terminal_to_preserve,
        })
    }
}
