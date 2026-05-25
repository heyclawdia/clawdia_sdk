//! Shell tool executor. Use this module only when a host policy explicitly allows
//! command execution. Successful execution starts a host process and captures the
//! current buffered stdout/stderr result for a core tool output; hosts that run
//! untrusted commands should add output bounds before enabling this executor.
//!
use agent_sdk_core::{
    AgentError, EffectTerminalStatus, ExecutorRef, PolicyKind, PolicyRef, ToolExecutionOutput,
    ToolExecutionRequest, ToolExecutor, ToolPackId, ToolPackKind, ToolPackSnapshot,
    domain::ContentRef,
    policy::{CapabilityPermission, EffectClass, RiskClass},
};

use crate::{
    packs::{ToolkitPackBundle, tool_snapshot},
    testing::{InMemoryJsonArgumentStore, InMemoryToolkitContentStore},
};

use super::{command::run_command, policy::ShellExecutionPolicy, types::ShellRequest};

#[derive(Clone)]
/// Shell shell executor request or result value.
/// Creating the value does not spawn a process; shell executors document policy checks and command side effects.
pub struct ShellExecutor {
    executor_ref: ExecutorRef,
    policy: ShellExecutionPolicy,
    arguments: InMemoryJsonArgumentStore,
    content: InMemoryToolkitContentStore,
}

impl ShellExecutor {
    /// Creates a new shell::executor value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
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

    /// Pack bundle.
    /// This returns the toolkit pack bundle that registers the operation route; it does not
    /// execute the operation.
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
