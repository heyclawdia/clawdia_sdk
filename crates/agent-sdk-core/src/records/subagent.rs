//! Durable and observable SDK records. Use these DTOs for events, journals, effects,
//! context, output, and feature evidence. Constructing records is data-only;
//! persistence, publication, and external actions happen through ports or application
//! coordinators. This file contains the subagent portion of that contract.
//!
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
/// Enumerates the finite subagent record cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum SubagentRecord {
    /// Use this variant when the contract needs to represent started; selecting it has no side effect by itself.
    Started(SubagentStartedRecord),
    /// Use this variant when the contract needs to represent handoff; selecting it has no side effect by itself.
    Handoff(SubagentHandoffRecord),
    /// Use this variant when the contract needs to represent wrapped event; selecting it has no side effect by itself.
    WrappedEvent(SubagentWrappedEventRecord),
    /// Use this variant when the contract needs to represent usage rolled up; selecting it has no side effect by itself.
    UsageRolledUp(SubagentUsageRolledUpRecord),
    /// Use this variant when the contract needs to represent completed; selecting it has no side effect by itself.
    Completed(SubagentCompletedRecord),
    /// Use this variant when the contract needs to represent child lifecycle; selecting it has no side effect by itself.
    ChildLifecycle(ChildLifecycleRecord),
}

impl SubagentRecord {
    /// Returns the kind currently held by this value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
/// Carries the run journal ref record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct RunJournalRef {
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Typed journal partition ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub journal_partition_ref: String,
}

impl RunJournalRef {
    /// Builds the for run value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn for_run(run_id: RunId) -> Self {
        Self {
            journal_partition_ref: format!("journal.run.{}", run_id.as_str()),
            run_id,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the subagent started record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct SubagentStartedRecord {
    /// Stable request id used for typed lineage, lookup, or dedupe.
    pub request_id: SubagentRequestId,
    /// Stable parent run id used for typed lineage, lookup, or dedupe.
    pub parent_run_id: RunId,
    /// Stable child run id used for typed lineage, lookup, or dedupe.
    pub child_run_id: RunId,
    /// Stable parent tool call id used for typed lineage, lookup, or dedupe.
    pub parent_tool_call_id: ToolCallId,
    /// Stable child agent id used for typed lineage, lookup, or dedupe.
    pub child_agent_id: AgentId,
    /// Deterministic child package fingerprint used for stale checks, package
    /// evidence, or replay comparisons.
    pub child_package_fingerprint: RuntimePackageFingerprint,
    /// Typed child journal ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub child_journal_ref: RunJournalRef,
    /// Handoff policy used by this record or request.
    pub handoff_policy: ContextHandoffPolicy,
    /// Tool policy used by this record or request.
    pub tool_policy: SubagentToolPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Identifiers used to select or correlate message values.
    /// Use them for typed lookup, filtering, or lineage instead of stringly typed matching.
    pub message_ids: Vec<MessageId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Identifiers used to select or correlate wake condition values.
    /// Use them for typed lookup, filtering, or lineage instead of stringly typed matching.
    pub wake_condition_ids: Vec<WakeConditionId>,
    /// Effect intent used by this record or request.
    pub effect_intent: EffectIntent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the subagent handoff record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct SubagentHandoffRecord {
    /// Stable request id used for typed lineage, lookup, or dedupe.
    pub request_id: SubagentRequestId,
    /// Stable parent run id used for typed lineage, lookup, or dedupe.
    pub parent_run_id: RunId,
    /// Stable child run id used for typed lineage, lookup, or dedupe.
    pub child_run_id: RunId,
    /// Handoff policy used by this record or request.
    pub handoff_policy: ContextHandoffPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed selected content refs references. Resolving them is separate
    /// from constructing this record.
    pub selected_content_refs: Vec<ContentRefId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed projection audit ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub projection_audit_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Stable redaction policy id used for typed lineage, lookup, or dedupe.
    pub redaction_policy_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the subagent wrapped event record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct SubagentWrappedEventRecord {
    /// Stable parent run id used for typed lineage, lookup, or dedupe.
    pub parent_run_id: RunId,
    /// Stable child run id used for typed lineage, lookup, or dedupe.
    pub child_run_id: RunId,
    /// Stable child agent id used for typed lineage, lookup, or dedupe.
    pub child_agent_id: AgentId,
    /// Stable original child event id used for typed lineage, lookup, or
    /// dedupe.
    pub original_child_event_id: EventId,
    /// Kind discriminator for original child event kind.
    /// Use it to route finite match arms without parsing display text.
    pub original_child_event_kind: EventKind,
    /// Typed wrapped event ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub wrapped_event_ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub child_journal_cursor: Option<crate::domain::JournalCursor>,
    /// Typed child journal ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub child_journal_ref: RunJournalRef,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the subagent usage rolled up record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct SubagentUsageRolledUpRecord {
    /// Stable child run id used for typed lineage, lookup, or dedupe.
    pub child_run_id: RunId,
    /// Stable parent run id used for typed lineage, lookup, or dedupe.
    pub parent_run_id: RunId,
    /// Typed child usage ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub child_usage_ref: String,
    /// Typed parent usage ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub parent_usage_ref: String,
    /// Input tokens used by this record or request.
    pub input_tokens: u32,
    /// Output tokens used by this record or request.
    pub output_tokens: u32,
    /// Total tokens used by this record or request.
    pub total_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional cost micros value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub cost_micros: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Currency code for the cost amount.
    /// Cost accounting uses it with amount micros and rate-table version.
    pub currency: Option<String>,
    /// Terminal status used by this record or request.
    pub terminal_status: SubagentTerminalStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the subagent completed record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct SubagentCompletedRecord {
    /// Stable child run id used for typed lineage, lookup, or dedupe.
    pub child_run_id: RunId,
    /// Stable parent run id used for typed lineage, lookup, or dedupe.
    pub parent_run_id: RunId,
    /// Terminal status used by this record or request.
    pub terminal_status: SubagentTerminalStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed result ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub result_ref: Option<ContentRefId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed error ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub error_ref: Option<String>,
    /// Policy outcome used by this record or request.
    pub policy_outcome: String,
    /// Effect result used by this record or request.
    pub effect_result: EffectResult,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the child lifecycle record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ChildLifecycleRecord {
    /// Stable child run id used for typed lineage, lookup, or dedupe.
    pub child_run_id: RunId,
    /// Stable parent run id used for typed lineage, lookup, or dedupe.
    pub parent_run_id: RunId,
    /// Kind discriminator for artifact kind.
    /// Use it to route finite match arms without parsing display text.
    pub artifact_kind: ChildArtifactKind,
    /// Action used by this record or request.
    pub action: ChildLifecycleAction,
    /// Finite status for this record or lifecycle stage.
    pub status: ChildLifecycleStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed host ack ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub host_ack_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed reclaim policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub reclaim_policy_ref: Option<PolicyRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional effect intent value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub effect_intent: Option<EffectIntent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional effect result value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub effect_result: Option<EffectResult>,
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency_key: IdempotencyKey,
}

impl ChildLifecycleRecord {
    /// Builds the shutdown intent record or result value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Builds the shutdown completed record or result value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Detach intent.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Detached.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
/// Enumerates the finite child artifact kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ChildArtifactKind {
    /// Use this variant when the contract needs to represent subagent run; selecting it has no side effect by itself.
    SubagentRun,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite child lifecycle action cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ChildLifecycleAction {
    /// Use this variant when the contract needs to represent shutdown intent; selecting it has no side effect by itself.
    ShutdownIntent,
    /// Use this variant when the contract needs to represent shutdown completed; selecting it has no side effect by itself.
    ShutdownCompleted,
    /// Use this variant when the contract needs to represent detach intent; selecting it has no side effect by itself.
    DetachIntent,
    /// Use this variant when the contract needs to represent detached; selecting it has no side effect by itself.
    Detached,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite child lifecycle status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ChildLifecycleStatus {
    /// Use this variant when the contract needs to represent requested; selecting it has no side effect by itself.
    Requested,
    /// Use this variant when the contract needs to represent completed; selecting it has no side effect by itself.
    Completed,
    /// Use this variant when the contract needs to represent detached; selecting it has no side effect by itself.
    Detached,
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite subagent terminal status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum SubagentTerminalStatus {
    /// Use this variant when the contract needs to represent completed; selecting it has no side effect by itself.
    Completed,
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed,
    /// Use this variant when the contract needs to represent cancelled; selecting it has no side effect by itself.
    Cancelled,
    /// Use this variant when the contract needs to represent detached; selecting it has no side effect by itself.
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
