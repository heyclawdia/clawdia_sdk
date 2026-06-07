use std::path::{Path, PathBuf};

use crate::{
    FileAgentPoolStore, FileCheckpointStore, FileContentStore, FileEventArchive,
    FileProviderArgumentStore, FileRunJournal,
};

#[derive(Clone, Debug)]
/// Filesystem-backed store bundle sharing one root directory.
pub struct FileStoreBundle {
    root: PathBuf,
}

impl FileStoreBundle {
    /// Creates a store bundle rooted under the provided directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Returns the root directory for this bundle.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns a run journal adapter.
    pub fn journal(&self) -> FileRunJournal {
        FileRunJournal::new(self.root.clone())
    }

    /// Returns a checkpoint store adapter.
    pub fn checkpoints(&self) -> FileCheckpointStore {
        FileCheckpointStore::new(self.root.clone())
    }

    /// Returns a content store adapter.
    pub fn content(&self) -> FileContentStore {
        FileContentStore::new(self.root.clone())
    }

    /// Returns an event archive adapter.
    pub fn event_archive(&self) -> FileEventArchive {
        FileEventArchive::new(self.root.clone())
    }

    /// Returns a provider argument store adapter.
    pub fn provider_arguments(&self) -> FileProviderArgumentStore {
        FileProviderArgumentStore::new(self.root.clone())
    }

    /// Returns an agent-pool store adapter.
    pub fn agent_pool(&self) -> FileAgentPoolStore {
        FileAgentPoolStore::new(self.root.clone())
    }
}
