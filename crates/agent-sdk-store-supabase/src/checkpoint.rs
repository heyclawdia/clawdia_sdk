use agent_sdk_core::{
    AgentError, CheckpointPrunePolicy, CheckpointPruneReport, CheckpointSaveOutcome,
    CheckpointStore, JournalCursor, RunCheckpoint, RunId,
};
use serde_json::json;

use crate::{client::SupabaseClient, transport::supabase_error};

#[derive(Clone)]
/// Supabase-backed checkpoint store.
pub struct SupabaseCheckpointStore {
    client: SupabaseClient,
}

impl SupabaseCheckpointStore {
    pub fn new(client: SupabaseClient) -> Self {
        Self { client }
    }
}

impl CheckpointStore for SupabaseCheckpointStore {
    fn save(
        &self,
        checkpoint: RunCheckpoint,
        latest_journal_seq: u64,
    ) -> Result<CheckpointSaveOutcome, AgentError> {
        checkpoint.validate_against_latest_seq(latest_journal_seq)?;
        let checkpoint_ref = checkpoint.checkpoint_id.clone();
        let covers_journal_seq = checkpoint.covers_journal_seq;
        let response = self.client.insert(
            "agent_sdk_checkpoints",
            &json!({
                "store_scope": self.client.config().store_scope(),
                "run_id": checkpoint.run_id.as_str(),
                "checkpoint_id": checkpoint.checkpoint_id,
                "covers_journal_seq": checkpoint.covers_journal_seq,
                "checkpoint": checkpoint,
            }),
        )?;
        if !(200..300).contains(&response.status) {
            return Err(supabase_error(format!(
                "supabase checkpoint save failed with status {}",
                response.status
            )));
        }
        Ok(CheckpointSaveOutcome {
            checkpoint_ref,
            covers_journal_seq,
            terminal_checkpoint: false,
        })
    }

    fn load_latest(&self, run_id: &RunId) -> Result<Option<RunCheckpoint>, AgentError> {
        let query = format!(
            "store_scope=eq.{}&run_id=eq.{}&select=checkpoint&order=covers_journal_seq.desc&limit=1",
            self.client.config().store_scope(),
            run_id.as_str()
        );
        load_checkpoint(self.client.select("agent_sdk_checkpoints", &query)?)
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
        let query = format!(
            "store_scope=eq.{}&run_id=eq.{}&covers_journal_seq=lte.{}&select=checkpoint&order=covers_journal_seq.desc&limit=1",
            self.client.config().store_scope(),
            run_id.as_str(),
            seq
        );
        load_checkpoint(self.client.select("agent_sdk_checkpoints", &query)?)
    }

    fn prune(
        &self,
        run_id: &RunId,
        policy: CheckpointPrunePolicy,
    ) -> Result<CheckpointPruneReport, AgentError> {
        let response = self.client.rpc(
            "agent_sdk_prune_checkpoints",
            &json!({
                "p_store_scope": self.client.config().store_scope(),
                "p_run_id": run_id.as_str(),
                "p_prune_covered_before": policy.prune_covered_before,
                "p_preserve_latest_terminal": policy.preserve_latest_terminal,
            }),
        )?;
        if !(200..300).contains(&response.status) {
            return Err(supabase_error(format!(
                "supabase checkpoint prune failed with status {}",
                response.status
            )));
        }
        Ok(CheckpointPruneReport {
            run_id: run_id.clone(),
            pruned_count: 0,
            retained_count: 0,
            preserved_terminal_checkpoint: None,
        })
    }
}

fn load_checkpoint(
    response: crate::transport::SupabaseHttpResponse,
) -> Result<Option<RunCheckpoint>, AgentError> {
    if !(200..300).contains(&response.status) {
        return Err(supabase_error(format!(
            "supabase checkpoint read failed with status {}",
            response.status
        )));
    }
    let rows = serde_json::from_slice::<Vec<serde_json::Value>>(&response.body)
        .map_err(|error| supabase_error(error.to_string()))?;
    rows.into_iter()
        .next()
        .map(|row| {
            serde_json::from_value::<RunCheckpoint>(row["checkpoint"].clone())
                .map_err(|error| supabase_error(error.to_string()))
        })
        .transpose()
}
