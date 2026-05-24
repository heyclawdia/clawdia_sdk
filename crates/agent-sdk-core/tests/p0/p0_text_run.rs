use serde_json::{Value, json};

use agent_sdk_core::{
    Agent, AgentEventBus, AgentId, AgentRuntime, InMemoryAgentEventBus, PolicyDecision, PolicyKind,
    PolicyOutcome, PolicyRef, PolicyStage, PrivacyClass, ProviderRouteSnapshot, RetentionClass,
    RunId, RunRequest, RunStatus, RuntimePackage, RuntimePackageId, RuntimePolicyPort, SourceKind,
    SourceRef,
    event::EventKind,
    testing::{FakeContentResolver, FakeJournalStore, FakeProvider},
};

#[derive(Clone, Debug)]
struct AllowP0Policy;

impl RuntimePolicyPort for AllowP0Policy {
    fn evaluate_run_start(
        &self,
        request: &RunRequest,
        _package: &RuntimePackage,
    ) -> Result<PolicyOutcome, agent_sdk_core::AgentError> {
        Ok(PolicyOutcome {
            stage: PolicyStage::Input,
            decision: PolicyDecision::allow("policy.p0.allow"),
            subject: None,
            source: Some(request.source.clone()),
            destination: None,
            policy_refs: vec![PolicyRef::with_kind(
                PolicyKind::RuntimePackage,
                "policy.p0",
            )],
            privacy: PrivacyClass::Internal,
            retention: RetentionClass::RunScoped,
        })
    }
}

#[test]
fn p0_fake_provider_text_run_emits_journal_events_and_result() {
    let agent = p0_agent();
    let provider = FakeProvider::with_responses(["hello from fake provider"]);
    let journal = FakeJournalStore::default();
    let event_bus = InMemoryAgentEventBus::default();
    let runtime = p0_runtime(&agent, provider.clone(), journal.clone(), event_bus.clone());

    let result = agent
        .run_text(
            &runtime,
            RunId::new("run.p0.text"),
            SourceRef::with_kind(SourceKind::Host, "source.p0.test"),
            "hello sdk",
        )
        .expect("P0 run succeeds");

    assert_eq!(result.status, RunStatus::Completed);
    assert_eq!(result.output, "hello from fake provider");

    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].projection_item_count, 1);
    assert_eq!(requests[0].messages[0].content, "hello sdk");

    let journal_summary = journal_summary(&journal.records());
    assert_eq!(
        journal_summary,
        fixture("tests/fixtures/p0/text-run-journal.json")
    );

    let frames = event_bus
        .subscribe_run(RunId::new("run.p0.text"), None)
        .expect("event stream")
        .collect::<Vec<_>>();
    assert_eq!(
        event_summary(&frames),
        fixture("tests/fixtures/p0/text-run-events.json")
    );
    assert_eq!(
        frames.last().unwrap().event.envelope.event_kind,
        EventKind::RunCompleted
    );
    assert!(
        frames
            .last()
            .unwrap()
            .event
            .envelope
            .journal_cursor
            .is_some()
    );
}

#[test]
fn agent_and_runtime_text_paths_use_the_same_run_request_shape() {
    let agent = p0_agent();
    let source = SourceRef::with_kind(SourceKind::Host, "source.p0.request-shape");
    let expected = RunRequest::text(
        RunId::new("run.p0.request-shape"),
        agent.id().clone(),
        source.clone(),
        "same input",
    );

    let provider = FakeProvider::with_responses(["same output"]);
    let runtime = p0_runtime(
        &agent,
        provider.clone(),
        FakeJournalStore::default(),
        InMemoryAgentEventBus::default(),
    );

    let result = runtime
        .run_text(expected.clone())
        .expect("explicit runtime text run succeeds");

    assert_eq!(result.output, "same output");
    assert_eq!(provider.requests()[0].messages[0].content, expected.input);
    assert_eq!(expected.agent_id, agent.id().clone());
    assert_eq!(expected.source, source);
}

#[test]
fn p0_event_bus_assigns_monotonic_sequences_across_runs() {
    let agent = p0_agent();
    let provider = FakeProvider::with_responses(["first output", "second output"]);
    let event_bus = InMemoryAgentEventBus::default();
    let runtime = p0_runtime(
        &agent,
        provider,
        FakeJournalStore::default(),
        event_bus.clone(),
    );

    runtime
        .run_text(RunRequest::text(
            RunId::new("run.p0.cursor.first"),
            agent.id().clone(),
            SourceRef::with_kind(SourceKind::Host, "source.p0.cursor"),
            "first",
        ))
        .expect("first run");
    runtime
        .run_text(RunRequest::text(
            RunId::new("run.p0.cursor.second"),
            agent.id().clone(),
            SourceRef::with_kind(SourceKind::Host, "source.p0.cursor"),
            "second",
        ))
        .expect("second run");

    let all_frames = event_bus
        .subscribe_all(None)
        .expect("all stream")
        .collect::<Vec<_>>();
    assert_eq!(
        all_frames
            .iter()
            .map(|frame| frame.event.envelope.event_seq)
            .collect::<Vec<_>>(),
        (1..=12).collect::<Vec<_>>()
    );

    let first_terminal_cursor = all_frames[5].cursor.clone();
    let resumed_all = event_bus
        .subscribe_all(Some(first_terminal_cursor.clone()))
        .expect("resume all")
        .collect::<Vec<_>>();
    assert_eq!(resumed_all.len(), 6);
    assert!(
        resumed_all
            .iter()
            .all(|frame| { frame.event.envelope.run_id == RunId::new("run.p0.cursor.second") })
    );

    let agent_frames = event_bus
        .subscribe_agent(agent.id().clone(), None)
        .expect("agent stream")
        .collect::<Vec<_>>();
    let first_agent_terminal_cursor = agent_frames[5].cursor.clone();
    let resumed_agent = event_bus
        .subscribe_agent(agent.id().clone(), Some(first_agent_terminal_cursor))
        .expect("resume agent")
        .collect::<Vec<_>>();
    assert_eq!(resumed_agent.len(), 6);
    assert_eq!(
        resumed_agent[0].event.envelope.event_seq,
        resumed_all[0].event.envelope.event_seq
    );
}

fn p0_runtime(
    agent: &Agent,
    provider: FakeProvider,
    journal: FakeJournalStore,
    event_bus: InMemoryAgentEventBus,
) -> AgentRuntime {
    AgentRuntime::builder()
        .default_package(p0_package(agent))
        .provider("provider.fake", provider)
        .expect("provider route registers")
        .journal(journal)
        .event_bus(event_bus)
        .content(FakeContentResolver::default())
        .policy(AllowP0Policy)
        .build()
        .expect("runtime builds")
}

fn p0_agent() -> Agent {
    Agent::builder()
        .id(AgentId::new("agent.p0.contract"))
        .name("p0 contract")
        .build()
        .expect("agent builds")
}

fn p0_package(agent: &Agent) -> RuntimePackage {
    RuntimePackage::builder(RuntimePackageId::new("package.p0.contract"))
        .agent(agent.snapshot())
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake.p0"))
        .policy(PolicyRef::with_kind(
            PolicyKind::RuntimePackage,
            "policy.p0.package",
        ))
        .build()
        .expect("package builds")
}

fn event_summary(frames: &[agent_sdk_core::EventFrame]) -> Value {
    json!({
        "schema_version": 1,
        "events": frames.iter().map(|frame| {
            json!({
                "event_seq": frame.event.envelope.event_seq,
                "event_family": format!("{:?}", frame.event.envelope.event_family),
                "event_kind": format!("{:?}", frame.event.envelope.event_kind),
                "delivery_semantics": format!("{:?}", frame.event.envelope.delivery_semantics),
                "journal_cursor": frame.event.envelope.journal_cursor.as_ref().map(|cursor| cursor.as_str().to_string()),
            })
        }).collect::<Vec<_>>()
    })
}

fn journal_summary(records: &[agent_sdk_core::JournalRecord]) -> Value {
    json!({
        "schema_version": 1,
        "records": records.iter().map(|record| {
            json!({
                "journal_seq": record.journal_seq,
                "record_kind": format!("{:?}", record.record_kind),
                "payload_type": payload_type(&record.payload),
                "delivery_semantics": record.delivery_semantics,
            })
        }).collect::<Vec<_>>()
    })
}

fn payload_type(payload: &agent_sdk_core::JournalRecordPayload) -> &'static str {
    match payload {
        agent_sdk_core::JournalRecordPayload::RunLifecycle(_) => "run_lifecycle",
        agent_sdk_core::JournalRecordPayload::ContextProjection(_) => "context_projection",
        agent_sdk_core::JournalRecordPayload::ModelAttempt(_) => "model_attempt",
        agent_sdk_core::JournalRecordPayload::Message(_) => "message",
        agent_sdk_core::JournalRecordPayload::StructuredOutput(_) => "structured_output",
        agent_sdk_core::JournalRecordPayload::Approval(_) => "approval",
        agent_sdk_core::JournalRecordPayload::Tool(_) => "tool",
        agent_sdk_core::JournalRecordPayload::OutputDelivery(_) => "output_delivery",
        agent_sdk_core::JournalRecordPayload::Hook(_) => "hook",
        agent_sdk_core::JournalRecordPayload::StreamRule(_) => "stream_rule",
        agent_sdk_core::JournalRecordPayload::RealtimeSession(_) => "realtime_session",
        agent_sdk_core::JournalRecordPayload::Isolation(_) => "isolation",
        agent_sdk_core::JournalRecordPayload::ChildLifecycle(_) => "child_lifecycle",
        agent_sdk_core::JournalRecordPayload::AgentPool(_) => "agent_pool",
        agent_sdk_core::JournalRecordPayload::RunMessage(_) => "run_message",
        agent_sdk_core::JournalRecordPayload::Wake(_) => "wake",
        agent_sdk_core::JournalRecordPayload::Subagent(_) => "subagent",
        agent_sdk_core::JournalRecordPayload::ExtensionAction(_) => "extension_action",
        agent_sdk_core::JournalRecordPayload::EffectIntent(_) => "effect_intent",
        agent_sdk_core::JournalRecordPayload::EffectResult(_) => "effect_result",
        agent_sdk_core::JournalRecordPayload::Checkpoint(_) => "checkpoint",
        agent_sdk_core::JournalRecordPayload::Recovery(_) => "recovery",
        agent_sdk_core::JournalRecordPayload::TerminalResult(_) => "terminal_result",
    }
}

fn fixture(path: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture readable"))
        .expect("fixture JSON")
}
