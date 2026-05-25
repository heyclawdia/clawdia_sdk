//! Concrete shell tool helpers layered over core policy and effect contracts. Use
//! these modules only behind host approval, sandbox, timeout, and network policy.
//! Execution starts host processes; request and policy types are data-only. This file
//! contains the types portion of that contract.
//!
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Shell shell request request or result value.
/// Creating the value does not spawn a process; shell executors document policy checks and command side effects.
pub struct ShellRequest {
    /// Command and arguments requested for shell execution. The first element
    /// is the executable path/name.
    pub argv: Vec<String>,
    /// Working directory requested for command execution; hosts must keep it
    /// inside approved bounds.
    pub cwd: Option<PathBuf>,
    /// Environment overrides requested for shell execution. Hosts should
    /// treat values as sensitive unless policy says otherwise.
    pub env: Vec<(String, String)>,
    /// Timeout budget in milliseconds for the requested operation.
    pub timeout_ms: u64,
    /// Whether the request asks for network access. Host sandbox policy is
    /// still authoritative.
    pub network: bool,
    /// Whether the shell request should be cancelled before process launch.
    pub cancel_before_start: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Shell shell result request or result value.
/// Creating the value does not spawn a process; shell executors document policy checks and command side effects.
pub struct ShellResult {
    /// Process exit status when the process reported one.
    pub exit_code: Option<i32>,
    /// Captured standard output. Current shell execution captures the full
    /// buffered stream; hosts should add bounds before using it with
    /// untrusted commands.
    pub stdout: String,
    /// Captured standard error. Current shell execution captures the full
    /// buffered stream; hosts should add bounds before using it with
    /// untrusted commands.
    pub stderr: String,
    /// Whether execution ended because the timeout budget elapsed.
    pub timed_out: bool,
    /// Whether the SDK/tooling owns the launched process lifecycle for
    /// cancellation and cleanup evidence.
    pub agent_owned: bool,
}
