use agent_sdk_core::{
    Agent, AgentError, AgentErrorKind, AgentEvent, AgentId, AgentRuntime, DestinationKind,
    DestinationRef, EntityRef, EventEnvelope, EventFrame, EventId, InMemoryAgentEventBus,
    JournalCursor, PolicyDecision, PolicyKind, PolicyOutcome, PolicyRef, PolicyStage, PrivacyClass,
    ProviderRouteSnapshot, RetentionClass, RetryClassification, RunId, RunRegistryStatus,
    RunRequest, RuntimePackage, RuntimePackageId, RuntimePolicyPort, SourceKind, SourceRef,
    TraceId,
    event::{
        ContentCaptureMode, EVENT_SCHEMA_VERSION, EventDeliverySemantics, EventFamily, EventKind,
        EventStreamScope,
    },
    ids::SpanId,
    testing::{FakeContentResolver, FakeJournalStore, FakeProvider},
};

#[derive(Clone, Debug)]
struct AllowRunStartPolicy;

impl RuntimePolicyPort for AllowRunStartPolicy {
    fn evaluate_run_start(
        &self,
        request: &RunRequest,
        _package: &RuntimePackage,
    ) -> Result<PolicyOutcome, AgentError> {
        Ok(policy_outcome(
            request,
            PolicyDecision::allow("policy.runtime.allow"),
        ))
    }
}

#[derive(Clone, Debug)]
struct DenyRunStartPolicy;

impl RuntimePolicyPort for DenyRunStartPolicy {
    fn evaluate_run_start(
        &self,
        request: &RunRequest,
        _package: &RuntimePackage,
    ) -> Result<PolicyOutcome, AgentError> {
        Ok(policy_outcome(
            request,
            PolicyDecision::deny("policy.runtime.deny"),
        ))
    }
}

#[test]
fn package_resolution_captures_deterministic_fingerprint_before_run_registry_entry() {
    let agent = runtime_agent();
    let package = runtime_package(&agent);
    let provider = FakeProvider::default();
    let journal = FakeJournalStore::default();
    let runtime = full_runtime(package.clone(), provider.clone(), journal.clone());
    let request = run_request(&agent, "run.runtime.fingerprint");

    let first = runtime
        .resolve_effective_package(&request)
        .expect("first package resolution");
    let second = runtime
        .resolve_effective_package(&request)
        .expect("second package resolution");
    assert_eq!(first.fingerprint, second.fingerprint);
    assert!(
        first
            .fingerprint
            .as_str()
            .starts_with("sha256:runtime-package-canonical-v1:")
    );
    assert_eq!(runtime.registered_run_count().unwrap(), 0);

    let handle = runtime.start_run(request.clone()).expect("run registered");
    assert_eq!(handle.run_id(), &request.run_id);

    let snapshot = runtime.run_snapshot(&request.run_id).expect("snapshot");
    assert_eq!(snapshot.status, RunRegistryStatus::Registered);
    assert_eq!(snapshot.runtime_package_id, package.package_id);
    assert_eq!(snapshot.runtime_package_fingerprint, first.fingerprint);
    assert!(!snapshot.cancellation_requested);
    assert!(
        provider.requests().is_empty(),
        "runtime shell must not call provider"
    );
    assert!(
        journal.records().is_empty(),
        "runtime shell must not append journal records yet"
    );
}

#[test]
fn missing_required_ports_return_typed_error_and_do_not_register_run() {
    let agent = runtime_agent();
    let runtime = AgentRuntime::builder()
        .default_package(runtime_package(&agent))
        .build()
        .expect("runtime builds with absent ports so start_run can fail closed");
    let request = run_request(&agent, "run.runtime.missing-ports");

    let error = runtime
        .start_run(request)
        .expect_err("missing required ports fail closed");

    assert_eq!(error.kind(), AgentErrorKind::HostConfigurationNeeded);
    assert_eq!(error.retry(), RetryClassification::HostConfigurationNeeded);
    assert_eq!(runtime.registered_run_count().unwrap(), 0);
}

#[test]
fn missing_provider_adapter_fails_closed_after_package_resolution() {
    let agent = runtime_agent();
    let runtime = AgentRuntime::builder()
        .default_package(runtime_package(&agent))
        .journal(FakeJournalStore::default())
        .event_bus(InMemoryAgentEventBus::default())
        .content(FakeContentResolver::default())
        .policy(AllowRunStartPolicy)
        .build()
        .expect("runtime builds");
    let request = run_request(&agent, "run.runtime.missing-provider");

    let error = runtime
        .start_run(request)
        .expect_err("provider route without adapter fails closed");

    assert_eq!(error.kind(), AgentErrorKind::ProviderFailure);
    assert_eq!(error.retry(), RetryClassification::HostConfigurationNeeded);
    assert_eq!(runtime.registered_run_count().unwrap(), 0);
}

#[test]
fn policy_denial_fails_closed_without_provider_execution() {
    let agent = runtime_agent();
    let provider = FakeProvider::default();
    let runtime = AgentRuntime::builder()
        .default_package(runtime_package(&agent))
        .provider("provider.fake", provider.clone())
        .expect("provider route registers")
        .journal(FakeJournalStore::default())
        .event_bus(InMemoryAgentEventBus::default())
        .content(FakeContentResolver::default())
        .policy(DenyRunStartPolicy)
        .build()
        .expect("runtime builds");
    let request = run_request(&agent, "run.runtime.policy-denied");

    let error = runtime
        .start_run(request)
        .expect_err("policy denial fails closed");

    assert_eq!(error.kind(), AgentErrorKind::PolicyDenial);
    assert_eq!(runtime.registered_run_count().unwrap(), 0);
    assert!(provider.requests().is_empty());
}

#[test]
fn cancellation_token_is_created_and_run_registry_updates_idempotently() {
    let agent = runtime_agent();
    let runtime = full_runtime(
        runtime_package(&agent),
        FakeProvider::default(),
        FakeJournalStore::default(),
    );
    let request = run_request(&agent, "run.runtime.cancel");
    let handle = runtime.start_run(request.clone()).expect("run registered");

    assert!(
        !runtime
            .run_snapshot(&request.run_id)
            .expect("snapshot")
            .cancellation_requested
    );

    handle
        .cancel()
        .expect("handle cancel routes through runtime");
    handle.cancel().expect("second cancel is idempotent");
    runtime
        .cancel_run(&request.run_id)
        .expect("runtime cancel is idempotent");

    let snapshot = runtime.run_snapshot(&request.run_id).expect("snapshot");
    assert_eq!(snapshot.status, RunRegistryStatus::CancellationRequested);
    assert!(snapshot.cancellation_requested);
}

#[test]
fn runtime_subscription_helpers_and_handle_stream_use_event_bus() {
    let agent = runtime_agent();
    let event_bus = InMemoryAgentEventBus::default();
    let runtime = AgentRuntime::builder()
        .default_package(runtime_package(&agent))
        .provider("provider.fake", FakeProvider::default())
        .expect("provider route registers")
        .journal(FakeJournalStore::default())
        .event_bus(event_bus.clone())
        .content(FakeContentResolver::default())
        .policy(AllowRunStartPolicy)
        .build()
        .expect("runtime builds");
    let request = run_request(&agent, "run.runtime.subscription");
    let handle = runtime.start_run(request.clone()).expect("run registered");
    let frame = runtime_event_frame(&request.run_id, agent.id(), 1, EventKind::RunCompleted);

    event_bus
        .publish(frame.clone())
        .expect("publish to event bus");

    let runtime_frames = runtime
        .subscribe_run(request.run_id.clone(), None)
        .expect("runtime subscribe")
        .collect::<Vec<_>>();
    let handle_frames = handle
        .stream_from(None)
        .expect("handle subscribe")
        .collect::<Vec<_>>();

    assert_eq!(runtime_frames, vec![frame.clone()]);
    assert_eq!(handle_frames, vec![frame.clone()]);
    assert_eq!(
        handle
            .stream_from(Some(handle_frames[0].cursor.clone()))
            .expect("resume after cursor")
            .count(),
        0
    );
}

fn full_runtime(
    package: RuntimePackage,
    provider: FakeProvider,
    journal: FakeJournalStore,
) -> AgentRuntime {
    AgentRuntime::builder()
        .default_package(package)
        .provider("provider.fake", provider)
        .expect("provider route registers")
        .journal(journal)
        .event_bus(InMemoryAgentEventBus::default())
        .content(FakeContentResolver::default())
        .policy(AllowRunStartPolicy)
        .build()
        .expect("runtime builds")
}

fn runtime_event_frame(
    run_id: &RunId,
    agent_id: &AgentId,
    event_seq: u64,
    event_kind: EventKind,
) -> EventFrame {
    let event = AgentEvent::with_redacted_summary(
        EventEnvelope {
            schema_version: EVENT_SCHEMA_VERSION,
            event_id: EventId::new(format!("event.runtime.subscription.{event_seq}")),
            event_seq,
            event_family: EventFamily::Run,
            event_kind,
            payload_schema_version: 1,
            timestamp: "1970-01-01T00:00:00Z".to_string(),
            recorded_at: "1970-01-01T00:00:00Z".to_string(),
            run_id: run_id.clone(),
            session_id: None,
            agent_id: agent_id.clone(),
            turn_id: None,
            attempt_id: None,
            message_id: None,
            context_item_id: None,
            trace_id: TraceId::new(format!("trace.runtime.subscription.{event_seq}")),
            span_id: SpanId::new(format!("span.runtime.subscription.{event_seq}")),
            parent_event_id: None,
            caused_by: None,
            subject_ref: EntityRef::run(run_id.clone()),
            related_refs: Vec::new(),
            causal_refs: Vec::new(),
            correlation: Default::default(),
            tags: Vec::new(),
            source: SourceRef::with_kind(SourceKind::Sdk, "source.runtime.subscription"),
            destination: Some(DestinationRef::with_kind(
                DestinationKind::EventStream,
                "destination.event_stream.runtime",
            )),
            policy_refs: Vec::new(),
            journal_cursor: Some(JournalCursor::new(format!("journal.{event_seq}"))),
            state_before: None,
            state_after: None,
            delivery_semantics: EventDeliverySemantics::JournalBacked,
            privacy: PrivacyClass::ContentRefsOnly,
            content_capture: ContentCaptureMode::Off,
            redaction_policy_id: "policy.redaction.default".to_string(),
            runtime_package_fingerprint: "runtime.package.fingerprint.runtime".to_string(),
        },
        "runtime subscription event",
    );
    EventFrame {
        cursor: event.envelope.cursor(EventStreamScope::Run(run_id.clone())),
        event,
        archive_cursor: None,
        overflow: None,
    }
}

fn runtime_agent() -> Agent {
    Agent::builder()
        .id(AgentId::new("agent.runtime.contract"))
        .name("runtime contract")
        .build()
        .expect("agent builds")
}

fn runtime_package(agent: &Agent) -> RuntimePackage {
    RuntimePackage::builder(RuntimePackageId::new("package.runtime.contract"))
        .agent(agent.snapshot())
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake.p0"))
        .policy(PolicyRef::with_kind(
            PolicyKind::RuntimePackage,
            "policy.runtime.package",
        ))
        .build()
        .expect("package builds")
}

fn run_request(agent: &Agent, run_id: &str) -> RunRequest {
    RunRequest::text(
        RunId::new(run_id),
        agent.id().clone(),
        SourceRef::with_kind(SourceKind::Host, "source.runtime.contract"),
        "runtime shell should not execute this input",
    )
}

fn policy_outcome(request: &RunRequest, decision: PolicyDecision) -> PolicyOutcome {
    PolicyOutcome {
        stage: PolicyStage::Input,
        decision,
        subject: None,
        source: Some(request.source.clone()),
        destination: None,
        policy_refs: vec![PolicyRef::with_kind(
            PolicyKind::RuntimePackage,
            "policy.runtime.start",
        )],
        privacy: PrivacyClass::Internal,
        retention: RetentionClass::RunScoped,
    }
}
