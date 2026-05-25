//! SQLite-backed agent-pool coordination store.
//!
//! This module provides an optional concrete `AgentPoolStore` adapter. Core owns
//! the portable pool records and semantics; this toolkit module owns SQLite I/O.

mod sqlite_store;

pub use sqlite_store::SqliteAgentPoolStore;
