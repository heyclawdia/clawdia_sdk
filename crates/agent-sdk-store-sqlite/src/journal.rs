use std::path::{Path, PathBuf};

use agent_sdk_core::{
    AgentError, JournalCursor, JournalRecord, RunId, RunJournal, RunJournalReader,
    journal::JOURNAL_SCHEMA_VERSION,
};
use rusqlite::params;

use crate::util::{decode, encode, journal_error, open, sqlite_error};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS run_journal_records (
    run_id TEXT NOT NULL,
    journal_seq INTEGER NOT NULL,
    record_id TEXT NOT NULL,
    record_json TEXT NOT NULL,
    PRIMARY KEY (run_id, journal_seq)
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_run_journal_record_id
ON run_journal_records(run_id, record_id);
";

#[derive(Clone, Debug)]
/// SQLite-backed run journal adapter.
pub struct SqliteRunJournal {
    path: PathBuf,
}

impl SqliteRunJournal {
    /// Opens or creates a SQLite run journal.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AgentError> {
        crate::util::init(path.as_ref(), SCHEMA)?;
        Ok(Self {
            path: path.as_ref().to_path_buf(),
        })
    }

    /// Returns the backing SQLite database path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl RunJournal for SqliteRunJournal {
    fn append(&self, record: JournalRecord) -> Result<JournalCursor, AgentError> {
        if record.journal_schema_version != JOURNAL_SCHEMA_VERSION {
            return Err(journal_error("journal record schema version mismatch"));
        }
        let connection = open(&self.path)?;
        if let Ok(existing_seq) = connection.query_row(
            "SELECT journal_seq FROM run_journal_records
             WHERE run_id = ?1 AND record_id = ?2",
            params![record.run_id.as_str(), record.record_id],
            |row| row.get::<_, i64>(0),
        ) {
            return Ok(JournalCursor::new(format!("journal.{existing_seq}")));
        }
        let latest_seq = connection
            .query_row(
                "SELECT COALESCE(MAX(journal_seq), 0)
                 FROM run_journal_records WHERE run_id = ?1",
                params![record.run_id.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .map_err(sqlite_error)? as u64;
        if record.journal_seq <= latest_seq {
            return Err(journal_error(
                "journal_seq must be strictly increasing for a run journal",
            ));
        }
        connection
            .execute(
                "INSERT INTO run_journal_records
                 (run_id, journal_seq, record_id, record_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    record.run_id.as_str(),
                    record.journal_seq as i64,
                    record.record_id,
                    encode(&record)?,
                ],
            )
            .map_err(sqlite_error)?;
        Ok(JournalCursor::new(format!(
            "journal.{}",
            record.journal_seq
        )))
    }
}

impl RunJournalReader for SqliteRunJournal {
    fn records_for_run(&self, run_id: &RunId) -> Result<Vec<JournalRecord>, AgentError> {
        let connection = open(&self.path)?;
        let mut statement = connection
            .prepare(
                "SELECT record_json FROM run_journal_records
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
}
