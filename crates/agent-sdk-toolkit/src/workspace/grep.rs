//! Workspace search helpers. Use this toolkit module for bounded regex or glob-like
//! discovery before narrowing a read or edit. Searches read the workspace but do not
//! mutate files.
//!
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
/// Workspace workspace search request request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceSearchRequest {
    /// Search pattern supplied by the caller.
    /// The grep executor compiles it under regex and output bounds before reading files.
    pub pattern: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Workspace workspace search output request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceSearchOutput {
    /// Search pattern supplied by the caller.
    /// The grep executor compiles it under regex and output bounds before reading files.
    pub pattern: String,
    /// Collection of matches values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub matches: Vec<SearchMatch>,
    /// Whether output was shortened by byte, item, page, archive, or parser
    /// limits.
    pub truncated: bool,
    /// Maximum number of matches to return.
    /// Use it to keep search output bounded for model context.
    pub max_matches: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Workspace search match request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct SearchMatch {
    /// Workspace-relative or resource path selected by the request or result.
    pub path: String,
    /// Line used by this record or request.
    pub line: usize,
    /// Deterministic line hash used for stale checks, package evidence, or
    /// replay comparisons.
    pub line_hash: String,
    /// Human-readable preview for a search, edit, or write result.
    /// It is bounded display data and should not be treated as durable file contents.
    pub preview: String,
}

impl BoundedWorkspace {
    /// Searches bounded workspace files under the configured policy.
    /// This reads directory metadata and matching file contents, returns
    /// bounded previews, and never mutates workspace files.
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
/// Workspace workspace search executor request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceSearchExecutor {
    executor_ref: ExecutorRef,
    workspace: Arc<BoundedWorkspace>,
    arguments: InMemoryJsonArgumentStore,
    content: InMemoryToolkitContentStore,
}

impl WorkspaceSearchExecutor {
    /// Creates a new workspace::grep value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
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

    /// Pack bundle.
    /// This returns the toolkit pack bundle that registers the operation route; it does not
    /// execute the operation.
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
