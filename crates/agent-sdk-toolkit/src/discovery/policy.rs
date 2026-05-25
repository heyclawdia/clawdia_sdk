//! Tool discovery helpers for optional toolkit capabilities. Use these modules to
//! index hidden candidates, return model-facing discovery results, and construct
//! package deltas for host-approved activation. Searching is data-only; package
//! mutation happens only when a delta is applied. This file contains the policy
//! portion of that contract.
//!
use agent_sdk_core::{PolicyKind, PolicyRef};

/// Returns discovery policy for the current value.
/// This is a read-only or data-construction helper unless the method body explicitly calls a
/// port or store.
pub fn discovery_policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::RuntimePackage, id)
}
