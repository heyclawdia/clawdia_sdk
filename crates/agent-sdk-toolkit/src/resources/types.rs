//! Resource-reader helpers layered over explicit URI resolvers and core content refs.
//! Use these modules when a host wants toolkit tools to read approved resources.
//! Resolver implementations own any backing-store or network side effects. This file
//! contains the types portion of that contract.
//!
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Resource resource reader request request or result value.
/// Creating the value does not fetch content; resource executors document resolver and content-store effects.
pub struct ResourceReaderRequest {
    /// Resource URI selected for explicit resolution.
    pub uri: String,
    /// Maximum byte budget the caller requested before truncation or summary
    /// behavior is applied.
    pub max_bytes: u64,
}
