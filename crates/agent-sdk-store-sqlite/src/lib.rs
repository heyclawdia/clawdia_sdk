//! SQLite-backed durable store adapters for the Agent SDK.
//!
//! This crate stores canonical SDK DTOs in explicit SQLite tables. It does not
//! execute tools, call providers, own approval policy, or replace the run
//! journal as durable truth.

pub mod agent_pool;
pub mod bundle;
pub mod checkpoint;
pub mod content;
pub mod event_archive;
pub mod journal;
pub mod provider_arguments;
pub mod tool_execution;
mod util;

pub use agent_pool::SqliteAgentPoolStore;
pub use bundle::SqliteStoreBundle;
pub use checkpoint::SqliteCheckpointStore;
pub use content::SqliteContentStore;
pub use event_archive::SqliteEventArchive;
pub use journal::SqliteRunJournal;
pub use provider_arguments::SqliteProviderArgumentStore;
pub use tool_execution::SqliteToolExecutionStore;
