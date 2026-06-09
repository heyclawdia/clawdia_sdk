use std::path::{Path, PathBuf};

use agent_sdk_core::{
    AgentError, CheckpointPrunePolicy, CheckpointPruneReport, CheckpointSaveOutcome,
    CheckpointStore, JournalCursor, RunCheckpoint, RunId,
};
use rusqlite::params;

use crate::util::{decode, encode, open, sqlite_error};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS checkpoints (
    run_id TEXT NOT NULL,
    checkpoint_id TEXT NOT NULL,
    latest_journal_seq INTEGER NOT NULL,
    checkpoint_json TEXT NOT NULL,
    PRIMARY KEY (run_id, checkpoint_id)
);
CREATE INDEX IF NOT EXISTS idx_checkpoints_latest
ON checkpoints(run_id, latest_journal_seq);
";

#[derive(Clone, Debug)]
/// SQLite-backed checkpoint store.
pub struct SqliteCheckpointStore {
    path: PathBuf,
}

impl SqliteCheckpointStore {
    /// Opens or creates a SQLite checkpoint store.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AgentError> {
        crate::util::init(path.as_ref(), SCHEMA)?;
        Ok(Self {
            path: path.as_ref().to_path_buf(),
        })
    }
}

impl CheckpointStore for SqliteCheckpointStore {
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
        let connection = open(&self.path)?;
        connection
            .execute(
                "INSERT OR REPLACE INTO checkpoints
                 (run_id, checkpoint_id, latest_journal_seq, checkpoint_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    checkpoint.run_id.as_str(),
                    checkpoint.checkpoint_id,
                    covers_journal_seq as i64,
                    encode(&checkpoint)?,
                ],
            )
            .map_err(sqlite_error)?;
        Ok(CheckpointSaveOutcome {
            checkpoint_ref,
            covers_journal_seq,
            terminal_checkpoint,
        })
    }

    fn load_latest(&self, run_id: &RunId) -> Result<Option<RunCheckpoint>, AgentError> {
        let connection = open(&self.path)?;
        let mut statement = connection
            .prepare(
                "SELECT checkpoint_json FROM checkpoints
                 WHERE run_id = ?1 ORDER BY latest_journal_seq DESC LIMIT 1",
            )
            .map_err(sqlite_error)?;
        let mut rows = statement
            .query_map(params![run_id.as_str()], |row| row.get::<_, String>(0))
            .map_err(sqlite_error)?;
        match rows.next() {
            Some(row) => Ok(Some(decode(&row.map_err(sqlite_error)?)?)),
            None => Ok(None),
        }
    }

    fn load_at_or_before(
        &self,
        run_id: &RunId,
        cursor: &JournalCursor,
    ) -> Result<Option<RunCheckpoint>, AgentError> {
        let latest_seq = cursor
            .as_str()
            .strip_prefix("journal.")
            .unwrap_or(cursor.as_str())
            .parse::<u64>()
            .unwrap_or_default();
        let connection = open(&self.path)?;
        let mut statement = connection
            .prepare(
                "SELECT checkpoint_json FROM checkpoints
                 WHERE run_id = ?1 AND latest_journal_seq <= ?2
                 ORDER BY latest_journal_seq DESC LIMIT 1",
            )
            .map_err(sqlite_error)?;
        let mut rows = statement
            .query_map(params![run_id.as_str(), latest_seq as i64], |row| {
                row.get::<_, String>(0)
            })
            .map_err(sqlite_error)?;
        match rows.next() {
            Some(row) => Ok(Some(decode(&row.map_err(sqlite_error)?)?)),
            None => Ok(None),
        }
    }

    fn prune(
        &self,
        run_id: &RunId,
        policy: CheckpointPrunePolicy,
    ) -> Result<CheckpointPruneReport, AgentError> {
        let connection = open(&self.path)?;
        let keep_latest = self
            .load_latest(run_id)?
            .map(|checkpoint| checkpoint.checkpoint_id);
        let mut statement = connection
            .prepare(
                "SELECT checkpoint_id FROM checkpoints
                 WHERE run_id = ?1 AND latest_journal_seq < ?2",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map(
                params![run_id.as_str(), policy.prune_covered_before as i64],
                |row| row.get::<_, String>(0),
            )
            .map_err(sqlite_error)?;
        let retained_before = connection
            .query_row(
                "SELECT COUNT(*) FROM checkpoints WHERE run_id = ?1",
                params![run_id.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .map_err(sqlite_error)? as usize;
        let mut deleted = 0;
        for row in rows {
            let checkpoint_id = row.map_err(sqlite_error)?;
            if policy.preserve_latest_terminal && Some(checkpoint_id.clone()) == keep_latest {
                continue;
            }
            deleted += connection
                .execute(
                    "DELETE FROM checkpoints WHERE run_id = ?1 AND checkpoint_id = ?2",
                    params![run_id.as_str(), checkpoint_id],
                )
                .map_err(sqlite_error)?;
        }
        Ok(CheckpointPruneReport {
            run_id: run_id.clone(),
            pruned_count: deleted,
            retained_count: retained_before.saturating_sub(deleted),
            preserved_terminal_checkpoint: if policy.preserve_latest_terminal {
                keep_latest
            } else {
                None
            },
        })
    }
}
