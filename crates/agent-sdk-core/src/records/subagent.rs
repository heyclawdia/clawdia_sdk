use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        AgentId, ContentRef as ContentRefId, EffectId, EventId, IdempotencyKey, MessageId,
        PolicyRef, PrivacyClass, RunId, ToolCallId, WakeConditionId,
    },
    effect::{EffectIntent, EffectKind, EffectResult, EffectTerminalStatus},
    event::EventKind,
    package::{ContextHandoffPolicy, RuntimePackageFingerprint, SubagentToolPolicy},
    subagent::SubagentRequestId,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "record_type", content = "record", rename_all = "snake_case")]
pub enum SubagentRecord {
    Started(SubagentStartedRecord),
    Handoff(SubagentHandoffRecord),
    WrappedEvent(SubagentWrappedEventRecord),
    UsageRolledUp(SubagentUsageRolledUpRecord),
    Completed(SubagentCompletedRecord),
    ChildLifecycle(ChildLifecycleRecord),
}

impl SubagentRecord {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Started(_) => "subagent_started",
            Self::Handoff(_) => "subagent_handoff",
            Self::WrappedEvent(_) => "subagent_event",
            Self::UsageRolledUp(_) => "subagent_usage_rolled_up",
            Self::Completed(_) => "subagent_completed",
            Self::ChildLifecycle(_) => "child_lifecycle",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RunJournalRef {
    pub run_id: RunId,
    pub journal_partition_ref: String,
}

impl RunJournalRef {
    pub fn for_run(run_id: RunId) -> Self {
        Self {
            journal_partition_ref: format!("journal.run.{}", run_id.as_str()),
            run_id,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubagentStartedRecord {
    pub request_id: SubagentRequestId,
    pub parent_run_id: RunId,
    pub child_run_id: RunId,
    pub parent_tool_call_id: ToolCallId,
    pub child_agent_id: AgentId,
    pub child_package_fingerprint: RuntimePackageFingerprint,
    pub child_journal_ref: RunJournalRef,
    pub handoff_policy: ContextHandoffPolicy,
    pub tool_policy: SubagentToolPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub message_ids: Vec<MessageId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wake_condition_ids: Vec<WakeConditionId>,
    pub effect_intent: EffectIntent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubagentHandoffRecord {
    pub request_id: SubagentRequestId,
    pub parent_run_id: RunId,
    pub child_run_id: RunId,
    pub handoff_policy: ContextHandoffPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_content_refs: Vec<ContentRefId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projection_audit_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub redaction_policy_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubagentWrappedEventRecord {
    pub parent_run_id: RunId,
    pub child_run_id: RunId,
    pub child_agent_id: AgentId,
    pub original_child_event_id: EventId,
    pub original_child_event_kind: EventKind,
    pub wrapped_event_ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_journal_cursor: Option<crate::domain::JournalCursor>,
    pub child_journal_ref: RunJournalRef,
    pub privacy: PrivacyClass,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubagentUsageRolledUpRecord {
    pub child_run_id: RunId,
    pub parent_run_id: RunId,
    pub child_usage_ref: String,
    pub parent_usage_ref: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_micros: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    pub terminal_status: SubagentTerminalStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubagentCompletedRecord {
    pub child_run_id: RunId,
    pub parent_run_id: RunId,
    pub terminal_status: SubagentTerminalStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_ref: Option<ContentRefId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_ref: Option<String>,
    pub policy_outcome: String,
    pub effect_result: EffectResult,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChildLifecycleRecord {
    pub child_run_id: RunId,
    pub parent_run_id: RunId,
    pub artifact_kind: ChildArtifactKind,
    pub action: ChildLifecycleAction,
    pub status: ChildLifecycleStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_ack_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reclaim_policy_ref: Option<PolicyRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_intent: Option<EffectIntent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_result: Option<EffectResult>,
    pub idempotency_key: IdempotencyKey,
}

impl ChildLifecycleRecord {
    pub fn shutdown_intent(
        parent_run_id: RunId,
        child_run_id: RunId,
        policy_refs: Vec<PolicyRef>,
        idempotency_key: IdempotencyKey,
    ) -> Self {
        let effect_id = shutdown_effect_id(&child_run_id);
        let mut intent = EffectIntent::new(
            effect_id,
            EffectKind::ChildArtifactShutdown,
            crate::domain::EntityRef::new(
                crate::domain::EntityKind::SubagentRun,
                child_run_id.as_str(),
            ),
            crate::domain::SourceRef::with_kind(
                crate::domain::SourceKind::Sdk,
                "source.sdk.subagent",
            ),
            "parent requested child subagent shutdown",
        );
        intent.policy_refs = policy_refs.clone();
        intent.idempotency_key = Some(idempotency_key.clone());

        Self {
            child_run_id,
            parent_run_id,
            artifact_kind: ChildArtifactKind::SubagentRun,
            action: ChildLifecycleAction::ShutdownIntent,
            status: ChildLifecycleStatus::Requested,
            policy_refs,
            host_ack_ref: None,
            reclaim_policy_ref: None,
            effect_intent: Some(intent),
            effect_result: None,
            idempotency_key,
        }
    }

    pub fn shutdown_completed(&self) -> Self {
        let mut completed = self.clone();
        completed.action = ChildLifecycleAction::ShutdownCompleted;
        completed.status = ChildLifecycleStatus::Completed;
        completed.effect_result = Some(EffectResult {
            effect_id: shutdown_effect_id(&completed.child_run_id),
            terminal_status: EffectTerminalStatus::Cancelled,
            external_operation_id: None,
            reconciliation_ref: None,
            error_ref: None,
            content_refs: Vec::new(),
            redacted_summary: "parent-owned child subagent shutdown completed".to_string(),
        });
        completed
    }

    pub fn detach_intent(
        parent_run_id: RunId,
        child_run_id: RunId,
        policy_refs: Vec<PolicyRef>,
        host_ack_ref: String,
        reclaim_policy_ref: PolicyRef,
        idempotency_key: IdempotencyKey,
    ) -> Self {
        let effect_id = detach_effect_id(&child_run_id);
        let mut intent = EffectIntent::new(
            effect_id,
            EffectKind::DetachTransfer,
            crate::domain::EntityRef::new(
                crate::domain::EntityKind::SubagentRun,
                child_run_id.as_str(),
            ),
            crate::domain::SourceRef::with_kind(
                crate::domain::SourceKind::Sdk,
                "source.sdk.subagent",
            ),
            "parent requested explicit child subagent detach",
        );
        intent.policy_refs = policy_refs.clone();
        intent.idempotency_key = Some(idempotency_key.clone());

        Self {
            child_run_id,
            parent_run_id,
            artifact_kind: ChildArtifactKind::SubagentRun,
            action: ChildLifecycleAction::DetachIntent,
            status: ChildLifecycleStatus::Requested,
            policy_refs,
            host_ack_ref: Some(host_ack_ref),
            reclaim_policy_ref: Some(reclaim_policy_ref),
            effect_intent: Some(intent),
            effect_result: None,
            idempotency_key,
        }
    }

    pub fn detached(&self) -> Self {
        let mut detached = self.clone();
        detached.action = ChildLifecycleAction::Detached;
        detached.status = ChildLifecycleStatus::Detached;
        detached.effect_result = Some(EffectResult {
            effect_id: detach_effect_id(&detached.child_run_id),
            terminal_status: EffectTerminalStatus::Completed,
            external_operation_id: detached.host_ack_ref.clone(),
            reconciliation_ref: detached
                .reclaim_policy_ref
                .as_ref()
                .map(|policy| policy.as_str().to_string()),
            error_ref: None,
            content_refs: Vec::new(),
            redacted_summary: "parent-owned child subagent detach completed".to_string(),
        });
        detached
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildArtifactKind {
    SubagentRun,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildLifecycleAction {
    ShutdownIntent,
    ShutdownCompleted,
    DetachIntent,
    Detached,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildLifecycleStatus {
    Requested,
    Completed,
    Detached,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubagentTerminalStatus {
    Completed,
    Failed,
    Cancelled,
    Detached,
}

fn shutdown_effect_id(child_run_id: &RunId) -> EffectId {
    EffectId::new(format!(
        "effect.subagent.shutdown.{}",
        child_run_id.as_str()
    ))
}

fn detach_effect_id(child_run_id: &RunId) -> EffectId {
    EffectId::new(format!("effect.subagent.detach.{}", child_run_id.as_str()))
}
