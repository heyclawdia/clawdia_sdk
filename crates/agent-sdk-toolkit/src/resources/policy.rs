//! Resource-reader helpers layered over explicit URI resolvers and core content refs.
//! Use these modules when a host wants toolkit tools to read approved resources.
//! Resolver implementations own any backing-store or network side effects. This file
//! contains the policy portion of that contract.
//!
use agent_sdk_core::{PolicyKind, PolicyRef};

/// Builds the memory read policy value.
/// This is data construction and performs no I/O, journal append, event publication, or process
pub fn memory_read_policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::Permission, id)
}
