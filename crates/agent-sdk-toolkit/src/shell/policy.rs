use agent_sdk_core::{PolicyKind, PolicyRef};

#[derive(Clone, Debug)]
pub struct ShellExecutionPolicy {
    pub sandbox_policy_ref: PolicyRef,
    pub allow_host_execution: bool,
    pub network_enabled: bool,
    pub max_timeout_ms: u64,
}

impl ShellExecutionPolicy {
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
