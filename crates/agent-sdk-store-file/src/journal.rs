use std::path::{Path, PathBuf};

use agent_sdk_core::{
    AgentError, JournalCursor, JournalRecord, RunId, RunJournal, RunJournalReader,
    journal::JOURNAL_SCHEMA_VERSION,
};

use crate::util::{append_json_line, journal_error, read_json_lines, root_join, safe_segment};

#[derive(Clone, Debug)]
/// Filesystem-backed run journal adapter.
pub struct FileRunJournal {
    root: PathBuf,
}

impl FileRunJournal {
    /// Creates a journal adapter rooted under the provided directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Returns the path used for one run's append-only journal file.
    pub fn journal_path(&self, run_id: &RunId) -> PathBuf {
        root_join(
            &self.root,
            &[
                "runs".to_string(),
                safe_segment(run_id.as_str()),
                "journal.ndjson".to_string(),
            ],
        )
    }

    fn records_at(path: &Path) -> Result<Vec<JournalRecord>, AgentError> {
        read_json_lines(path).map_err(|error| journal_error(error.context().message))
    }
}

impl RunJournal for FileRunJournal {
    fn append(&self, record: JournalRecord) -> Result<JournalCursor, AgentError> {
        if record.journal_schema_version != JOURNAL_SCHEMA_VERSION {
            return Err(journal_error("journal record schema version mismatch"));
        }
        let path = self.journal_path(&record.run_id);
        let records = Self::records_at(&path)?;
        if let Some(existing) = records
            .iter()
            .find(|existing| existing.record_id == record.record_id)
        {
            return Ok(JournalCursor::new(format!(
                "journal.{}",
                existing.journal_seq
            )));
        }
        if records
            .last()
            .is_some_and(|existing| record.journal_seq <= existing.journal_seq)
        {
            return Err(journal_error(
                "journal_seq must be strictly increasing for a run journal",
            ));
        }
        append_json_line(&path, &record).map_err(|error| journal_error(error.context().message))?;
        Ok(JournalCursor::new(format!(
            "journal.{}",
            record.journal_seq
        )))
    }
}

impl RunJournalReader for FileRunJournal {
    fn records_for_run(&self, run_id: &RunId) -> Result<Vec<JournalRecord>, AgentError> {
        let mut records = Self::records_at(&self.journal_path(run_id))?
            .into_iter()
            .filter(|record| &record.run_id == run_id)
            .collect::<Vec<_>>();
        records.sort_by_key(|record| record.journal_seq);
        Ok(records)
    }
}
