//! Feature-layer agent-pool coordination over runs, messages, wake conditions, and
//! subscriptions. Use this module for generic run-to-run coordination without
//! introducing workflow-engine or product swarm behavior. Side-effecting operations may
//! update pool membership, append source-run journal records, and publish agent-pool
//! events through the configured runtime ports.
//!
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        AgentError, AgentErrorKind, AgentId, AgentPoolId, ContentRef, DestinationKind,
        DestinationRef, EffectId, EntityRef, EventId, IdempotencyKey, MessageId, PolicyRef,
        PrivacyClass, RetryClassification, RunId, SourceKind, SourceRef, SpanId, TopicId, TraceId,
        WakeConditionId,
    },
    effect::{EffectIntent, EffectKind, EffectResult, EffectTerminalStatus},
    event::{
        AgentEvent, CompiledEventFilter, ContentCaptureMode, EVENT_SCHEMA_VERSION,
        EventCorrelation, EventDeliverySemantics, EventEnvelope, EventFamily, EventFilter,
        EventFilterSet, EventFrame, EventKind, EventStreamScope, PayloadAccessMode,
    },
    event_bus::AgentEventStream,
    journal::{
        AgentPoolLifecycleStatus, AgentPoolRecord, EventIndexProjection, JOURNAL_SCHEMA_VERSION,
        JournalCursor, JournalRecord, JournalRecordKind, JournalRecordPayload,
        RunMessageAddressTargetRecord, RunMessageDeliveryStatus, RunMessageRecord, WakeRecord,
        WakeResumeInputPolicyRecord, WakeTriggerStatus,
    },
    run::RunRequest,
    run_handle::RunHandle,
    runtime::AgentRuntime,
};

#[derive(Clone)]
/// Holds agent pool application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct AgentPool {
    pool_id: AgentPoolId,
    runtime: AgentRuntime,
    store: Arc<dyn AgentPoolStore>,
}

impl AgentPool {
    /// Starts a builder for this application::agent_pool value.
    /// Building is data-only; runtime side effects occur only when a
    /// later coordinator or host port executes the built configuration.
    pub fn builder(pool_id: AgentPoolId) -> AgentPoolBuilder {
        AgentPoolBuilder {
            pool_id,
            runtime: None,
            message_policy: AgentPoolMessagePolicy::bounded_defaults(),
            wake_policy: AgentPoolWakePolicy::safe_defaults(),
            policy_refs: Vec::new(),
            store: None,
        }
    }

    /// Returns the pool id currently held by this value.
    /// This is a data-only accessor and does not change membership or wake state.
    pub fn pool_id(&self) -> &AgentPoolId {
        &self.pool_id
    }

    /// Starts a run through the shared runtime and joins it to this pool.
    /// Runtime registration and provider-loop effects stay in `AgentRuntime`;
    /// the pool side effect is membership tracking for coordination.
    pub fn start_run(&self, request: RunRequest) -> Result<RunHandle, AgentError> {
        let handle = self.runtime.start_run(request.clone())?;
        self.join_run(AgentPoolMember::new(request.run_id, request.agent_id))?;
        Ok(handle)
    }

    /// Join run.
    /// This records pool membership in the coordinator so later pool messages and subscriptions
    /// can target the run.
    pub fn join_run(&self, member: AgentPoolMember) -> Result<(), AgentError> {
        let should_create = {
            let snapshot = self.snapshot()?;
            !snapshot.created
        };

        if should_create {
            self.append_pool_record(
                &member.run_id,
                &member.agent_id,
                AgentPoolLifecycleStatus::Created,
                EventKind::AgentPoolCreated,
            )?;
            self.store.record_pool_created(&self.pool_id)?;
        }

        self.store.join_member(&self.pool_id, member.clone())?;

        self.append_pool_record(
            &member.run_id,
            &member.agent_id,
            AgentPoolLifecycleStatus::RunJoined,
            EventKind::AgentPoolRunJoined,
        )?;
        Ok(())
    }

    /// Returns the members currently held by this value.
    /// This reads current pool membership without starting, stopping, or messaging runs.
    pub fn members(&self) -> Result<Vec<AgentPoolMember>, AgentError> {
        Ok(self.snapshot()?.members)
    }

    /// Records that a run has left the pool.
    /// This removes membership from the shared store, appends a lifecycle record, and publishes
    /// a pool event. It does not cancel or otherwise mutate the run itself.
    pub fn leave_run(&self, run_id: &RunId) -> Result<AgentPoolMember, AgentError> {
        let (member, _) = self.store.leave_member(&self.pool_id, run_id)?;
        self.append_pool_record(
            &member.run_id,
            &member.agent_id,
            AgentPoolLifecycleStatus::RunLeft,
            EventKind::AgentPoolRunLeft,
        )?;
        Ok(member)
    }

    /// Sends a run message through the pool coordinator.
    /// This resolves the addressed members, applies pool message policy, appends accepted
    /// and terminal delivery records to the source run journal, publishes the matching
    /// agent-pool events, and deduplicates repeated calls by idempotency key.
    pub fn send(&self, message: RunMessage) -> Result<MessageReceipt, AgentError> {
        if let Some(receipt) = self
            .store
            .message_receipt(&self.pool_id, &message.idempotency_key)?
        {
            return Ok(receipt);
        }

        let delivered_to = self.resolve_address(&message);
        let terminal_status = if message.expires_at_millis == Some(0) {
            MessageStatus::Expired
        } else if delivered_to.is_empty() {
            MessageStatus::Failed
        } else {
            MessageStatus::Delivered
        };

        if terminal_status == MessageStatus::Expired {
            let receipt =
                self.record_message_status(&message, MessageStatus::Expired, Vec::new())?;
            return Ok(receipt);
        }

        if terminal_status == MessageStatus::Failed {
            let receipt =
                self.record_message_status(&message, MessageStatus::Failed, Vec::new())?;
            return Ok(receipt);
        }

        self.record_message_status(&message, MessageStatus::Accepted, delivered_to.clone())?;
        let receipt =
            self.record_message_status(&message, MessageStatus::Delivered, delivered_to)?;
        Ok(receipt)
    }

    /// Records one run-message status transition.
    /// This appends the status record to the source run journal, publishes the matching
    /// agent-pool event on the runtime event bus, and returns a receipt carrying the journal
    /// cursor. Use [`AgentPool::send`] for the full accept-to-terminal delivery flow.
    pub fn record_message_status(
        &self,
        message: &RunMessage,
        status: MessageStatus,
        delivered_to: Vec<RunId>,
    ) -> Result<MessageReceipt, AgentError> {
        let source_member = self.member(&message.from)?;
        let journal = self.runtime.journal_port(&message.from)?;
        let record = self.run_message_record(message, status.clone(), delivered_to.clone())?;
        let cursor = journal.append(record)?;
        let frame = self.publish_agent_pool_event(
            message.from.clone(),
            source_member.agent_id,
            status.event_kind(),
            Some(message.message_id.clone()),
            None,
            EntityRef::message(message.message_id.clone()),
            message.target_related_refs(&delivered_to),
            Some(message.to.destination_ref.clone()),
            message.policy_refs.clone(),
            Some(cursor.clone()),
            status.redacted_summary(),
        )?;

        let receipt = MessageReceipt {
            message_id: message.message_id.clone(),
            status,
            delivered_to,
            journal_cursor: Some(cursor),
        };
        self.store
            .record_message(&self.pool_id, message.clone(), receipt.clone())?;
        self.trigger_matching_wakes(&frame)?;
        Ok(receipt)
    }

    /// Subscribe.
    /// This creates a read-only subscription scoped by pool membership and the supplied filter.
    pub fn subscribe(
        &self,
        filter: EventFilter,
        cursor: Option<crate::event::EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        let compiled = self.compile_scoped_filter(filter)?;
        self.runtime.subscribe_events(compiled, cursor)
    }

    /// Computes or returns compile scoped filter for the
    /// application::agent_pool contract without external I/O or side effects.
    pub fn compile_scoped_filter(
        &self,
        filter: EventFilter,
    ) -> Result<CompiledEventFilter, AgentError> {
        self.scope_filter(filter).compile()
    }

    /// Returns scope filter derived from the supplied state.
    /// This operates on the named coordinator state or selected port; it does not create a
    /// parallel runtime path.
    pub fn scope_filter(&self, mut filter: EventFilter) -> EventFilter {
        let allowed_runs = self.observable_member_runs();
        filter.run_ids = intersect_run_ids(&filter.run_ids, &allowed_runs);
        let envelope_only = self
            .snapshot()
            .map(|snapshot| snapshot.wake_policy.envelope_only)
            .unwrap_or(true);
        if envelope_only {
            filter.payload_access = PayloadAccessMode::EnvelopeOnly;
        }
        filter
    }

    /// Registers a wake condition for a pool member run.
    /// This mutates the pool's wake registry and dedupe index, scopes the event filter to current
    /// members, and may poll the configured event subscription port to trigger immediately.
    pub fn suspend_until(
        &self,
        run_id: RunId,
        condition: WakeCondition,
    ) -> Result<WakeRegistration, AgentError> {
        if run_id != condition.run_id {
            return Err(AgentError::new(
                AgentErrorKind::InvalidStateTransition,
                RetryClassification::NotRetryable,
                "wake registration run_id must match condition run_id",
            ));
        }

        if let Some(registration) = self
            .store
            .wake_registration(&self.pool_id, &condition.idempotency_key)?
        {
            return Ok(registration);
        }

        self.member(&condition.run_id)?;
        let compiled = self.compile_scoped_filter(condition.filter.clone())?;
        let mut registration = self.record_wake_status(
            &condition,
            compiled.clone(),
            WakeRegistrationStatus::Registered,
            None,
        )?;

        if condition.timeout_millis == Some(0) {
            registration = self.record_wake_status(
                &condition,
                compiled,
                WakeRegistrationStatus::TimedOut,
                None,
            )?;
        } else if let Some(frame) = self
            .runtime
            .subscribe_events(compiled.clone(), None)?
            .next()
        {
            registration = self.record_wake_status(
                &condition,
                compiled,
                WakeRegistrationStatus::Triggered,
                Some(frame.event.envelope.event_id),
            )?;
        }

        Ok(registration)
    }

    /// Polls a registered wake condition for a matching event.
    /// This reads and may update pool wake state through `record_wake_status`; it creates a
    /// read-only event subscription but does not cancel or advance the target run.
    pub fn poll_wake(
        &self,
        condition_id: &WakeConditionId,
    ) -> Result<WakeRegistration, AgentError> {
        let stored = self
            .store
            .wake(&self.pool_id, condition_id)?
            .ok_or_else(|| AgentError::contract_violation("wake condition is not registered"))?;

        if stored.registration.status != WakeRegistrationStatus::Registered {
            return Ok(stored.registration);
        }

        let Some(frame) = self
            .runtime
            .subscribe_events(stored.compiled_filter.clone(), None)?
            .next()
        else {
            return Ok(stored.registration);
        };

        self.record_wake_status(
            &stored.condition,
            stored.compiled_filter,
            WakeRegistrationStatus::Triggered,
            Some(frame.event.envelope.event_id),
        )
    }

    /// Cancel wake.
    /// This marks a registered wake condition as cancelled in pool state; it does not cancel
    /// the run itself.
    pub fn cancel_wake(
        &self,
        condition_id: &WakeConditionId,
    ) -> Result<WakeRegistration, AgentError> {
        let stored = self
            .store
            .wake(&self.pool_id, condition_id)?
            .ok_or_else(|| AgentError::contract_violation("wake condition is not registered"))?;
        self.record_wake_status(
            &stored.condition,
            stored.compiled_filter,
            WakeRegistrationStatus::Cancelled,
            None,
        )
    }

    fn record_wake_status(
        &self,
        condition: &WakeCondition,
        compiled_filter: CompiledEventFilter,
        status: WakeRegistrationStatus,
        matched_event_id: Option<EventId>,
    ) -> Result<WakeRegistration, AgentError> {
        let member = self.member(&condition.run_id)?;
        let journal = self.runtime.journal_port(&condition.run_id)?;
        let wake_record = WakeRecord {
            condition_id: condition.condition_id.clone(),
            run_id: condition.run_id.clone(),
            event_filter_fingerprint: compiled_filter.filter_fingerprint.clone(),
            timeout_millis: condition.timeout_millis,
            resume_policy: condition.resume_with.clone().into(),
            trigger_status: status.clone().into(),
            policy_refs: condition.policy_refs.clone(),
            idempotency_key: condition.idempotency_key.clone(),
            matched_event_id,
        };
        let record = self.journal_record(
            condition.run_id.clone(),
            member.agent_id.clone(),
            JournalRecordKind::Wake,
            "agent_pool",
            status.event_kind().wire_name(),
            EntityRef::wake_condition(condition.condition_id.clone()),
            vec![EntityRef::run(condition.run_id.clone())],
            condition.policy_refs.clone(),
            Vec::new(),
            Some(condition.idempotency_key.clone()),
            JournalRecordPayload::Wake(wake_record),
        )?;
        let cursor = journal.append(record)?;
        self.publish_agent_pool_event(
            condition.run_id.clone(),
            member.agent_id,
            status.event_kind(),
            None,
            Some(condition.condition_id.clone()),
            EntityRef::wake_condition(condition.condition_id.clone()),
            vec![EntityRef::run(condition.run_id.clone())],
            Some(DestinationRef::with_kind(
                DestinationKind::Agent,
                condition.run_id.as_str(),
            )),
            condition.policy_refs.clone(),
            Some(cursor.clone()),
            status.redacted_summary(),
        )?;

        let registration = WakeRegistration {
            condition_id: condition.condition_id.clone(),
            run_id: condition.run_id.clone(),
            status,
            journal_cursor: Some(cursor),
        };

        self.store.record_wake(
            &self.pool_id,
            condition.clone(),
            compiled_filter,
            registration.clone(),
        )?;

        Ok(registration)
    }

    fn append_pool_record(
        &self,
        run_id: &RunId,
        agent_id: &AgentId,
        status: AgentPoolLifecycleStatus,
        event_kind: EventKind,
    ) -> Result<(), AgentError> {
        let journal = self.runtime.journal_port(run_id)?;
        let snapshot = self.snapshot()?;
        let member_run_ids = snapshot
            .members
            .iter()
            .map(|member| member.run_id.clone())
            .collect::<Vec<_>>();
        let topics = snapshot.topics;
        let policy_refs = snapshot.policy_refs;

        let record = AgentPoolRecord {
            pool_id: self.pool_id.clone(),
            member_run_ids,
            topics,
            policy_refs: policy_refs.clone(),
            lifecycle_status: status,
        };
        let journal_record = self.journal_record(
            run_id.clone(),
            agent_id.clone(),
            JournalRecordKind::AgentPool,
            "agent_pool",
            event_kind.wire_name(),
            EntityRef::run(run_id.clone()),
            Vec::new(),
            policy_refs.clone(),
            Vec::new(),
            None,
            JournalRecordPayload::AgentPool(record),
        )?;
        let cursor = journal.append(journal_record)?;
        self.publish_agent_pool_event(
            run_id.clone(),
            agent_id.clone(),
            event_kind,
            None,
            None,
            EntityRef::run(run_id.clone()),
            Vec::new(),
            Some(DestinationRef::with_kind(
                DestinationKind::Agent,
                run_id.as_str(),
            )),
            policy_refs,
            Some(cursor),
            "agent pool membership updated",
        )?;
        Ok(())
    }

    fn run_message_record(
        &self,
        message: &RunMessage,
        status: MessageStatus,
        delivered_to: Vec<RunId>,
    ) -> Result<JournalRecord, AgentError> {
        let member = self.member(&message.from)?;
        let mut effect_intent = None;
        let mut effect_result = None;
        let effect_id = EffectId::new(format!(
            "effect.run_message.{}",
            message.message_id.as_str()
        ));

        if status == MessageStatus::Accepted {
            let mut intent = EffectIntent::new(
                effect_id.clone(),
                EffectKind::RunMessageDelivery,
                EntityRef::message(message.message_id.clone()),
                SourceRef::with_kind(SourceKind::Sdk, "source.sdk.agent_pool"),
                "run message delivery intent",
            );
            intent.destination = Some(message.to.destination_ref.clone());
            intent.policy_refs = message.policy_refs.clone();
            intent.idempotency_key = Some(message.idempotency_key.clone());
            intent.content_refs = vec![message.content_ref.clone()];
            effect_intent = Some(intent);
        }

        if status.is_terminal_delivery() {
            effect_result = Some(EffectResult {
                effect_id,
                terminal_status: status.effect_terminal_status(),
                external_operation_id: None,
                reconciliation_ref: None,
                error_ref: None,
                content_refs: vec![message.content_ref.clone()],
                redacted_summary: status.redacted_summary().to_string(),
            });
        }

        let record = RunMessageRecord {
            message_id: message.message_id.clone(),
            source_run_id: message.from.clone(),
            address_target: message.to.target.clone().into(),
            content_ref: message.content_ref.clone(),
            correlation: message.correlation.clone(),
            reply_to: message.reply_to.clone(),
            delivery_status: status.clone().into(),
            delivered_to: delivered_to.clone(),
            policy_refs: message.policy_refs.clone(),
            idempotency_key: message.idempotency_key.clone(),
            effect_intent,
            effect_result,
        };

        self.journal_record(
            message.from.clone(),
            member.agent_id,
            JournalRecordKind::RunMessage,
            "agent_pool",
            status.event_kind().wire_name(),
            EntityRef::message(message.message_id.clone()),
            message.target_related_refs(&delivered_to),
            message.policy_refs.clone(),
            vec![message.content_ref.clone()],
            Some(message.idempotency_key.clone()),
            JournalRecordPayload::RunMessage(record),
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "journal-backed pool records intentionally spell out lineage, refs, and payload until a dedicated record-builder API replaces this private helper"
    )]
    fn journal_record(
        &self,
        run_id: RunId,
        agent_id: AgentId,
        record_kind: JournalRecordKind,
        event_family: impl Into<String>,
        event_kind: impl Into<String>,
        subject_ref: EntityRef,
        related_refs: Vec<EntityRef>,
        _policy_refs: Vec<PolicyRef>,
        content_refs: Vec<ContentRef>,
        idempotency_key: Option<IdempotencyKey>,
        payload: JournalRecordPayload,
    ) -> Result<JournalRecord, AgentError> {
        let journal_seq = self.runtime.next_journal_seq();
        let source = SourceRef::with_kind(SourceKind::Sdk, "source.sdk.agent_pool");
        let fingerprint = self
            .runtime
            .run_snapshot(&run_id)
            .map(|snapshot| snapshot.runtime_package_fingerprint.as_str().to_string())
            .unwrap_or_else(|_| "runtime.package.fingerprint.agent_pool".to_string());
        let event_family = event_family.into();
        let event_kind = event_kind.into();

        Ok(JournalRecord {
            journal_schema_version: JOURNAL_SCHEMA_VERSION,
            journal_seq,
            record_id: format!("journal.record.agent_pool.{journal_seq}"),
            record_kind,
            run_id: run_id.clone(),
            agent_id: agent_id.clone(),
            turn_id: None,
            attempt_id: None,
            subject_ref: subject_ref.clone(),
            related_refs: related_refs.clone(),
            causal_refs: Vec::new(),
            source: source.clone(),
            destination: Some(DestinationRef::with_kind(
                DestinationKind::Journal,
                "destination.journal.agent_pool",
            )),
            correlation_keys: Vec::new(),
            tags: vec!["feature:agent_pool".to_string()],
            delivery_semantics: "journal_backed".to_string(),
            event_index: EventIndexProjection {
                run_id,
                agent_id,
                turn_id: None,
                event_family,
                event_kind,
                source,
                destination: Some(DestinationRef::with_kind(
                    DestinationKind::EventStream,
                    "destination.event_stream.agent_pool",
                )),
                subject_ref,
                related_refs,
                correlation_keys: Vec::new(),
                tags: vec!["feature:agent_pool".to_string()],
                privacy_class: PrivacyClass::ContentRefsOnly,
                delivery_semantics: "journal_backed".to_string(),
            },
            timestamp_millis: journal_seq,
            runtime_package_fingerprint: fingerprint,
            privacy: PrivacyClass::ContentRefsOnly,
            content_refs,
            redaction_policy_id: "redaction.agent_pool.default".to_string(),
            idempotency_key,
            dedupe_key: None,
            checkpoint_ref: None,
            payload,
        })
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "event publication mirrors the durable event envelope fields so lineage stays explicit at the call site"
    )]
    fn publish_agent_pool_event(
        &self,
        run_id: RunId,
        agent_id: AgentId,
        event_kind: EventKind,
        message_id: Option<MessageId>,
        wake_condition_id: Option<WakeConditionId>,
        subject_ref: EntityRef,
        mut related_refs: Vec<EntityRef>,
        destination: Option<DestinationRef>,
        policy_refs: Vec<PolicyRef>,
        journal_cursor: Option<JournalCursor>,
        summary: impl Into<String>,
    ) -> Result<EventFrame, AgentError> {
        if let Some(condition_id) = wake_condition_id {
            related_refs.push(EntityRef::wake_condition(condition_id));
        }
        let event_counter = self.store.next_event_sequence(&self.pool_id)?;
        let fingerprint = self
            .runtime
            .run_snapshot(&run_id)
            .map(|snapshot| snapshot.runtime_package_fingerprint.as_str().to_string())
            .unwrap_or_else(|_| "runtime.package.fingerprint.agent_pool".to_string());
        let event = AgentEvent::with_redacted_summary(
            EventEnvelope {
                schema_version: EVENT_SCHEMA_VERSION,
                event_id: EventId::new(format!(
                    "event.agent_pool.{}.{}",
                    self.pool_id.as_str(),
                    event_counter
                )),
                event_seq: 0,
                event_family: EventFamily::AgentPool,
                event_kind,
                payload_schema_version: 1,
                timestamp: format!("1970-01-01T00:00:{event_counter:02}Z"),
                recorded_at: format!("1970-01-01T00:00:{event_counter:02}Z"),
                run_id,
                agent_id,
                turn_id: None,
                attempt_id: None,
                message_id,
                context_item_id: None,
                trace_id: TraceId::new(format!("trace.agent_pool.{}", self.pool_id.as_str())),
                span_id: SpanId::new(format!(
                    "span.agent_pool.{}.{}",
                    self.pool_id.as_str(),
                    event_counter
                )),
                parent_event_id: None,
                caused_by: None,
                subject_ref,
                related_refs,
                causal_refs: Vec::new(),
                correlation: EventCorrelation::default(),
                tags: vec![crate::event::EventTag::new("feature:agent_pool")],
                source: SourceRef::with_kind(SourceKind::Sdk, "source.sdk.agent_pool"),
                destination,
                policy_refs,
                journal_cursor,
                state_before: None,
                state_after: None,
                delivery_semantics: EventDeliverySemantics::JournalBacked,
                privacy: PrivacyClass::ContentRefsOnly,
                content_capture: ContentCaptureMode::Off,
                redaction_policy_id: "redaction.agent_pool.default".to_string(),
                runtime_package_fingerprint: fingerprint,
            },
            summary,
        );
        let frame = EventFrame {
            cursor: event.envelope.cursor(EventStreamScope::All),
            event,
            archive_cursor: None,
            overflow: None,
        };
        self.runtime
            .event_bus_port(&frame.event.envelope.run_id)?
            .publish(frame.clone())?;
        Ok(frame)
    }

    fn resolve_address(&self, message: &RunMessage) -> Vec<RunId> {
        let Ok(snapshot) = self.snapshot() else {
            return Vec::new();
        };
        let members = snapshot
            .members
            .iter()
            .cloned()
            .map(|member| (member.run_id.clone(), member))
            .collect::<BTreeMap<_, _>>();
        let topics = topics_from_members(&snapshot.members);

        if !members.contains_key(&message.from) || !snapshot.message_policy.allows(message) {
            return Vec::new();
        }

        let mut candidates = match &message.to.target {
            RunAddressTarget::Run { run_id } => vec![run_id.clone()],
            RunAddressTarget::Agent { agent_id } => members
                .values()
                .filter(|member| &member.agent_id == agent_id)
                .map(|member| member.run_id.clone())
                .collect::<Vec<_>>(),
            RunAddressTarget::Topic { topic_id } => topics
                .get(topic_id)
                .map(|runs| runs.iter().cloned().collect::<Vec<_>>())
                .unwrap_or_default(),
            RunAddressTarget::Pool { pool_id } if pool_id == &self.pool_id => {
                members.keys().cloned().collect::<Vec<_>>()
            }
            RunAddressTarget::Pool { .. } => Vec::new(),
        };

        candidates.retain(|run_id| {
            members
                .get(run_id)
                .is_some_and(|member| member.allows_message_policies(&message.policy_refs))
        });

        if matches!(message.to.target, RunAddressTarget::Pool { .. })
            && !snapshot.message_policy.include_sender_in_pool_broadcast
        {
            candidates.retain(|run_id| run_id != &message.from);
        }

        candidates.sort();
        candidates.dedup();
        candidates
    }

    fn observable_member_runs(&self) -> Vec<RunId> {
        self.snapshot()
            .map(|snapshot| {
                snapshot
                    .members
                    .iter()
                    .filter(|member| member.allows_message_policies(&snapshot.policy_refs))
                    .map(|member| member.run_id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn member(&self, run_id: &RunId) -> Result<AgentPoolMember, AgentError> {
        self.snapshot()?
            .members
            .into_iter()
            .find(|member| &member.run_id == run_id)
            .ok_or_else(|| {
                AgentError::new(
                    AgentErrorKind::InvalidStateTransition,
                    RetryClassification::NotRetryable,
                    "run is not a member of this agent pool",
                )
            })
    }

    /// Rehydrates the current durable pool snapshot from the configured store.
    /// This returns only pool-backed membership, message, wake, policy, and cursor state; it
    /// does not subscribe to the global event bus or synthesize missing records.
    pub fn snapshot(&self) -> Result<AgentPoolSnapshot, AgentError> {
        self.store.snapshot(&self.pool_id)
    }

    fn trigger_matching_wakes(&self, frame: &EventFrame) -> Result<(), AgentError> {
        if matches!(
            frame.event.envelope.event_kind,
            EventKind::WakeConditionRegistered
                | EventKind::WakeConditionTriggered
                | EventKind::WakeConditionTimedOut
                | EventKind::WakeConditionCancelled
                | EventKind::WakeConditionFailed
        ) {
            return Ok(());
        }

        let wakes = self.snapshot()?.wakes;
        for wake in wakes
            .into_iter()
            .filter(|wake| wake.registration.status == WakeRegistrationStatus::Registered)
        {
            if wake.compiled_filter.matches_envelope(&frame.event.envelope) {
                self.record_wake_status(
                    &wake.condition,
                    wake.compiled_filter,
                    WakeRegistrationStatus::Triggered,
                    Some(frame.event.envelope.event_id.clone()),
                )?;
            }
        }
        Ok(())
    }

    /// Watches durable pool-store changes after the supplied cursor.
    /// This is a pool-scoped coordination-record stream, not a global event bus.
    pub fn watch_pool(
        &self,
        cursor: Option<AgentPoolStoreCursor>,
    ) -> Result<AgentPoolStoreStream, AgentError> {
        self.store.watch(&self.pool_id, cursor)
    }
}

#[derive(Clone)]
/// Holds agent pool builder application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct AgentPoolBuilder {
    pool_id: AgentPoolId,
    runtime: Option<AgentRuntime>,
    message_policy: AgentPoolMessagePolicy,
    wake_policy: AgentPoolWakePolicy,
    policy_refs: Vec<PolicyRef>,
    store: Option<Arc<dyn AgentPoolStore>>,
}

impl AgentPoolBuilder {
    /// Returns an updated value with runtime configured.
    /// This stores the runtime used by the pool builder; no run is started until `start_run` is
    /// called.
    pub fn runtime(mut self, runtime: AgentRuntime) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Returns an updated value with message policy configured.
    /// This is builder configuration only and performs no I/O or run coordination.
    pub fn message_policy(mut self, policy: AgentPoolMessagePolicy) -> Self {
        self.message_policy = policy;
        self
    }

    /// Returns an updated value with wake policy configured.
    /// This is builder configuration only and performs no I/O or run coordination.
    pub fn wake_policy(mut self, policy: AgentPoolWakePolicy) -> Self {
        self.wake_policy = policy;
        self
    }

    /// Returns an updated value with policy ref configured.
    /// This sets the policy reference on the coordination value and performs no I/O.
    pub fn policy_ref(mut self, policy_ref: PolicyRef) -> Self {
        self.policy_refs.push(policy_ref);
        self
    }

    /// Returns an updated value with the pool store configured.
    /// The store is the shared coordination authority for membership,
    /// messages, wake registrations, dedupe, rehydration, and pool watch.
    pub fn store<S>(mut self, store: S) -> Self
    where
        S: AgentPoolStore + 'static,
    {
        self.store = Some(Arc::new(store));
        self
    }

    /// Returns an updated value with a dynamically dispatched pool store.
    /// Use this when sharing one store instance across multiple pool handles
    /// or when a host provides its own adapter.
    pub fn shared_store(mut self, store: Arc<dyn AgentPoolStore>) -> Self {
        self.store = Some(store);
        self
    }

    /// Finishes builder validation and returns the configured value.
    /// This is data-only unless the surrounding builder explicitly
    /// documents adapter or store access.
    pub fn build(self) -> Result<AgentPool, AgentError> {
        let runtime = self
            .runtime
            .ok_or_else(|| AgentError::host_configuration_needed("agent pool requires runtime"))?;
        let store = self
            .store
            .unwrap_or_else(|| Arc::new(InMemoryAgentPoolStore::default()));
        store.open_pool(
            self.pool_id.clone(),
            AgentPoolStoreConfig {
                message_policy: self.message_policy,
                wake_policy: self.wake_policy,
                policy_refs: self.policy_refs,
            },
        )?;
        Ok(AgentPool {
            pool_id: self.pool_id,
            runtime,
            store,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Store configuration for one logical agent pool.
/// This is durable pool metadata; constructing it does not open a
/// concrete store or start coordination work.
pub struct AgentPoolStoreConfig {
    /// Message policy used when resolving pool messages.
    pub message_policy: AgentPoolMessagePolicy,
    /// Wake policy used when scoping wake filters.
    pub wake_policy: AgentPoolWakePolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy refs that scope pool-level observation and membership.
    pub policy_refs: Vec<PolicyRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Durable cursor for pool-scoped store records.
/// Cursors are scoped by the pool passed to `watch` and should not be
/// used as global event or journal cursors.
pub struct AgentPoolStoreCursor {
    /// Monotonic sequence within a logical pool store partition.
    pub sequence: u64,
}

impl AgentPoolStoreCursor {
    /// Builds the initial cursor before any pool-store record.
    pub fn start() -> Self {
        Self { sequence: 0 }
    }

    /// Builds a cursor for a known sequence.
    pub fn new(sequence: u64) -> Self {
        Self { sequence }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Rehydratable snapshot for one logical agent pool.
/// The snapshot is derived only from durable store records; callers must
/// not synthesize members, messages, or wakes outside the store.
pub struct AgentPoolSnapshot {
    /// Stable pool id used for typed lineage, lookup, or dedupe.
    pub pool_id: AgentPoolId,
    /// Whether the pool-created lifecycle record has been persisted.
    pub created: bool,
    /// Current members visible in the pool.
    pub members: Vec<AgentPoolMember>,
    /// Current topic ids known to the pool.
    pub topics: Vec<TopicId>,
    /// Message policy used when resolving pool messages.
    pub message_policy: AgentPoolMessagePolicy,
    /// Wake policy used when scoping wake filters.
    pub wake_policy: AgentPoolWakePolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy refs that scope pool-level observation and membership.
    pub policy_refs: Vec<PolicyRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Durable run-message status records known to the pool.
    pub messages: Vec<AgentPoolStoredMessage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Durable wake registrations known to the pool.
    pub wakes: Vec<AgentPoolStoredWake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Latest store cursor represented by this snapshot.
    pub cursor: Option<AgentPoolStoreCursor>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Durable message status stored for pool rehydration and dedupe.
pub struct AgentPoolStoredMessage {
    /// Original run message request.
    pub message: RunMessage,
    /// Receipt for the stored status transition.
    pub receipt: MessageReceipt,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Durable wake state stored for pool rehydration and cross-handle wake
/// triggering.
pub struct AgentPoolStoredWake {
    /// Original wake condition.
    pub condition: WakeCondition,
    /// Scoped, compiled filter used for envelope matching.
    pub compiled_filter: CompiledEventFilter,
    /// Latest durable registration status.
    pub registration: WakeRegistration,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// One append-only pool-store record. These records are a durable
/// coordination view linked to journal-backed events; they are not a
/// replacement for `AgentEventBus`.
pub struct AgentPoolStoreRecord {
    /// Stable pool id used for typed lineage, lookup, or dedupe.
    pub pool_id: AgentPoolId,
    /// Cursor assigned by the store.
    pub cursor: AgentPoolStoreCursor,
    /// Stored pool change.
    pub payload: AgentPoolStoreRecordPayload,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Finite pool-store record variants.
#[expect(
    clippy::large_enum_variant,
    reason = "pool store payloads are durable serde records; preserve direct variant ergonomics until a separate storage-envelope redesign"
)]
pub enum AgentPoolStoreRecordPayload {
    /// Pool metadata was opened or initialized.
    PoolOpened {
        /// Configuration persisted for this pool.
        config: AgentPoolStoreConfig,
    },
    /// Pool lifecycle was marked created by a journal-backed operation.
    PoolCreated,
    /// A run joined the pool.
    MemberJoined {
        /// Joined member.
        member: AgentPoolMember,
    },
    /// A run left the pool.
    MemberLeft {
        /// Left member.
        member: AgentPoolMember,
    },
    /// A run-message status was persisted.
    RunMessage {
        /// Stored message transition.
        stored: AgentPoolStoredMessage,
    },
    /// A wake status was persisted.
    Wake {
        /// Stored wake transition.
        stored: AgentPoolStoredWake,
    },
}

#[derive(Clone, Debug)]
/// Iterator over durable pool-store records for one logical pool.
pub struct AgentPoolStoreStream {
    records: VecDeque<AgentPoolStoreRecord>,
}

impl AgentPoolStoreStream {
    /// Builds a store stream from records already loaded by a store.
    pub fn new(records: impl IntoIterator<Item = AgentPoolStoreRecord>) -> Self {
        Self {
            records: records.into_iter().collect(),
        }
    }
}

impl Iterator for AgentPoolStoreStream {
    type Item = AgentPoolStoreRecord;

    fn next(&mut self) -> Option<Self::Item> {
        self.records.pop_front()
    }
}

/// Port for durable/shared agent-pool coordination.
/// Implementations may use memory, SQLite, RPC, MCP, or another backing
/// service, but they must preserve the same pool-scoped records,
/// snapshots, idempotency, and watch semantics.
pub trait AgentPoolStore: Send + Sync {
    /// Create or open a logical pool and return the durable snapshot.
    fn open_pool(
        &self,
        pool_id: AgentPoolId,
        config: AgentPoolStoreConfig,
    ) -> Result<AgentPoolSnapshot, AgentError>;

    /// Rehydrate the current durable pool snapshot.
    fn snapshot(&self, pool_id: &AgentPoolId) -> Result<AgentPoolSnapshot, AgentError>;

    /// Mark the pool-created lifecycle as durable.
    fn record_pool_created(
        &self,
        pool_id: &AgentPoolId,
    ) -> Result<AgentPoolStoreCursor, AgentError>;

    /// Persist member join.
    fn join_member(
        &self,
        pool_id: &AgentPoolId,
        member: AgentPoolMember,
    ) -> Result<AgentPoolStoreCursor, AgentError>;

    /// Persist member leave and return the removed member.
    fn leave_member(
        &self,
        pool_id: &AgentPoolId,
        run_id: &RunId,
    ) -> Result<(AgentPoolMember, AgentPoolStoreCursor), AgentError>;

    /// Look up message dedupe state by idempotency key.
    fn message_receipt(
        &self,
        pool_id: &AgentPoolId,
        idempotency_key: &IdempotencyKey,
    ) -> Result<Option<MessageReceipt>, AgentError>;

    /// Persist one message status transition.
    fn record_message(
        &self,
        pool_id: &AgentPoolId,
        message: RunMessage,
        receipt: MessageReceipt,
    ) -> Result<AgentPoolStoreCursor, AgentError>;

    /// Look up wake dedupe state by idempotency key.
    fn wake_registration(
        &self,
        pool_id: &AgentPoolId,
        idempotency_key: &IdempotencyKey,
    ) -> Result<Option<WakeRegistration>, AgentError>;

    /// Look up one stored wake by condition id.
    fn wake(
        &self,
        pool_id: &AgentPoolId,
        condition_id: &WakeConditionId,
    ) -> Result<Option<AgentPoolStoredWake>, AgentError>;

    /// Persist one wake status transition.
    fn record_wake(
        &self,
        pool_id: &AgentPoolId,
        condition: WakeCondition,
        compiled_filter: CompiledEventFilter,
        registration: WakeRegistration,
    ) -> Result<AgentPoolStoreCursor, AgentError>;

    /// Read durable pool changes after the supplied cursor.
    fn watch(
        &self,
        pool_id: &AgentPoolId,
        cursor: Option<AgentPoolStoreCursor>,
    ) -> Result<AgentPoolStoreStream, AgentError>;

    /// Allocate a unique event sequence for pool event IDs.
    fn next_event_sequence(&self, pool_id: &AgentPoolId) -> Result<u64, AgentError>;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds agent pool member application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct AgentPoolMember {
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of topics values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub topics: Vec<TopicId>,
}

impl AgentPoolMember {
    /// Creates a new application::agent_pool value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(run_id: RunId, agent_id: AgentId) -> Self {
        Self {
            run_id,
            agent_id,
            policy_refs: Vec::new(),
            topics: Vec::new(),
        }
    }

    /// Returns an updated value with policy ref configured.
    /// This sets the policy reference on the coordination value and performs no I/O.
    pub fn policy_ref(mut self, policy_ref: PolicyRef) -> Self {
        self.policy_refs.push(policy_ref);
        self
    }

    /// Returns an updated value with topic configured.
    /// This sets the topic id on the address/filter value and performs no subscription by
    /// itself.
    pub fn topic(mut self, topic_id: TopicId) -> Self {
        self.topics.push(topic_id);
        self
    }

    fn allows_message_policies(&self, required: &[PolicyRef]) -> bool {
        required
            .iter()
            .all(|required| self.policy_refs.contains(required))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds agent pool message policy application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct AgentPoolMessagePolicy {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed required policy refs references. Resolving them is separate from
    /// constructing this record.
    pub required_policy_refs: Vec<PolicyRef>,
    /// Whether pool broadcast delivery includes the sender run as a recipient.
    /// Use this for explicit loopback semantics; the default coordination path should avoid
    /// accidental self-delivery.
    pub include_sender_in_pool_broadcast: bool,
}

impl AgentPoolMessagePolicy {
    /// Builds the bounded defaults value with the documented defaults.
    /// This uses only local coordinator state and performs no hidden host work.
    pub fn bounded_defaults() -> Self {
        Self {
            required_policy_refs: Vec::new(),
            include_sender_in_pool_broadcast: false,
        }
    }

    fn allows(&self, message: &RunMessage) -> bool {
        self.required_policy_refs
            .iter()
            .all(|required| message.policy_refs.contains(required))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds agent pool wake policy application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct AgentPoolWakePolicy {
    /// Whether envelope only is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub envelope_only: bool,
}

impl AgentPoolWakePolicy {
    /// Returns an updated value with safe defaults configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn safe_defaults() -> Self {
        Self {
            envelope_only: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds run address application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct RunAddress {
    /// Target used by this record or request.
    pub target: RunAddressTarget,
    /// Typed destination reference that records where this item is being sent
    /// or projected.
    pub destination_ref: DestinationRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed related refs references. Resolving them is separate from
    /// constructing this record.
    pub related_refs: Vec<EntityRef>,
}

impl RunAddress {
    /// Builds the run value with the documented defaults.
    /// This uses only local coordinator state and performs no hidden host work.
    pub fn run(run_id: RunId) -> Self {
        Self {
            destination_ref: DestinationRef::with_kind(DestinationKind::Agent, run_id.as_str()),
            related_refs: vec![EntityRef::run(run_id.clone())],
            target: RunAddressTarget::Run { run_id },
        }
    }

    /// Returns agent for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn agent(agent_id: AgentId) -> Self {
        Self {
            destination_ref: DestinationRef::with_kind(DestinationKind::Agent, agent_id.as_str()),
            related_refs: vec![EntityRef::agent(agent_id.clone())],
            target: RunAddressTarget::Agent { agent_id },
        }
    }

    /// Returns an updated value with topic configured.
    /// This sets the topic id on the address/filter value and performs no subscription by
    /// itself.
    pub fn topic(topic_id: TopicId) -> Self {
        Self {
            destination_ref: DestinationRef::with_kind(DestinationKind::Topic, topic_id.as_str()),
            related_refs: vec![EntityRef::topic(topic_id.clone())],
            target: RunAddressTarget::Topic { topic_id },
        }
    }

    /// Builds the pool value with the documented defaults.
    /// This uses only local coordinator state and performs no hidden host work.
    pub fn pool(pool_id: AgentPoolId) -> Self {
        Self {
            destination_ref: DestinationRef::with_kind(
                DestinationKind::AgentPool,
                pool_id.as_str(),
            ),
            related_refs: vec![EntityRef::agent_pool(pool_id.clone())],
            target: RunAddressTarget::Pool { pool_id },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Enumerates the finite run address target cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RunAddressTarget {
    /// Use this variant when the contract needs to represent run; selecting it has no side effect by itself.
    Run {
        /// Run identifier used for lineage, filtering, replay, and dedupe.
        run_id: RunId,
    },
    /// Use this variant when the contract needs to represent agent; selecting it has no side effect by itself.
    Agent {
        /// Agent identifier used for lineage, filtering, and ownership
        /// checks.
        agent_id: AgentId,
    },
    /// Use this variant when the contract needs to represent topic; selecting it has no side effect by itself.
    Topic {
        /// Stable topic id used for typed lineage, lookup, or dedupe.
        topic_id: TopicId,
    },
    /// Use this variant when the contract needs to represent pool; selecting it has no side effect by itself.
    Pool {
        /// Stable pool id used for typed lineage, lookup, or dedupe.
        pool_id: AgentPoolId,
    },
}

impl RunAddressTarget {
    /// Returns run id for this application::agent_pool value without
    /// performing external I/O.
    pub fn run_id(&self) -> Option<&RunId> {
        match self {
            Self::Run { run_id } => Some(run_id),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds run message application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct RunMessage {
    /// Message identifier for transcript, projection, or provider-response
    /// lineage.
    pub message_id: MessageId,
    /// From used by this record or request.
    pub from: RunId,
    /// To used by this record or request.
    pub to: RunAddress,
    /// Content reference where payload bytes or structured tool output are
    /// stored.
    pub content_ref: ContentRef,
    /// Correlation used by this record or request.
    pub correlation: EventCorrelation,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional reply to value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub reply_to: Option<MessageId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional response contract value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub response_contract: Option<MessageResponseContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Time value in milliseconds for expires at millis.
    /// Use it for timeout, ordering, or diagnostic calculations.
    pub expires_at_millis: Option<u64>,
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency_key: IdempotencyKey,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
}

impl RunMessage {
    /// Creates a new application::agent_pool value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(
        message_id: MessageId,
        from: RunId,
        to: RunAddress,
        content_ref: ContentRef,
        idempotency_key: IdempotencyKey,
    ) -> Self {
        Self {
            message_id,
            from,
            to,
            content_ref,
            correlation: EventCorrelation::default(),
            reply_to: None,
            response_contract: None,
            expires_at_millis: None,
            idempotency_key,
            policy_refs: Vec::new(),
        }
    }

    /// Returns an updated value with policy ref configured.
    /// This sets the policy reference on the coordination value and performs no I/O.
    pub fn policy_ref(mut self, policy_ref: PolicyRef) -> Self {
        self.policy_refs.push(policy_ref);
        self
    }

    fn target_related_refs(&self, delivered_to: &[RunId]) -> Vec<EntityRef> {
        let mut refs = self.to.related_refs.clone();
        refs.extend(delivered_to.iter().cloned().map(EntityRef::run));
        refs.sort_by(|left, right| {
            left.kind
                .cmp(&right.kind)
                .then_with(|| left.as_str().cmp(right.as_str()))
        });
        refs.dedup_by(|left, right| left.kind == right.kind && left.as_str() == right.as_str());
        refs
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds message response contract application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct MessageResponseContract {
    /// Expected responses used by this record or request.
    pub expected_responses: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Time value in milliseconds for timeout millis.
    /// Use it for timeout, ordering, or diagnostic calculations.
    pub timeout_millis: Option<u64>,
}

impl MessageResponseContract {
    /// Builds the one response value with the documented defaults.
    /// This uses only local coordinator state and performs no hidden host work.
    pub fn one_response(timeout_millis: u64) -> Self {
        Self {
            expected_responses: 1,
            timeout_millis: Some(timeout_millis),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds message receipt application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct MessageReceipt {
    /// Message identifier for transcript, projection, or provider-response
    /// lineage.
    pub message_id: MessageId,
    /// Finite status for this record or lifecycle stage.
    pub status: MessageStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of delivered to values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub delivered_to: Vec<RunId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub journal_cursor: Option<JournalCursor>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite message status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum MessageStatus {
    /// Use this variant when the contract needs to represent accepted; selecting it has no side effect by itself.
    Accepted,
    /// Use this variant when the contract needs to represent delivered; selecting it has no side effect by itself.
    Delivered,
    /// Use this variant when the contract needs to represent responded; selecting it has no side effect by itself.
    Responded,
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed,
    /// Use this variant when the contract needs to represent timed out; selecting it has no side effect by itself.
    TimedOut,
    /// Use this variant when the contract needs to represent expired; selecting it has no side effect by itself.
    Expired,
    /// Use this variant when the contract needs to represent cancelled; selecting it has no side effect by itself.
    Cancelled,
}

impl MessageStatus {
    fn event_kind(&self) -> EventKind {
        match self {
            Self::Accepted => EventKind::RunMessageAccepted,
            Self::Delivered => EventKind::RunMessageDelivered,
            Self::Responded => EventKind::RunMessageResponded,
            Self::Failed => EventKind::RunMessageFailed,
            Self::TimedOut => EventKind::RunMessageTimedOut,
            Self::Expired => EventKind::RunMessageExpired,
            Self::Cancelled => EventKind::RunMessageCancelled,
        }
    }

    fn redacted_summary(&self) -> &'static str {
        match self {
            Self::Accepted => "run message accepted",
            Self::Delivered => "run message delivered",
            Self::Responded => "run message responded",
            Self::Failed => "run message failed",
            Self::TimedOut => "run message timed out",
            Self::Expired => "run message expired",
            Self::Cancelled => "run message cancelled",
        }
    }

    fn is_terminal_delivery(&self) -> bool {
        matches!(
            self,
            Self::Delivered
                | Self::Responded
                | Self::Failed
                | Self::TimedOut
                | Self::Expired
                | Self::Cancelled
        )
    }

    fn effect_terminal_status(&self) -> EffectTerminalStatus {
        match self {
            Self::Delivered | Self::Responded => EffectTerminalStatus::Completed,
            Self::TimedOut => EffectTerminalStatus::TimedOut,
            Self::Cancelled => EffectTerminalStatus::Cancelled,
            Self::Accepted => EffectTerminalStatus::Unknown,
            Self::Failed | Self::Expired => EffectTerminalStatus::Failed,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds wake condition application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct WakeCondition {
    /// Stable condition id used for typed lineage, lookup, or dedupe.
    pub condition_id: WakeConditionId,
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Filter used by this record or request.
    pub filter: EventFilter,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Time value in milliseconds for timeout millis.
    /// Use it for timeout, ordering, or diagnostic calculations.
    pub timeout_millis: Option<u64>,
    /// Resume with used by this record or request.
    pub resume_with: ResumeInputPolicy,
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency_key: IdempotencyKey,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
}

impl WakeCondition {
    /// Creates a new application::agent_pool value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(
        condition_id: WakeConditionId,
        run_id: RunId,
        filter: EventFilter,
        idempotency_key: IdempotencyKey,
    ) -> Self {
        Self {
            condition_id,
            run_id,
            filter,
            timeout_millis: None,
            resume_with: ResumeInputPolicy::MatchingEventRefs,
            idempotency_key,
            policy_refs: Vec::new(),
        }
    }

    /// Returns an updated value with timeout millis configured.
    /// This updates the wake timeout on the condition value and performs no scheduling by
    /// itself.
    pub fn timeout_millis(mut self, timeout_millis: u64) -> Self {
        self.timeout_millis = Some(timeout_millis);
        self
    }

    /// Returns an updated value with policy ref configured.
    /// This sets the policy reference on the coordination value and performs no I/O.
    pub fn policy_ref(mut self, policy_ref: PolicyRef) -> Self {
        self.policy_refs.push(policy_ref);
        self
    }

    /// Computes or returns compile envelope filter for the
    /// application::agent_pool contract without external I/O or side effects.
    pub fn compile_envelope_filter(&self) -> Result<CompiledEventFilter, AgentError> {
        let mut filter = self.filter.clone();
        filter.payload_access = PayloadAccessMode::EnvelopeOnly;
        filter.compile()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite resume input policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ResumeInputPolicy {
    /// Use this variant when the contract needs to represent matching event refs; selecting it has no side effect by itself.
    MatchingEventRefs,
    /// Use this variant when the contract needs to represent redacted summary; selecting it has no side effect by itself.
    RedactedSummary,
    /// Use this variant when the contract needs to represent none; selecting it has no side effect by itself.
    None,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds wake registration application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct WakeRegistration {
    /// Stable condition id used for typed lineage, lookup, or dedupe.
    pub condition_id: WakeConditionId,
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Finite status for this record or lifecycle stage.
    pub status: WakeRegistrationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub journal_cursor: Option<JournalCursor>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite wake registration status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum WakeRegistrationStatus {
    /// Use this variant when the contract needs to represent registered; selecting it has no side effect by itself.
    Registered,
    /// Use this variant when the contract needs to represent triggered; selecting it has no side effect by itself.
    Triggered,
    /// Use this variant when the contract needs to represent timed out; selecting it has no side effect by itself.
    TimedOut,
    /// Use this variant when the contract needs to represent cancelled; selecting it has no side effect by itself.
    Cancelled,
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed,
}

impl WakeRegistrationStatus {
    fn event_kind(&self) -> EventKind {
        match self {
            Self::Registered => EventKind::WakeConditionRegistered,
            Self::Triggered => EventKind::WakeConditionTriggered,
            Self::TimedOut => EventKind::WakeConditionTimedOut,
            Self::Cancelled => EventKind::WakeConditionCancelled,
            Self::Failed => EventKind::WakeConditionFailed,
        }
    }

    fn redacted_summary(&self) -> &'static str {
        match self {
            Self::Registered => "wake condition registered",
            Self::Triggered => "wake condition triggered",
            Self::TimedOut => "wake condition timed out",
            Self::Cancelled => "wake condition cancelled",
            Self::Failed => "wake condition failed",
        }
    }
}

#[derive(Clone, Debug)]
struct AgentPoolState {
    created: bool,
    members: BTreeMap<RunId, AgentPoolMember>,
    topics: BTreeMap<TopicId, BTreeSet<RunId>>,
    message_policy: AgentPoolMessagePolicy,
    wake_policy: AgentPoolWakePolicy,
    policy_refs: Vec<PolicyRef>,
    messages: BTreeMap<MessageId, AgentPoolStoredMessage>,
    message_dedupe: BTreeMap<IdempotencyKey, MessageReceipt>,
    wake_dedupe: BTreeMap<IdempotencyKey, WakeRegistration>,
    wakes: BTreeMap<WakeConditionId, AgentPoolStoredWake>,
    next_event_counter: u64,
}

impl AgentPoolState {
    fn new(config: AgentPoolStoreConfig) -> Self {
        Self {
            created: false,
            members: BTreeMap::new(),
            topics: BTreeMap::new(),
            message_policy: config.message_policy,
            wake_policy: config.wake_policy,
            policy_refs: config.policy_refs,
            messages: BTreeMap::new(),
            message_dedupe: BTreeMap::new(),
            wake_dedupe: BTreeMap::new(),
            wakes: BTreeMap::new(),
            next_event_counter: 0,
        }
    }

    fn config(&self) -> AgentPoolStoreConfig {
        AgentPoolStoreConfig {
            message_policy: self.message_policy.clone(),
            wake_policy: self.wake_policy.clone(),
            policy_refs: self.policy_refs.clone(),
        }
    }

    fn snapshot(
        &self,
        pool_id: AgentPoolId,
        cursor: Option<AgentPoolStoreCursor>,
    ) -> AgentPoolSnapshot {
        AgentPoolSnapshot {
            pool_id,
            created: self.created,
            members: self.members.values().cloned().collect(),
            topics: self.topics.keys().cloned().collect(),
            message_policy: self.message_policy.clone(),
            wake_policy: self.wake_policy.clone(),
            policy_refs: self.policy_refs.clone(),
            messages: self.messages.values().cloned().collect(),
            wakes: self.wakes.values().cloned().collect(),
            cursor,
        }
    }

    fn index_member(&mut self, member: AgentPoolMember) {
        for topic in &member.topics {
            self.topics
                .entry(topic.clone())
                .or_default()
                .insert(member.run_id.clone());
        }
        self.members.insert(member.run_id.clone(), member);
    }

    fn remove_member(&mut self, run_id: &RunId) -> Result<AgentPoolMember, AgentError> {
        let member = self.members.remove(run_id).ok_or_else(|| {
            AgentError::new(
                AgentErrorKind::InvalidStateTransition,
                RetryClassification::NotRetryable,
                "run is not a member of this agent pool",
            )
        })?;
        for topic in &member.topics {
            let remove_topic = if let Some(runs) = self.topics.get_mut(topic) {
                runs.remove(run_id);
                runs.is_empty()
            } else {
                false
            };
            if remove_topic {
                self.topics.remove(topic);
            }
        }
        Ok(member)
    }
}

#[derive(Clone, Debug, Default)]
/// In-memory `AgentPoolStore` implementation.
/// Cloning this value shares the same backing map, making it useful for
/// tests that simulate two process-local `AgentPool` handles sharing one
/// coordination authority. Separate default values are isolated.
pub struct InMemoryAgentPoolStore {
    pools: Arc<Mutex<BTreeMap<AgentPoolId, AgentPoolState>>>,
    records: Arc<Mutex<BTreeMap<AgentPoolId, Vec<AgentPoolStoreRecord>>>>,
}

impl InMemoryAgentPoolStore {
    fn with_pool_state<T>(
        &self,
        pool_id: &AgentPoolId,
        f: impl FnOnce(&mut AgentPoolState) -> Result<T, AgentError>,
    ) -> Result<T, AgentError> {
        let mut pools = self
            .pools
            .lock()
            .map_err(|_| AgentError::contract_violation("agent pool store lock poisoned"))?;
        let state = pools.get_mut(pool_id).ok_or_else(|| {
            AgentError::new(
                AgentErrorKind::HostConfigurationNeeded,
                RetryClassification::HostConfigurationNeeded,
                "agent pool store has not opened this pool",
            )
        })?;
        f(state)
    }

    fn append_record(
        &self,
        pool_id: &AgentPoolId,
        payload: AgentPoolStoreRecordPayload,
    ) -> Result<AgentPoolStoreCursor, AgentError> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| AgentError::contract_violation("agent pool store lock poisoned"))?;
        let entries = records.entry(pool_id.clone()).or_default();
        let cursor = AgentPoolStoreCursor::new(entries.len() as u64 + 1);
        entries.push(AgentPoolStoreRecord {
            pool_id: pool_id.clone(),
            cursor: cursor.clone(),
            payload,
        });
        Ok(cursor)
    }

    fn latest_cursor(
        &self,
        pool_id: &AgentPoolId,
    ) -> Result<Option<AgentPoolStoreCursor>, AgentError> {
        let records = self
            .records
            .lock()
            .map_err(|_| AgentError::contract_violation("agent pool store lock poisoned"))?;
        Ok(records
            .get(pool_id)
            .and_then(|records| records.last().map(|record| record.cursor.clone())))
    }
}

impl AgentPoolStore for InMemoryAgentPoolStore {
    fn open_pool(
        &self,
        pool_id: AgentPoolId,
        config: AgentPoolStoreConfig,
    ) -> Result<AgentPoolSnapshot, AgentError> {
        {
            let mut pools = self
                .pools
                .lock()
                .map_err(|_| AgentError::contract_violation("agent pool store lock poisoned"))?;
            if let Some(existing) = pools.get(&pool_id) {
                if existing.config() != config {
                    return Err(AgentError::new(
                        AgentErrorKind::InvalidStateTransition,
                        RetryClassification::RepairNeeded,
                        "agent pool store config conflicts with existing pool",
                    ));
                }
            } else {
                pools.insert(pool_id.clone(), AgentPoolState::new(config.clone()));
                drop(pools);
                self.append_record(&pool_id, AgentPoolStoreRecordPayload::PoolOpened { config })?;
            }
        }
        self.snapshot(&pool_id)
    }

    fn snapshot(&self, pool_id: &AgentPoolId) -> Result<AgentPoolSnapshot, AgentError> {
        let cursor = self.latest_cursor(pool_id)?;
        let pools = self
            .pools
            .lock()
            .map_err(|_| AgentError::contract_violation("agent pool store lock poisoned"))?;
        pools
            .get(pool_id)
            .map(|state| state.snapshot(pool_id.clone(), cursor))
            .ok_or_else(|| {
                AgentError::new(
                    AgentErrorKind::HostConfigurationNeeded,
                    RetryClassification::HostConfigurationNeeded,
                    "agent pool store has not opened this pool",
                )
            })
    }

    fn record_pool_created(
        &self,
        pool_id: &AgentPoolId,
    ) -> Result<AgentPoolStoreCursor, AgentError> {
        self.with_pool_state(pool_id, |state| {
            state.created = true;
            Ok(())
        })?;
        self.append_record(pool_id, AgentPoolStoreRecordPayload::PoolCreated)
    }

    fn join_member(
        &self,
        pool_id: &AgentPoolId,
        member: AgentPoolMember,
    ) -> Result<AgentPoolStoreCursor, AgentError> {
        self.with_pool_state(pool_id, |state| {
            state.index_member(member.clone());
            Ok(())
        })?;
        self.append_record(
            pool_id,
            AgentPoolStoreRecordPayload::MemberJoined { member },
        )
    }

    fn leave_member(
        &self,
        pool_id: &AgentPoolId,
        run_id: &RunId,
    ) -> Result<(AgentPoolMember, AgentPoolStoreCursor), AgentError> {
        let member = self.with_pool_state(pool_id, |state| state.remove_member(run_id))?;
        let cursor = self.append_record(
            pool_id,
            AgentPoolStoreRecordPayload::MemberLeft {
                member: member.clone(),
            },
        )?;
        Ok((member, cursor))
    }

    fn message_receipt(
        &self,
        pool_id: &AgentPoolId,
        idempotency_key: &IdempotencyKey,
    ) -> Result<Option<MessageReceipt>, AgentError> {
        self.with_pool_state(pool_id, |state| {
            Ok(state.message_dedupe.get(idempotency_key).cloned())
        })
    }

    fn record_message(
        &self,
        pool_id: &AgentPoolId,
        message: RunMessage,
        receipt: MessageReceipt,
    ) -> Result<AgentPoolStoreCursor, AgentError> {
        let stored = AgentPoolStoredMessage { message, receipt };
        self.with_pool_state(pool_id, |state| {
            state.message_dedupe.insert(
                stored.message.idempotency_key.clone(),
                stored.receipt.clone(),
            );
            state
                .messages
                .insert(stored.message.message_id.clone(), stored.clone());
            Ok(())
        })?;
        self.append_record(pool_id, AgentPoolStoreRecordPayload::RunMessage { stored })
    }

    fn wake_registration(
        &self,
        pool_id: &AgentPoolId,
        idempotency_key: &IdempotencyKey,
    ) -> Result<Option<WakeRegistration>, AgentError> {
        self.with_pool_state(pool_id, |state| {
            Ok(state.wake_dedupe.get(idempotency_key).cloned())
        })
    }

    fn wake(
        &self,
        pool_id: &AgentPoolId,
        condition_id: &WakeConditionId,
    ) -> Result<Option<AgentPoolStoredWake>, AgentError> {
        self.with_pool_state(pool_id, |state| Ok(state.wakes.get(condition_id).cloned()))
    }

    fn record_wake(
        &self,
        pool_id: &AgentPoolId,
        condition: WakeCondition,
        compiled_filter: CompiledEventFilter,
        registration: WakeRegistration,
    ) -> Result<AgentPoolStoreCursor, AgentError> {
        let stored = AgentPoolStoredWake {
            condition,
            compiled_filter,
            registration,
        };
        self.with_pool_state(pool_id, |state| {
            state.wake_dedupe.insert(
                stored.condition.idempotency_key.clone(),
                stored.registration.clone(),
            );
            state
                .wakes
                .insert(stored.condition.condition_id.clone(), stored.clone());
            Ok(())
        })?;
        self.append_record(pool_id, AgentPoolStoreRecordPayload::Wake { stored })
    }

    fn watch(
        &self,
        pool_id: &AgentPoolId,
        cursor: Option<AgentPoolStoreCursor>,
    ) -> Result<AgentPoolStoreStream, AgentError> {
        let start_after = cursor.map(|cursor| cursor.sequence).unwrap_or(0);
        let records = self
            .records
            .lock()
            .map_err(|_| AgentError::contract_violation("agent pool store lock poisoned"))?;
        Ok(AgentPoolStoreStream::new(
            records
                .get(pool_id)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|record| record.cursor.sequence > start_after),
        ))
    }

    fn next_event_sequence(&self, pool_id: &AgentPoolId) -> Result<u64, AgentError> {
        self.with_pool_state(pool_id, |state| {
            state.next_event_counter += 1;
            Ok(state.next_event_counter)
        })
    }
}

impl From<RunAddressTarget> for RunMessageAddressTargetRecord {
    fn from(value: RunAddressTarget) -> Self {
        match value {
            RunAddressTarget::Run { run_id } => Self::Run { run_id },
            RunAddressTarget::Agent { agent_id } => Self::Agent { agent_id },
            RunAddressTarget::Topic { topic_id } => Self::Topic { topic_id },
            RunAddressTarget::Pool { pool_id } => Self::Pool { pool_id },
        }
    }
}

impl From<MessageStatus> for RunMessageDeliveryStatus {
    fn from(value: MessageStatus) -> Self {
        match value {
            MessageStatus::Accepted => Self::Accepted,
            MessageStatus::Delivered => Self::Delivered,
            MessageStatus::Responded => Self::Responded,
            MessageStatus::Failed => Self::Failed,
            MessageStatus::TimedOut => Self::TimedOut,
            MessageStatus::Expired => Self::Expired,
            MessageStatus::Cancelled => Self::Cancelled,
        }
    }
}

impl From<ResumeInputPolicy> for WakeResumeInputPolicyRecord {
    fn from(value: ResumeInputPolicy) -> Self {
        match value {
            ResumeInputPolicy::MatchingEventRefs => Self::MatchingEventRefs,
            ResumeInputPolicy::RedactedSummary => Self::RedactedSummary,
            ResumeInputPolicy::None => Self::None,
        }
    }
}

impl From<WakeRegistrationStatus> for WakeTriggerStatus {
    fn from(value: WakeRegistrationStatus) -> Self {
        match value {
            WakeRegistrationStatus::Registered => Self::Registered,
            WakeRegistrationStatus::Triggered => Self::Triggered,
            WakeRegistrationStatus::TimedOut => Self::TimedOut,
            WakeRegistrationStatus::Cancelled => Self::Cancelled,
            WakeRegistrationStatus::Failed => Self::Failed,
        }
    }
}

trait AgentPoolEventKindName {
    fn wire_name(&self) -> &'static str;
}

impl AgentPoolEventKindName for EventKind {
    fn wire_name(&self) -> &'static str {
        match self {
            EventKind::AgentPoolCreated => "agent_pool_created",
            EventKind::AgentPoolRunJoined => "agent_pool_run_joined",
            EventKind::AgentPoolRunLeft => "agent_pool_run_left",
            EventKind::RunMessageAccepted => "run_message_accepted",
            EventKind::RunMessageDelivered => "run_message_delivered",
            EventKind::RunMessageResponded => "run_message_responded",
            EventKind::RunMessageFailed => "run_message_failed",
            EventKind::RunMessageTimedOut => "run_message_timed_out",
            EventKind::RunMessageExpired => "run_message_expired",
            EventKind::RunMessageCancelled => "run_message_cancelled",
            EventKind::WakeConditionRegistered => "wake_condition_registered",
            EventKind::WakeConditionTriggered => "wake_condition_triggered",
            EventKind::WakeConditionTimedOut => "wake_condition_timed_out",
            EventKind::WakeConditionCancelled => "wake_condition_cancelled",
            EventKind::WakeConditionFailed => "wake_condition_failed",
            _ => "agent_pool_event",
        }
    }
}

fn intersect_run_ids(filter: &EventFilterSet<RunId>, allowed: &[RunId]) -> EventFilterSet<RunId> {
    match filter {
        EventFilterSet::Any => EventFilterSet::Include(allowed.to_vec()),
        EventFilterSet::Include(requested) => EventFilterSet::Include(
            requested
                .iter()
                .filter(|run_id| allowed.contains(run_id))
                .cloned()
                .collect(),
        ),
    }
}

fn topics_from_members(members: &[AgentPoolMember]) -> BTreeMap<TopicId, BTreeSet<RunId>> {
    let mut topics = BTreeMap::new();
    for member in members {
        for topic in &member.topics {
            topics
                .entry(topic.clone())
                .or_insert_with(BTreeSet::new)
                .insert(member.run_id.clone());
        }
    }
    topics
}
