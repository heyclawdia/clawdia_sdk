//! Scripted Postgres-style durable store adapters for the Agent SDK.
//!
//! This crate proves SQL statement shape, bound parameters, row decoding, and
//! store-surface mapping without provisioning a live database. Hosts own
//! connections, migrations, RLS, backups, and retention policy.

pub mod agent_pool;
pub mod bundle;
pub mod checkpoint;
pub mod client;
pub mod content;
pub mod event_archive;
pub mod journal;
pub mod provider_arguments;
pub mod tool_execution;
mod util;

pub use agent_pool::PostgresAgentPoolStore;
pub use bundle::PostgresStoreBundle;
pub use checkpoint::PostgresCheckpointStore;
pub use client::{
    PostgresSqlRequest, PostgresSqlResponse, PostgresSqlTransport, PostgresStoreClient,
    PostgresStoreConfig,
};
pub use content::PostgresContentStore;
pub use event_archive::PostgresEventArchive;
pub use journal::PostgresRunJournal;
pub use provider_arguments::PostgresProviderArgumentStore;
pub use tool_execution::PostgresToolExecutionStore;
