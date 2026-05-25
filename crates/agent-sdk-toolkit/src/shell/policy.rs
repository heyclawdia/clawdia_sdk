//! Concrete shell tool helpers layered over core policy and effect contracts. Use
//! these modules only behind host approval, sandbox, timeout, and network policy.
//! Execution starts host processes; request and policy types are data-only. This file
//! contains the policy portion of that contract.
//!
use agent_sdk_core::{PolicyKind, PolicyRef};

#[derive(Clone, Debug)]
/// Shell shell execution policy request or result value.
/// Creating the value does not spawn a process; shell executors document policy checks and command side effects.
pub struct ShellExecutionPolicy {
    /// Typed sandbox policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub sandbox_policy_ref: PolicyRef,
    /// Boolean policy/capability flag for whether allow host execution is
    /// enabled.
    pub allow_host_execution: bool,
    /// Whether network enabled is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub network_enabled: bool,
    /// max timeout ms duration in milliseconds.
    pub max_timeout_ms: u64,
}

impl ShellExecutionPolicy {
    /// Returns an updated value with deny host execution configured.
    /// This constructs a policy that denies host shell execution and performs no command
    /// execution.
    pub fn deny_host_execution() -> Self {
        Self {
            sandbox_policy_ref: PolicyRef::with_kind(
                PolicyKind::Sandbox,
                "policy.sandbox.shell.deny_host",
            ),
            allow_host_execution: false,
            network_enabled: false,
            max_timeout_ms: 30_000,
        }
    }
}
