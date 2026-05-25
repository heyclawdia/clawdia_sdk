//! Durable and observable SDK records. Use these DTOs for events, journals, effects,
//! context, output, and feature evidence. Constructing records is data-only;
//! persistence, publication, and external actions happen through ports or application
//! coordinators. This file contains the tool pack portion of that contract.
//!
use serde::{Deserialize, Serialize};

use crate::{
    domain::{ContentRef, DestinationRef, EffectId, EntityRef, PolicyRef, SourceRef},
    effect::EffectKind,
    package::tool_pack::ToolPackId,
    tool_records::CanonicalToolName,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the tool pack effect lineage record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ToolPackEffectLineage {
    /// Stable pack id used for typed lineage, lookup, or dedupe.
    pub pack_id: ToolPackId,
    /// Tool name used by this record or request.
    pub tool_name: CanonicalToolName,
    /// Stable effect id used for typed lineage, lookup, or dedupe.
    pub effect_id: EffectId,
    /// Kind discriminator for effect kind.
    /// Use it to route finite match arms without parsing display text.
    pub effect_kind: EffectKind,
    /// Typed subject ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub subject_ref: EntityRef,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: DestinationRef,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Content references associated with this record; resolving them is a
    /// separate policy-gated step.
    pub content_refs: Vec<ContentRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional mutation value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub mutation: Option<WorkspaceMutationLineage>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

impl ToolPackEffectLineage {
    /// Returns whether inverse candidate is advisory applies for this contract.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn inverse_candidate_is_advisory(&self) -> bool {
        self.mutation
            .as_ref()
            .and_then(|mutation| mutation.inverse_candidate_ref.as_ref())
            .is_some()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the workspace mutation lineage record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct WorkspaceMutationLineage {
    /// Workspace-relative or resource path selected by the request or result.
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Deterministic before hash used for stale checks, package evidence, or
    /// replay comparisons.
    pub before_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Deterministic after hash used for stale checks, package evidence, or
    /// replay comparisons.
    pub after_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed diff ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub diff_ref: Option<ContentRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed inverse candidate ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub inverse_candidate_ref: Option<ContentRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional non reversible reason value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub non_reversible_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the workspace read lineage record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct WorkspaceReadLineage {
    /// Workspace-relative or resource path selected by the request or result.
    pub path: String,
    /// Stable hash for the bytes or canonical payload used for stale checks
    /// and fingerprints.
    pub content_hash: String,
    /// Observed byte length for the source, sidecar, or extracted record.
    pub byte_len: u64,
    /// Whether output was shortened by byte, item, page, archive, or parser
    /// limits.
    pub truncated: bool,
    /// Detected or declared MIME type used for reader selection and
    /// provider-safe summaries.
    pub mime_type: String,
    /// Typed anchors ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub anchors_ref: ContentRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the shell process lineage record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ShellProcessLineage {
    /// Redacted summary for display, logs, events, or telemetry.
    /// It should describe the value without exposing raw private content.
    pub argv_summary: String,
    /// Typed sandbox policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub sandbox_policy_ref: PolicyRef,
    /// Timeout budget in milliseconds for the requested operation.
    pub timeout_ms: u64,
    /// Whether the SDK/tooling owns the launched process lifecycle for
    /// cancellation and cleanup evidence.
    pub agent_owned: bool,
    /// Whether detach requested is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub detach_requested: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the tool discovery lineage record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ToolDiscoveryLineage {
    /// Stable discovery index id used for typed lineage, lookup, or dedupe.
    pub discovery_index_id: String,
    /// Search query supplied by the caller.
    pub query: String,
    /// Collection of returned candidates values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub returned_candidates: Vec<String>,
    /// Whether activation must be applied as a package delta before use.
    pub package_delta_required: bool,
}
