use serde_json::{Value, json};

use agent_sdk_core::{
    Agent, AgentError, AgentEventBus, AgentId, AgentPool, AgentPoolId, AgentPoolMember,
    AgentPoolMessagePolicy, DestinationKind, DestinationRef, EntityRef, EventEnvelope, EventFrame,
    EventId, IdempotencyKey, InMemoryAgentEventBus, MessageId, MessageStatus, PolicyDecision,
    PolicyKind, PolicyOutcome, PolicyRef, PolicyStage, PrivacyClass, ProviderRouteSnapshot,
    RetentionClass, RunAddress, RunAddressTarget, RunId, RunMessage, RuntimePackage,
    RuntimePackageId, RuntimePolicyPort, SourceKind, SourceRef, TopicId, TraceId, WakeCondition,
    WakeConditionId, WakeRegistrationStatus,
    domain::ContentRef as ContentRefId,
    event::{
        ContentCaptureMode, EVENT_SCHEMA_VERSION, EventDeliverySemantics, EventFamily, EventFilter,
        EventFilterSet, EventKind, EventStreamScope, PayloadAccessMode,
    },
    ids::SpanId,
    testing::{
        FakeContentResolver, FakeJournalStore, FakeProvider, normalize_json_value, read_fixture,
    },
};

#[derive(Clone, Debug)]
struct AllowPolicy;

impl RuntimePolicyPort for AllowPolicy {
    fn evaluate_run_start(
        &self,
        request: &agent_sdk_core::RunRequest,
        _package: &RuntimePackage,
    ) -> Result<PolicyOutcome, AgentError> {
        Ok(PolicyOutcome {
            stage: PolicyStage::Input,
            decision: PolicyDecision::allow("policy.agent_pool.runtime_allow"),
            subject: None,
            source: Some(request.source.clone()),
            destination: None,
            policy_refs: vec![PolicyRef::with_kind(
                PolicyKind::RuntimePackage,
                "policy.agent_pool.runtime",
            )],
            privacy: PrivacyClass::Internal,
            retention: RetentionClass::RunScoped,
        })
    }
}

#[test]
fn agent_pool_ids_use_shared_typed_id_serde_pattern() {
    let value = json!({
        "agent_pool_id": AgentPoolId::new("pool.contract"),
        "topic_id": TopicId::new("topic.contract"),
        "wake_condition_id": WakeConditionId::new("wake.contract"),
    });

    assert_eq!(
        value,
        read_fixture("tests/fixtures/agent_pool/typed-ids.json").expect("id fixture")
    );
    assert_eq!(
        format!("{:?}", AgentPoolId::new("pool.secret")),
        "AgentPoolId(redacted)"
    );
    assert!(AgentPoolId::try_new("").is_err());
    assert!(TopicId::try_new("topic.bad\nid").is_err());
    assert!(WakeConditionId::try_new("wake.valid").is_ok());
}

#[test]
fn run_address_is_wrapper_over_existing_refs_not_parallel_identity() {
    let run_id = RunId::new("run.address.1");
    let run_address = RunAddress::run(run_id.clone());
    assert_eq!(run_address.target.run_id(), Some(&run_id));
    assert_eq!(run_address.destination_ref.kind, DestinationKind::Agent);
    assert_eq!(run_address.related_refs, vec![EntityRef::run(run_id)]);

    let agent_id = AgentId::new("agent.address.1");
    let agent_address = RunAddress::agent(agent_id.clone());
    assert!(matches!(
        agent_address.target,
        RunAddressTarget::Agent { agent_id: ref target } if target == &agent_id
    ));
    assert_eq!(agent_address.related_refs, vec![EntityRef::agent(agent_id)]);

    let pool_address = RunAddress::pool(AgentPoolId::new("pool.address.1"));
    assert!(matches!(pool_address.target, RunAddressTarget::Pool { .. }));
    assert_eq!(
        pool_address.destination_ref.kind,
        DestinationKind::AgentPool
    );
    assert_eq!(
        pool_address.related_refs,
        vec![EntityRef::agent_pool(AgentPoolId::new("pool.address.1"))]
    );

    let topic_address = RunAddress::topic(TopicId::new("topic.address.1"));
    assert!(matches!(
        topic_address.target,
        RunAddressTarget::Topic { .. }
    ));
    assert_eq!(topic_address.destination_ref.kind, DestinationKind::Topic);
    assert_eq!(
        topic_address.related_refs,
        vec![EntityRef::topic(TopicId::new("topic.address.1"))]
    );
}

#[test]
fn pool_lifecycle_created_and_joined_records_have_golden_fixtures() {
    let journal = FakeJournalStore::default();
    let event_bus = InMemoryAgentEventBus::default();
    let pool = pool_with_ports(journal.clone(), event_bus.clone(), None);

    join_two_members(&pool);

    assert_eq!(
        agent_pool_record_summary(&journal.records()),
        read_fixture("tests/fixtures/agent_pool/pool-lifecycle-journal.json")
            .expect("pool lifecycle journal fixture")
    );
    assert_eq!(
        agent_pool_event_summary(
            event_bus
                .subscribe_all(None)
                .unwrap()
                .collect::<Vec<_>>()
                .as_slice(),
            &[EventKind::AgentPoolCreated, EventKind::AgentPoolRunJoined],
        ),
        read_fixture("tests/fixtures/agent_pool/pool-lifecycle-events.json")
            .expect("pool lifecycle event fixture")
    );
}

#[test]
fn run_message_accept_and_deliver_are_journaled_with_content_refs() {
    let journal = FakeJournalStore::default();
    let event_bus = InMemoryAgentEventBus::default();
    let pool = pool_with_ports(journal.clone(), event_bus.clone(), None);
    join_two_members(&pool);

    let message = run_message(
        "message.agent_pool.1",
        "run.pool.parent",
        RunAddress::run(RunId::new("run.pool.child")),
    );
    let receipt = pool.send(message).expect("message sent");

    assert_eq!(receipt.status, MessageStatus::Delivered);
    assert_eq!(receipt.delivered_to, vec![RunId::new("run.pool.child")]);

    let message_records = run_message_record_summary(&journal.records());
    assert_eq!(
        message_records,
        read_fixture("tests/fixtures/agent_pool/run-message-accepted-delivered-journal.json")
            .expect("run message journal fixture")
    );

    let message_events = agent_pool_event_summary(
        event_bus
            .subscribe_all(None)
            .unwrap()
            .collect::<Vec<_>>()
            .as_slice(),
        &[
            EventKind::RunMessageAccepted,
            EventKind::RunMessageDelivered,
        ],
    );
    assert_eq!(
        message_events,
        read_fixture("tests/fixtures/agent_pool/run-message-accepted-delivered-events.json")
            .expect("run message event fixture")
    );
    assert!(
        !message_events.to_string().contains("secret body"),
        "events carry content refs and redacted summaries, not raw bodies"
    );
}

#[test]
fn duplicate_run_message_is_deduped_by_idempotency_key() {
    let journal = FakeJournalStore::default();
    let event_bus = InMemoryAgentEventBus::default();
    let pool = pool_with_ports(journal.clone(), event_bus, None);
    join_two_members(&pool);

    let message = run_message(
        "message.agent_pool.dedupe",
        "run.pool.parent",
        RunAddress::run(RunId::new("run.pool.child")),
    );
    let first = pool.send(message.clone()).expect("first send");
    let second = pool.send(message).expect("deduped send");

    assert_eq!(first, second);
    assert_eq!(
        journal
            .records()
            .iter()
            .filter(|record| record.record_kind == agent_sdk_core::JournalRecordKind::RunMessage)
            .count(),
        2,
        "dedupe returns the first receipt without duplicate delivery records"
    );
}

#[test]
fn duplicate_wake_registration_is_deduped_by_idempotency_key() {
    let journal = FakeJournalStore::default();
    let event_bus = InMemoryAgentEventBus::default();
    let pool = pool_with_ports(journal.clone(), event_bus, None);
    join_two_members(&pool);

    let condition = wake_condition(
        "wake.agent_pool.dedupe",
        "run.pool.parent",
        EventFilter {
            families: EventFilterSet::Include(vec![EventFamily::Tool]),
            ..EventFilter::default()
        },
    );
    let first = pool
        .suspend_until(RunId::new("run.pool.parent"), condition.clone())
        .expect("first registration");
    let second = pool
        .suspend_until(RunId::new("run.pool.parent"), condition)
        .expect("deduped registration");

    assert_eq!(first, second);
    assert_eq!(
        journal
            .records()
            .iter()
            .filter(|record| record.record_kind == agent_sdk_core::JournalRecordKind::Wake)
            .count(),
        1,
        "dedupe returns the first wake registration without duplicate wake records"
    );
}

#[test]
fn agent_address_selects_existing_policy_members_without_starting_runs() {
    let policy_ref = PolicyRef::with_kind(PolicyKind::Permission, "policy.agent_pool.message");
    let journal = FakeJournalStore::default();
    let event_bus = InMemoryAgentEventBus::default();
    let pool = pool_with_ports(journal, event_bus, None);
    pool.join_run(
        AgentPoolMember::new(RunId::new("run.pool.parent"), AgentId::new("agent.parent"))
            .policy_ref(policy_ref.clone()),
    )
    .expect("parent join");
    pool.join_run(
        AgentPoolMember::new(RunId::new("run.pool.allowed"), AgentId::new("agent.worker"))
            .policy_ref(policy_ref.clone()),
    )
    .expect("allowed join");
    pool.join_run(AgentPoolMember::new(
        RunId::new("run.pool.denied"),
        AgentId::new("agent.worker"),
    ))
    .expect("denied join");

    let receipt = pool
        .send(
            run_message(
                "message.agent_pool.agent_target",
                "run.pool.parent",
                RunAddress::agent(AgentId::new("agent.worker")),
            )
            .policy_ref(policy_ref),
        )
        .expect("message sent");

    assert_eq!(receipt.delivered_to, vec![RunId::new("run.pool.allowed")]);
    assert_eq!(
        pool_with_ports(
            FakeJournalStore::default(),
            InMemoryAgentEventBus::default(),
            None
        )
        .members()
        .unwrap()
        .len(),
        0
    );
}

#[test]
fn pool_address_broadcasts_only_current_pool_policy_members() {
    let policy_ref = PolicyRef::with_kind(PolicyKind::Permission, "policy.agent_pool.broadcast");
    let journal = FakeJournalStore::default();
    let event_bus = InMemoryAgentEventBus::default();
    let pool = pool_with_ports(
        journal,
        event_bus,
        Some(AgentPoolMessagePolicy {
            required_policy_refs: vec![policy_ref.clone()],
            include_sender_in_pool_broadcast: false,
        }),
    );
    pool.join_run(
        AgentPoolMember::new(RunId::new("run.pool.sender"), AgentId::new("agent.sender"))
            .policy_ref(policy_ref.clone()),
    )
    .expect("sender join");
    pool.join_run(
        AgentPoolMember::new(RunId::new("run.pool.peer"), AgentId::new("agent.peer"))
            .policy_ref(policy_ref.clone()),
    )
    .expect("peer join");
    pool.join_run(AgentPoolMember::new(
        RunId::new("run.pool.no-policy"),
        AgentId::new("agent.no_policy"),
    ))
    .expect("no-policy join");

    let receipt = pool
        .send(
            run_message(
                "message.agent_pool.broadcast",
                "run.pool.sender",
                RunAddress::pool(pool.pool_id().clone()),
            )
            .policy_ref(policy_ref),
        )
        .expect("broadcast sent");

    assert_eq!(receipt.delivered_to, vec![RunId::new("run.pool.peer")]);
}

#[test]
fn pool_subscription_intersects_filter_with_membership_and_policy() {
    let policy_ref = PolicyRef::with_kind(PolicyKind::Permission, "policy.agent_pool.observe");
    let journal = FakeJournalStore::default();
    let event_bus = InMemoryAgentEventBus::default();
    let pool = AgentPool::builder(AgentPoolId::new("pool.observe"))
        .runtime(
            agent_pool_runtime(journal, event_bus.clone())
                .build()
                .expect("runtime builds"),
        )
        .policy_ref(policy_ref.clone())
        .build()
        .expect("pool builds");
    pool.join_run(
        AgentPoolMember::new(
            RunId::new("run.pool.visible"),
            AgentId::new("agent.visible"),
        )
        .policy_ref(policy_ref),
    )
    .expect("visible join");
    pool.join_run(AgentPoolMember::new(
        RunId::new("run.pool.hidden-by-policy"),
        AgentId::new("agent.hidden"),
    ))
    .expect("hidden join");

    event_bus
        .publish(runtime_event_frame(
            "run.pool.visible",
            "agent.visible",
            1,
            EventKind::RunCompleted,
        ))
        .expect("visible event");
    event_bus
        .publish(runtime_event_frame(
            "run.pool.outside",
            "agent.outside",
            2,
            EventKind::RunCompleted,
        ))
        .expect("outside event");
    event_bus
        .publish(runtime_event_frame(
            "run.pool.hidden-by-policy",
            "agent.hidden",
            3,
            EventKind::RunCompleted,
        ))
        .expect("policy hidden event");

    let run_event_filter = EventFilter {
        families: EventFilterSet::Include(vec![EventFamily::Run]),
        ..EventFilter::default()
    };
    let scoped = pool.scope_filter(run_event_filter.clone());
    assert!(matches!(scoped.run_ids, EventFilterSet::Include(_)));
    let frames = pool
        .subscribe(run_event_filter, None)
        .expect("pool subscription")
        .collect::<Vec<_>>();

    assert_eq!(frames.len(), 1);
    assert_eq!(
        frames[0].event.envelope.run_id,
        RunId::new("run.pool.visible")
    );
}

#[test]
fn wake_conditions_use_envelope_filters_and_emit_trigger_timeout_cancel_fixtures() {
    let journal = FakeJournalStore::default();
    let event_bus = InMemoryAgentEventBus::default();
    let pool = pool_with_ports(journal.clone(), event_bus.clone(), None);
    join_two_members(&pool);
    event_bus
        .publish(runtime_event_frame(
            "run.pool.child",
            "agent.child",
            1,
            EventKind::RunCompleted,
        ))
        .expect("target event");

    let trigger = wake_condition("wake.agent_pool.trigger", "run.pool.parent", wake_filter());
    let compiled = trigger.compile_envelope_filter().expect("compiled wake");
    assert_eq!(compiled.payload_access, PayloadAccessMode::EnvelopeOnly);
    assert!(
        compiled
            .indexed_fields
            .contains(&agent_sdk_core::event::EventIndexField::RunId)
    );

    let triggered = pool
        .suspend_until(RunId::new("run.pool.parent"), trigger)
        .expect("wake triggered");
    assert_eq!(triggered.status, WakeRegistrationStatus::Triggered);

    let timed_out_condition =
        wake_condition("wake.agent_pool.timeout", "run.pool.parent", wake_filter())
            .timeout_millis(0);
    let timed_out = pool
        .suspend_until(RunId::new("run.pool.parent"), timed_out_condition)
        .expect("wake timeout");
    assert_eq!(timed_out.status, WakeRegistrationStatus::TimedOut);

    let cancel_condition = wake_condition(
        "wake.agent_pool.cancel",
        "run.pool.parent",
        EventFilter {
            families: EventFilterSet::Include(vec![EventFamily::Tool]),
            ..EventFilter::default()
        },
    );
    let registered = pool
        .suspend_until(RunId::new("run.pool.parent"), cancel_condition)
        .expect("wake registered");
    let cancelled = pool
        .cancel_wake(&registered.condition_id)
        .expect("wake cancelled");
    assert_eq!(cancelled.status, WakeRegistrationStatus::Cancelled);

    assert_eq!(
        wake_record_summary(&journal.records()),
        read_fixture("tests/fixtures/agent_pool/wake-records.json").expect("wake record fixture")
    );
    assert_eq!(
        agent_pool_event_summary(
            event_bus
                .subscribe_all(None)
                .unwrap()
                .collect::<Vec<_>>()
                .as_slice(),
            &[
                EventKind::WakeConditionRegistered,
                EventKind::WakeConditionTriggered,
                EventKind::WakeConditionTimedOut,
                EventKind::WakeConditionCancelled,
            ],
        ),
        read_fixture("tests/fixtures/agent_pool/wake-events.json").expect("wake event fixture")
    );
}

#[test]
fn wake_timeout_does_not_cancel_target_run() {
    let agent = Agent::builder()
        .id(AgentId::new("agent.timeout"))
        .name("timeout")
        .build()
        .expect("agent");
    let runtime = agent_pool_runtime(
        FakeJournalStore::default(),
        InMemoryAgentEventBus::default(),
    )
    .default_package(runtime_package(&agent))
    .provider("provider.fake", FakeProvider::default())
    .expect("provider route registers")
    .content(FakeContentResolver::default())
    .policy(AllowPolicy)
    .build()
    .expect("runtime builds");
    let request = agent_sdk_core::RunRequest::text(
        RunId::new("run.timeout.target"),
        agent.id().clone(),
        SourceRef::with_kind(SourceKind::Host, "source.timeout"),
        "wait but do not cancel",
    );
    runtime.start_run(request.clone()).expect("run starts");
    let pool = AgentPool::builder(AgentPoolId::new("pool.timeout"))
        .runtime(runtime.clone())
        .build()
        .expect("pool builds");
    pool.join_run(AgentPoolMember::new(
        request.run_id.clone(),
        request.agent_id.clone(),
    ))
    .expect("join target");

    let registration = pool
        .suspend_until(
            request.run_id.clone(),
            wake_condition(
                "wake.timeout.no-cancel",
                "run.timeout.target",
                EventFilter::default(),
            )
            .timeout_millis(0),
        )
        .expect("timeout wake");

    assert_eq!(registration.status, WakeRegistrationStatus::TimedOut);
    assert!(
        !runtime
            .run_snapshot(&request.run_id)
            .expect("snapshot")
            .cancellation_requested,
        "pool timeout must not call AgentRuntime::cancel_run"
    );
}

#[test]
fn agent_pool_source_contains_no_workflow_engine_public_types() {
    let source = include_str!("../../src/application/agent_pool.rs");
    for forbidden in [
        "pub struct Workflow",
        "pub enum Workflow",
        "pub struct Dag",
        "pub enum Dag",
        "pub struct Barrier",
        "pub enum Barrier",
        "pub struct Schedule",
        "pub enum Schedule",
        "pub struct Compensation",
        "pub enum Compensation",
    ] {
        assert!(
            !source.contains(forbidden),
            "{forbidden} must stay out of core"
        );
    }
}

fn pool_with_ports(
    journal: FakeJournalStore,
    event_bus: InMemoryAgentEventBus,
    message_policy: Option<AgentPoolMessagePolicy>,
) -> AgentPool {
    let mut builder = AgentPool::builder(AgentPoolId::new("pool.contract")).runtime(
        agent_pool_runtime(journal, event_bus)
            .build()
            .expect("runtime builds"),
    );
    if let Some(policy) = message_policy {
        builder = builder.message_policy(policy);
    }
    builder.build().expect("pool builds")
}

fn agent_pool_runtime(
    journal: FakeJournalStore,
    event_bus: InMemoryAgentEventBus,
) -> agent_sdk_core::AgentRuntimeBuilder {
    agent_sdk_core::AgentRuntime::builder()
        .journal(journal)
        .event_bus(event_bus)
}

fn join_two_members(pool: &AgentPool) {
    pool.join_run(AgentPoolMember::new(
        RunId::new("run.pool.parent"),
        AgentId::new("agent.parent"),
    ))
    .expect("parent join");
    pool.join_run(AgentPoolMember::new(
        RunId::new("run.pool.child"),
        AgentId::new("agent.child"),
    ))
    .expect("child join");
}

fn run_message(message_id: &str, from: &str, to: RunAddress) -> RunMessage {
    RunMessage::new(
        MessageId::new(message_id),
        RunId::new(from),
        to,
        ContentRefId::new(format!("content.{message_id}")),
        IdempotencyKey::new(format!("idem.{message_id}")),
    )
}

fn wake_condition(condition_id: &str, run_id: &str, filter: EventFilter) -> WakeCondition {
    WakeCondition::new(
        WakeConditionId::new(condition_id),
        RunId::new(run_id),
        filter,
        IdempotencyKey::new(format!("idem.{condition_id}")),
    )
}

fn wake_filter() -> EventFilter {
    EventFilter {
        run_ids: EventFilterSet::Include(vec![RunId::new("run.pool.child")]),
        families: EventFilterSet::Include(vec![EventFamily::Run]),
        kinds: EventFilterSet::Include(vec![EventKind::RunCompleted]),
        ..EventFilter::default()
    }
}

fn runtime_event_frame(
    run_id: &str,
    agent_id: &str,
    event_seq: u64,
    event_kind: EventKind,
) -> EventFrame {
    let event = agent_sdk_core::AgentEvent::with_redacted_summary(
        EventEnvelope {
            schema_version: EVENT_SCHEMA_VERSION,
            event_id: EventId::new(format!("event.runtime.{event_seq}")),
            event_seq,
            event_family: EventFamily::Run,
            event_kind,
            payload_schema_version: 1,
            timestamp: "1970-01-01T00:00:00Z".to_string(),
            recorded_at: "1970-01-01T00:00:00Z".to_string(),
            run_id: RunId::new(run_id),
            agent_id: AgentId::new(agent_id),
            turn_id: None,
            attempt_id: None,
            message_id: None,
            context_item_id: None,
            trace_id: TraceId::new(format!("trace.runtime.{event_seq}")),
            span_id: SpanId::new(format!("span.runtime.{event_seq}")),
            parent_event_id: None,
            caused_by: None,
            subject_ref: EntityRef::run(RunId::new(run_id)),
            related_refs: Vec::new(),
            causal_refs: Vec::new(),
            correlation: Default::default(),
            tags: Vec::new(),
            source: SourceRef::with_kind(SourceKind::Sdk, "source.runtime.test"),
            destination: Some(DestinationRef::with_kind(
                DestinationKind::EventStream,
                "destination.event_stream.test",
            )),
            policy_refs: Vec::new(),
            journal_cursor: None,
            state_before: None,
            state_after: None,
            delivery_semantics: EventDeliverySemantics::JournalBacked,
            privacy: PrivacyClass::ContentRefsOnly,
            content_capture: ContentCaptureMode::Off,
            redaction_policy_id: "redaction.test".to_string(),
            runtime_package_fingerprint: "runtime.package.fingerprint.test".to_string(),
        },
        "runtime event",
    );
    EventFrame {
        cursor: event.envelope.cursor(EventStreamScope::All),
        event,
        archive_cursor: None,
        overflow: None,
    }
}

fn runtime_package(agent: &Agent) -> RuntimePackage {
    RuntimePackage::builder(RuntimePackageId::new("package.agent_pool.timeout"))
        .agent(agent.snapshot())
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake"))
        .policy(PolicyRef::with_kind(
            PolicyKind::RuntimePackage,
            "policy.agent_pool.package",
        ))
        .build()
        .expect("package builds")
}

fn run_message_record_summary(records: &[agent_sdk_core::JournalRecord]) -> Value {
    normalize_json_value(json!({
        "schema_version": 1,
        "records": records.iter()
            .filter_map(|record| match &record.payload {
                agent_sdk_core::JournalRecordPayload::RunMessage(payload) => Some(json!({
                    "record_kind": record.record_kind,
                    "event_kind": record.event_index.event_kind,
                    "message_id": payload.message_id,
                    "source_run_id": payload.source_run_id,
                    "address_target": payload.address_target,
                    "content_ref": payload.content_ref,
                    "delivery_status": payload.delivery_status,
                    "delivered_to": payload.delivered_to,
                    "idempotency_key": payload.idempotency_key,
                    "effect_intent_kind": payload.effect_intent.as_ref().map(|intent| &intent.kind),
                    "effect_result_status": payload.effect_result.as_ref().map(|result| &result.terminal_status),
                })),
                _ => None,
            })
            .collect::<Vec<_>>()
    }))
}

fn agent_pool_record_summary(records: &[agent_sdk_core::JournalRecord]) -> Value {
    normalize_json_value(json!({
        "schema_version": 1,
        "records": records.iter()
            .filter_map(|record| match &record.payload {
                agent_sdk_core::JournalRecordPayload::AgentPool(payload) => Some(json!({
                    "record_kind": record.record_kind,
                    "event_kind": record.event_index.event_kind,
                    "pool_id": payload.pool_id,
                    "member_run_ids": payload.member_run_ids,
                    "topics": payload.topics,
                    "lifecycle_status": payload.lifecycle_status,
                })),
                _ => None,
            })
            .collect::<Vec<_>>()
    }))
}

fn wake_record_summary(records: &[agent_sdk_core::JournalRecord]) -> Value {
    normalize_json_value(json!({
        "schema_version": 1,
        "records": records.iter()
            .filter_map(|record| match &record.payload {
                agent_sdk_core::JournalRecordPayload::Wake(payload) => Some(json!({
                    "record_kind": record.record_kind,
                    "event_kind": record.event_index.event_kind,
                    "condition_id": payload.condition_id,
                    "run_id": payload.run_id,
                    "timeout_millis": payload.timeout_millis,
                    "resume_policy": payload.resume_policy,
                    "trigger_status": payload.trigger_status,
                    "idempotency_key": payload.idempotency_key,
                    "matched_event_id": payload.matched_event_id,
                })),
                _ => None,
            })
            .collect::<Vec<_>>()
    }))
}

fn agent_pool_event_summary(frames: &[EventFrame], kinds: &[EventKind]) -> Value {
    normalize_json_value(json!({
        "schema_version": 1,
        "events": frames.iter()
            .filter(|frame| kinds.contains(&frame.event.envelope.event_kind))
            .map(|frame| json!({
                "family": frame.event.envelope.event_family,
                "kind": frame.event.envelope.event_kind,
                "run_id": frame.event.envelope.run_id,
                "agent_id": frame.event.envelope.agent_id,
                "message_id": frame.event.envelope.message_id,
                "journal_cursor": frame.event.envelope.journal_cursor,
                "privacy": frame.event.envelope.privacy,
                "content_capture": frame.event.envelope.content_capture,
                "summary": frame.event.redacted_summary(),
            }))
            .collect::<Vec<_>>()
    }))
}
