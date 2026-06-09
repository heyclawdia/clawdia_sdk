use agent_sdk_core::{
    AgentError, EffectId, IdempotencyKey, JournalCursor, RunId, ToolCallId, ToolExecutionStore,
    ToolExecutionStoreCursor, ToolExecutionStoreRecord,
};
use serde_json::Value;

use crate::{
    PostgresStoreClient,
    util::{decode_row, json_value},
};

#[derive(Clone)]
pub struct PostgresToolExecutionStore {
    client: PostgresStoreClient,
}

impl PostgresToolExecutionStore {
    pub fn new(client: PostgresStoreClient) -> Self {
        Self { client }
    }
}

impl ToolExecutionStore for PostgresToolExecutionStore {
    fn put_tool_execution_record(
        &self,
        record: ToolExecutionStoreRecord,
    ) -> Result<ToolExecutionStoreCursor, AgentError> {
        self.client.execute(
            format!("insert into {} (store_scope, run_id, tool_call_id, journal_seq, idempotency_key, effect_id, record_json) values ($1, $2, $3, $4, $5, $6, $7) on conflict (store_scope, run_id, tool_call_id, journal_seq) do update set idempotency_key = excluded.idempotency_key, effect_id = excluded.effect_id, record_json = excluded.record_json", self.client.table("agent_sdk_tool_execution")),
            vec![
                self.client.scope(),
                Value::String(record.run_id.as_str().to_string()),
                Value::String(record.tool_call_id.as_str().to_string()),
                Value::from(record.journal_seq),
                record.idempotency_key.as_ref().map(|key| Value::String(key.as_str().to_string())).unwrap_or(Value::Null),
                record.effect_id.as_ref().map(|effect_id| Value::String(effect_id.as_str().to_string())).unwrap_or(Value::Null),
                json_value(&record)?,
            ],
        )?;
        Ok(ToolExecutionStoreCursor::new(record.journal_seq))
    }

    fn records_for_run(&self, run_id: &RunId) -> Result<Vec<ToolExecutionStoreRecord>, AgentError> {
        let response = self.client.execute(
            format!("select record_json from {} where store_scope = $1 and run_id = $2 order by journal_seq asc", self.client.table("agent_sdk_tool_execution")),
            vec![self.client.scope(), Value::String(run_id.as_str().to_string())],
        )?;
        response
            .rows
            .into_iter()
            .map(|row| decode_row(row, "record_json"))
            .collect()
    }

    fn records_for_effect_id(
        &self,
        effect_id: &EffectId,
    ) -> Result<Vec<ToolExecutionStoreRecord>, AgentError> {
        let response = self.client.execute(
            format!("select record_json from {} where store_scope = $1 and effect_id = $2 order by run_id asc, journal_seq asc", self.client.table("agent_sdk_tool_execution")),
            vec![self.client.scope(), Value::String(effect_id.as_str().to_string())],
        )?;
        response
            .rows
            .into_iter()
            .map(|row| decode_row(row, "record_json"))
            .collect()
    }

    fn record_for_tool_call(
        &self,
        run_id: &RunId,
        tool_call_id: &ToolCallId,
    ) -> Result<Option<ToolExecutionStoreRecord>, AgentError> {
        let response = self.client.execute(
            format!("select record_json from {} where store_scope = $1 and run_id = $2 and tool_call_id = $3 order by journal_seq desc limit 1", self.client.table("agent_sdk_tool_execution")),
            vec![self.client.scope(), Value::String(run_id.as_str().to_string()), Value::String(tool_call_id.as_str().to_string())],
        )?;
        response
            .rows
            .into_iter()
            .next()
            .map(|row| decode_row(row, "record_json"))
            .transpose()
    }

    fn records_for_idempotency_key(
        &self,
        idempotency_key: &IdempotencyKey,
    ) -> Result<Vec<ToolExecutionStoreRecord>, AgentError> {
        let response = self.client.execute(
            format!("select record_json from {} where store_scope = $1 and idempotency_key = $2 order by run_id asc, journal_seq asc", self.client.table("agent_sdk_tool_execution")),
            vec![self.client.scope(), Value::String(idempotency_key.as_str().to_string())],
        )?;
        response
            .rows
            .into_iter()
            .map(|row| decode_row(row, "record_json"))
            .collect()
    }

    fn records_after_journal_seq(
        &self,
        run_id: &RunId,
        journal_seq: u64,
    ) -> Result<Vec<ToolExecutionStoreRecord>, AgentError> {
        let response = self.client.execute(
            format!("select record_json from {} where store_scope = $1 and run_id = $2 and journal_seq > $3 order by journal_seq asc", self.client.table("agent_sdk_tool_execution")),
            vec![self.client.scope(), Value::String(run_id.as_str().to_string()), Value::from(journal_seq)],
        )?;
        response
            .rows
            .into_iter()
            .map(|row| decode_row(row, "record_json"))
            .collect()
    }

    fn records_in_journal_cursor_range(
        &self,
        run_id: &RunId,
        after: Option<&JournalCursor>,
        through: Option<&JournalCursor>,
    ) -> Result<Vec<ToolExecutionStoreRecord>, AgentError> {
        let after_seq = after
            .map(|cursor| {
                ToolExecutionStoreRecord::journal_sequence_for_cursor(cursor).ok_or_else(|| {
                    AgentError::contract_violation(
                        "tool execution cursor range uses an unsupported journal cursor",
                    )
                })
            })
            .transpose()?;
        let through_seq = through
            .map(|cursor| {
                ToolExecutionStoreRecord::journal_sequence_for_cursor(cursor).ok_or_else(|| {
                    AgentError::contract_violation(
                        "tool execution cursor range uses an unsupported journal cursor",
                    )
                })
            })
            .transpose()?;
        let response = self.client.execute(
            format!("select record_json from {} where store_scope = $1 and run_id = $2 and ($3 is null or journal_seq > $3) and ($4 is null or journal_seq <= $4) order by journal_seq asc", self.client.table("agent_sdk_tool_execution")),
            vec![
                self.client.scope(),
                Value::String(run_id.as_str().to_string()),
                after_seq.map(Value::from).unwrap_or(Value::Null),
                through_seq.map(Value::from).unwrap_or(Value::Null),
            ],
        )?;
        response
            .rows
            .into_iter()
            .map(|row| decode_row(row, "record_json"))
            .collect()
    }
}
