use std::{
    any::type_name,
    mem::size_of,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use agent_sdk_core::{
    Agent, AgentError, AgentErrorKind, AgentEvent, AgentEventBus, AgentId, AgentRuntime,
    CapabilitySpec, ContentResolver, ContextItem, ContextProjection, EntityRef, EventFamily,
    EventKind, InMemoryAgentEventBus, JournalRecord, JournalRecordPayload, OutputContract,
    OutputSchemaId, OutputSchemaRef, PolicyDecision, PolicyKind, PolicyOutcome, PolicyRef,
    PolicyStage, PrivacyClass, ProviderAdapter, ProviderHintPolicy, ProviderRouteSnapshot,
    RetentionClass, RunId, RunJournal, RunRequest, RunResult, RunStatus, RuntimePackage,
    RuntimePackageBuilder, RuntimePackageId, RuntimePolicyPort, SchemaVersion, SourceKind,
    SourceRef, StructuredOutputRecord, TelemetryContentCaptureMode, TelemetryTerminalStatus,
    TypedOutputModel, ValidatedOutput,
    event::ContentCaptureMode as EventContentCaptureMode,
    policy::ContentCaptureMode as PolicyContentCaptureMode,
    terminal_run_projection_from_event,
    testing::{FakeContentResolver, FakeJournalStore, FakeProvider},
};
use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq)]
struct PublicTodo;

impl TypedOutputModel for PublicTodo {
    const SCHEMA_ID: &'static str = "schema.public_api.todo";
    const SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);

    fn schema_ref() -> OutputSchemaRef {
        OutputContract::inline_json_schema(
            OutputSchemaId::new(Self::SCHEMA_ID),
            Self::SCHEMA_VERSION,
            public_todo_schema(),
        )
        .schema
    }
}

#[derive(Clone, Debug)]
struct CountingPolicy {
    calls: Arc<AtomicUsize>,
}

impl CountingPolicy {
    fn new(calls: Arc<AtomicUsize>) -> Self {
        Self { calls }
    }
}

impl RuntimePolicyPort for CountingPolicy {
    fn evaluate_run_start(
        &self,
        request: &RunRequest,
        _package: &RuntimePackage,
    ) -> Result<PolicyOutcome, AgentError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(PolicyOutcome {
            stage: PolicyStage::Input,
            decision: PolicyDecision::allow("policy.public_api.allow"),
            subject: Some(EntityRef::run(request.run_id.clone())),
            source: Some(request.source.clone()),
            destination: None,
            policy_refs: vec![PolicyRef::with_kind(
                PolicyKind::RuntimePackage,
                "policy.public_api.runtime",
            )],
            privacy: PrivacyClass::Internal,
            retention: RetentionClass::RunScoped,
        })
    }
}

fn assert_typed_model_exported<T: TypedOutputModel>() {}

#[test]
fn api_exports_match_phase_12b_contract() {
    let exported = [
        type_name::<Agent>(),
        type_name::<AgentRuntime>(),
        type_name::<RunRequest>(),
        type_name::<agent_sdk_core::RunHandle>(),
        type_name::<RunResult>(),
        type_name::<RuntimePackage>(),
        type_name::<RuntimePackageBuilder>(),
        type_name::<CapabilitySpec>(),
        type_name::<AgentEvent>(),
        type_name::<agent_sdk_core::EventFrame>(),
        type_name::<JournalRecord>(),
        type_name::<ContextProjection>(),
        type_name::<ContextItem>(),
        type_name::<agent_sdk_core::AgentMessage>(),
        type_name::<OutputContract>(),
        type_name::<ValidatedOutput>(),
        type_name::<PolicyDecision>(),
        type_name::<dyn ProviderAdapter>(),
        type_name::<dyn RunJournal>(),
        type_name::<dyn AgentEventBus>(),
        type_name::<dyn ContentResolver>(),
        type_name::<dyn RuntimePolicyPort>(),
    ];

    assert_typed_model_exported::<PublicTodo>();
    assert!(exported.iter().all(|path| path.contains("agent_sdk_core")));
    assert!(exported.iter().any(|path| path.contains("ProviderAdapter")));
    assert!(
        exported
            .iter()
            .any(|path| path.contains("RuntimePolicyPort"))
    );
}

#[test]
fn one_line_helpers_lower_into_canonical_dtos() {
    let agent = public_api_agent();
    let source = SourceRef::with_kind(SourceKind::Host, "source.public_api.lowering");
    let helper_request = agent.typed_text_request::<PublicTodo>(
        RunId::new("run.public_api.helper.lowering"),
        source.clone(),
        "extract helper",
    );
    let canonical_request = RunRequest::text(
        RunId::new("run.public_api.helper.lowering"),
        agent.id().clone(),
        source,
        "extract helper",
    )
    .with_output_contract(OutputContract::for_type::<PublicTodo>());

    assert_eq!(helper_request, canonical_request);
    let contract = helper_request
        .output_contract
        .as_ref()
        .expect("typed helper lowers to output contract");
    assert_eq!(
        contract.schema_id,
        OutputSchemaId::new(PublicTodo::SCHEMA_ID)
    );
    assert_eq!(contract.schema_version, PublicTodo::SCHEMA_VERSION);
    assert_eq!(
        contract.projection_hint.provider_hint_policy,
        ProviderHintPolicy::SchemaRequired
    );
    assert_eq!(contract.content_policy.mode, PolicyContentCaptureMode::Off);
}

#[test]
fn helper_and_explicit_requests_share_validation_policy_journal_event_and_telemetry_path() {
    let agent = public_api_agent();

    let helper = run_public_typed(&agent, "run.public_api.helper.path", RunRequestKind::Helper);
    let explicit = run_public_typed(
        &agent,
        "run.public_api.explicit.path",
        RunRequestKind::Explicit,
    );

    assert_eq!(helper.result.status, RunStatus::Completed);
    assert_eq!(explicit.result.status, RunStatus::Completed);
    assert_eq!(helper.policy_calls.load(Ordering::SeqCst), 1);
    assert_eq!(explicit.policy_calls.load(Ordering::SeqCst), 1);
    assert_eq!(helper.provider.requests().len(), 1);
    assert_eq!(explicit.provider.requests().len(), 1);
    assert_eq!(
        event_kinds(&helper.frames),
        event_kinds(&explicit.frames),
        "helper and explicit paths emit equivalent event vocabulary"
    );
    assert_eq!(
        journal_payload_kinds(&helper.journal.records()),
        journal_payload_kinds(&explicit.journal.records()),
        "helper and explicit paths append equivalent durable record shapes"
    );

    assert_provider_request_is_structured(&helper.provider.requests()[0]);
    assert_validation_journal_event_and_redaction_path(&helper);
}

#[test]
fn public_facade_is_product_neutral_and_semver_documented() {
    let crate_root = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("crate root readable");
    for forbidden in [
        "Clawdia", "clawdia", "Pawtrace", "pawtrace", "iMessage", "ACP",
    ] {
        assert!(
            !crate_root.contains(forbidden),
            "product-specific public facade text leaked: {forbidden}"
        );
    }

    assert!(
        crate_root.contains("# SemVer Posture"),
        "crate docs must document public import stability posture"
    );
    for required_export in [
        "pub use agent::{Agent, AgentBuilder};",
        "pub use runtime::",
        "pub use run::{RunRequest, RunResult",
        "pub use run_handle::{InMemoryRunControlStore, RunControlStore, RunHandle};",
        "pub use package::{",
        "pub use event::{",
        "pub use journal::{",
        "pub use context::{",
        "pub use output::{",
        "pub use policy::{",
        "pub use ports::{",
        "pub mod testing;",
        "pub mod prelude {",
    ] {
        assert!(
            crate_root.contains(required_export),
            "missing expected public facade export marker: {required_export}"
        );
    }
}

#[test]
fn prelude_exports_common_app_building_surface_without_new_behavior() {
    use agent_sdk_core::prelude::*;

    let agent = Agent::builder()
        .id(AgentId::new("agent.public_api.prelude"))
        .name("prelude")
        .build()
        .expect("agent builds");
    let source = SourceRef::with_kind(SourceKind::Host, "source.public_api.prelude");
    let request = agent.typed_text_request::<PublicTodo>(
        RunId::new("run.public_api.prelude"),
        source.clone(),
        "extract with prelude",
    );
    let canonical = RunRequest::text(
        RunId::new("run.public_api.prelude"),
        agent.id().clone(),
        source,
        "extract with prelude",
    )
    .with_output_contract(OutputContract::for_type::<PublicTodo>());

    assert_eq!(
        request, canonical,
        "prelude imports must not create a behavior path separate from canonical lowering"
    );

    let exported = [
        type_name::<AgentRuntime>(),
        type_name::<RuntimePackage>(),
        type_name::<RuntimePackageBuilder>(),
        type_name::<CapabilitySpec>(),
        type_name::<AgentEvent>(),
        type_name::<EventFrame>(),
        type_name::<EventFilter>(),
        type_name::<JournalRecord>(),
        type_name::<ContextContribution>(),
        type_name::<ContextItem>(),
        type_name::<ContextProjection>(),
        type_name::<AgentMessage>(),
        type_name::<OutputSchemaRef>(),
        type_name::<OutputSchemaId>(),
        type_name::<SchemaVersion>(),
        type_name::<ValidatedOutput>(),
        type_name::<PolicyDecision>(),
        type_name::<PolicyOutcome>(),
        type_name::<PrivacyClass>(),
        type_name::<RetentionClass>(),
        type_name::<TrustClass>(),
        type_name::<SourceKind>(),
        type_name::<DestinationKind>(),
        type_name::<EntityRef>(),
        type_name::<dyn ProviderAdapter>(),
        type_name::<dyn RunJournal>(),
        type_name::<dyn AgentEventBus>(),
        type_name::<dyn ContentResolver>(),
        type_name::<dyn RuntimePolicyPort>(),
    ];

    assert!(exported.iter().all(|path| path.contains("agent_sdk_core")));
}

#[test]
fn public_error_type_is_small_and_keeps_constructor_accessor_contract() {
    assert!(
        size_of::<AgentError>() <= 128,
        "AgentError should remain cheap enough for public Result<T, AgentError> APIs"
    );

    let error = AgentError::new(
        AgentErrorKind::PolicyDenial,
        agent_sdk_core::RetryClassification::UserActionNeeded,
        "approval required",
    )
    .with_policy_ref(PolicyRef::with_kind(
        PolicyKind::RuntimePackage,
        "policy.public_api.error",
    ))
    .with_causal_ids(agent_sdk_core::CausalIds {
        run_id: Some(RunId::new("run.public_api.error")),
        ..agent_sdk_core::CausalIds::default()
    });

    assert_eq!(error.kind(), AgentErrorKind::PolicyDenial);
    assert_eq!(
        error.retry(),
        agent_sdk_core::RetryClassification::UserActionNeeded
    );
    assert_eq!(error.context().message, "approval required");
    assert_eq!(
        error.causal_ids().run_id.as_ref().map(RunId::as_str),
        Some("run.public_api.error")
    );

    let serialized = serde_json::to_value(&error).expect("AgentError serializes");
    assert_eq!(
        serialized["Classified"]["context"]["message"], "approval required",
        "boxing AgentError internals must not change the stable serialized context shape"
    );
    assert_eq!(
        serialized["Classified"]["causal_ids"]["run_id"], "run.public_api.error",
        "boxing AgentError internals must not change the stable serialized causal-id shape"
    );
}

#[test]
fn public_large_error_payloads_are_cheap_without_changing_json_contracts() {
    assert!(
        size_of::<agent_sdk_core::ContentResolutionError>() <= 128,
        "content resolver errors should stay cheap for public Result APIs"
    );
    assert!(
        size_of::<
            Result<agent_sdk_core::ValidationSuccess, Box<agent_sdk_core::ValidationErrorReport>>,
        >() < size_of::<agent_sdk_core::ValidationErrorReport>(),
        "validation failures should be returned behind indirection"
    );

    let content_ref = agent_sdk_core::ContentRef::new(
        agent_sdk_core::ContentId::new("content.public_api.error"),
        agent_sdk_core::ContentVersion::new("v1"),
        agent_sdk_core::ContentKind::Document,
        agent_sdk_core::ContentScope::Run,
        EntityRef::run(RunId::new("run.public_api.error")),
        SourceRef::with_kind(SourceKind::Host, "source.public_api.error"),
        agent_sdk_core::AdapterRef::new("adapter.public_api.content"),
        "content error fixture",
    );
    let content_error = agent_sdk_core::ContentResolutionError {
        kind: agent_sdk_core::ContentResolutionErrorKind::Missing,
        content_ref: Box::new(content_ref),
        policy_refs: vec![PolicyRef::with_kind(
            PolicyKind::RuntimePackage,
            "policy.public_api.content",
        )],
        redacted_summary: "content missing".to_string(),
    };
    let serialized_content_error =
        serde_json::to_value(&content_error).expect("content error serializes");
    assert_eq!(
        serialized_content_error["content_ref"]["content_id"], "content.public_api.error",
        "boxing ContentResolutionError internals must not change the serialized content-ref shape"
    );

    let validator = agent_sdk_core::JsonSchemaSubsetValidator::default();
    let validation_result = agent_sdk_core::StructuredOutputValidator::validate_candidate(
        &validator,
        &OutputContract::for_type::<PublicTodo>(),
        agent_sdk_core::ValidationAttemptId::new("validation.public_api.error"),
        &agent_sdk_core::OutputCandidate::new(
            agent_sdk_core::AttemptId::new("attempt.public_api.error"),
            agent_sdk_core::domain::ContentRef::new("content.validation.public_api.error"),
            "{}",
        ),
    );
    let report = validation_result.expect_err("invalid typed output returns report");
    let serialized_report = serde_json::to_value(&report).expect("validation report serializes");
    assert_eq!(
        serialized_report["candidate_content_ref"], "content.validation.public_api.error",
        "boxing the validation Result error must not change the report JSON shape"
    );
}

struct RunEvidence {
    result: RunResult,
    provider: FakeProvider,
    journal: FakeJournalStore,
    frames: Vec<agent_sdk_core::EventFrame>,
    policy_calls: Arc<AtomicUsize>,
}

enum RunRequestKind {
    Helper,
    Explicit,
}

fn run_public_typed(agent: &Agent, run_id: &str, kind: RunRequestKind) -> RunEvidence {
    let provider = FakeProvider::with_responses([public_todo_json()]);
    let journal = FakeJournalStore::default();
    let event_bus = InMemoryAgentEventBus::default();
    let content = FakeContentResolver::default();
    let policy_calls = Arc::new(AtomicUsize::new(0));
    let runtime = AgentRuntime::builder()
        .default_package(public_api_package(agent))
        .provider("provider.fake", provider.clone())
        .expect("provider route registers")
        .journal(journal.clone())
        .event_bus(event_bus.clone())
        .content(content)
        .policy(CountingPolicy::new(policy_calls.clone()))
        .build()
        .expect("runtime builds");

    let source = SourceRef::with_kind(SourceKind::Host, "source.public_api.run");
    let result = match kind {
        RunRequestKind::Helper => agent.run_typed::<PublicTodo>(
            &runtime,
            RunId::new(run_id),
            source,
            "extract public todo",
        ),
        RunRequestKind::Explicit => runtime.run_text(
            RunRequest::text(
                RunId::new(run_id),
                agent.id().clone(),
                source,
                "extract public todo",
            )
            .with_output_contract(OutputContract::for_type::<PublicTodo>()),
        ),
    }
    .expect("typed run succeeds");

    let frames = event_bus
        .subscribe_run(RunId::new(run_id), None)
        .expect("run events")
        .collect::<Vec<_>>();

    RunEvidence {
        result,
        provider,
        journal,
        frames,
        policy_calls,
    }
}

fn assert_validation_journal_event_and_redaction_path(evidence: &RunEvidence) {
    assert!(
        evidence.result.structured_output.is_some(),
        "typed helper must run local structured-output validation"
    );
    let structured_output = evidence
        .result
        .structured_output
        .as_ref()
        .expect("structured output artifacts");
    assert_eq!(
        structured_output.validated_output.privacy,
        PrivacyClass::ContentRefsOnly
    );
    assert!(
        !structured_output
            .validated_output
            .lineage
            .lineage_ref
            .policy_refs
            .is_empty(),
        "validated output preserves policy lineage"
    );
    assert!(
        structured_output
            .validated_output
            .lineage
            .lineage_ref
            .destination
            .is_some(),
        "validated output has typed destination lineage"
    );
    assert!(
        !structured_output.validated_output.policy_refs.is_empty(),
        "validated output carries validator and repair policy refs"
    );

    let frames = &evidence.frames;
    assert!(
        frames
            .iter()
            .all(|frame| frame.event.envelope.journal_cursor.is_some()),
        "public helper events are published after journal append"
    );
    assert!(
        frames
            .iter()
            .all(|frame| frame.event.envelope.content_capture == EventContentCaptureMode::Off),
        "public helper event stream is raw-content-free by default"
    );
    assert!(
        frames
            .iter()
            .all(|frame| frame.event.envelope.privacy == PrivacyClass::ContentRefsOnly),
        "public helper event stream uses content refs by default"
    );
    assert!(
        frames
            .iter()
            .all(|frame| frame.event.redacted_summary().is_some()),
        "public helper emits redacted summaries instead of raw payloads"
    );
    assert!(
        frames.iter().any(|frame| {
            frame.event.envelope.event_family == EventFamily::StructuredOutput
                && frame.event.envelope.event_kind == EventKind::StructuredOutputValidated
        }),
        "typed helper must emit structured-output validation event"
    );

    let records = evidence.journal.records();
    assert!(
        records
            .iter()
            .all(|record| record.privacy == PrivacyClass::ContentRefsOnly),
        "journal records use content refs by default"
    );
    assert!(
        records
            .iter()
            .all(|record| !record.redaction_policy_id.is_empty()),
        "journal records carry redaction policy ids"
    );
    assert_ordered(&records, "effect_intent", "model_attempt");
    assert_ordered(&records, "effect_intent", "effect_result");
    assert!(
        records.iter().any(|record| matches!(
            &record.payload,
            JournalRecordPayload::StructuredOutput(StructuredOutputRecord::Validation(_))
        )),
        "typed helper must journal validation records"
    );

    let terminal_event = frames
        .iter()
        .find(|frame| frame.event.envelope.event_kind == EventKind::RunCompleted)
        .expect("terminal event")
        .event
        .clone();
    let telemetry = terminal_run_projection_from_event(terminal_event);
    assert_eq!(
        telemetry.content_capture,
        TelemetryContentCaptureMode::Off,
        "terminal telemetry projection is raw-content-free by default"
    );
    assert_eq!(
        telemetry.terminal_status,
        Some(TelemetryTerminalStatus::Completed)
    );
    assert!(
        telemetry.raw_content.is_none(),
        "telemetry projection must not expose raw content by default"
    );
    assert!(
        telemetry.journal_cursor.is_some(),
        "telemetry derives from the journal-backed terminal event"
    );
}

fn assert_provider_request_is_structured(request: &agent_sdk_core::ProviderRequest) {
    let hint = request
        .structured_output_hint
        .as_ref()
        .expect("provider request includes structured output hint");
    assert_eq!(hint.schema_id, OutputSchemaId::new(PublicTodo::SCHEMA_ID));
    assert_eq!(hint.schema_version, PublicTodo::SCHEMA_VERSION);
    assert_eq!(
        hint.provider_hint_policy,
        ProviderHintPolicy::SchemaRequired
    );
}

fn assert_ordered(records: &[JournalRecord], before: &str, after: &str) {
    let before_index = records
        .iter()
        .position(|record| payload_kind(&record.payload) == before)
        .expect("before record exists");
    let after_index = records
        .iter()
        .position(|record| payload_kind(&record.payload) == after)
        .expect("after record exists");
    assert!(
        before_index < after_index,
        "{before} must be journaled before {after}"
    );
}

fn event_kinds(frames: &[agent_sdk_core::EventFrame]) -> Vec<String> {
    frames
        .iter()
        .map(|frame| format!("{:?}", frame.event.envelope.event_kind))
        .collect()
}

fn journal_payload_kinds(records: &[JournalRecord]) -> Vec<&'static str> {
    records
        .iter()
        .map(|record| payload_kind(&record.payload))
        .collect()
}

fn payload_kind(payload: &JournalRecordPayload) -> &'static str {
    match payload {
        JournalRecordPayload::RunLifecycle(_) => "run_lifecycle",
        JournalRecordPayload::ContextProjection(_) => "context_projection",
        JournalRecordPayload::ModelAttempt(_) => "model_attempt",
        JournalRecordPayload::Message(_) => "message",
        JournalRecordPayload::StructuredOutput(record) => match record {
            StructuredOutputRecord::Lifecycle(_) => "structured_output_lifecycle",
            StructuredOutputRecord::Validation(_) => "structured_output_validation",
            StructuredOutputRecord::Repair(_) => "structured_output_repair",
            StructuredOutputRecord::RepairExhaustion(_) => "structured_output_repair_exhaustion",
            StructuredOutputRecord::ValidationReport(_) => "structured_output_validation_report",
            StructuredOutputRecord::ValidatedOutput(_) => "structured_output_validated_output",
            StructuredOutputRecord::TypedResultPublication(_) => "typed_result_publication",
        },
        JournalRecordPayload::EffectIntent(_) => "effect_intent",
        JournalRecordPayload::EffectResult(_) => "effect_result",
        JournalRecordPayload::TerminalResult(_) => "terminal_result",
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
        JournalRecordPayload::Checkpoint(_) => "checkpoint",
        JournalRecordPayload::Recovery(_) => "recovery",
    }
}

fn public_api_agent() -> Agent {
    Agent::builder()
        .id(AgentId::new("agent.public_api"))
        .name("public api")
        .build()
        .expect("agent builds")
}

fn public_api_package(agent: &Agent) -> RuntimePackage {
    RuntimePackage::builder(RuntimePackageId::new("package.public_api"))
        .agent(agent.snapshot())
        .provider_route(ProviderRouteSnapshot::new(
            "provider.fake",
            "model.fake.public",
        ))
        .policy(PolicyRef::with_kind(
            PolicyKind::RuntimePackage,
            "policy.public_api.package",
        ))
        .build()
        .expect("package builds")
}

fn public_todo_schema() -> Value {
    json!({
        "type": "object",
        "required": ["title", "priority"],
        "properties": {
            "title": { "type": "string" },
            "priority": { "enum": ["low", "medium", "high"] }
        }
    })
}

fn public_todo_json() -> String {
    serde_json::to_string(&json!({
        "title": "Pay invoice",
        "priority": "high",
    }))
    .expect("public todo JSON serializes")
}
