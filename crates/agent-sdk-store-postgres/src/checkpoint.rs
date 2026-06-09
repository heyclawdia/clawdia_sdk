use agent_sdk_core::{
    AgentError, CheckpointPrunePolicy, CheckpointPruneReport, CheckpointSaveOutcome,
    CheckpointStore, JournalCursor, RunCheckpoint, RunId,
};
use serde_json::Value;

use crate::{
    PostgresStoreClient,
    util::{decode_row, json_value},
};

#[derive(Clone)]
pub struct PostgresCheckpointStore {
    client: PostgresStoreClient,
}

impl PostgresCheckpointStore {
    pub fn new(client: PostgresStoreClient) -> Self {
        Self { client }
    }
}

impl CheckpointStore for PostgresCheckpointStore {
    fn save(
        &self,
        checkpoint: RunCheckpoint,
        latest_journal_seq: u64,
    ) -> Result<CheckpointSaveOutcome, AgentError> {
        checkpoint.validate_against_latest_seq(latest_journal_seq)?;
        let terminal_checkpoint = checkpoint.pending_side_effects.is_empty()
            && checkpoint.pending_approvals.is_empty()
            && checkpoint.loop_state == "terminal";
        self.client.execute(
            format!("insert into {} (store_scope, run_id, checkpoint_id, covers_journal_seq, checkpoint_json) values ($1, $2, $3, $4, $5) on conflict (store_scope, run_id, checkpoint_id) do update set covers_journal_seq = excluded.covers_journal_seq, checkpoint_json = excluded.checkpoint_json", self.client.table("agent_sdk_checkpoints")),
            vec![
                self.client.scope(),
                Value::String(checkpoint.run_id.as_str().to_string()),
                Value::String(checkpoint.checkpoint_id.clone()),
                Value::from(checkpoint.covers_journal_seq),
                json_value(&checkpoint)?,
            ],
        )?;
        Ok(CheckpointSaveOutcome {
            checkpoint_ref: checkpoint.checkpoint_id,
            covers_journal_seq: checkpoint.covers_journal_seq,
            terminal_checkpoint,
        })
    }

    fn load_latest(&self, run_id: &RunId) -> Result<Option<RunCheckpoint>, AgentError> {
        let response = self.client.execute(
            format!("select checkpoint_json from {} where store_scope = $1 and run_id = $2 order by covers_journal_seq desc limit 1", self.client.table("agent_sdk_checkpoints")),
            vec![self.client.scope(), Value::String(run_id.as_str().to_string())],
        )?;
        response
            .rows
            .into_iter()
            .next()
            .map(|row| decode_row(row, "checkpoint_json"))
            .transpose()
    }

    fn load_at_or_before(
        &self,
        run_id: &RunId,
        cursor: &JournalCursor,
    ) -> Result<Option<RunCheckpoint>, AgentError> {
        let seq = cursor
            .as_str()
            .strip_prefix("journal.")
            .unwrap_or(cursor.as_str());
        let response = self.client.execute(
            format!("select checkpoint_json from {} where store_scope = $1 and run_id = $2 and covers_journal_seq <= $3 order by covers_journal_seq desc limit 1", self.client.table("agent_sdk_checkpoints")),
            vec![self.client.scope(), Value::String(run_id.as_str().to_string()), Value::String(seq.to_string())],
        )?;
        response
            .rows
            .into_iter()
            .next()
            .map(|row| decode_row(row, "checkpoint_json"))
            .transpose()
    }

    fn prune(
        &self,
        run_id: &RunId,
        policy: CheckpointPrunePolicy,
    ) -> Result<CheckpointPruneReport, AgentError> {
        let response = self.client.execute(
            format!(
                "delete from {} where store_scope = $1 and run_id = $2 and covers_journal_seq < $3",
                self.client.table("agent_sdk_checkpoints")
            ),
            vec![
                self.client.scope(),
                Value::String(run_id.as_str().to_string()),
                Value::from(policy.prune_covered_before),
            ],
        )?;
        Ok(CheckpointPruneReport {
            run_id: run_id.clone(),
            pruned_count: response.affected as usize,
            retained_count: 0,
            preserved_terminal_checkpoint: None,
        })
    }
}
