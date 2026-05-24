use serde::{Deserialize, Serialize};

use crate::domain::{
    ContentRef, DedupeKey, DestinationRef, EffectId, EntityRef, IdempotencyKey, PolicyRef,
    SourceRef,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    ProviderRequest,
    ToolExecution,
    ApprovalDispatch,
    MemoryWrite,
    ExtensionAction,
    OutputDelivery,
    FileWrite,
    ProcessStart,
    ProcessSignal,
    IsolatedProcessStart,
    ChildAgentStart,
    RunMessageDelivery,
    ChildArtifactShutdown,
    DetachTransfer,
    HookMutation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EffectIntent {
    pub effect_id: EffectId,
    pub kind: EffectKind,
    pub subject_ref: EntityRef,
    pub source: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<DestinationRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedupe_key: Option<DedupeKey>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_refs: Vec<ContentRef>,
    pub redacted_summary: String,
}

impl EffectIntent {
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
pub enum EffectTerminalStatus {
    Completed,
    Failed,
    TimedOut,
    Cancelled,
    DeniedBeforeExecution,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EffectResult {
    pub effect_id: EffectId,
    pub terminal_status: EffectTerminalStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconciliation_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_refs: Vec<ContentRef>,
    pub redacted_summary: String,
}

impl EffectResult {
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
