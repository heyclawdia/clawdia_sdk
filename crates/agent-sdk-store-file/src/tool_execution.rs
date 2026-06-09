use std::path::PathBuf;

use agent_sdk_core::{
    AgentError, EffectId, IdempotencyKey, JournalCursor, RunId, ToolCallId, ToolExecutionStore,
    ToolExecutionStoreCursor, ToolExecutionStoreRecord,
};

use crate::util::{read_json_lines, remove_file_if_exists, root_join, safe_segment, store_error};

#[derive(Clone, Debug)]
/// Filesystem-backed rebuildable tool-execution projection store.
pub struct FileToolExecutionStore {
    root: PathBuf,
}

impl FileToolExecutionStore {
    /// Creates a tool-execution projection store rooted under the provided path.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn run_path(&self, run_id: &RunId) -> PathBuf {
        root_join(
            &self.root,
            &[
                "tool_execution".to_string(),
                safe_segment(run_id.as_str()),
                "records.ndjson".to_string(),
            ],
        )
    }

    fn all_records(&self, run_id: &RunId) -> Result<Vec<ToolExecutionStoreRecord>, AgentError> {
        let mut records = read_json_lines::<ToolExecutionStoreRecord>(&self.run_path(run_id))?;
        records.sort_by_key(|record| record.journal_seq);
        Ok(records)
    }

    fn records_from_all_runs(&self) -> Result<Vec<ToolExecutionStoreRecord>, AgentError> {
        let root = self.root.join("tool_execution");
        if !root.exists() {
            return Ok(Vec::new());
        }
        let mut matches = Vec::new();
        for entry in std::fs::read_dir(root).map_err(|error| store_error(error.to_string()))? {
            let entry = entry.map_err(|error| store_error(error.to_string()))?;
            let path = entry.path().join("records.ndjson");
            matches.extend(read_json_lines::<ToolExecutionStoreRecord>(&path)?);
        }
        matches.sort_by(|left, right| {
            left.run_id
                .as_str()
                .cmp(right.run_id.as_str())
                .then(left.journal_seq.cmp(&right.journal_seq))
        });
        Ok(matches)
    }
}

impl ToolExecutionStore for FileToolExecutionStore {
    fn put_tool_execution_record(
        &self,
        record: ToolExecutionStoreRecord,
    ) -> Result<ToolExecutionStoreCursor, AgentError> {
        let mut records = self.all_records(&record.run_id)?;
        records.retain(|existing| {
            existing.tool_call_id != record.tool_call_id
                || existing.journal_seq != record.journal_seq
        });
        records.push(record.clone());
        records.sort_by_key(|record| record.journal_seq);
        let path = self.run_path(&record.run_id);
        remove_file_if_exists(&path)?;
        for record in &records {
            crate::util::append_json_line(&path, record)?;
        }
        Ok(ToolExecutionStoreCursor::new(records.len() as u64))
    }

    fn records_for_run(&self, run_id: &RunId) -> Result<Vec<ToolExecutionStoreRecord>, AgentError> {
        self.all_records(run_id)
    }

    fn record_for_tool_call(
        &self,
        run_id: &RunId,
        tool_call_id: &ToolCallId,
    ) -> Result<Option<ToolExecutionStoreRecord>, AgentError> {
        Ok(self
            .all_records(run_id)?
            .into_iter()
            .rev()
            .find(|record| &record.tool_call_id == tool_call_id))
    }

    fn records_for_idempotency_key(
        &self,
        idempotency_key: &IdempotencyKey,
    ) -> Result<Vec<ToolExecutionStoreRecord>, AgentError> {
        Ok(self
            .records_from_all_runs()?
            .into_iter()
            .filter(|record| record.idempotency_key.as_ref() == Some(idempotency_key))
            .collect())
    }

    fn records_for_effect_id(
        &self,
        effect_id: &EffectId,
    ) -> Result<Vec<ToolExecutionStoreRecord>, AgentError> {
        Ok(self
            .records_from_all_runs()?
            .into_iter()
            .filter(|record| record.effect_id.as_ref() == Some(effect_id))
            .collect())
    }

    fn records_after_journal_seq(
        &self,
        run_id: &RunId,
        journal_seq: u64,
    ) -> Result<Vec<ToolExecutionStoreRecord>, AgentError> {
        Ok(self
            .all_records(run_id)?
            .into_iter()
            .filter(|record| record.journal_seq > journal_seq)
            .collect())
    }

    fn records_in_journal_cursor_range(
        &self,
        run_id: &RunId,
        after: Option<&JournalCursor>,
        through: Option<&JournalCursor>,
    ) -> Result<Vec<ToolExecutionStoreRecord>, AgentError> {
        Ok(self
            .all_records(run_id)?
            .into_iter()
            .filter(|record| record.is_in_journal_cursor_range(after, through))
            .collect())
    }
}
