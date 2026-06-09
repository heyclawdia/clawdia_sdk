use crate::{
    PostgresAgentPoolStore, PostgresCheckpointStore, PostgresContentStore, PostgresEventArchive,
    PostgresProviderArgumentStore, PostgresRunJournal, PostgresStoreClient,
    PostgresToolExecutionStore,
};

#[derive(Clone)]
/// Postgres-style store bundle sharing one client.
pub struct PostgresStoreBundle {
    client: PostgresStoreClient,
}

impl PostgresStoreBundle {
    /// Creates a store bundle over a host-owned SQL transport.
    pub fn new(client: PostgresStoreClient) -> Self {
        Self { client }
    }

    pub fn journal(&self) -> PostgresRunJournal {
        PostgresRunJournal::new(self.client.clone())
    }

    pub fn checkpoints(&self) -> PostgresCheckpointStore {
        PostgresCheckpointStore::new(self.client.clone())
    }

    pub fn content(&self) -> PostgresContentStore {
        PostgresContentStore::new(self.client.clone())
    }

    pub fn event_archive(&self) -> PostgresEventArchive {
        PostgresEventArchive::new(self.client.clone())
    }

    pub fn agent_pool(&self) -> PostgresAgentPoolStore {
        PostgresAgentPoolStore::new(self.client.clone())
    }

    pub fn tool_execution(&self) -> PostgresToolExecutionStore {
        PostgresToolExecutionStore::new(self.client.clone())
    }

    pub fn provider_arguments(&self) -> PostgresProviderArgumentStore {
        PostgresProviderArgumentStore::new(self.client.clone())
    }
}
