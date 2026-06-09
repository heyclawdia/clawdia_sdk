use std::path::{Path, PathBuf};

use agent_sdk_core::{
    AgentError, EffectId, IdempotencyKey, JournalCursor, RunId, ToolCallId, ToolExecutionStore,
    ToolExecutionStoreCursor, ToolExecutionStoreRecord,
};
use rusqlite::{OptionalExtension, params};

use crate::util::{decode, encode, open, sqlite_error};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS tool_execution_records (
    run_id TEXT NOT NULL,
    tool_call_id TEXT NOT NULL,
    journal_seq INTEGER NOT NULL,
    idempotency_key TEXT,
    effect_id TEXT,
    record_json TEXT NOT NULL,
    PRIMARY KEY (run_id, tool_call_id, journal_seq)
);
CREATE INDEX IF NOT EXISTS idx_tool_execution_idempotency
ON tool_execution_records(idempotency_key);
CREATE INDEX IF NOT EXISTS idx_tool_execution_effect
ON tool_execution_records(effect_id);
CREATE INDEX IF NOT EXISTS idx_tool_execution_run_seq
ON tool_execution_records(run_id, journal_seq);
";

#[derive(Clone, Debug)]
/// SQLite-backed rebuildable tool-execution projection store.
pub struct SqliteToolExecutionStore {
    path: PathBuf,
}

impl SqliteToolExecutionStore {
    /// Opens or creates a SQLite tool-execution projection store.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AgentError> {
        crate::util::init(path.as_ref(), SCHEMA)?;
        Ok(Self {
            path: path.as_ref().to_path_buf(),
        })
    }
}

impl ToolExecutionStore for SqliteToolExecutionStore {
    fn put_tool_execution_record(
        &self,
        record: ToolExecutionStoreRecord,
    ) -> Result<ToolExecutionStoreCursor, AgentError> {
        let connection = open(&self.path)?;
        connection
            .execute(
                "INSERT OR REPLACE INTO tool_execution_records
                 (run_id, tool_call_id, journal_seq, idempotency_key, effect_id, record_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    record.run_id.as_str(),
                    record.tool_call_id.as_str(),
                    record.journal_seq as i64,
                    record.idempotency_key.as_ref().map(|key| key.as_str()),
                    record
                        .effect_id
                        .as_ref()
                        .map(|effect_id| effect_id.as_str()),
                    encode(&record)?,
                ],
            )
            .map_err(sqlite_error)?;
        let sequence = connection
            .query_row(
                "SELECT COUNT(*) FROM tool_execution_records WHERE run_id = ?1",
                params![record.run_id.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .map_err(sqlite_error)?;
        Ok(ToolExecutionStoreCursor::new(sequence as u64))
    }

    fn records_for_run(&self, run_id: &RunId) -> Result<Vec<ToolExecutionStoreRecord>, AgentError> {
        let connection = open(&self.path)?;
        let mut statement = connection
            .prepare(
                "SELECT record_json FROM tool_execution_records
                 WHERE run_id = ?1 ORDER BY journal_seq ASC",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map(params![run_id.as_str()], |row| row.get::<_, String>(0))
            .map_err(sqlite_error)?;
        let mut records = Vec::new();
        for row in rows {
            records.push(decode(&row.map_err(sqlite_error)?)?);
        }
        Ok(records)
    }

    fn record_for_tool_call(
        &self,
        run_id: &RunId,
        tool_call_id: &ToolCallId,
    ) -> Result<Option<ToolExecutionStoreRecord>, AgentError> {
        let connection = open(&self.path)?;
        let row = connection
            .query_row(
                "SELECT record_json FROM tool_execution_records
                 WHERE run_id = ?1 AND tool_call_id = ?2
                 ORDER BY journal_seq DESC LIMIT 1",
                params![run_id.as_str(), tool_call_id.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sqlite_error)?;
        row.map(|json| decode(&json)).transpose()
    }

    fn records_for_idempotency_key(
        &self,
        idempotency_key: &IdempotencyKey,
    ) -> Result<Vec<ToolExecutionStoreRecord>, AgentError> {
        let connection = open(&self.path)?;
        let mut statement = connection
            .prepare(
                "SELECT record_json FROM tool_execution_records
                 WHERE idempotency_key = ?1 ORDER BY run_id ASC, journal_seq ASC",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map(params![idempotency_key.as_str()], |row| {
                row.get::<_, String>(0)
            })
            .map_err(sqlite_error)?;
        let mut records = Vec::new();
        for row in rows {
            records.push(decode(&row.map_err(sqlite_error)?)?);
        }
        Ok(records)
    }

    fn records_for_effect_id(
        &self,
        effect_id: &EffectId,
    ) -> Result<Vec<ToolExecutionStoreRecord>, AgentError> {
        let connection = open(&self.path)?;
        let mut statement = connection
            .prepare(
                "SELECT record_json FROM tool_execution_records
                 WHERE effect_id = ?1 ORDER BY run_id ASC, journal_seq ASC",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map(params![effect_id.as_str()], |row| row.get::<_, String>(0))
            .map_err(sqlite_error)?;
        let mut records = Vec::new();
        for row in rows {
            records.push(decode(&row.map_err(sqlite_error)?)?);
        }
        Ok(records)
    }

    fn records_after_journal_seq(
        &self,
        run_id: &RunId,
        journal_seq: u64,
    ) -> Result<Vec<ToolExecutionStoreRecord>, AgentError> {
        let connection = open(&self.path)?;
        let mut statement = connection
            .prepare(
                "SELECT record_json FROM tool_execution_records
                 WHERE run_id = ?1 AND journal_seq > ?2 ORDER BY journal_seq ASC",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map(params![run_id.as_str(), journal_seq as i64], |row| {
                row.get::<_, String>(0)
            })
            .map_err(sqlite_error)?;
        let mut records = Vec::new();
        for row in rows {
            records.push(decode(&row.map_err(sqlite_error)?)?);
        }
        Ok(records)
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
        let connection = open(&self.path)?;
        let mut statement = connection
            .prepare(
                "SELECT record_json FROM tool_execution_records
                 WHERE run_id = ?1
                   AND (?2 IS NULL OR journal_seq > ?2)
                   AND (?3 IS NULL OR journal_seq <= ?3)
                 ORDER BY journal_seq ASC",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map(
                params![
                    run_id.as_str(),
                    after_seq.map(|seq| seq as i64),
                    through_seq.map(|seq| seq as i64),
                ],
                |row| row.get::<_, String>(0),
            )
            .map_err(sqlite_error)?;
        let mut records = Vec::new();
        for row in rows {
            records.push(decode(&row.map_err(sqlite_error)?)?);
        }
        Ok(records)
    }
}
