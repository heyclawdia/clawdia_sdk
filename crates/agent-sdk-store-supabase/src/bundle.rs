use crate::{
    SupabaseAgentPoolStore, SupabaseCheckpointStore, SupabaseClient, SupabaseContentStore,
    SupabaseEventArchive, SupabaseProviderArgumentStore, SupabaseRunJournal,
};

#[derive(Clone)]
/// Supabase-backed store bundle sharing one client.
pub struct SupabaseStoreBundle {
    client: SupabaseClient,
}

impl SupabaseStoreBundle {
    pub fn new(client: SupabaseClient) -> Self {
        Self { client }
    }

    pub fn journal(&self) -> SupabaseRunJournal {
        SupabaseRunJournal::new(self.client.clone())
    }

    pub fn checkpoints(&self) -> SupabaseCheckpointStore {
        SupabaseCheckpointStore::new(self.client.clone())
    }

    pub fn content(&self) -> SupabaseContentStore {
        SupabaseContentStore::new(self.client.clone())
    }

    pub fn event_archive(&self) -> SupabaseEventArchive {
        SupabaseEventArchive::new(self.client.clone())
    }

    pub fn provider_arguments(&self) -> SupabaseProviderArgumentStore {
        SupabaseProviderArgumentStore::new(self.client.clone())
    }

    pub fn agent_pool(&self) -> SupabaseAgentPoolStore {
        SupabaseAgentPoolStore::new(self.client.clone())
    }
}
