//! Concrete shell tool helpers layered over core policy and effect contracts. Use
//! these modules only behind host approval, sandbox, timeout, and network policy.
//! Execution starts host processes; request and policy types are data-only.
//!
mod command;
mod executor;
mod policy;
mod types;

pub use executor::ShellExecutor;
pub use policy::ShellExecutionPolicy;
pub use types::{ShellRequest, ShellResult};
