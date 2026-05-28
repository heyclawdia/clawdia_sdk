//! Environment profile helpers layered over the core isolation contract.
//! These builders are data-only: they produce core `ExecutionEnvironment`
//! values, isolation sidecars, and network policy DTOs without starting
//! containers, opening sockets, or configuring host firewalls.
mod egress;
mod profile;
mod runtime;

pub use egress::{EgressAllowlist, EgressProtocol, EgressTarget};
pub use profile::{AgentWorkspaceEnvironment, AgentWorkspaceEnvironmentProfile};
pub use runtime::EnvironmentRuntime;
