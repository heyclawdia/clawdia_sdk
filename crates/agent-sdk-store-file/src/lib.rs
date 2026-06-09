//! Filesystem-backed durable store adapters for the Agent SDK.
//!
//! Each adapter implements a canonical SDK port and stores contract DTOs as
//! JSON or NDJSON under a caller-provided root. The crate does not call model
//! providers, execute tools, or synthesize journal truth.

pub mod agent_pool;
pub mod bundle;
pub mod checkpoint;
pub mod content;
pub mod event_archive;
pub mod journal;
pub mod provider_arguments;
pub mod tool_execution;
mod util;

pub use agent_pool::FileAgentPoolStore;
pub use bundle::FileStoreBundle;
pub use checkpoint::FileCheckpointStore;
pub use content::FileContentStore;
pub use event_archive::FileEventArchive;
pub use journal::FileRunJournal;
pub use provider_arguments::FileProviderArgumentStore;
pub use tool_execution::FileToolExecutionStore;
