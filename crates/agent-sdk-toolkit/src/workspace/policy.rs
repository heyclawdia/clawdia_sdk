//! Concrete workspace tool helpers layered over core tool/effect contracts. Use these
//! modules for bounded read, search, edit, write, and format-aware extraction
//! behavior under a host-selected workspace policy. Reads search local files;
//! edit/write helpers may mutate files only through explicit executor calls. This
//! file contains the policy portion of that contract.
//!
use std::path::PathBuf;

use agent_sdk_core::{PolicyKind, PolicyRef, WorkspaceBoundsSnapshot};

#[derive(Clone, Debug)]
/// Workspace workspace policy request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct WorkspacePolicy {
    /// Stable workspace id used for typed lineage, lookup, or dedupe.
    pub workspace_id: String,
    /// Root used by this record or request.
    pub root: PathBuf,
    /// max file bytes used for bounds checks, summaries, or truncation
    /// evidence.
    pub max_file_bytes: u64,
    /// max output bytes used for bounds checks, summaries, or truncation
    /// evidence.
    pub max_output_bytes: u64,
    /// Maximum number of matches to return.
    /// Use it to keep search output bounded for model context.
    pub max_matches: usize,
    /// Whether include hidden is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub include_hidden: bool,
    /// Whether follow symlinks is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub follow_symlinks: bool,
    /// Boolean policy/capability flag for whether allow create is enabled.
    pub allow_create: bool,
    /// Boolean policy/capability flag for whether allow overwrite is enabled.
    pub allow_overwrite: bool,
}

impl WorkspacePolicy {
    /// Creates a new workspace::policy value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
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

    /// Returns the bounds snapshot currently held by this value.
    /// This snapshots workspace bounds policy metadata without reading or writing workspace
    /// files.
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

/// Returns filesystem policy for the current value.
/// This is a read-only or data-construction helper unless the method body explicitly calls a
/// port or store.
pub fn filesystem_policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::Permission, id)
}

/// Returns approval policy for the current value.
/// This is a read-only or data-construction helper unless the method body explicitly calls a
/// port or store.
pub fn approval_policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::Approval, id)
}
