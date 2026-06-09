use std::{
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use agent_sdk_core::{
    AdapterRef, AgentError, AgentId, AgentPoolMember, AgentPoolMessagePolicy, AgentPoolStore,
    AgentPoolStoreConfig, CanonicalToolName, CapabilityId, CapabilityNamespace, CheckpointStore,
    ContentId, ContentKind, ContentResolutionPolicy, ContentResolveRequest, ContentResolver,
    ContentScope, ContentStore, ContentVersion, DestinationKind, DestinationRef, EffectId,
    EffectIntent, EffectKind, EntityKind, EntityRef, EventArchiveReader, EventEnvelope,
    EventFamily, EventFrame, EventId, EventKind, ExecutorRef, IdempotencyKey, JournalCursor,
    JournalRecord, JournalRecordBase, PackageSidecarRef, PolicyKind, PolicyRef, PrivacyClass,
    ProviderArgumentStore, RetentionClass, RunCheckpoint, RunId, RunJournal, RunJournalReader,
    SourceKind, SourceRef, ToolCallId, ToolExecutionStore, ToolExecutionStoreRecord, TraceId,
    agent_pool::AgentPoolWakePolicy,
    content::ContentRef as StoredContentRef,
    event::{
        ContentCaptureMode as EventContentCaptureMode, EVENT_SCHEMA_VERSION, EventCorrelation,
        EventDeliverySemantics, EventStreamScope, EventTag,
    },
    ids::SpanId,
    policy::{EffectClass, RiskClass},
    tool_records::{ToolCallRecord, ToolCallRecordParams, tool_call_journal_record},
};
use agent_sdk_store_postgres::{
    PostgresSqlRequest, PostgresSqlResponse, PostgresSqlTransport, PostgresStoreBundle,
    PostgresStoreClient, PostgresStoreConfig,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde_json::{Value, json};

#[test]
fn postgres_store_bundle_maps_all_durable_surfaces() {
    let transport = Arc::new(ScriptedPostgresTransport::default());
    let bundle = PostgresStoreBundle::new(PostgresStoreClient::new(
        PostgresStoreConfig::new("agent_sdk", "test"),
        transport,
    ));

    let _journal = bundle.journal();
    let _checkpoints = bundle.checkpoints();
    let _content = bundle.content();
    let _archive = bundle.event_archive();
    let _provider_arguments = bundle.provider_arguments();
    let _agent_pool = bundle.agent_pool();
    let _tool_execution = bundle.tool_execution();
}

#[test]
fn postgres_store_surfaces_use_host_owned_sql_transport() -> Result<(), AgentError> {
    let journal_fixture = journal_record(1, "journal.postgres.1");
    let content_ref = stored_content_ref();
    let checkpoint_fixture = checkpoint("checkpoint.postgres.1", 1);
    let second_frame = event_frame(2, EventKind::RunCompleted);
    let projection = ToolExecutionStoreRecord::from_journal_record(
        &tool_journal_record(2, "journal.postgres.tool.2"),
        Some(JournalCursor::new("journal.2")),
    )
    .expect("tool projection");
    let transport = Arc::new(ScriptedPostgresTransport::with_responses([
        PostgresSqlResponse::affected(1),
        PostgresSqlResponse::rows([json!({"record_json": journal_fixture})]),
        PostgresSqlResponse::affected(1),
        PostgresSqlResponse::rows([json!({
            "content_json": content_ref,
            "bytes_base64": BASE64.encode(b"hello postgres"),
        })]),
        PostgresSqlResponse::affected(1),
        PostgresSqlResponse::rows([json!({"raw_arguments": r#"{"path":"README.md"}"#})]),
        PostgresSqlResponse::affected(1),
        PostgresSqlResponse::rows([json!({"checkpoint_json": checkpoint_fixture})]),
        PostgresSqlResponse::rows([json!({"archive_seq": 1})]),
        PostgresSqlResponse::rows([json!({"frame_json": second_frame})]),
        PostgresSqlResponse::rows([]),
        PostgresSqlResponse::rows([json!({"seq": 1})]),
        PostgresSqlResponse::rows([json!({"seq": 2})]),
        PostgresSqlResponse::rows([json!({"next_sequence": 3})]),
        PostgresSqlResponse::affected(1),
        PostgresSqlResponse::rows([json!({"record_json": projection})]),
        PostgresSqlResponse::rows([json!({"record_json": projection})]),
        PostgresSqlResponse::rows([json!({"record_json": projection})]),
        PostgresSqlResponse::rows([json!({"record_json": projection})]),
    ]));
    let bundle = PostgresStoreBundle::new(PostgresStoreClient::new(
        PostgresStoreConfig::new("agent_sdk", "scope.test"),
        transport.clone(),
    ));

    let journal = bundle.journal();
    journal.append(journal_record(1, "journal.postgres.1"))?;
    assert_eq!(
        journal.records_for_run(&RunId::new("run.postgres.store"))?[0].record_id,
        "journal.postgres.1"
    );

    let content = bundle.content();
    content
        .put_content(&stored_content_ref(), b"hello postgres".to_vec())
        .map_err(|error| error.to_agent_error())?;
    let resolved = content
        .resolve(
            ContentResolveRequest::new(stored_content_ref()),
            ContentResolutionPolicy::raw_context(
                EntityRef::run(RunId::new("run.postgres.store")),
                DestinationRef::with_kind(DestinationKind::Host, "destination.postgres.content"),
                policy_ref(),
                1024,
            ),
        )
        .map_err(|error| error.to_agent_error())?;
    assert_eq!(resolved.bytes, Some(b"hello postgres".to_vec()));

    let provider_arguments = bundle.provider_arguments();
    let provider_args_ref = provider_arguments
        .store_provider_arguments(
            "provider.fake",
            "call.postgres.1",
            &CanonicalToolName::new("workspace_read"),
            r#"{"path":"README.md"}"#,
        )?
        .expect("provider arguments ref");
    assert_eq!(
        provider_arguments.load_provider_arguments_json(&provider_args_ref)?["path"],
        "README.md"
    );

    let checkpoints = bundle.checkpoints();
    checkpoints.save(checkpoint("checkpoint.postgres.1", 1), 1)?;
    assert_eq!(
        checkpoints
            .load_latest(&RunId::new("run.postgres.store"))?
            .expect("latest checkpoint")
            .checkpoint_id,
        "checkpoint.postgres.1"
    );

    let archive = bundle.event_archive();
    let cursor = archive.append_frame(event_frame(1, EventKind::RunStarted))?;
    assert_eq!(archive.frames_after(Some(cursor))?.len(), 1);

    let agent_pool = bundle.agent_pool();
    let pool_id = agent_sdk_core::AgentPoolId::new("pool.postgres.store");
    agent_pool.open_pool(
        pool_id.clone(),
        AgentPoolStoreConfig {
            message_policy: AgentPoolMessagePolicy::bounded_defaults(),
            wake_policy: AgentPoolWakePolicy::safe_defaults(),
            policy_refs: Vec::new(),
        },
    )?;
    agent_pool.record_pool_created(&pool_id)?;
    agent_pool.join_member(
        &pool_id,
        AgentPoolMember::new(
            RunId::new("run.postgres.store.member"),
            AgentId::new("agent.postgres.store.member"),
        ),
    )?;
    assert_eq!(agent_pool.next_event_sequence(&pool_id)?, 3);

    let tool_execution = bundle.tool_execution();
    let projection = ToolExecutionStoreRecord::from_journal_record(
        &tool_journal_record(2, "journal.postgres.tool.2"),
        Some(JournalCursor::new("journal.2")),
    )
    .expect("tool projection");
    tool_execution.put_tool_execution_record(projection.clone())?;
    assert_eq!(
        tool_execution
            .record_for_tool_call(
                &RunId::new("run.postgres.store"),
                &ToolCallId::new("tool.call.postgres.store")
            )?
            .expect("tool call record"),
        projection.clone()
    );
    assert_eq!(
        tool_execution
            .records_for_idempotency_key(&IdempotencyKey::new("idem.postgres.store.tool"))?
            .len(),
        1
    );
    assert_eq!(
        tool_execution
            .records_for_effect_id(&EffectId::new("effect.postgres.store.tool"))?
            .len(),
        1
    );
    assert_eq!(
        tool_execution
            .records_in_journal_cursor_range(
                &RunId::new("run.postgres.store"),
                Some(&JournalCursor::new("journal.1")),
                Some(&JournalCursor::new("journal.2")),
            )?
            .len(),
        1
    );

    let requests = transport.requests();
    assert!(
        requests
            .iter()
            .any(|request| request.statement.contains("append_journal_record"))
    );
    assert!(
        requests
            .iter()
            .any(|request| request.statement.contains("agent_sdk_tool_execution"))
    );
    assert!(
        requests
            .iter()
            .all(|request| !request.statement.contains("on conflict do update"))
    );
    assert!(requests.iter().any(|request| {
        request
            .statement
            .contains("on conflict (store_scope, run_id, checkpoint_id) do update set")
    }));
    assert!(requests.iter().any(|request| {
        request
            .statement
            .contains("on conflict (store_scope, content_id) do update set")
    }));
    assert!(requests.iter().any(|request| {
        request
            .statement
            .contains("on conflict (store_scope, content_ref) do update set")
    }));
    assert!(requests.iter().any(|request| {
        request
            .statement
            .contains("on conflict (store_scope, run_id, tool_call_id, journal_seq) do update set")
    }));
    assert!(requests.iter().all(|request| {
        request
            .params
            .first()
            .is_some_and(|param| param == &Value::String("scope.test".to_string()))
    }));
    assert_eq!(transport.remaining_responses(), 0);
    Ok(())
}

#[derive(Default)]
struct ScriptedPostgresTransport {
    responses: Mutex<Vec<PostgresSqlResponse>>,
    requests: Mutex<Vec<PostgresSqlRequest>>,
}

impl ScriptedPostgresTransport {
    fn with_responses(responses: impl IntoIterator<Item = PostgresSqlResponse>) -> Self {
        let mut responses = responses.into_iter().collect::<Vec<_>>();
        responses.reverse();
        Self {
            responses: Mutex::new(responses),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn requests(&self) -> Vec<PostgresSqlRequest> {
        self.requests.lock().expect("requests lock").clone()
    }

    fn remaining_responses(&self) -> usize {
        self.responses.lock().expect("responses lock").len()
    }
}

impl PostgresSqlTransport for ScriptedPostgresTransport {
    fn execute(&self, request: PostgresSqlRequest) -> Result<PostgresSqlResponse, AgentError> {
        self.requests.lock().expect("requests lock").push(request);
        self.responses
            .lock()
            .expect("responses lock")
            .pop()
            .ok_or_else(|| AgentError::contract_violation("scripted Postgres transport exhausted"))
    }
}

fn journal_record(journal_seq: u64, record_id: &str) -> JournalRecord {
    let mut intent = EffectIntent::new(
        EffectId::new("effect.postgres.store"),
        EffectKind::ToolExecution,
        EntityRef::new(EntityKind::ToolCall, "tool.call.postgres.store"),
        SourceRef::with_kind(SourceKind::Sdk, "source.postgres.store"),
        "execute postgres store test tool",
    );
    intent.destination = Some(DestinationRef::with_kind(
        DestinationKind::Tool,
        "destination.postgres.store.tool",
    ));
    intent.idempotency_key = Some(IdempotencyKey::new("idem.postgres.store.tool"));

    let mut base = JournalRecordBase::new(
        journal_seq,
        record_id,
        RunId::new("run.postgres.store"),
        AgentId::new("agent.postgres.store"),
        SourceRef::with_kind(SourceKind::Sdk, "source.postgres.store"),
    );
    base.timestamp_millis = now_millis() + journal_seq;
    JournalRecord::effect_intent(base, intent)
}

fn tool_journal_record(journal_seq: u64, record_id: &str) -> JournalRecord {
    let mut intent = EffectIntent::new(
        EffectId::new("effect.postgres.store.tool"),
        EffectKind::ToolExecution,
        EntityRef::new(EntityKind::ToolCall, "tool.call.postgres.store"),
        SourceRef::with_kind(SourceKind::Sdk, "source.postgres.store"),
        "execute postgres store test tool",
    );
    intent.destination = Some(DestinationRef::with_kind(
        DestinationKind::Tool,
        "destination.postgres.store.tool",
    ));
    intent.idempotency_key = Some(IdempotencyKey::new("idem.postgres.store.tool"));

    let record = ToolCallRecord::requested(ToolCallRecordParams {
        tool_call_id: ToolCallId::new("tool.call.postgres.store"),
        run_id: RunId::new("run.postgres.store"),
        turn_id: None,
        capability_id: CapabilityId::new("cap.postgres.store.workspace_read"),
        canonical_tool_name: CanonicalToolName::new("workspace_read"),
        namespace: CapabilityNamespace::new("tool.workspace_read"),
        source: SourceRef::with_kind(SourceKind::Sdk, "source.postgres.store"),
        destination: DestinationRef::with_kind(
            DestinationKind::Tool,
            "destination.postgres.store.tool",
        ),
        executor_ref: Some(ExecutorRef::new("executor.workspace_read.v1")),
        policy_refs: vec![policy_ref()],
        sidecar_refs: vec![PackageSidecarRef::new(
            "schema.workspace_read.v1",
            "json_schema",
            "v1",
        )],
        effect_class: EffectClass::Read,
        risk_class: RiskClass::Low,
        privacy: PrivacyClass::ContentRefsOnly,
        retention: RetentionClass::RunScoped,
        requested_args_refs: vec![agent_sdk_core::domain::ContentRef::new(
            "content.args.postgres.store",
        )],
        redacted_args_summary: "read README".to_string(),
        idempotency_key: Some(IdempotencyKey::new("idem.postgres.store.tool")),
    })
    .with_intent(intent);

    let mut base = JournalRecordBase::new(
        journal_seq,
        record_id,
        RunId::new("run.postgres.store"),
        AgentId::new("agent.postgres.store"),
        SourceRef::with_kind(SourceKind::Sdk, "source.postgres.store"),
    );
    base.timestamp_millis = now_millis() + journal_seq;
    tool_call_journal_record(base, record, "tool_intent_recorded")
}

fn stored_content_ref() -> StoredContentRef {
    StoredContentRef::new(
        ContentId::new("content.postgres.store"),
        ContentVersion::new("v1"),
        ContentKind::Text,
        ContentScope::Run,
        EntityRef::run(RunId::new("run.postgres.store")),
        SourceRef::with_kind(SourceKind::Host, "source.postgres.content"),
        AdapterRef::new("adapter.postgres.content"),
        "stored postgres content",
    )
}

fn checkpoint(checkpoint_id: &str, covers_journal_seq: u64) -> RunCheckpoint {
    RunCheckpoint {
        checkpoint_id: checkpoint_id.to_string(),
        run_id: RunId::new("run.postgres.store"),
        checkpoint_seq: covers_journal_seq,
        covers_journal_seq,
        loop_state: "running".to_string(),
        turn_id: None,
        attempt_id: None,
        runtime_package_fingerprint: "sha256:postgres-store-checkpoint".to_string(),
        pending_side_effects: Vec::new(),
        pending_approvals: Vec::new(),
        content_ref_manifest: Vec::new(),
        state_hash: format!("sha256:postgres-store-{covers_journal_seq}"),
        created_at_millis: now_millis() + covers_journal_seq,
        writer_id: "writer.postgres.store".to_string(),
    }
}

fn event_frame(seq: u64, kind: EventKind) -> EventFrame {
    let run_id = RunId::new("run.postgres.store");
    let event = agent_sdk_core::AgentEvent::with_redacted_summary(
        EventEnvelope {
            schema_version: EVENT_SCHEMA_VERSION,
            event_id: EventId::new(format!("event.postgres.store.{seq}")),
            event_seq: seq,
            event_family: EventFamily::Run,
            event_kind: kind,
            payload_schema_version: 1,
            timestamp: "2026-06-08T00:00:00Z".to_string(),
            recorded_at: "2026-06-08T00:00:00Z".to_string(),
            run_id: run_id.clone(),
            session_id: None,
            agent_id: AgentId::new("agent.postgres.store"),
            turn_id: None,
            attempt_id: None,
            message_id: None,
            context_item_id: None,
            trace_id: TraceId::new("trace.postgres.store"),
            span_id: SpanId::new(format!("span.postgres.store.{seq}")),
            parent_event_id: None,
            caused_by: None,
            subject_ref: EntityRef::run(run_id),
            related_refs: Vec::new(),
            causal_refs: Vec::new(),
            correlation: EventCorrelation::default(),
            tags: vec![EventTag::new("store:postgres")],
            source: SourceRef::with_kind(SourceKind::Sdk, "source.postgres.store"),
            destination: Some(DestinationRef::with_kind(
                DestinationKind::EventStream,
                "destination.postgres.archive",
            )),
            policy_refs: vec![policy_ref()],
            journal_cursor: None,
            state_before: None,
            state_after: None,
            delivery_semantics: EventDeliverySemantics::JournalBacked,
            privacy: PrivacyClass::ContentRefsOnly,
            content_capture: EventContentCaptureMode::Off,
            redaction_policy_id: "redaction.postgres.store".to_string(),
            runtime_package_fingerprint: "sha256:postgres-store-event".to_string(),
        },
        "postgres store event",
    );
    EventFrame {
        cursor: event.envelope.cursor(EventStreamScope::All),
        event,
        archive_cursor: None,
        overflow: None,
    }
}

fn policy_ref() -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::RuntimePackage, "policy.postgres.store")
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_millis() as u64
}
