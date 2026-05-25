//! Tool discovery helpers for optional toolkit capabilities. Use these modules to
//! index hidden candidates, return model-facing discovery results, and construct
//! package deltas for host-approved activation. Searching is data-only; package
//! mutation happens only when a delta is applied. This file contains the types
//! portion of that contract.
//!
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Discovery tool discovery request request or result value.
/// Creating the value does not register tools; discovery executors document catalog and package-bundle effects.
pub struct ToolDiscoveryRequest {
    /// Search query supplied by the caller.
    pub query: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Discovery tool discovery output request or result value.
/// Creating the value does not register tools; discovery executors document catalog and package-bundle effects.
pub struct ToolDiscoveryOutput {
    /// Search query supplied by the caller.
    pub query: String,
    /// Candidate capabilities, tools, resources, or package entries exposed
    /// for host-approved selection.
    pub candidates: Vec<ToolDiscoveryCandidate>,
    /// Whether activation must be applied as a package delta before use.
    pub package_delta_required: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Discovery tool discovery candidate request or result value.
/// Creating the value does not register tools; discovery executors document catalog and package-bundle effects.
pub struct ToolDiscoveryCandidate {
    /// Stable pack id used for typed lineage, lookup, or dedupe.
    pub pack_id: String,
    /// Tool names exposed in a discovery result.
    pub tool_names: Vec<String>,
    /// Whether activation must be applied as a package delta before use.
    pub package_delta_required: bool,
}
