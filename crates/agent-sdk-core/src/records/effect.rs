//! Shared effect-intent and effect-result records. Use these records before and after
//! externally visible work such as tools, output delivery, extension actions, or
//! child starts. The records themselves do not execute the effect.
//!
use serde::{Deserialize, Serialize};

use crate::domain::{
    ContentRef, DedupeKey, DestinationRef, EffectId, EntityRef, IdempotencyKey, PolicyRef,
    SourceRef,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite effect kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum EffectKind {
    /// Use this variant when the contract needs to represent provider request; selecting it has no side effect by itself.
    ProviderRequest,
    /// Use this variant when the contract needs to represent tool execution; selecting it has no side effect by itself.
    ToolExecution,
    /// Use this variant when the contract needs to represent approval dispatch; selecting it has no side effect by itself.
    ApprovalDispatch,
    /// Use this variant when the contract needs to represent memory write; selecting it has no side effect by itself.
    MemoryWrite,
    /// Use this variant when the contract needs to represent extension action; selecting it has no side effect by itself.
    ExtensionAction,
    /// Use this variant when the contract needs to represent output delivery; selecting it has no side effect by itself.
    OutputDelivery,
    /// Use this variant when the contract needs to represent file write; selecting it has no side effect by itself.
    FileWrite,
    /// Use this variant when the contract needs to represent process start; selecting it has no side effect by itself.
    ProcessStart,
    /// Use this variant when the contract needs to represent process signal; selecting it has no side effect by itself.
    ProcessSignal,
    /// Use this variant when the contract needs to represent isolated process start; selecting it has no side effect by itself.
    IsolatedProcessStart,
    /// Use this variant when the contract needs to represent child agent start; selecting it has no side effect by itself.
    ChildAgentStart,
    /// Use this variant when the contract needs to represent run message delivery; selecting it has no side effect by itself.
    RunMessageDelivery,
    /// Use this variant when the contract needs to represent child artifact shutdown; selecting it has no side effect by itself.
    ChildArtifactShutdown,
    /// Use this variant when the contract needs to represent detach transfer; selecting it has no side effect by itself.
    DetachTransfer,
    /// Use this variant when the contract needs to represent hook mutation; selecting it has no side effect by itself.
    HookMutation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the effect intent record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct EffectIntent {
    /// Stable effect id used for typed lineage, lookup, or dedupe.
    pub effect_id: EffectId,
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: EffectKind,
    /// Typed subject ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub subject_ref: EntityRef,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: Option<DestinationRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency_key: Option<IdempotencyKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Dedupe policy or key for a side-effecting operation.
    /// Replay and repair use it to avoid sending or executing the same effect twice.
    pub dedupe_key: Option<DedupeKey>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Content references associated with this record; resolving them is a
    /// separate policy-gated step.
    pub content_refs: Vec<ContentRef>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

impl EffectIntent {
    /// Creates a new records::effect value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(
        effect_id: EffectId,
        kind: EffectKind,
        subject_ref: EntityRef,
        source: SourceRef,
        redacted_summary: impl Into<String>,
    ) -> Self {
        Self {
            effect_id,
            kind,
            subject_ref,
            source,
            destination: None,
            policy_refs: Vec::new(),
            idempotency_key: None,
            dedupe_key: None,
            content_refs: Vec::new(),
            redacted_summary: redacted_summary.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite effect terminal status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum EffectTerminalStatus {
    /// Use this variant when the contract needs to represent completed; selecting it has no side effect by itself.
    Completed,
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed,
    /// Use this variant when the contract needs to represent timed out; selecting it has no side effect by itself.
    TimedOut,
    /// Use this variant when the contract needs to represent cancelled; selecting it has no side effect by itself.
    Cancelled,
    /// Use this variant when the contract needs to represent denied before execution; selecting it has no side effect by itself.
    DeniedBeforeExecution,
    /// Use this variant when the contract needs to represent unknown; selecting it has no side effect by itself.
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the effect result record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct EffectResult {
    /// Stable effect id used for typed lineage, lookup, or dedupe.
    pub effect_id: EffectId,
    /// Terminal status used by this record or request.
    pub terminal_status: EffectTerminalStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable external operation id used for typed lineage, lookup, or
    /// dedupe.
    pub external_operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed reconciliation ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub reconciliation_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed error ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub error_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Content references associated with this record; resolving them is a
    /// separate policy-gated step.
    pub content_refs: Vec<ContentRef>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

impl EffectResult {
    /// Returns an updated value with completed configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn completed(effect_id: EffectId, redacted_summary: impl Into<String>) -> Self {
        Self {
            effect_id,
            terminal_status: EffectTerminalStatus::Completed,
            external_operation_id: None,
            reconciliation_ref: None,
            error_ref: None,
            content_refs: Vec::new(),
            redacted_summary: redacted_summary.into(),
        }
    }

    /// Builds the unknown record or result value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn unknown(effect_id: EffectId, redacted_summary: impl Into<String>) -> Self {
        Self {
            effect_id,
            terminal_status: EffectTerminalStatus::Unknown,
            external_operation_id: None,
            reconciliation_ref: None,
            error_ref: None,
            content_refs: Vec::new(),
            redacted_summary: redacted_summary.into(),
        }
    }
}
