//! Supabase REST-backed durable store adapters for the Agent SDK.
//!
//! The crate owns only transport and storage adaptation. Core remains
//! authoritative for run execution, provider routing, tool execution,
//! journaling semantics, and redaction contracts.

pub mod agent_pool;
pub mod auth;
pub mod bundle;
pub mod checkpoint;
pub mod client;
pub mod config;
pub mod content;
pub mod event_archive;
pub mod journal;
pub mod provider_arguments;
pub mod transport;

pub use agent_pool::SupabaseAgentPoolStore;
pub use auth::SupabaseAuth;
pub use bundle::SupabaseStoreBundle;
pub use checkpoint::SupabaseCheckpointStore;
pub use client::SupabaseClient;
pub use config::SupabaseStoreConfig;
pub use content::SupabaseContentStore;
pub use event_archive::SupabaseEventArchive;
pub use journal::SupabaseRunJournal;
pub use provider_arguments::SupabaseProviderArgumentStore;
pub use transport::{SupabaseHttpRequest, SupabaseHttpResponse, SupabaseHttpTransport};
