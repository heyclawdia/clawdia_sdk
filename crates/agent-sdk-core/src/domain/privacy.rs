//! Domain primitives for stable SDK vocabulary. Use these items for IDs, refs,
//! policy, privacy, trust, and errors that cross crate or host boundaries. They are
//! data-only and must not perform provider, filesystem, network, or UI side effects.
//! This file contains the privacy portion of that contract.
//!
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite privacy class cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum PrivacyClass {
    /// Use this variant when the contract needs to represent public; selecting it has no side effect by itself.
    Public,
    /// Use this variant when the contract needs to represent internal; selecting it has no side effect by itself.
    Internal,
    /// Use this variant when the contract needs to represent content refs only; selecting it has no side effect by itself.
    ContentRefsOnly,
    /// Use this variant when the contract needs to represent sensitive; selecting it has no side effect by itself.
    Sensitive,
    /// Use this variant when the contract needs to represent secret; selecting it has no side effect by itself.
    Secret,
}

impl PrivacyClass {
    /// Returns whether allows raw content by default applies for this contract.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn allows_raw_content_by_default(self) -> bool {
        matches!(self, Self::Public)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite retention class cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RetentionClass {
    /// Use this variant when the contract needs to represent ephemeral; selecting it has no side effect by itself.
    Ephemeral,
    /// Use this variant when the contract needs to represent run scoped; selecting it has no side effect by itself.
    RunScoped,
    /// Use this variant when the contract needs to represent session scoped; selecting it has no side effect by itself.
    SessionScoped,
    /// Use this variant when the contract needs to represent durable; selecting it has no side effect by itself.
    Durable,
    /// Use this variant when the contract needs to represent persistent; selecting it has no side effect by itself.
    Persistent,
    /// Use this variant when the contract needs to represent host policy; selecting it has no side effect by itself.
    HostPolicy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite trust class cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum TrustClass {
    /// Use this variant when the contract needs to represent trusted; selecting it has no side effect by itself.
    Trusted,
    /// Use this variant when the contract needs to represent sdk generated; selecting it has no side effect by itself.
    SdkGenerated,
    /// Use this variant when the contract needs to represent host provided; selecting it has no side effect by itself.
    HostProvided,
    /// Use this variant when the contract needs to represent user provided; selecting it has no side effect by itself.
    UserProvided,
    /// Use this variant when the contract needs to represent external; selecting it has no side effect by itself.
    External,
    /// Use this variant when the contract needs to represent untrusted; selecting it has no side effect by itself.
    Untrusted,
}
