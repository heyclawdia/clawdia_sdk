//! Workspace read tool executor and request lowering. Use this toolkit module for
//! bounded reads that produce content refs and metadata. Reads touch the local
//! workspace through an explicit bounded policy and never grant ambient provider
//! visibility.
//!
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
/// Workspace workspace read request request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceReadRequest {
    /// Workspace-relative or resource path selected by the request or result.
    pub path: String,
    /// Maximum byte budget the caller requested before truncation or summary
    /// behavior is applied.
    pub max_bytes: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Workspace workspace read output request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceReadOutput {
    /// Workspace-relative or resource path selected by the request or result.
    pub path: String,
    /// Detected or declared MIME type used for reader selection and
    /// provider-safe summaries.
    pub mime_type: String,
    /// Detected used by this record or request.
    pub detected: WorkspaceReadDetection,
    /// Collection of reader pipeline values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub reader_pipeline: Vec<WorkspaceReaderStep>,
    /// Observed byte length for the source, sidecar, or extracted record.
    pub byte_len: u64,
    /// Stable hash for the bytes or canonical payload used for stale checks
    /// and fingerprints.
    pub content_hash: String,
    /// Whether output was shortened by byte, item, page, archive, or parser
    /// limits.
    pub truncated: bool,
    /// Whether the input is treated as binary so raw bytes are not exposed by
    /// default.
    pub binary: bool,
    /// Hashline anchors and line metadata used for stale-read detection and
    /// edit planning.
    pub anchors: Vec<HashLineAnchor>,
    /// Bounded textual content extracted for caller use; absent for binary
    /// summaries or denied raw access.
    pub content: String,
    /// Redacted or bounded summary used when raw content is absent or
    /// truncated.
    pub content_summary: Option<String>,
    /// Media metadata extracted without exposing raw media bytes.
    pub media: Option<WorkspaceMediaMetadata>,
    /// Document metadata such as parser, page/slide/sheet counts, and
    /// extraction warnings.
    pub document: Option<WorkspaceDocumentMetadata>,
    /// Archive listing metadata with truncation and decompression warnings.
    pub archive: Option<WorkspaceArchiveMetadata>,
    /// SQLite schema/sample metadata gathered under bounded read limits.
    pub sqlite: Option<WorkspaceSqliteMetadata>,
    /// Resource/URI metadata resolved through an explicit resolver.
    pub resource: Option<WorkspaceResourceMetadata>,
    /// Non-fatal warnings from bounded readers, parsers, or policy
    /// downgrades.
    pub warnings: Vec<String>,
}

impl BoundedWorkspace {
    /// Reads one bounded workspace path or supported URI. The method may inspect
    /// file bytes under `BoundedWorkspace` policy and returns structured
    /// metadata/content, but it never mutates files.
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
/// Workspace workspace read executor request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspaceReadExecutor {
    executor_ref: ExecutorRef,
    workspace: Arc<BoundedWorkspace>,
    arguments: InMemoryJsonArgumentStore,
    content: InMemoryToolkitContentStore,
}

impl WorkspaceReadExecutor {
    /// Creates a new workspace::read value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
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

    /// Pack bundle.
    /// This returns the toolkit pack bundle that registers the operation route; it does not
    /// execute the operation.
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
