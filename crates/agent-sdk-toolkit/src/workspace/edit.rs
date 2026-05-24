use std::{fs, sync::Arc};

use agent_sdk_core::{
    AgentError, AgentErrorKind, ExecutorRef, PolicyRef, RetryClassification, ToolExecutionOutput,
    ToolExecutionRequest, ToolExecutor, ToolPackId, ToolPackKind, ToolPackSnapshot,
    policy::{CapabilityPermission, EffectClass, RiskClass},
};
use serde::{Deserialize, Serialize};

use super::{
    anchor::HashLineAnchor,
    bounds::BoundedWorkspace,
    policy::WorkspacePolicy,
    util::{content_ref_for, first_arg_ref, hash_bytes, hash_line, policy_denial, tool_failure},
};
use crate::{
    packs::{ToolkitPackBundle, tool_snapshot},
    testing::{InMemoryJsonArgumentStore, InMemoryToolkitContentStore},
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceEditRequest {
    pub path: String,
    pub anchor: HashLineAnchor,
    pub replacement: String,
    pub preview_only: bool,
    pub preview_hash: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceEditOutput {
    pub path: String,
    pub preview_only: bool,
    pub applied: bool,
    pub before_hash: String,
    pub after_hash: String,
    pub preview_hash: String,
    pub diff: String,
    pub inverse_candidate: Option<String>,
}

impl BoundedWorkspace {
    pub(super) fn edit(
        &self,
        request: &WorkspaceEditRequest,
    ) -> Result<WorkspaceEditOutput, AgentError> {
        let path = self.resolve_existing_file(&request.path)?;
        if fs::metadata(&path).map_err(tool_failure)?.len() > self.policy.max_file_bytes {
            return Err(policy_denial("workspace edit exceeds max_file_bytes"));
        }
        let before = fs::read_to_string(&path).map_err(tool_failure)?;
        let mut lines = before.lines().map(str::to_string).collect::<Vec<_>>();
        let index =
            request.anchor.line.checked_sub(1).ok_or_else(|| {
                AgentError::contract_violation("hashline anchor line is one-based")
            })?;
        let Some(current_line) = lines.get(index) else {
            return Err(AgentError::new(
                AgentErrorKind::ToolFailure,
                RetryClassification::UserActionNeeded,
                "hashline anchor is outside the current file",
            ));
        };
        if hash_line(current_line) != request.anchor.before_hash {
            return Err(AgentError::new(
                AgentErrorKind::PolicyDenial,
                RetryClassification::UserActionNeeded,
                "stale hashline anchor prevented workspace edit",
            ));
        }
        let before_hash = hash_bytes(before.as_bytes());
        let inverse = current_line.clone();
        lines[index] = request.replacement.clone();
        let mut after = lines.join("\n");
        if before.ends_with('\n') {
            after.push('\n');
        }
        let after_hash = hash_bytes(after.as_bytes());
        let diff = format!(
            "--- {}\n+++ {}\n@@ line {} @@\n-{}\n+{}",
            request.path, request.path, request.anchor.line, inverse, request.replacement
        );
        let preview_hash = hash_bytes(format!("{before_hash}\n{after_hash}\n{diff}").as_bytes());
        if !request.preview_only {
            if request.preview_hash.as_deref() != Some(preview_hash.as_str()) {
                return Err(policy_denial(
                    "workspace edit apply requires matching preview_hash",
                ));
            }
            fs::write(&path, after.as_bytes()).map_err(tool_failure)?;
        }
        Ok(WorkspaceEditOutput {
            path: request.path.clone(),
            preview_only: request.preview_only,
            applied: !request.preview_only,
            before_hash,
            after_hash,
            preview_hash,
            diff,
            inverse_candidate: Some(inverse),
        })
    }
}

#[derive(Clone)]
pub struct WorkspaceEditExecutor {
    executor_ref: ExecutorRef,
    workspace: Arc<BoundedWorkspace>,
    arguments: InMemoryJsonArgumentStore,
    content: InMemoryToolkitContentStore,
}

impl WorkspaceEditExecutor {
    pub fn new(
        workspace: Arc<BoundedWorkspace>,
        arguments: InMemoryJsonArgumentStore,
        content: InMemoryToolkitContentStore,
    ) -> Self {
        Self {
            executor_ref: ExecutorRef::new("executor.toolkit.workspace_edit.v1"),
            workspace,
            arguments,
            content,
        }
    }

    pub fn pack_bundle(
        source: agent_sdk_core::SourceRef,
        policy_ref: PolicyRef,
        workspace: &WorkspacePolicy,
    ) -> Result<ToolkitPackBundle, AgentError> {
        let snapshot = ToolPackSnapshot::new(
            ToolPackId::new("toolpack.workspace_edit.v1"),
            ToolPackKind::WorkspaceEdit,
            "v1",
            source.clone(),
        )
        .with_workspace_bounds(workspace.bounds_snapshot(policy_ref.clone()))
        .with_tool(tool_snapshot(
            "cap.toolkit.workspace_edit",
            "workspace_edit",
            "executor.toolkit.workspace_edit.v1",
            "schema.toolkit.workspace_edit.v1",
            vec![policy_ref],
            vec![CapabilityPermission::FilesystemWrite],
            EffectClass::Write,
            RiskClass::High,
            &source,
        ));
        ToolkitPackBundle::from_snapshot(snapshot)
    }
}

impl ToolExecutor for WorkspaceEditExecutor {
    fn executor_ref(&self) -> &ExecutorRef {
        &self.executor_ref
    }

    fn execute(&self, request: &ToolExecutionRequest) -> Result<ToolExecutionOutput, AgentError> {
        let args_ref = first_arg_ref(request)?;
        let edit_request: WorkspaceEditRequest = self.arguments.get(args_ref)?;
        let output = self.workspace.edit(&edit_request)?;
        let content_ref = content_ref_for(request, "workspace_edit");
        self.content.put(content_ref.clone(), &output)?;
        let mut envelope = ToolExecutionOutput::completed(if output.applied {
            "workspace edit applied with before and after hashes"
        } else {
            "workspace edit preview returned diff without writing"
        });
        envelope.content_refs.push(content_ref);
        envelope.reconciliation_ref = Some(format!("reconcile.{}", output.after_hash));
        Ok(envelope)
    }
}
