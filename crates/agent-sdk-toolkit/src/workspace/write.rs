use std::{fs, sync::Arc};

use agent_sdk_core::{
    AgentError, ExecutorRef, PolicyRef, ToolExecutionOutput, ToolExecutionRequest, ToolExecutor,
    ToolPackId, ToolPackKind, ToolPackSnapshot,
    policy::{CapabilityPermission, EffectClass, RiskClass},
};
use serde::{Deserialize, Serialize};

use super::{
    bounds::BoundedWorkspace,
    policy::WorkspacePolicy,
    util::{content_ref_for, first_arg_ref, hash_bytes, policy_denial, tool_failure},
};
use crate::{
    packs::{ToolkitPackBundle, tool_snapshot},
    testing::{InMemoryJsonArgumentStore, InMemoryToolkitContentStore},
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceWriteRequest {
    pub path: String,
    pub contents: String,
    pub mode: WorkspaceWriteMode,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceWriteMode {
    CreateNew,
    Overwrite,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceWriteOutput {
    pub path: String,
    pub created: bool,
    pub overwritten: bool,
    pub before_hash: Option<String>,
    pub after_hash: String,
    pub non_reversible_reason: Option<String>,
}

impl BoundedWorkspace {
    pub(super) fn write(
        &self,
        request: &WorkspaceWriteRequest,
    ) -> Result<WorkspaceWriteOutput, AgentError> {
        if request.contents.len() as u64 > self.policy.max_file_bytes {
            return Err(policy_denial("workspace write exceeds max_file_bytes"));
        }
        let path = self.resolve_for_write(&request.path)?;
        let exists = path.exists();
        match (&request.mode, exists) {
            (WorkspaceWriteMode::CreateNew, true) => {
                return Err(policy_denial(
                    "workspace write create_new refused existing path",
                ));
            }
            (WorkspaceWriteMode::CreateNew, false) if !self.policy.allow_create => {
                return Err(policy_denial("workspace write create scope is not granted"));
            }
            (WorkspaceWriteMode::Overwrite, false) if !self.policy.allow_create => {
                return Err(policy_denial("workspace write missing create scope"));
            }
            (WorkspaceWriteMode::Overwrite, true) if !self.policy.allow_overwrite => {
                return Err(policy_denial(
                    "workspace write overwrite scope is not granted",
                ));
            }
            _ => {}
        }
        let before_hash = if exists {
            if fs::metadata(&path).map_err(tool_failure)?.len() > self.policy.max_file_bytes {
                return Err(policy_denial(
                    "workspace write existing file exceeds max_file_bytes",
                ));
            }
            Some(hash_bytes(&fs::read(&path).map_err(tool_failure)?))
        } else {
            None
        };
        fs::write(&path, request.contents.as_bytes()).map_err(tool_failure)?;
        let after_hash = hash_bytes(request.contents.as_bytes());
        Ok(WorkspaceWriteOutput {
            path: request.path.clone(),
            created: !exists,
            overwritten: exists,
            before_hash,
            after_hash,
            non_reversible_reason: if exists {
                None
            } else {
                Some("created file has no prior content to restore automatically".to_string())
            },
        })
    }
}

#[derive(Clone)]
pub struct WorkspaceWriteExecutor {
    executor_ref: ExecutorRef,
    workspace: Arc<BoundedWorkspace>,
    arguments: InMemoryJsonArgumentStore,
    content: InMemoryToolkitContentStore,
}

impl WorkspaceWriteExecutor {
    pub fn new(
        workspace: Arc<BoundedWorkspace>,
        arguments: InMemoryJsonArgumentStore,
        content: InMemoryToolkitContentStore,
    ) -> Self {
        Self {
            executor_ref: ExecutorRef::new("executor.toolkit.workspace_write.v1"),
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
            ToolPackId::new("toolpack.workspace_write.v1"),
            ToolPackKind::WorkspaceWrite,
            "v1",
            source.clone(),
        )
        .with_workspace_bounds(workspace.bounds_snapshot(policy_ref.clone()))
        .with_tool(tool_snapshot(
            "cap.toolkit.workspace_write",
            "workspace_write",
            "executor.toolkit.workspace_write.v1",
            "schema.toolkit.workspace_write.v1",
            vec![policy_ref],
            vec![CapabilityPermission::FilesystemWrite],
            EffectClass::Write,
            RiskClass::High,
            &source,
        ));
        ToolkitPackBundle::from_snapshot(snapshot)
    }
}

impl ToolExecutor for WorkspaceWriteExecutor {
    fn executor_ref(&self) -> &ExecutorRef {
        &self.executor_ref
    }

    fn execute(&self, request: &ToolExecutionRequest) -> Result<ToolExecutionOutput, AgentError> {
        let args_ref = first_arg_ref(request)?;
        let write_request: WorkspaceWriteRequest = self.arguments.get(args_ref)?;
        let output = self.workspace.write(&write_request)?;
        let content_ref = content_ref_for(request, "workspace_write");
        self.content.put(content_ref.clone(), &output)?;
        let mut envelope =
            ToolExecutionOutput::completed("workspace write recorded before and after metadata");
        envelope.content_refs.push(content_ref);
        envelope.reconciliation_ref = Some(format!("reconcile.{}", output.after_hash));
        Ok(envelope)
    }
}
