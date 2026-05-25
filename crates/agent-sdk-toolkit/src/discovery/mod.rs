//! Tool discovery helpers for optional toolkit capabilities. Use these modules to
//! index hidden candidates, return model-facing discovery results, and construct
//! package deltas for host-approved activation. Searching is data-only; package
//! mutation happens only when a delta is applied.
//!
mod executor;
mod index;
mod policy;
mod types;

pub use executor::ToolDiscoveryExecutor;
pub use index::ToolDiscoveryIndex;
pub use policy::discovery_policy;
pub use types::{ToolDiscoveryCandidate, ToolDiscoveryOutput, ToolDiscoveryRequest};
