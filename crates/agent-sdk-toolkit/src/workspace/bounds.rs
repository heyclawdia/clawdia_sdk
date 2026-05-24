use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use agent_sdk_core::{AgentError, AgentErrorKind, RetryClassification};

use super::{
    policy::WorkspacePolicy,
    util::{policy_denial, tool_failure},
};

#[derive(Clone, Debug)]
pub struct BoundedWorkspace {
    pub(super) policy: WorkspacePolicy,
}

impl BoundedWorkspace {
    pub fn new(policy: WorkspacePolicy) -> Self {
        Self { policy }
    }

    pub fn policy(&self) -> &WorkspacePolicy {
        &self.policy
    }

    pub(super) fn resolve_existing_file(&self, path: &str) -> Result<PathBuf, AgentError> {
        let path = self.resolve_workspace_path(path)?;
        self.validate_existing_path(&path)?;
        if !path.is_file() {
            return Err(AgentError::new(
                AgentErrorKind::ToolFailure,
                RetryClassification::UserActionNeeded,
                "workspace path is not a file",
            ));
        }
        Ok(path)
    }

    pub(super) fn resolve_for_write(&self, path: &str) -> Result<PathBuf, AgentError> {
        let path = self.resolve_workspace_path(path)?;
        let parent = path.parent().ok_or_else(|| {
            AgentError::contract_violation("workspace write path must have a parent")
        })?;
        if !parent.exists() {
            return Err(policy_denial(
                "workspace write parent directory does not exist",
            ));
        }
        self.validate_write_path(&path, parent)?;
        Ok(path)
    }

    fn resolve_workspace_path(&self, path: &str) -> Result<PathBuf, AgentError> {
        let relative = Path::new(path);
        if relative.is_absolute() {
            return Err(policy_denial("workspace path must be relative"));
        }
        for component in relative.components() {
            match component {
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(policy_denial("workspace path escapes root"));
                }
                Component::Normal(name)
                    if !self.policy.include_hidden && name.to_string_lossy().starts_with('.') =>
                {
                    return Err(policy_denial("hidden workspace paths are disabled"));
                }
                _ => {}
            }
        }
        Ok(self.policy.root.join(relative))
    }

    fn validate_existing_path(&self, path: &Path) -> Result<(), AgentError> {
        if !self.policy.follow_symlinks {
            self.validate_no_symlink_components(path)?;
        }
        let root = self.canonical_root()?;
        let target = path.canonicalize().map_err(tool_failure)?;
        if !target.starts_with(&root) {
            return Err(policy_denial("workspace path escapes root"));
        }
        Ok(())
    }

    fn validate_write_path(&self, path: &Path, parent: &Path) -> Result<(), AgentError> {
        if !self.policy.follow_symlinks {
            self.validate_no_symlink_components(parent)?;
            if path.exists()
                && fs::symlink_metadata(path)
                    .map_err(tool_failure)?
                    .file_type()
                    .is_symlink()
            {
                return Err(policy_denial("symlink traversal is disabled"));
            }
        }
        let root = self.canonical_root()?;
        let parent = parent.canonicalize().map_err(tool_failure)?;
        if !parent.starts_with(&root) {
            return Err(policy_denial("workspace path escapes root"));
        }
        if path.exists() {
            let target = path.canonicalize().map_err(tool_failure)?;
            if !target.starts_with(&root) {
                return Err(policy_denial("workspace path escapes root"));
            }
        }
        Ok(())
    }

    fn validate_no_symlink_components(&self, path: &Path) -> Result<(), AgentError> {
        let root = self.canonical_root()?;
        let relative = path
            .strip_prefix(&self.policy.root)
            .map_err(|_| policy_denial("workspace path escapes root"))?;
        let mut current = root;
        for component in relative.components() {
            let Component::Normal(name) = component else {
                continue;
            };
            current.push(name);
            if fs::symlink_metadata(&current)
                .map_err(tool_failure)?
                .file_type()
                .is_symlink()
            {
                return Err(policy_denial("symlink traversal is disabled"));
            }
        }
        Ok(())
    }

    fn canonical_root(&self) -> Result<PathBuf, AgentError> {
        self.policy.root.canonicalize().map_err(tool_failure)
    }

    pub(super) fn visit_files(
        &self,
        dir: &Path,
        visit: &mut dyn FnMut(&Path) -> Result<(), AgentError>,
    ) -> Result<(), AgentError> {
        for entry in fs::read_dir(dir).map_err(tool_failure)? {
            let entry = entry.map_err(tool_failure)?;
            let path = entry.path();
            let name = entry.file_name();
            if !self.policy.include_hidden && name.to_string_lossy().starts_with('.') {
                continue;
            }
            let file_type = entry.file_type().map_err(tool_failure)?;
            if file_type.is_symlink() && !self.policy.follow_symlinks {
                continue;
            }
            if file_type.is_dir() {
                self.visit_files(&path, visit)?;
            } else if file_type.is_file() {
                visit(&path)?;
            }
        }
        Ok(())
    }

    pub(super) fn relative_path(&self, path: &Path) -> Result<String, AgentError> {
        path.strip_prefix(&self.policy.root)
            .map_err(|_| policy_denial("workspace path escapes root"))?
            .to_str()
            .map(str::to_string)
            .ok_or_else(|| AgentError::contract_violation("workspace path is not UTF-8"))
    }
}
