use std::path::{Path, PathBuf};

use crate::{
    SqliteAgentPoolStore, SqliteCheckpointStore, SqliteContentStore, SqliteEventArchive,
    SqliteProviderArgumentStore, SqliteRunJournal, SqliteToolExecutionStore,
};

#[derive(Clone, Debug)]
/// SQLite-backed store bundle sharing one database path.
pub struct SqliteStoreBundle {
    path: PathBuf,
}

impl SqliteStoreBundle {
    /// Opens a SQLite store bundle rooted at one database file.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, agent_sdk_core::AgentError> {
        let path = path.into();
        let bundle = Self { path };
        let _ = bundle.journal()?;
        let _ = bundle.checkpoints()?;
        let _ = bundle.content()?;
        let _ = bundle.event_archive()?;
        let _ = bundle.provider_arguments()?;
        let _ = bundle.tool_execution()?;
        let _ = bundle.agent_pool()?;
        Ok(bundle)
    }

    /// Returns the backing database path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns a run journal adapter.
    pub fn journal(&self) -> Result<SqliteRunJournal, agent_sdk_core::AgentError> {
        SqliteRunJournal::open(&self.path)
    }

    /// Returns a checkpoint store adapter.
    pub fn checkpoints(&self) -> Result<SqliteCheckpointStore, agent_sdk_core::AgentError> {
        SqliteCheckpointStore::open(&self.path)
    }

    /// Returns a content store adapter.
    pub fn content(&self) -> Result<SqliteContentStore, agent_sdk_core::AgentError> {
        SqliteContentStore::open(&self.path)
    }

    /// Returns an event archive adapter.
    pub fn event_archive(&self) -> Result<SqliteEventArchive, agent_sdk_core::AgentError> {
        SqliteEventArchive::open(&self.path)
    }

    /// Returns a provider argument store adapter.
    pub fn provider_arguments(
        &self,
    ) -> Result<SqliteProviderArgumentStore, agent_sdk_core::AgentError> {
        SqliteProviderArgumentStore::open(&self.path)
    }

    /// Returns an agent-pool store adapter.
    pub fn agent_pool(&self) -> Result<SqliteAgentPoolStore, agent_sdk_core::AgentError> {
        SqliteAgentPoolStore::open(&self.path)
    }

    /// Returns a rebuildable tool-execution projection store adapter.
    pub fn tool_execution(&self) -> Result<SqliteToolExecutionStore, agent_sdk_core::AgentError> {
        SqliteToolExecutionStore::open(&self.path)
    }
}
