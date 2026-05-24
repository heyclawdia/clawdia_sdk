use std::{fs, io::Read, sync::Arc};

use agent_sdk_core::{
    AgentError, ExecutorRef, PolicyRef, ToolExecutionOutput, ToolExecutionRequest, ToolExecutor,
    ToolPackId, ToolPackKind, ToolPackSnapshot,
    policy::{CapabilityPermission, EffectClass, RiskClass},
};
use serde::{Deserialize, Serialize};

use super::{
    anchor::HashLineAnchor,
    bounds::BoundedWorkspace,
    policy::WorkspacePolicy,
    read_pipeline::{
        WorkspaceArchiveMetadata, WorkspaceDocumentMetadata, WorkspaceMediaMetadata,
        WorkspaceReadDetection, WorkspaceReaderStep, WorkspaceResourceMetadata,
        WorkspaceSqliteMetadata, detect_workspace_file,
    },
    readers::{render_bounded_prefix_read, render_workspace_read, render_workspace_uri},
    util::{content_ref_for, first_arg_ref, hash_bytes, tool_failure},
};
use crate::{
    packs::{ToolkitPackBundle, tool_snapshot},
    testing::{InMemoryJsonArgumentStore, InMemoryToolkitContentStore},
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceReadRequest {
    pub path: String,
    pub max_bytes: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceReadOutput {
    pub path: String,
    pub mime_type: String,
    pub detected: WorkspaceReadDetection,
    pub reader_pipeline: Vec<WorkspaceReaderStep>,
    pub byte_len: u64,
    pub content_hash: String,
    pub truncated: bool,
    pub binary: bool,
    pub anchors: Vec<HashLineAnchor>,
    pub content: String,
    pub content_summary: Option<String>,
    pub media: Option<WorkspaceMediaMetadata>,
    pub document: Option<WorkspaceDocumentMetadata>,
    pub archive: Option<WorkspaceArchiveMetadata>,
    pub sqlite: Option<WorkspaceSqliteMetadata>,
    pub resource: Option<WorkspaceResourceMetadata>,
    pub warnings: Vec<String>,
}

impl BoundedWorkspace {
    pub fn read(&self, request: &WorkspaceReadRequest) -> Result<WorkspaceReadOutput, AgentError> {
        let max_output_bytes = request
            .max_bytes
            .unwrap_or(self.policy.max_output_bytes)
            .min(self.policy.max_output_bytes);
        if is_uri_read(&request.path) {
            let uri_read =
                render_workspace_uri(&request.path, self.policy.max_file_bytes, max_output_bytes)?;
            return Ok(WorkspaceReadOutput {
                path: request.path.clone(),
                mime_type: uri_read.detection.mime_type.clone(),
                detected: uri_read.detection,
                reader_pipeline: uri_read.rendered.reader_pipeline,
                byte_len: uri_read.byte_len,
                content_hash: uri_read.content_hash,
                truncated: uri_read.rendered.truncated,
                binary: uri_read.rendered.binary,
                anchors: uri_read.rendered.anchors,
                content: uri_read.rendered.content,
                content_summary: uri_read.rendered.content_summary,
                media: uri_read.rendered.media,
                document: uri_read.rendered.document,
                archive: uri_read.rendered.archive,
                sqlite: uri_read.rendered.sqlite,
                resource: uri_read.rendered.resource,
                warnings: uri_read.rendered.warnings,
            });
        }

        let path = self.resolve_existing_file(&request.path)?;
        let byte_len = fs::metadata(&path).map_err(tool_failure)?.len();
        if byte_len > self.policy.max_file_bytes {
            let prefix_limit = self
                .policy
                .max_file_bytes
                .min(max_output_bytes.max(1))
                .max(1);
            let mut bytes = Vec::new();
            fs::File::open(&path)
                .map_err(tool_failure)?
                .take(prefix_limit)
                .read_to_end(&mut bytes)
                .map_err(tool_failure)?;
            let detected = detect_workspace_file(&path, &bytes);
            let rendered =
                render_bounded_prefix_read(&bytes, &detected, byte_len, max_output_bytes)?;
            return Ok(WorkspaceReadOutput {
                path: request.path.clone(),
                mime_type: detected.mime_type.clone(),
                detected,
                reader_pipeline: rendered.reader_pipeline,
                byte_len,
                content_hash: hash_bytes(&bytes),
                truncated: true,
                binary: rendered.binary,
                anchors: rendered.anchors,
                content: rendered.content,
                content_summary: rendered.content_summary,
                media: rendered.media,
                document: rendered.document,
                archive: rendered.archive,
                sqlite: rendered.sqlite,
                resource: rendered.resource,
                warnings: rendered.warnings,
            });
        }
        let bytes = fs::read(&path).map_err(tool_failure)?;
        let detected = detect_workspace_file(&path, &bytes);
        let rendered = render_workspace_read(&path, &bytes, &detected, max_output_bytes)?;
        Ok(WorkspaceReadOutput {
            path: request.path.clone(),
            mime_type: detected.mime_type.clone(),
            detected,
            reader_pipeline: rendered.reader_pipeline,
            byte_len: bytes.len() as u64,
            content_hash: hash_bytes(&bytes),
            truncated: rendered.truncated,
            binary: rendered.binary,
            anchors: rendered.anchors,
            content: rendered.content,
            content_summary: rendered.content_summary,
            media: rendered.media,
            document: rendered.document,
            archive: rendered.archive,
            sqlite: rendered.sqlite,
            resource: rendered.resource,
            warnings: rendered.warnings,
        })
    }
}

fn is_uri_read(path: &str) -> bool {
    path.starts_with("data:") || path.contains("://")
}

#[derive(Clone)]
pub struct WorkspaceReadExecutor {
    executor_ref: ExecutorRef,
    workspace: Arc<BoundedWorkspace>,
    arguments: InMemoryJsonArgumentStore,
    content: InMemoryToolkitContentStore,
}

impl WorkspaceReadExecutor {
    pub fn new(
        workspace: Arc<BoundedWorkspace>,
        arguments: InMemoryJsonArgumentStore,
        content: InMemoryToolkitContentStore,
    ) -> Self {
        Self {
            executor_ref: ExecutorRef::new("executor.toolkit.workspace_read.v1"),
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
            ToolPackId::new("toolpack.workspace_readonly.v1"),
            ToolPackKind::WorkspaceReadOnly,
            "v1",
            source.clone(),
        )
        .with_workspace_bounds(workspace.bounds_snapshot(policy_ref.clone()))
        .with_tool(tool_snapshot(
            "cap.toolkit.workspace_read",
            "workspace_read",
            "executor.toolkit.workspace_read.v1",
            "schema.toolkit.workspace_read.v1",
            vec![policy_ref],
            vec![CapabilityPermission::FilesystemRead],
            EffectClass::Read,
            RiskClass::Low,
            &source,
        ));
        ToolkitPackBundle::from_snapshot(snapshot)
    }
}

impl ToolExecutor for WorkspaceReadExecutor {
    fn executor_ref(&self) -> &ExecutorRef {
        &self.executor_ref
    }

    fn execute(&self, request: &ToolExecutionRequest) -> Result<ToolExecutionOutput, AgentError> {
        let args_ref = first_arg_ref(request)?;
        let read_request: WorkspaceReadRequest = self.arguments.get(args_ref)?;
        let output = self.workspace.read(&read_request)?;
        let content_ref = content_ref_for(request, "workspace_read");
        self.content.put(content_ref.clone(), &output)?;
        let mut envelope = ToolExecutionOutput::completed("workspace read returned content ref");
        envelope.content_refs.push(content_ref);
        Ok(envelope)
    }
}
