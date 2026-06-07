use std::path::PathBuf;

use agent_sdk_core::{
    AgentError, ProviderArgumentStore, domain::ContentRef, tool_records::CanonicalToolName,
};

use crate::util::{read_bytes, root_join, safe_segment, sha256_hex, write_bytes};

#[derive(Clone, Debug)]
/// Filesystem-backed raw provider argument store.
pub struct FileProviderArgumentStore {
    root: PathBuf,
}

impl FileProviderArgumentStore {
    /// Creates a provider argument store rooted under the provided directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn argument_path(
        &self,
        provider_ref: &str,
        call_id: &str,
        canonical_tool_name: &CanonicalToolName,
    ) -> PathBuf {
        root_join(
            &self.root,
            &[
                "provider_arguments".to_string(),
                safe_segment(provider_ref),
                safe_segment(canonical_tool_name.as_str()),
                format!("{}.json", safe_segment(call_id)),
            ],
        )
    }

    fn argument_ref_path(&self, content_ref: &ContentRef) -> PathBuf {
        root_join(
            &self.root,
            &[
                "provider_arguments".to_string(),
                "by_ref".to_string(),
                format!("{}.json", safe_segment(content_ref.as_str())),
            ],
        )
    }
}

impl ProviderArgumentStore for FileProviderArgumentStore {
    fn store_provider_arguments(
        &self,
        provider_ref: &str,
        call_id: &str,
        canonical_tool_name: &CanonicalToolName,
        raw_arguments: &str,
    ) -> Result<Option<ContentRef>, AgentError> {
        let path = self.argument_path(provider_ref, call_id, canonical_tool_name);
        write_bytes(&path, raw_arguments.as_bytes())?;
        let digest = sha256_hex(raw_arguments.as_bytes());
        let content_ref = ContentRef::new(format!("content.provider_arguments.{}", &digest[..24]));
        write_bytes(
            &self.argument_ref_path(&content_ref),
            raw_arguments.as_bytes(),
        )?;
        Ok(Some(content_ref))
    }

    fn load_provider_arguments_json(
        &self,
        content_ref: &ContentRef,
    ) -> Result<serde_json::Value, AgentError> {
        let bytes = read_bytes(&self.argument_ref_path(content_ref))?;
        serde_json::from_slice(&bytes).map_err(|error| {
            AgentError::contract_violation(format!(
                "stored provider arguments are not valid JSON: {error}"
            ))
        })
    }
}
