//! Workspace write helper. Use this toolkit module for policy-bounded file creation
//! or replacement. Successful execution mutates files and returns content
//! refs/metadata for the effect result.
//!
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
/// Workspace workspace write request request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceWriteRequest {
    /// Workspace-relative or resource path selected by the request or result.
    pub path: String,
    /// UTF-8 contents that the workspace write executor will place at the target path.
    /// Use the write mode to decide whether these bytes create, overwrite, or preview the file.
    pub contents: String,
    /// Mode that selects how this operation or contract should behave.
    /// Callers use it to choose the explicit execution path instead of relying on hidden
    /// defaults.
    pub mode: WorkspaceWriteMode,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite workspace write mode cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum WorkspaceWriteMode {
    /// Use this variant when the contract needs to represent create new; selecting it has no side effect by itself.
    CreateNew,
    /// Use this variant when the contract needs to represent overwrite; selecting it has no side effect by itself.
    Overwrite,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Workspace workspace write output request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceWriteOutput {
    /// Workspace-relative or resource path selected by the request or result.
    pub path: String,
    /// Whether created is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub created: bool,
    /// Whether overwritten is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub overwritten: bool,
    /// Deterministic before hash used for stale checks, package evidence, or
    /// replay comparisons.
    pub before_hash: Option<String>,
    /// Deterministic after hash used for stale checks, package evidence, or
    /// replay comparisons.
    pub after_hash: String,
    /// Optional non reversible reason value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub non_reversible_reason: Option<String>,
}

impl BoundedWorkspace {
    /// Write.
    /// This writes to the policy-resolved workspace path and may create or overwrite files only
    /// according to the request mode.
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
/// Workspace workspace write executor request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceWriteExecutor {
    executor_ref: ExecutorRef,
    workspace: Arc<BoundedWorkspace>,
    arguments: InMemoryJsonArgumentStore,
    content: InMemoryToolkitContentStore,
}

impl WorkspaceWriteExecutor {
    /// Creates a new workspace::write value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
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

    /// Pack bundle.
    /// This returns the toolkit pack bundle that registers the operation route; it does not
    /// execute the operation.
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
