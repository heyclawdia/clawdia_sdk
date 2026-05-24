use std::{fs, sync::Arc};

use agent_sdk_core::{
    AgentError, AgentErrorKind, ExecutorRef, PolicyRef, RetryClassification, ToolExecutionOutput,
    ToolExecutionRequest, ToolExecutor, ToolPackId, ToolPackKind, ToolPackSnapshot,
    policy::{CapabilityPermission, EffectClass, RiskClass},
};
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::{
    bounds::BoundedWorkspace,
    policy::WorkspacePolicy,
    read_pipeline::detect_workspace_file,
    util::{content_ref_for, first_arg_ref, hash_line, tool_failure, truncate_bytes},
};
use crate::{
    packs::{ToolkitPackBundle, tool_snapshot},
    testing::{InMemoryJsonArgumentStore, InMemoryToolkitContentStore},
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceSearchRequest {
    pub pattern: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceSearchOutput {
    pub pattern: String,
    pub matches: Vec<SearchMatch>,
    pub truncated: bool,
    pub max_matches: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SearchMatch {
    pub path: String,
    pub line: usize,
    pub line_hash: String,
    pub preview: String,
}

impl BoundedWorkspace {
    pub fn search(
        &self,
        request: &WorkspaceSearchRequest,
    ) -> Result<WorkspaceSearchOutput, AgentError> {
        if request.pattern.is_empty() {
            return Err(AgentError::new(
                AgentErrorKind::ToolFailure,
                RetryClassification::UserActionNeeded,
                "regex pattern must not be empty",
            ));
        }
        let regex = Regex::new(&request.pattern).map_err(|error| {
            AgentError::new(
                AgentErrorKind::ToolFailure,
                RetryClassification::UserActionNeeded,
                format!("regex compile error: {error}"),
            )
        })?;
        let mut matches = Vec::new();
        self.visit_files(&self.policy.root, &mut |path| {
            if matches.len() >= self.policy.max_matches {
                return Ok(());
            }
            let rel = self.relative_path(path)?;
            let byte_len = fs::metadata(path).map_err(tool_failure)?.len();
            if byte_len > self.policy.max_file_bytes {
                return Ok(());
            }
            let bytes = fs::read(path).map_err(tool_failure)?;
            if detect_workspace_file(path, &bytes).binary {
                return Ok(());
            }
            let text = String::from_utf8(bytes).map_err(|error| {
                AgentError::new(
                    AgentErrorKind::ToolFailure,
                    RetryClassification::UserActionNeeded,
                    format!("workspace search expected UTF-8 text after detection: {error}"),
                )
            })?;
            for (index, line) in text.lines().enumerate() {
                if regex.is_match(line) {
                    matches.push(SearchMatch {
                        path: rel.clone(),
                        line: index + 1,
                        line_hash: hash_line(line),
                        preview: truncate_bytes(line, self.policy.max_output_bytes as usize),
                    });
                    if matches.len() >= self.policy.max_matches {
                        break;
                    }
                }
            }
            Ok(())
        })?;
        let truncated = matches.len() >= self.policy.max_matches;
        Ok(WorkspaceSearchOutput {
            pattern: request.pattern.clone(),
            matches,
            truncated,
            max_matches: self.policy.max_matches,
        })
    }
}

#[derive(Clone)]
pub struct WorkspaceSearchExecutor {
    executor_ref: ExecutorRef,
    workspace: Arc<BoundedWorkspace>,
    arguments: InMemoryJsonArgumentStore,
    content: InMemoryToolkitContentStore,
}

impl WorkspaceSearchExecutor {
    pub fn new(
        workspace: Arc<BoundedWorkspace>,
        arguments: InMemoryJsonArgumentStore,
        content: InMemoryToolkitContentStore,
    ) -> Self {
        Self {
            executor_ref: ExecutorRef::new("executor.toolkit.workspace_search.v1"),
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
            ToolPackId::new("toolpack.workspace_search.v1"),
            ToolPackKind::WorkspaceSearch,
            "v1",
            source.clone(),
        )
        .with_workspace_bounds(workspace.bounds_snapshot(policy_ref.clone()))
        .with_tool(tool_snapshot(
            "cap.toolkit.workspace_search",
            "workspace_search",
            "executor.toolkit.workspace_search.v1",
            "schema.toolkit.workspace_search.v1",
            vec![policy_ref],
            vec![CapabilityPermission::FilesystemRead],
            EffectClass::Read,
            RiskClass::Low,
            &source,
        ));
        ToolkitPackBundle::from_snapshot(snapshot)
    }
}

impl ToolExecutor for WorkspaceSearchExecutor {
    fn executor_ref(&self) -> &ExecutorRef {
        &self.executor_ref
    }

    fn execute(&self, request: &ToolExecutionRequest) -> Result<ToolExecutionOutput, AgentError> {
        let args_ref = first_arg_ref(request)?;
        let search_request: WorkspaceSearchRequest = self.arguments.get(args_ref)?;
        let output = self.workspace.search(&search_request)?;
        let content_ref = content_ref_for(request, "workspace_search");
        self.content.put(content_ref.clone(), &output)?;
        let mut envelope = ToolExecutionOutput::completed("workspace search returned content ref");
        envelope.content_refs.push(content_ref);
        Ok(envelope)
    }
}
