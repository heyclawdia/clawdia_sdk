use serde::{Deserialize, Serialize};

use crate::{
    domain::{ContentRef, DestinationRef, EffectId, EntityRef, PolicyRef, SourceRef},
    effect::EffectKind,
    package::tool_pack::ToolPackId,
    tool_records::CanonicalToolName,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolPackEffectLineage {
    pub pack_id: ToolPackId,
    pub tool_name: CanonicalToolName,
    pub effect_id: EffectId,
    pub effect_kind: EffectKind,
    pub subject_ref: EntityRef,
    pub source: SourceRef,
    pub destination: DestinationRef,
    pub policy_refs: Vec<PolicyRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_refs: Vec<ContentRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutation: Option<WorkspaceMutationLineage>,
    pub redacted_summary: String,
}

impl ToolPackEffectLineage {
    pub fn inverse_candidate_is_advisory(&self) -> bool {
        self.mutation
            .as_ref()
            .and_then(|mutation| mutation.inverse_candidate_ref.as_ref())
            .is_some()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceMutationLineage {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_ref: Option<ContentRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inverse_candidate_ref: Option<ContentRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_reversible_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceReadLineage {
    pub path: String,
    pub content_hash: String,
    pub byte_len: u64,
    pub truncated: bool,
    pub mime_type: String,
    pub anchors_ref: ContentRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShellProcessLineage {
    pub argv_summary: String,
    pub sandbox_policy_ref: PolicyRef,
    pub timeout_ms: u64,
    pub agent_owned: bool,
    pub detach_requested: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolDiscoveryLineage {
    pub discovery_index_id: String,
    pub query: String,
    pub returned_candidates: Vec<String>,
    pub package_delta_required: bool,
}
