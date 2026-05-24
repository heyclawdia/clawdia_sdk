use std::path::PathBuf;

use agent_sdk_core::{PolicyKind, PolicyRef, WorkspaceBoundsSnapshot};

#[derive(Clone, Debug)]
pub struct WorkspacePolicy {
    pub workspace_id: String,
    pub root: PathBuf,
    pub max_file_bytes: u64,
    pub max_output_bytes: u64,
    pub max_matches: usize,
    pub include_hidden: bool,
    pub follow_symlinks: bool,
    pub allow_create: bool,
    pub allow_overwrite: bool,
}

impl WorkspacePolicy {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_id: "workspace.default".to_string(),
            root: root.into(),
            max_file_bytes: 64 * 1024,
            max_output_bytes: 64 * 1024,
            max_matches: 100,
            include_hidden: false,
            follow_symlinks: false,
            allow_create: false,
            allow_overwrite: false,
        }
    }

    pub fn bounds_snapshot(&self, root_policy_ref: PolicyRef) -> WorkspaceBoundsSnapshot {
        WorkspaceBoundsSnapshot {
            workspace_id: self.workspace_id.clone(),
            root_policy_ref,
            max_file_bytes: self.max_file_bytes,
            max_output_bytes: self.max_output_bytes,
            max_matches: self.max_matches,
            follow_symlinks: self.follow_symlinks,
            include_hidden: self.include_hidden,
            anchor_validation: agent_sdk_core::AnchorValidationRequirement::HashLineRequired,
            preview_apply: agent_sdk_core::PreviewApplyRequirement::ApplyRequiresPreviewAndApproval,
        }
    }
}

pub fn filesystem_policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::Permission, id)
}

pub fn approval_policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::Approval, id)
}
