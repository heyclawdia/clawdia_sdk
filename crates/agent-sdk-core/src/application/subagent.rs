use core::fmt;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Deserializer, Serialize, de::Error as DeError};

use crate::{
    agent_pool::{
        AgentPool, AgentPoolMember, MessageReceipt, RunMessage, WakeCondition, WakeRegistration,
    },
    domain::{
        AgentError, AgentErrorKind, AgentId, ContentRef as ContentRefId, DestinationKind,
        DestinationRef, EffectId, EntityKind, EntityRef, EventId, IdempotencyKey, MessageId,
        PolicyKind, PolicyRef, PrivacyClass, RetryClassification, RunId, SourceKind, SourceRef,
        SpanId, ToolCallId, TraceId,
    },
    effect::{EffectIntent, EffectKind, EffectResult, EffectTerminalStatus},
    event::{
        AgentEvent, CompiledEventFilter, ContentCaptureMode, EVENT_SCHEMA_VERSION,
        EventCorrelation, EventDeliverySemantics, EventEnvelope, EventFamily, EventFilter,
        EventFilterSet, EventFrame, EventKind, EventStreamScope,
    },
    ids::{IdValidationError, validate_identifier},
    journal::{
        JournalCursor, JournalRecord, JournalRecordBase, JournalRecordKind, JournalRecordPayload,
    },
    package::{
        ChildRuntimePackage, ChildRuntimePackagePolicy, ContextHandoffPolicy, DepthBudget,
        RuntimePackage, RuntimePackageFingerprint, SubagentRoutePolicy, SubagentToolPolicy,
        build_child_runtime_package,
    },
    run::RunRequest,
    run_handle::RunHandle,
    runtime::AgentRuntime,
    subagent_records::{
        ChildLifecycleRecord, RunJournalRef, SubagentCompletedRecord, SubagentHandoffRecord,
        SubagentRecord, SubagentStartedRecord, SubagentTerminalStatus, SubagentUsageRolledUpRecord,
        SubagentWrappedEventRecord,
    },
};

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct SubagentRequestId(String);

impl SubagentRequestId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("SubagentRequestId must be valid")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, IdValidationError> {
        let value = value.into();
        validate_identifier(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for SubagentRequestId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(D::Error::custom)
    }
}

impl fmt::Debug for SubagentRequestId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SubagentRequestId(redacted)")
    }
}

impl fmt::Display for SubagentRequestId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SubagentRequestId(redacted)")
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubagentRequest {
    pub request_id: SubagentRequestId,
    pub parent_run_id: RunId,
    pub parent_agent_id: AgentId,
    pub parent_tool_call_id: ToolCallId,
    pub child_run_id: RunId,
    pub child_agent_id: AgentId,
    pub child_source: SourceRef,
    pub child_destination: DestinationRef,
    pub route_policy: SubagentRoutePolicy,
    pub context_handoff: ContextHandoffPolicy,
    pub child_package_policy: ChildRuntimePackagePolicy,
    pub child_tool_policy: SubagentToolPolicy,
    pub message_policy_ref: PolicyRef,
    pub wake_policy_ref: PolicyRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifecycle_policy_ref: Option<PolicyRef>,
    pub depth_budget: DepthBudget,
    pub idempotency_key: IdempotencyKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_message_ref: Option<ContentRefId>,
}

impl SubagentRequest {
    pub fn validate(&self) -> Result<(), AgentError> {
        self.depth_budget.validate_child_start()?;
        self.context_handoff.validate()?;
        if self.child_destination.kind != DestinationKind::Subagent {
            return Err(AgentError::contract_violation(
                "subagent child destination must use DestinationKind::Subagent",
            ));
        }
        if self.child_source.kind != SourceKind::Subagent {
            return Err(AgentError::contract_violation(
                "subagent child source must use SourceKind::Subagent",
            ));
        }
        if self.message_policy_ref.kind == PolicyKind::Host
            || self.wake_policy_ref.kind == PolicyKind::Host
        {
            return Err(AgentError::contract_violation(
                "subagent message and wake policies must be explicit SDK policy refs",
            ));
        }
        Ok(())
    }

    pub fn child_run_request(&self) -> RunRequest {
        RunRequest::text(
            self.child_run_id.clone(),
            self.child_agent_id.clone(),
            self.child_source.clone(),
            format!("subagent child run {}", self.request_id.as_str()),
        )
    }
}

#[derive(Clone)]
pub struct SubagentSupervisor {
    runtime: AgentRuntime,
    pool: AgentPool,
    parent_package: RuntimePackage,
    state: Arc<Mutex<SubagentSupervisorState>>,
}

impl SubagentSupervisor {
    pub fn new(runtime: AgentRuntime, pool: AgentPool, parent_package: RuntimePackage) -> Self {
        Self {
            runtime,
            pool,
            parent_package,
            state: Arc::new(Mutex::new(SubagentSupervisorState::default())),
        }
    }

    pub fn start_child(&self, request: SubagentRequest) -> Result<ChildRunHandle, AgentError> {
        request.validate()?;

        let child_package = build_child_runtime_package(
            &self.parent_package,
            request.child_agent_id.clone(),
            &request.route_policy,
            &request.context_handoff,
            &request.child_package_policy,
            &request.child_tool_policy,
        )?;
        self.runtime.provider_for_route(
            &child_package.package.provider_route.route_id,
            &request.child_run_id,
        )?;

        let start_effect = child_start_intent(&request);
        let journal_cursor = self.append_parent_effect_intent(&request, start_effect.clone())?;

        let child_journal_ref = RunJournalRef::for_run(request.child_run_id.clone());
        let message_ids = request
            .initial_message_ref
            .as_ref()
            .map(|_| {
                vec![MessageId::new(format!(
                    "message.{}.initial",
                    request.request_id.as_str()
                ))]
            })
            .unwrap_or_default();

        let started = SubagentStartedRecord {
            request_id: request.request_id.clone(),
            parent_run_id: request.parent_run_id.clone(),
            child_run_id: request.child_run_id.clone(),
            parent_tool_call_id: request.parent_tool_call_id.clone(),
            child_agent_id: request.child_agent_id.clone(),
            child_package_fingerprint: child_package.fingerprint.clone(),
            child_journal_ref: child_journal_ref.clone(),
            handoff_policy: request.context_handoff.clone(),
            tool_policy: request.child_tool_policy.clone(),
            message_ids: message_ids.clone(),
            wake_condition_ids: Vec::new(),
            effect_intent: start_effect,
        };
        let handoff = SubagentHandoffRecord {
            request_id: request.request_id.clone(),
            parent_run_id: request.parent_run_id.clone(),
            child_run_id: request.child_run_id.clone(),
            handoff_policy: request.context_handoff.clone(),
            selected_content_refs: request.context_handoff.selected_content_refs(),
            projection_audit_ref: match request.context_handoff {
                ContextHandoffPolicy::FullHistoryWithPolicy { .. } => {
                    Some(format!("projection.audit.{}", request.request_id.as_str()))
                }
                _ => None,
            },
            policy_refs: request
                .context_handoff
                .policy_refs()
                .into_iter()
                .chain([request.child_package_policy.redaction_policy_ref.clone()])
                .collect(),
            redaction_policy_id: request
                .child_package_policy
                .redaction_policy_ref
                .as_str()
                .to_string(),
        };
        self.append_parent_subagent_record(&request, SubagentRecord::Started(started.clone()))?;
        self.append_parent_subagent_record(&request, SubagentRecord::Handoff(handoff.clone()))?;

        self.pool.join_run(pool_member_with_subagent_policies(
            request.parent_run_id.clone(),
            request.parent_agent_id.clone(),
            &request,
        ))?;
        let run_handle = self.pool.start_run(request.child_run_request())?;
        self.pool.join_run(pool_member_with_subagent_policies(
            request.child_run_id.clone(),
            request.child_agent_id.clone(),
            &request,
        ))?;

        if let Some(content_ref) = request.initial_message_ref.clone() {
            let message = RunMessage::new(
                message_ids
                    .first()
                    .cloned()
                    .expect("initial message id was precomputed"),
                request.parent_run_id.clone(),
                crate::agent_pool::RunAddress::run(request.child_run_id.clone()),
                content_ref,
                IdempotencyKey::new(format!("idem.{}.initial", request.request_id.as_str())),
            )
            .policy_ref(request.message_policy_ref.clone());
            self.pool.send(message)?;
        }

        let handle = ChildRunHandle {
            child_run_id: request.child_run_id.clone(),
            child_agent_id: request.child_agent_id.clone(),
            parent_run_id: request.parent_run_id.clone(),
            child_package_fingerprint: child_package.fingerprint.clone(),
            child_journal_ref,
            wrapped_event_filter: child_event_filter(request.child_run_id.clone())?,
            run_handle,
            child_package,
            start_journal_cursor: Some(journal_cursor),
        };

        let mut state = self.state()?;
        state.children.insert(
            request.child_run_id.clone(),
            ChildRunState {
                request: request.clone(),
                handle: handle.clone_without_run_handle(),
                detached: false,
                terminal: false,
            },
        );
        state.records.push(SubagentRecord::Started(started));
        state.records.push(SubagentRecord::Handoff(handoff));

        Ok(handle)
    }

    pub fn send_message(&self, message: RunMessage) -> Result<MessageReceipt, AgentError> {
        self.pool.send(message)
    }

    pub fn suspend_until(
        &self,
        run_id: RunId,
        condition: WakeCondition,
    ) -> Result<WakeRegistration, AgentError> {
        self.pool.suspend_until(run_id, condition)
    }

    pub fn wrap_child_event(
        &self,
        event: AgentEvent,
    ) -> Result<SubagentWrappedEventRecord, AgentError> {
        let child_run_id = event.envelope.run_id.clone();
        let child = self.child(&child_run_id)?;
        let record = SubagentWrappedEventRecord {
            parent_run_id: child.request.parent_run_id.clone(),
            child_run_id: child_run_id.clone(),
            child_agent_id: child.request.child_agent_id.clone(),
            original_child_event_id: event.envelope.event_id.clone(),
            original_child_event_kind: event.envelope.event_kind.clone(),
            wrapped_event_ref: format!(
                "subagent.event.{}.{}",
                child.request.parent_run_id.as_str(),
                event.envelope.event_id.as_str()
            ),
            child_journal_cursor: event.envelope.journal_cursor.clone(),
            child_journal_ref: child.handle.child_journal_ref.clone(),
            privacy: event.envelope.privacy.clone(),
        };
        self.append_parent_subagent_record(
            &child.request,
            SubagentRecord::WrappedEvent(record.clone()),
        )?;
        self.state()?
            .records
            .push(SubagentRecord::WrappedEvent(record.clone()));
        Ok(record)
    }

    pub fn rollup_usage(
        &self,
        child_run_id: RunId,
        child_usage_ref: impl Into<String>,
        input_tokens: u32,
        output_tokens: u32,
        cost_micros: Option<u64>,
        currency: Option<String>,
        terminal_status: SubagentTerminalStatus,
    ) -> Result<SubagentUsageRolledUpRecord, AgentError> {
        let child_usage_ref = child_usage_ref.into();
        let child = self.child(&child_run_id)?;
        let dedupe_key = format!("{}:{child_usage_ref}", child_run_id.as_str());

        let mut state = self.state()?;
        if !state.usage_rollup_dedupe.insert(dedupe_key) {
            return state
                .records
                .iter()
                .find_map(|record| match record {
                    SubagentRecord::UsageRolledUp(record)
                        if record.child_run_id == child_run_id
                            && record.child_usage_ref == child_usage_ref =>
                    {
                        Some(record.clone())
                    }
                    _ => None,
                })
                .ok_or_else(|| AgentError::contract_violation("usage rollup dedupe lost record"));
        }

        let record = SubagentUsageRolledUpRecord {
            child_run_id: child_run_id.clone(),
            parent_run_id: child.request.parent_run_id.clone(),
            child_usage_ref: child_usage_ref.clone(),
            parent_usage_ref: format!("usage.parent.{}.{}", child_run_id.as_str(), child_usage_ref),
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            cost_micros,
            currency,
            terminal_status,
        };
        self.append_parent_subagent_record(
            &child.request,
            SubagentRecord::UsageRolledUp(record.clone()),
        )?;
        state
            .records
            .push(SubagentRecord::UsageRolledUp(record.clone()));
        Ok(record)
    }

    pub fn complete_child(
        &self,
        child_run_id: RunId,
        terminal_status: SubagentTerminalStatus,
        result_ref: Option<ContentRefId>,
        error_ref: Option<String>,
    ) -> Result<SubagentCompletedRecord, AgentError> {
        let child = self.child(&child_run_id)?;
        let effect_id = EffectId::new(format!("effect.subagent.start.{}", child_run_id.as_str()));
        let record = SubagentCompletedRecord {
            child_run_id: child_run_id.clone(),
            parent_run_id: child.request.parent_run_id.clone(),
            terminal_status: terminal_status.clone(),
            result_ref,
            error_ref,
            policy_outcome: "policy.subagent.terminal".to_string(),
            effect_result: EffectResult {
                effect_id,
                terminal_status: match terminal_status {
                    SubagentTerminalStatus::Completed | SubagentTerminalStatus::Detached => {
                        EffectTerminalStatus::Completed
                    }
                    SubagentTerminalStatus::Failed => EffectTerminalStatus::Failed,
                    SubagentTerminalStatus::Cancelled => EffectTerminalStatus::Cancelled,
                },
                external_operation_id: None,
                reconciliation_ref: None,
                error_ref: None,
                content_refs: Vec::new(),
                redacted_summary: "subagent terminal status recorded".to_string(),
            },
        };
        self.append_parent_effect_result(&child.request, record.effect_result.clone())?;
        self.append_parent_subagent_record(
            &child.request,
            SubagentRecord::Completed(record.clone()),
        )?;

        let mut state = self.state()?;
        if let Some(child) = state.children.get_mut(&child_run_id) {
            child.terminal = true;
        }
        state
            .records
            .push(SubagentRecord::Completed(record.clone()));
        Ok(record)
    }

    pub fn cancel_child(
        &self,
        child_run_id: RunId,
    ) -> Result<Vec<ChildLifecycleRecord>, AgentError> {
        let child = self.child(&child_run_id)?;
        if child.detached {
            return Err(AgentError::new(
                AgentErrorKind::ChildLifecycleFailure,
                RetryClassification::HostConfigurationNeeded,
                "detached child lifecycle is host-owned after detach acknowledgement",
            ));
        }
        let idempotency_key =
            IdempotencyKey::new(format!("idem.subagent.cancel.{}", child_run_id.as_str()));
        let intent = ChildLifecycleRecord::shutdown_intent(
            child.request.parent_run_id.clone(),
            child_run_id.clone(),
            child.lifecycle_policy_refs(),
            idempotency_key,
        );
        self.append_parent_child_lifecycle_record(&child.request, intent.clone())?;
        self.runtime.cancel_run(&child_run_id)?;
        let completed = intent.shutdown_completed();
        self.append_parent_child_lifecycle_record(&child.request, completed.clone())?;
        let mut state = self.state()?;
        state
            .records
            .push(SubagentRecord::ChildLifecycle(intent.clone()));
        state
            .records
            .push(SubagentRecord::ChildLifecycle(completed.clone()));
        Ok(vec![intent, completed])
    }

    pub fn detach_child(
        &self,
        child_run_id: RunId,
        host_ack_ref: impl Into<String>,
        reclaim_policy_ref: PolicyRef,
    ) -> Result<Vec<ChildLifecycleRecord>, AgentError> {
        let child = self.child(&child_run_id)?;
        let host_ack_ref = host_ack_ref.into();
        if host_ack_ref.is_empty() {
            return Err(AgentError::contract_violation(
                "detached child run requires host acknowledgement",
            ));
        }
        let idempotency_key =
            IdempotencyKey::new(format!("idem.subagent.detach.{}", child_run_id.as_str()));
        let intent = ChildLifecycleRecord::detach_intent(
            child.request.parent_run_id.clone(),
            child_run_id.clone(),
            child.lifecycle_policy_refs(),
            host_ack_ref,
            reclaim_policy_ref,
            idempotency_key,
        );
        let detached = intent.detached();
        self.append_parent_child_lifecycle_record(&child.request, intent.clone())?;
        self.append_parent_child_lifecycle_record(&child.request, detached.clone())?;
        let mut state = self.state()?;
        if let Some(child) = state.children.get_mut(&child_run_id) {
            child.detached = true;
        }
        state
            .records
            .push(SubagentRecord::ChildLifecycle(intent.clone()));
        state
            .records
            .push(SubagentRecord::ChildLifecycle(detached.clone()));
        Ok(vec![intent, detached])
    }

    pub fn records(&self) -> Result<Vec<SubagentRecord>, AgentError> {
        Ok(self.state()?.records.clone())
    }

    pub fn child_can_be_addressed_as_user_chat(
        &self,
        child_run_id: &RunId,
    ) -> Result<bool, AgentError> {
        self.child(child_run_id)?;
        Ok(false)
    }

    pub fn child_requires_terminal_rollup_or_detach(
        &self,
        child_run_id: &RunId,
    ) -> Result<bool, AgentError> {
        let child = self.child(child_run_id)?;
        Ok(!child.detached && !child.terminal)
    }

    fn append_parent_effect_intent(
        &self,
        request: &SubagentRequest,
        intent: EffectIntent,
    ) -> Result<JournalCursor, AgentError> {
        let parent_journal = self.runtime.journal_port(&request.parent_run_id)?;
        let mut base = JournalRecordBase::new(
            self.runtime.next_journal_seq(),
            format!(
                "journal.record.subagent.start.{}",
                request.request_id.as_str()
            ),
            request.parent_run_id.clone(),
            request.parent_agent_id.clone(),
            request.child_source.clone(),
        );
        base.destination = Some(request.child_destination.clone());
        base.tags = vec!["feature:subagent".to_string()];
        base.runtime_package_fingerprint = self
            .parent_package
            .fingerprint()
            .map(|fingerprint| fingerprint.as_str().to_string())?;
        base.privacy = PrivacyClass::ContentRefsOnly;
        base.redaction_policy_id = request
            .child_package_policy
            .redaction_policy_ref
            .as_str()
            .to_string();
        parent_journal.append(JournalRecord::effect_intent(base, intent))
    }

    fn append_parent_effect_result(
        &self,
        request: &SubagentRequest,
        result: EffectResult,
    ) -> Result<JournalCursor, AgentError> {
        let parent_journal = self.runtime.journal_port(&request.parent_run_id)?;
        let mut base = self.parent_record_base(
            request,
            format!(
                "journal.record.subagent.effect.result.{}",
                request.request_id.as_str()
            ),
        )?;
        base.source = request.child_source.clone();
        parent_journal.append(JournalRecord::effect_result(base, result))
    }

    fn append_parent_subagent_record(
        &self,
        request: &SubagentRequest,
        record: SubagentRecord,
    ) -> Result<JournalCursor, AgentError> {
        let parent_journal = self.runtime.journal_port(&request.parent_run_id)?;
        let base = self.parent_record_base(
            request,
            format!(
                "journal.record.{}.{}",
                record.kind().replace('_', "."),
                request.request_id.as_str()
            ),
        )?;
        parent_journal.append(JournalRecord::feature_record(
            base,
            JournalRecordKind::Subagent,
            "subagent",
            record.kind(),
            EntityRef::new(EntityKind::SubagentRun, request.child_run_id.as_str()),
            vec![EntityRef::run(request.parent_run_id.clone())],
            subagent_content_refs(&record),
            JournalRecordPayload::Subagent(record),
        ))
    }

    fn append_parent_child_lifecycle_record(
        &self,
        request: &SubagentRequest,
        record: ChildLifecycleRecord,
    ) -> Result<JournalCursor, AgentError> {
        let parent_journal = self.runtime.journal_port(&request.parent_run_id)?;
        let event_kind = match record.status {
            crate::subagent_records::ChildLifecycleStatus::Requested => "child_lifecycle_requested",
            crate::subagent_records::ChildLifecycleStatus::Completed => "child_lifecycle_completed",
            crate::subagent_records::ChildLifecycleStatus::Detached => "child_lifecycle_detached",
            crate::subagent_records::ChildLifecycleStatus::Failed => "child_lifecycle_failed",
        };
        let base = self.parent_record_base(
            request,
            format!(
                "journal.record.{}.{}",
                event_kind.replace('_', "."),
                request.request_id.as_str()
            ),
        )?;
        parent_journal.append(JournalRecord::feature_record(
            base,
            JournalRecordKind::ChildLifecycle,
            "child_lifecycle",
            event_kind,
            EntityRef::new(EntityKind::SubagentRun, record.child_run_id.as_str()),
            vec![EntityRef::run(record.parent_run_id.clone())],
            Vec::new(),
            JournalRecordPayload::ChildLifecycle(record),
        ))
    }

    fn parent_record_base(
        &self,
        request: &SubagentRequest,
        record_id: String,
    ) -> Result<JournalRecordBase, AgentError> {
        let mut base = JournalRecordBase::new(
            self.runtime.next_journal_seq(),
            record_id,
            request.parent_run_id.clone(),
            request.parent_agent_id.clone(),
            request.child_source.clone(),
        );
        base.destination = Some(request.child_destination.clone());
        base.tags = vec!["feature:subagent".to_string()];
        base.runtime_package_fingerprint = self
            .parent_package
            .fingerprint()
            .map(|fingerprint| fingerprint.as_str().to_string())?;
        base.privacy = PrivacyClass::ContentRefsOnly;
        base.redaction_policy_id = request
            .child_package_policy
            .redaction_policy_ref
            .as_str()
            .to_string();
        Ok(base)
    }

    fn child(&self, child_run_id: &RunId) -> Result<ChildRunState, AgentError> {
        self.state()?
            .children
            .get(child_run_id)
            .cloned()
            .ok_or_else(|| {
                AgentError::new(
                    AgentErrorKind::SubagentFailure,
                    RetryClassification::RepairNeeded,
                    "child run is not supervised by this subagent supervisor",
                )
            })
    }

    fn state(&self) -> Result<std::sync::MutexGuard<'_, SubagentSupervisorState>, AgentError> {
        self.state
            .lock()
            .map_err(|_| AgentError::contract_violation("subagent supervisor state lock poisoned"))
    }
}

#[derive(Clone, Debug)]
pub struct ChildRunHandle {
    pub child_run_id: RunId,
    pub child_agent_id: AgentId,
    pub parent_run_id: RunId,
    pub child_package_fingerprint: RuntimePackageFingerprint,
    pub child_journal_ref: RunJournalRef,
    pub wrapped_event_filter: CompiledEventFilter,
    pub run_handle: RunHandle,
    pub child_package: ChildRuntimePackage,
    pub start_journal_cursor: Option<JournalCursor>,
}

impl ChildRunHandle {
    fn clone_without_run_handle(&self) -> ChildRunHandleSnapshot {
        ChildRunHandleSnapshot {
            child_journal_ref: self.child_journal_ref.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct ChildRunHandleSnapshot {
    child_journal_ref: RunJournalRef,
}

#[derive(Clone)]
struct ChildRunState {
    request: SubagentRequest,
    handle: ChildRunHandleSnapshot,
    detached: bool,
    terminal: bool,
}

impl ChildRunState {
    fn lifecycle_policy_refs(&self) -> Vec<PolicyRef> {
        let mut refs = vec![
            self.request
                .child_package_policy
                .child_lifecycle_bounds
                .clone(),
            self.request.message_policy_ref.clone(),
            self.request.wake_policy_ref.clone(),
        ];
        if let Some(policy) = &self.request.lifecycle_policy_ref {
            refs.push(policy.clone());
        }
        refs
    }
}

#[derive(Default)]
struct SubagentSupervisorState {
    children: BTreeMap<RunId, ChildRunState>,
    records: Vec<SubagentRecord>,
    usage_rollup_dedupe: BTreeSet<String>,
}

fn child_start_intent(request: &SubagentRequest) -> EffectIntent {
    let mut intent = EffectIntent::new(
        EffectId::new(format!(
            "effect.subagent.start.{}",
            request.child_run_id.as_str()
        )),
        EffectKind::ChildAgentStart,
        EntityRef::new(EntityKind::SubagentRun, request.child_run_id.as_str()),
        request.child_source.clone(),
        "parent requested child subagent start",
    );
    intent.destination = Some(request.child_destination.clone());
    intent.policy_refs = vec![
        request.message_policy_ref.clone(),
        request.wake_policy_ref.clone(),
        request.child_package_policy.child_lifecycle_bounds.clone(),
        request.child_package_policy.redaction_policy_ref.clone(),
    ];
    intent.idempotency_key = Some(request.idempotency_key.clone());
    if let Some(content_ref) = &request.initial_message_ref {
        intent.content_refs.push(content_ref.clone());
    }
    intent
}

fn subagent_content_refs(record: &SubagentRecord) -> Vec<ContentRefId> {
    match record {
        SubagentRecord::Started(record) => {
            record.effect_intent.content_refs.iter().cloned().collect()
        }
        SubagentRecord::Handoff(record) => record.selected_content_refs.clone(),
        SubagentRecord::Completed(record) => record.result_ref.iter().cloned().collect(),
        SubagentRecord::WrappedEvent(_)
        | SubagentRecord::UsageRolledUp(_)
        | SubagentRecord::ChildLifecycle(_) => Vec::new(),
    }
}

fn pool_member_with_subagent_policies(
    run_id: RunId,
    agent_id: AgentId,
    request: &SubagentRequest,
) -> AgentPoolMember {
    let mut member = AgentPoolMember::new(run_id, agent_id)
        .policy_ref(request.message_policy_ref.clone())
        .policy_ref(request.wake_policy_ref.clone())
        .policy_ref(request.child_package_policy.child_lifecycle_bounds.clone());
    if let Some(policy_ref) = &request.lifecycle_policy_ref {
        member = member.policy_ref(policy_ref.clone());
    }
    member
}

fn child_event_filter(child_run_id: RunId) -> Result<CompiledEventFilter, AgentError> {
    EventFilter {
        run_ids: EventFilterSet::Include(vec![child_run_id]),
        ..EventFilter::default()
    }
    .compile()
}

pub fn subagent_runtime_event_frame(
    parent_run_id: RunId,
    child_run_id: RunId,
    child_agent_id: AgentId,
    event_seq: u64,
    event_kind: EventKind,
    journal_cursor: Option<JournalCursor>,
) -> EventFrame {
    let event = AgentEvent::with_redacted_summary(
        EventEnvelope {
            schema_version: EVENT_SCHEMA_VERSION,
            event_id: EventId::new(format!("event.subagent.child.{event_seq}")),
            event_seq,
            event_family: EventFamily::Run,
            event_kind,
            payload_schema_version: 1,
            timestamp: "1970-01-01T00:00:00Z".to_string(),
            recorded_at: "1970-01-01T00:00:00Z".to_string(),
            run_id: child_run_id.clone(),
            agent_id: child_agent_id,
            turn_id: None,
            attempt_id: None,
            message_id: None,
            context_item_id: None,
            trace_id: TraceId::new(format!("trace.subagent.{}", parent_run_id.as_str())),
            span_id: SpanId::new(format!("span.subagent.child.{event_seq}")),
            parent_event_id: None,
            caused_by: None,
            subject_ref: EntityRef::new(EntityKind::SubagentRun, child_run_id.as_str()),
            related_refs: vec![EntityRef::run(parent_run_id)],
            causal_refs: Vec::new(),
            correlation: EventCorrelation::default(),
            tags: vec![crate::event::EventTag::new("feature:subagent")],
            source: SourceRef::with_kind(SourceKind::Sdk, "source.sdk.subagent"),
            destination: Some(DestinationRef::with_kind(
                DestinationKind::EventStream,
                "destination.event_stream.subagent",
            )),
            policy_refs: Vec::new(),
            journal_cursor,
            state_before: None,
            state_after: None,
            delivery_semantics: EventDeliverySemantics::JournalBacked,
            privacy: PrivacyClass::ContentRefsOnly,
            content_capture: ContentCaptureMode::Off,
            redaction_policy_id: "policy.redaction.subagent.default".to_string(),
            runtime_package_fingerprint: "runtime.package.fingerprint.subagent.child".to_string(),
        },
        "child event wrapped by subagent supervisor",
    );
    EventFrame {
        cursor: event.envelope.cursor(EventStreamScope::All),
        event,
        archive_cursor: None,
        overflow: None,
    }
}
