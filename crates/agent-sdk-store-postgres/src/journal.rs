use agent_sdk_core::{
    AgentError, JournalCursor, JournalRecord, RunId, RunJournal, RunJournalReader,
};
use serde_json::Value;

use crate::{
    PostgresStoreClient,
    util::{decode_row, json_value},
};

#[derive(Clone)]
pub struct PostgresRunJournal {
    client: PostgresStoreClient,
}

impl PostgresRunJournal {
    pub fn new(client: PostgresStoreClient) -> Self {
        Self { client }
    }
}

impl RunJournal for PostgresRunJournal {
    fn append(&self, record: JournalRecord) -> Result<JournalCursor, AgentError> {
        self.client.execute(
            format!(
                "select {}.append_journal_record($1, $2, $3, $4)",
                self.client.config.schema
            ),
            vec![
                self.client.scope(),
                Value::String(record.run_id.as_str().to_string()),
                Value::from(record.journal_seq),
                json_value(&record)?,
            ],
        )?;
        Ok(JournalCursor::new(format!(
            "journal.{}",
            record.journal_seq
        )))
    }
}

impl RunJournalReader for PostgresRunJournal {
    fn records_for_run(&self, run_id: &RunId) -> Result<Vec<JournalRecord>, AgentError> {
        let response = self.client.execute(
            format!(
                "select record_json from {} where store_scope = $1 and run_id = $2 order by journal_seq asc",
                self.client.table("agent_sdk_journal_records")
            ),
            vec![self.client.scope(), Value::String(run_id.as_str().to_string())],
        )?;
        response
            .rows
            .into_iter()
            .map(|row| decode_row(row, "record_json"))
            .collect()
    }
}
