use agent_sdk_core::{
    AgentError, JournalCursor, JournalRecord, RunId, RunJournal, RunJournalReader,
};
use serde_json::json;

use crate::{client::SupabaseClient, transport::supabase_error};

#[derive(Clone)]
/// Supabase-backed run journal adapter.
pub struct SupabaseRunJournal {
    client: SupabaseClient,
}

impl SupabaseRunJournal {
    pub fn new(client: SupabaseClient) -> Self {
        Self { client }
    }
}

impl RunJournal for SupabaseRunJournal {
    fn append(&self, record: JournalRecord) -> Result<JournalCursor, AgentError> {
        let response = self.client.rpc(
            "agent_sdk_append_journal_record",
            &json!({
                "p_store_scope": self.client.config().store_scope(),
                "p_run_id": record.run_id.as_str(),
                "p_journal_seq": record.journal_seq,
                "p_record": record,
            }),
        )?;
        if !(200..300).contains(&response.status) {
            return Err(supabase_error(format!(
                "supabase journal append failed with status {}",
                response.status
            )));
        }
        Ok(JournalCursor::new(format!(
            "journal.{}",
            record.journal_seq
        )))
    }
}

impl RunJournalReader for SupabaseRunJournal {
    fn records_for_run(&self, run_id: &RunId) -> Result<Vec<JournalRecord>, AgentError> {
        let query = format!(
            "store_scope=eq.{}&run_id=eq.{}&select=record&order=journal_seq.asc",
            self.client.config().store_scope(),
            run_id.as_str()
        );
        let response = self.client.select("agent_sdk_journal_records", &query)?;
        if !(200..300).contains(&response.status) {
            return Err(supabase_error(format!(
                "supabase journal read failed with status {}",
                response.status
            )));
        }
        let rows = serde_json::from_slice::<Vec<serde_json::Value>>(&response.body)
            .map_err(|error| supabase_error(error.to_string()))?;
        rows.into_iter()
            .map(|row| {
                serde_json::from_value::<JournalRecord>(row.get("record").cloned().unwrap_or(row))
                    .map_err(|error| supabase_error(error.to_string()))
            })
            .collect()
    }
}
