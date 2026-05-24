use std::{
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use agent_sdk_core::{
    AgentError, AgentErrorKind, EffectTerminalStatus, ExecutorRef, PolicyKind, PolicyRef,
    RetryClassification, ToolExecutionOutput, ToolExecutionRequest, ToolExecutor, ToolPackId,
    ToolPackKind, ToolPackSnapshot,
    domain::ContentRef,
    policy::{CapabilityPermission, EffectClass, RiskClass},
};
use serde::{Deserialize, Serialize};

use crate::{
    packs::{ToolkitPackBundle, tool_snapshot},
    testing::{InMemoryJsonArgumentStore, InMemoryToolkitContentStore},
};

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

#[derive(Clone)]
pub struct ShellExecutor {
    executor_ref: ExecutorRef,
    policy: ShellExecutionPolicy,
    arguments: InMemoryJsonArgumentStore,
    content: InMemoryToolkitContentStore,
}

impl ShellExecutor {
    pub fn new(
        policy: ShellExecutionPolicy,
        arguments: InMemoryJsonArgumentStore,
        content: InMemoryToolkitContentStore,
    ) -> Self {
        Self {
            executor_ref: ExecutorRef::new("executor.toolkit.shell.v1"),
            policy,
            arguments,
            content,
        }
    }

    pub fn pack_bundle(
        source: agent_sdk_core::SourceRef,
        policy_ref: PolicyRef,
    ) -> Result<ToolkitPackBundle, AgentError> {
        let snapshot = ToolPackSnapshot::new(
            ToolPackId::new("toolpack.shell.v1"),
            ToolPackKind::Shell,
            "v1",
            source.clone(),
        )
        .with_tool(tool_snapshot(
            "cap.toolkit.shell",
            "shell",
            "executor.toolkit.shell.v1",
            "schema.toolkit.shell.v1",
            vec![
                policy_ref,
                PolicyRef::with_kind(PolicyKind::Sandbox, "policy.sandbox.shell.required"),
            ],
            vec![CapabilityPermission::Shell],
            EffectClass::Process,
            RiskClass::High,
            &source,
        ));
        ToolkitPackBundle::from_snapshot(snapshot)
    }
}

impl ToolExecutor for ShellExecutor {
    fn executor_ref(&self) -> &ExecutorRef {
        &self.executor_ref
    }

    fn execute(&self, request: &ToolExecutionRequest) -> Result<ToolExecutionOutput, AgentError> {
        let args_ref = request
            .effect_intent
            .content_refs
            .first()
            .ok_or_else(|| AgentError::missing_required_field("shell.argument_content_ref"))?;
        let shell_request: ShellRequest = self.arguments.get(args_ref)?;
        if shell_request.cancel_before_start {
            return Ok(ToolExecutionOutput {
                terminal_status: EffectTerminalStatus::Cancelled,
                content_refs: Vec::new(),
                redacted_summary: "shell execution cancelled before process start".to_string(),
                external_operation_id: None,
                reconciliation_ref: None,
                error_ref: None,
            });
        }
        if shell_request.argv.is_empty() {
            return Ok(ToolExecutionOutput::failed(
                "shell argv must be structured and non-empty",
                "shell.argv.empty",
            ));
        }
        if shell_request.timeout_ms == 0 || shell_request.timeout_ms > self.policy.max_timeout_ms {
            return Ok(ToolExecutionOutput::failed(
                "shell requires timeout within sandbox policy",
                "shell.timeout.policy",
            ));
        }
        if shell_request.network && !self.policy.network_enabled {
            return Ok(ToolExecutionOutput::failed(
                "shell network access denied by sandbox policy",
                "shell.network.policy",
            ));
        }
        if !self.policy.allow_host_execution {
            return Ok(ToolExecutionOutput::failed(
                "shell host execution denied by sandbox policy",
                self.policy.sandbox_policy_ref.as_str(),
            ));
        }

        let result = run_command(&shell_request)?;
        let content_ref = ContentRef::new(format!(
            "content.{}.shell",
            request.resolved_call.request.tool_call_id.as_str()
        ));
        self.content.put(content_ref.clone(), &result)?;
        let mut output = ToolExecutionOutput::completed("shell process completed with output refs");
        if result.exit_code != Some(0) {
            output.terminal_status = EffectTerminalStatus::Failed;
        }
        output.content_refs.push(content_ref);
        output.external_operation_id = Some("process.agent_owned".to_string());
        output.reconciliation_ref = Some("process.exit_status.recorded".to_string());
        Ok(output)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShellRequest {
    pub argv: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    pub timeout_ms: u64,
    pub network: bool,
    pub cancel_before_start: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShellResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub agent_owned: bool,
}

fn run_command(request: &ShellRequest) -> Result<ShellResult, AgentError> {
    let mut command = Command::new(&request.argv[0]);
    command.args(&request.argv[1..]);
    if let Some(cwd) = &request.cwd {
        command.current_dir(cwd);
    }
    for (key, value) in &request.env {
        command.env(key, value);
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(tool_failure)?;
    let deadline = Instant::now() + Duration::from_millis(request.timeout_ms);
    loop {
        if child.try_wait().map_err(tool_failure)?.is_some() {
            let output = child.wait_with_output().map_err(tool_failure)?;
            return Ok(ShellResult {
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                timed_out: false,
                agent_owned: true,
            });
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let output = child.wait_with_output().map_err(tool_failure)?;
            return Ok(ShellResult {
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                timed_out: true,
                agent_owned: true,
            });
        }
        thread::sleep(Duration::from_millis(5));
    }
}

fn tool_failure(error: std::io::Error) -> AgentError {
    AgentError::new(
        AgentErrorKind::ToolFailure,
        RetryClassification::UserActionNeeded,
        error.to_string(),
    )
}
