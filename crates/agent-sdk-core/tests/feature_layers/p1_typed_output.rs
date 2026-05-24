use agent_sdk_core::{
    AdapterRef, Agent, AgentError, AgentErrorKind, AgentEventBus, AgentId, AgentRuntime, ContentId,
    ContentKind, ContentRef, ContentResolutionPolicy, ContentResolutionPurpose,
    ContentResolveRequest, ContentResolver, ContentScope, ContentVersion, DecodedTypedOutput,
    DestinationKind, DestinationRef, EntityKind, EntityRef, InMemoryAgentEventBus,
    MissingContentPolicy, OutputContract, OutputSchemaId, OutputSchemaRef, PolicyDecision,
    PolicyKind, PolicyOutcome, PolicyRef, PolicyStage, PrivacyClass, ProviderHintPolicy,
    ProviderRouteSnapshot, RetentionClass, RetentionUse, RunId, RunRequest, RunStatus,
    RuntimePackage, RuntimePackageId, RuntimePolicyPort, SchemaVersion, SourceKind, SourceRef,
    StructuredOutputRecord, TrustClass, TypedOutputDeserializer, TypedOutputError,
    TypedOutputModel,
    journal::JournalRecordPayload,
    testing::{FakeContentResolver, FakeJournalStore, FakeProvider},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct TodoExtraction {
    title: String,
    priority: String,
}

impl TypedOutputModel for TodoExtraction {
    const SCHEMA_ID: &'static str = "schema.todo_extraction";
    const SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);

    fn schema_ref() -> OutputSchemaRef {
        OutputContract::inline_json_schema(
            OutputSchemaId::new(Self::SCHEMA_ID),
            Self::SCHEMA_VERSION,
            todo_schema(),
        )
        .schema
    }
}

#[derive(Clone, Debug)]
struct AllowP1Policy;

impl RuntimePolicyPort for AllowP1Policy {
    fn evaluate_run_start(
        &self,
        request: &RunRequest,
        _package: &RuntimePackage,
    ) -> Result<PolicyOutcome, AgentError> {
        Ok(PolicyOutcome {
            stage: PolicyStage::Input,
            decision: PolicyDecision::allow("policy.p1.allow"),
            subject: None,
            source: Some(request.source.clone()),
            destination: None,
            policy_refs: vec![PolicyRef::with_kind(
                PolicyKind::RuntimePackage,
                "policy.p1",
            )],
            privacy: PrivacyClass::Internal,
            retention: RetentionClass::RunScoped,
        })
    }
}

#[test]
fn run_typed_valid_output_uses_p0_loop_and_returns_typed_value() {
    let agent = p1_agent();
    let provider = FakeProvider::with_responses([todo_json("Book dentist", "medium")]);
    let journal = FakeJournalStore::default();
    let event_bus = InMemoryAgentEventBus::default();
    let content = FakeContentResolver::default();
    let runtime = p1_runtime(
        &agent,
        provider.clone(),
        journal.clone(),
        event_bus.clone(),
        content.clone(),
    );

    let result = agent
        .run_typed::<TodoExtraction>(
            &runtime,
            RunId::new("run.p1.typed.valid"),
            SourceRef::with_kind(SourceKind::Host, "source.p1.test"),
            "extract the todo",
        )
        .expect("typed run succeeds");

    assert_eq!(result.status, RunStatus::Completed);
    assert!(result.structured_output.is_some());
    let provider_requests = provider.requests();
    assert_eq!(provider_requests.len(), 1);
    assert_structured_output_hint(&provider_requests[0]);

    let typed = extract_todo(&result, &content).expect("typed extraction succeeds");
    assert_eq!(
        typed.output,
        TodoExtraction {
            title: "Book dentist".to_string(),
            priority: "medium".to_string(),
        }
    );

    let frames = event_bus
        .subscribe_run(RunId::new("run.p1.typed.valid"), None)
        .expect("event stream")
        .collect::<Vec<_>>();
    assert_eq!(
        event_summary(&frames),
        fixture("tests/fixtures/p1/typed-run-events.json")
    );
    assert_eq!(
        journal_summary(&journal.records()),
        fixture("tests/fixtures/p1/typed-run-journal.json")
    );
    assert_structured_events_are_journal_backed(&frames);
}

#[test]
fn explicit_output_contract_and_typed_helper_share_runtime_path() {
    let agent = p1_agent();
    let helper_provider = FakeProvider::with_responses([valid_todo_json()]);
    let helper_event_bus = InMemoryAgentEventBus::default();
    let helper_journal = FakeJournalStore::default();
    let helper_content = FakeContentResolver::default();
    let helper_runtime = p1_runtime(
        &agent,
        helper_provider.clone(),
        helper_journal.clone(),
        helper_event_bus.clone(),
        helper_content,
    );
    let helper_result = agent
        .run_typed::<TodoExtraction>(
            &helper_runtime,
            RunId::new("run.p1.helper.path"),
            SourceRef::with_kind(SourceKind::Host, "source.p1.helper"),
            "extract helper",
        )
        .expect("helper run succeeds");

    let explicit_provider = FakeProvider::with_responses([valid_todo_json()]);
    let explicit_event_bus = InMemoryAgentEventBus::default();
    let explicit_journal = FakeJournalStore::default();
    let explicit_content = FakeContentResolver::default();
    let explicit_runtime = p1_runtime(
        &agent,
        explicit_provider.clone(),
        explicit_journal.clone(),
        explicit_event_bus.clone(),
        explicit_content,
    );
    let explicit_request = RunRequest::text(
        RunId::new("run.p1.explicit.path"),
        agent.id().clone(),
        SourceRef::with_kind(SourceKind::Host, "source.p1.explicit"),
        "extract explicit",
    )
    .with_output_contract(OutputContract::for_type::<TodoExtraction>());
    let explicit_result = explicit_runtime
        .run_text(explicit_request)
        .expect("explicit output contract run succeeds");

    assert_eq!(helper_result.status, explicit_result.status);
    assert_eq!(
        helper_provider.requests().len(),
        explicit_provider.requests().len()
    );
    assert_eq!(
        event_kinds(&helper_event_bus, "run.p1.helper.path"),
        event_kinds(&explicit_event_bus, "run.p1.explicit.path")
    );
    assert_eq!(
        journal_payload_kinds(&helper_journal.records()),
        journal_payload_kinds(&explicit_journal.records())
    );
}

#[test]
fn invalid_output_repairs_and_publishes_typed_result_after_evidence() {
    let agent = p1_agent();
    let provider = FakeProvider::with_responses([invalid_todo_json(), valid_todo_json()]);
    let journal = FakeJournalStore::default();
    let event_bus = InMemoryAgentEventBus::default();
    let content = FakeContentResolver::default();
    let runtime = p1_runtime(
        &agent,
        provider.clone(),
        journal.clone(),
        event_bus.clone(),
        content.clone(),
    );

    let result = agent
        .run_typed::<TodoExtraction>(
            &runtime,
            RunId::new("run.p1.typed.repair"),
            SourceRef::with_kind(SourceKind::Host, "source.p1.repair"),
            "extract and repair",
        )
        .expect("repair succeeds");

    assert_eq!(provider.requests().len(), 2);
    assert_eq!(result.status, RunStatus::Completed);
    assert_eq!(
        extract_todo(&result, &content)
            .expect("typed extraction after repair")
            .output
            .priority,
        "high"
    );

    let frames = event_bus
        .subscribe_run(RunId::new("run.p1.typed.repair"), None)
        .expect("event stream")
        .collect::<Vec<_>>();
    assert_eq!(
        event_summary(&frames),
        fixture("tests/fixtures/p1/repair-success-events.json")
    );
    assert_eq!(
        journal_summary(&journal.records()),
        fixture("tests/fixtures/p1/repair-success-journal.json")
    );
    assert_structured_events_are_journal_backed(&frames);
}

#[test]
fn repair_exhaustion_returns_structured_output_failure_without_typed_value() {
    let agent = p1_agent();
    let provider = FakeProvider::with_responses([
        invalid_todo_json(),
        invalid_todo_json(),
        invalid_todo_json(),
    ]);
    let journal = FakeJournalStore::default();
    let event_bus = InMemoryAgentEventBus::default();
    let runtime = p1_runtime(
        &agent,
        provider,
        journal.clone(),
        event_bus.clone(),
        FakeContentResolver::default(),
    );

    let error = agent
        .run_typed::<TodoExtraction>(
            &runtime,
            RunId::new("run.p1.typed.exhausted"),
            SourceRef::with_kind(SourceKind::Host, "source.p1.exhausted"),
            "extract but exhaust",
        )
        .expect_err("repair exhaustion fails the run");

    assert_eq!(error.kind(), AgentErrorKind::StructuredOutputFailure);
    let frames = event_bus
        .subscribe_run(RunId::new("run.p1.typed.exhausted"), None)
        .expect("event stream")
        .collect::<Vec<_>>();
    assert_eq!(
        event_summary(&frames),
        fixture("tests/fixtures/p1/repair-exhausted-events.json")
    );
    assert_eq!(
        journal_summary(&journal.records()),
        fixture("tests/fixtures/p1/repair-exhausted-journal.json")
    );
    assert_terminal_failure_records_all_validation_attempts(&journal.records());
    assert!(
        frames
            .iter()
            .any(|frame| frame.event.envelope.event_kind == agent_sdk_core::EventKind::RunFailed)
    );
}

#[test]
fn typed_extraction_requires_canonical_validated_content_ref() {
    let (result, _) = successful_typed_result("run.p1.typed.ref-mismatch");
    let artifacts = result
        .structured_output
        .as_ref()
        .expect("structured output");
    let wrong_ref = content_ref(
        "content.p1.wrong.canonical",
        ContentKind::OutputPayload,
        "wrong canonical output",
    );

    let error = result
        .structured_output(&JsonTodoDeserializer::new(
            wrong_ref,
            serde_json::from_str(&valid_todo_json()).expect("valid todo JSON"),
        ))
        .expect_err("typed extraction must reject a wrong canonical ref");

    assert!(matches!(
        error,
        TypedOutputError::CanonicalValueRefMismatch { .. }
    ));
    assert_ne!(
        artifacts
            .validated_output
            .canonical_value_ref
            .content_id
            .as_str(),
        "content.p1.wrong.canonical"
    );
}

#[test]
fn output_delivery_sink_not_required_for_p1_typed_result() {
    let (result, content) = successful_typed_result("run.p1.no-output-sink");
    assert_eq!(
        extract_todo(&result, &content)
            .expect("typed output without output sink")
            .output
            .title,
        "Pay invoice"
    );
}

#[test]
fn output_contract_changes_runtime_package_fingerprint_for_typed_run() {
    let agent = p1_agent();
    let runtime = p1_runtime(
        &agent,
        FakeProvider::with_responses([valid_todo_json()]),
        FakeJournalStore::default(),
        InMemoryAgentEventBus::default(),
        FakeContentResolver::default(),
    );
    let source = SourceRef::with_kind(SourceKind::Host, "source.p1.fingerprint");
    let text_request = RunRequest::text(
        RunId::new("run.p1.fingerprint.text"),
        agent.id().clone(),
        source.clone(),
        "plain",
    );
    let typed_request = RunRequest::typed_text::<TodoExtraction>(
        RunId::new("run.p1.fingerprint.typed"),
        agent.id().clone(),
        source,
        "typed",
    );

    let text_fingerprint = runtime
        .resolve_effective_package(&text_request)
        .expect("text package")
        .fingerprint;
    let typed_fingerprint = runtime
        .resolve_effective_package(&typed_request)
        .expect("typed package")
        .fingerprint;

    assert_ne!(text_fingerprint, typed_fingerprint);
}

#[test]
fn p1_structured_output_events_and_journal_match_golden_fixtures() {
    run_typed_valid_output_uses_p0_loop_and_returns_typed_value();
    invalid_output_repairs_and_publishes_typed_result_after_evidence();
    repair_exhaustion_returns_structured_output_failure_without_typed_value();
}

fn successful_typed_result(run_id: &str) -> (agent_sdk_core::RunResult, FakeContentResolver) {
    let agent = p1_agent();
    let content = FakeContentResolver::default();
    let runtime = p1_runtime(
        &agent,
        FakeProvider::with_responses([valid_todo_json()]),
        FakeJournalStore::default(),
        InMemoryAgentEventBus::default(),
        content.clone(),
    );
    let result = agent
        .run_typed::<TodoExtraction>(
            &runtime,
            RunId::new(run_id),
            SourceRef::with_kind(SourceKind::Host, "source.p1.success-helper"),
            "extract",
        )
        .expect("typed run succeeds");
    (result, content)
}

fn extract_todo(
    result: &agent_sdk_core::RunResult,
    content: &FakeContentResolver,
) -> Result<agent_sdk_core::StructuredOutputResult<TodoExtraction>, TypedOutputError> {
    result.structured_output(&ResolverTodoDeserializer::new(content.clone()))
}

struct JsonTodoDeserializer {
    decoded_ref: ContentRef,
    value: Value,
}

impl JsonTodoDeserializer {
    fn new(decoded_ref: ContentRef, value: Value) -> Self {
        Self { decoded_ref, value }
    }
}

impl TypedOutputDeserializer<TodoExtraction> for JsonTodoDeserializer {
    fn deserialize(
        &self,
        _canonical_value_ref: &ContentRef,
    ) -> Result<DecodedTypedOutput<TodoExtraction>, TypedOutputError> {
        let output = serde_json::from_value::<TodoExtraction>(self.value.clone())
            .expect("test JSON decodes into TodoExtraction");
        Ok(DecodedTypedOutput::new(self.decoded_ref.clone(), output))
    }
}

struct ResolverTodoDeserializer {
    content: FakeContentResolver,
}

impl ResolverTodoDeserializer {
    fn new(content: FakeContentResolver) -> Self {
        Self { content }
    }
}

impl TypedOutputDeserializer<TodoExtraction> for ResolverTodoDeserializer {
    fn deserialize(
        &self,
        canonical_value_ref: &ContentRef,
    ) -> Result<DecodedTypedOutput<TodoExtraction>, TypedOutputError> {
        let resolved = self
            .content
            .resolve(
                ContentResolveRequest::new(canonical_value_ref.clone()),
                ContentResolutionPolicy {
                    caller_ref: EntityRef::new(EntityKind::Content, "content.p1.typed.decoder"),
                    destination_ref: DestinationRef::with_kind(
                        DestinationKind::Host,
                        "destination.p1.typed.decoder",
                    ),
                    purpose: ContentResolutionPurpose::OutputValidation,
                    allowed_privacy_classes: vec![PrivacyClass::ContentRefsOnly],
                    max_bytes: 32 * 1024,
                    require_hash_match: true,
                    retention_use: RetentionUse::Validation,
                    on_missing: MissingContentPolicy::Fail,
                    allow_raw_content: true,
                    policy_refs: vec![PolicyRef::with_kind(
                        PolicyKind::RuntimePackage,
                        "policy.p1.typed.decoder",
                    )],
                },
            )
            .expect("canonical structured output content resolves");
        let bytes = resolved.bytes.expect("raw canonical content included");
        let output = serde_json::from_slice::<TodoExtraction>(&bytes)
            .expect("canonical JSON decodes into TodoExtraction");
        Ok(DecodedTypedOutput::new(resolved.content_ref, output))
    }
}

fn p1_runtime(
    agent: &Agent,
    provider: FakeProvider,
    journal: FakeJournalStore,
    event_bus: InMemoryAgentEventBus,
    content: FakeContentResolver,
) -> AgentRuntime {
    AgentRuntime::builder()
        .default_package(p1_package(agent))
        .provider("provider.fake", provider)
        .expect("provider route registers")
        .journal(journal)
        .event_bus(event_bus)
        .content(content)
        .policy(AllowP1Policy)
        .build()
        .expect("runtime builds")
}

fn p1_agent() -> Agent {
    Agent::builder()
        .id(AgentId::new("agent.p1.typed"))
        .name("p1 typed")
        .build()
        .expect("agent builds")
}

fn p1_package(agent: &Agent) -> RuntimePackage {
    RuntimePackage::builder(RuntimePackageId::new("package.p1.typed"))
        .agent(agent.snapshot())
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake.p1"))
        .policy(PolicyRef::with_kind(
            PolicyKind::RuntimePackage,
            "policy.p1.package",
        ))
        .build()
        .expect("package builds")
}

fn todo_schema() -> Value {
    json!({
        "type": "object",
        "required": ["title", "priority"],
        "properties": {
            "title": { "type": "string" },
            "priority": { "enum": ["low", "medium", "high"] }
        }
    })
}

fn valid_todo_json() -> String {
    todo_json("Pay invoice", "high")
}

fn todo_json(title: &str, priority: &str) -> String {
    serde_json::to_string(&json!({
        "title": title,
        "priority": priority,
    }))
    .expect("todo JSON serializes")
}

fn invalid_todo_json() -> String {
    r#"{"title":"Pay invoice"}"#.to_string()
}

fn event_kinds(event_bus: &InMemoryAgentEventBus, run_id: &str) -> Vec<String> {
    event_bus
        .subscribe_run(RunId::new(run_id), None)
        .expect("event stream")
        .map(|frame| format!("{:?}", frame.event.envelope.event_kind))
        .collect()
}

fn journal_payload_kinds(records: &[agent_sdk_core::JournalRecord]) -> Vec<String> {
    records
        .iter()
        .map(|record| payload_type(&record.payload).to_string())
        .collect()
}

fn assert_structured_output_hint(request: &agent_sdk_core::ProviderRequest) {
    let hint = request
        .structured_output_hint
        .as_ref()
        .expect("typed run projects a structured output hint to the provider request");
    assert_eq!(
        hint.schema_id,
        OutputSchemaId::new(TodoExtraction::SCHEMA_ID)
    );
    assert_eq!(hint.schema_version, TodoExtraction::SCHEMA_VERSION);
    assert_eq!(
        hint.provider_hint_policy,
        ProviderHintPolicy::SchemaRequired
    );
    assert!(hint.include_schema_ref);
}

fn assert_structured_events_are_journal_backed(frames: &[agent_sdk_core::EventFrame]) {
    for frame in frames.iter().filter(|frame| {
        frame.event.envelope.event_family == agent_sdk_core::EventFamily::StructuredOutput
    }) {
        assert!(
            frame.event.envelope.journal_cursor.is_some(),
            "structured-output events are emitted only after journal append"
        );
    }
    assert!(
        frames.iter().all(|frame| {
            !(frame.event.envelope.event_family == agent_sdk_core::EventFamily::Output
                && matches!(
                    frame.event.envelope.event_kind,
                    agent_sdk_core::EventKind::StructuredOutputRequested
                        | agent_sdk_core::EventKind::StructuredOutputValidationStarted
                        | agent_sdk_core::EventKind::StructuredOutputValidationFailed
                        | agent_sdk_core::EventKind::StructuredOutputRepairRequested
                        | agent_sdk_core::EventKind::StructuredOutputValidated
                        | agent_sdk_core::EventKind::StructuredOutputFailed
                ))
        }),
        "structured-output events must not collide with output-delivery family semantics"
    );
}

fn assert_terminal_failure_records_all_validation_attempts(
    records: &[agent_sdk_core::JournalRecord],
) {
    let mut exhaustion_counts = None;
    let mut terminal_failure_counts = None;

    for record in records {
        match &record.payload {
            JournalRecordPayload::StructuredOutput(StructuredOutputRecord::RepairExhaustion(
                exhaustion,
            )) => {
                exhaustion_counts = Some((
                    exhaustion.validation_attempts.len(),
                    exhaustion.repair_attempts.len(),
                    exhaustion.source_attempt_ids.len(),
                ));
            }
            JournalRecordPayload::StructuredOutput(StructuredOutputRecord::Validation(
                validation,
            )) if validation.record_kind
                == agent_sdk_core::ValidationRecordKind::TerminalFailure =>
            {
                terminal_failure_counts = Some((
                    validation.validation_attempts.len(),
                    validation.repair_attempts.len(),
                    validation.source_attempt_ids.len(),
                ));
            }
            _ => {}
        }
    }

    assert_eq!(exhaustion_counts, Some((3, 2, 3)));
    assert_eq!(terminal_failure_counts, Some((3, 2, 3)));
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

fn payload_type(payload: &JournalRecordPayload) -> &'static str {
    match payload {
        JournalRecordPayload::RunLifecycle(_) => "run_lifecycle",
        JournalRecordPayload::ContextProjection(_) => "context_projection",
        JournalRecordPayload::ModelAttempt(_) => "model_attempt",
        JournalRecordPayload::Message(_) => "message",
        JournalRecordPayload::StructuredOutput(record) => structured_payload_type(record),
        JournalRecordPayload::Approval(_) => "approval",
        JournalRecordPayload::Tool(_) => "tool",
        JournalRecordPayload::OutputDelivery(_) => "output_delivery",
        JournalRecordPayload::Hook(_) => "hook",
        JournalRecordPayload::StreamRule(_) => "stream_rule",
        JournalRecordPayload::RealtimeSession(_) => "realtime_session",
        JournalRecordPayload::Isolation(_) => "isolation",
        JournalRecordPayload::ChildLifecycle(_) => "child_lifecycle",
        JournalRecordPayload::AgentPool(_) => "agent_pool",
        JournalRecordPayload::RunMessage(_) => "run_message",
        JournalRecordPayload::Wake(_) => "wake",
        JournalRecordPayload::Subagent(_) => "subagent",
        JournalRecordPayload::ExtensionAction(_) => "extension_action",
        JournalRecordPayload::EffectIntent(_) => "effect_intent",
        JournalRecordPayload::EffectResult(_) => "effect_result",
        JournalRecordPayload::Checkpoint(_) => "checkpoint",
        JournalRecordPayload::Recovery(_) => "recovery",
        JournalRecordPayload::TerminalResult(_) => "terminal_result",
    }
}

fn structured_payload_type(record: &StructuredOutputRecord) -> &'static str {
    match record {
        StructuredOutputRecord::Lifecycle(record) => match record.record_kind {
            agent_sdk_core::StructuredOutputLifecycleKind::Requested => {
                "structured_output_requested"
            }
            agent_sdk_core::StructuredOutputLifecycleKind::ValidationStarted => {
                "structured_output_validation_started"
            }
        },
        StructuredOutputRecord::Validation(record) => match record.record_kind {
            agent_sdk_core::ValidationRecordKind::ValidationSucceeded => {
                "structured_output_validation_succeeded"
            }
            agent_sdk_core::ValidationRecordKind::ValidationFailed => {
                "structured_output_validation_failed"
            }
            agent_sdk_core::ValidationRecordKind::SchemaRejected => {
                "structured_output_schema_rejected"
            }
            agent_sdk_core::ValidationRecordKind::TerminalFailure => {
                "structured_output_terminal_failure"
            }
        },
        StructuredOutputRecord::Repair(_) => "structured_output_repair_requested",
        StructuredOutputRecord::RepairExhaustion(_) => "structured_output_repair_exhausted",
        StructuredOutputRecord::ValidationReport(_) => "structured_output_validation_report",
        StructuredOutputRecord::ValidatedOutput(_) => "structured_output_validated_output",
        StructuredOutputRecord::TypedResultPublication(_) => {
            "structured_output_typed_result_publication"
        }
    }
}

fn fixture(path: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture readable"))
        .expect("fixture JSON")
}

fn content_ref(id: &str, kind: ContentKind, summary: &str) -> ContentRef {
    let mut content_ref = ContentRef::new(
        ContentId::new(id),
        ContentVersion::new("v1"),
        kind,
        ContentScope::Run,
        EntityRef::new(EntityKind::Run, RunId::new("run.p1.deserializer")),
        SourceRef::with_kind(SourceKind::Sdk, "source.p1.deserializer"),
        AdapterRef::new("resolver.content.fake"),
        summary,
    );
    content_ref.mime = Some("application/json".to_string());
    content_ref.size_bytes = Some(128);
    content_ref.content_hash =
        Some("sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".to_string());
    content_ref.privacy_class = PrivacyClass::ContentRefsOnly;
    content_ref.retention_class = RetentionClass::RunScoped;
    content_ref.trust_class = TrustClass::SdkGenerated;
    content_ref
}
