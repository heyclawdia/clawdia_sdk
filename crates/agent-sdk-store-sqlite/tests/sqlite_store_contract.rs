use std::{
    fs,
    path::PathBuf,
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
use agent_sdk_store_sqlite::{
    SqliteAgentPoolStore, SqliteCheckpointStore, SqliteContentStore, SqliteEventArchive,
    SqliteProviderArgumentStore, SqliteRunJournal, SqliteStoreBundle, SqliteToolExecutionStore,
};

#[test]
fn sqlite_store_bundle_maps_all_durable_surfaces() -> Result<(), AgentError> {
    let db_path = temp_db("bundle");
    let bundle = SqliteStoreBundle::open(&db_path)?;

    let _journal: SqliteRunJournal = bundle.journal()?;
    let _checkpoints: SqliteCheckpointStore = bundle.checkpoints()?;
    let _content: SqliteContentStore = bundle.content()?;
    let _archive: SqliteEventArchive = bundle.event_archive()?;
    let _provider_arguments: SqliteProviderArgumentStore = bundle.provider_arguments()?;
    let _agent_pool: SqliteAgentPoolStore = bundle.agent_pool()?;
    let _tool_execution: SqliteToolExecutionStore = bundle.tool_execution()?;

    drop(fs::remove_file(db_path));
    Ok(())
}

#[test]
fn sqlite_store_round_trips_core_truth_and_projection_surfaces() -> Result<(), AgentError> {
    let db_path = temp_db("contract");
    let bundle = SqliteStoreBundle::open(&db_path)?;
    let journal = bundle.journal()?;
    let checkpoints = bundle.checkpoints()?;
    let content = bundle.content()?;
    let archive = bundle.event_archive()?;
    let provider_arguments = bundle.provider_arguments()?;
    let agent_pool = bundle.agent_pool()?;
    let tool_execution = bundle.tool_execution()?;

    journal.append(journal_record(1, "journal.sqlite.1"))?;
    assert_eq!(
        journal.records_for_run(&RunId::new("run.sqlite.store"))?[0].record_id,
        "journal.sqlite.1"
    );

    content
        .put_content(&stored_content_ref(), b"hello sqlite content".to_vec())
        .map_err(|error| error.to_agent_error())?;
    let resolved = content
        .resolve(
            ContentResolveRequest::new(stored_content_ref()),
            ContentResolutionPolicy::raw_context(
                EntityRef::run(RunId::new("run.sqlite.store")),
                DestinationRef::with_kind(DestinationKind::Host, "destination.sqlite.content"),
                policy_ref(),
                1024,
            ),
        )
        .map_err(|error| error.to_agent_error())?;
    assert_eq!(resolved.bytes, Some(b"hello sqlite content".to_vec()));

    let provider_args_ref = provider_arguments
        .store_provider_arguments(
            "provider.fake",
            "call.sqlite.1",
            &CanonicalToolName::new("workspace_read"),
            r#"{"path":"README.md"}"#,
        )?
        .expect("provider arguments ref");
    assert_eq!(
        provider_arguments.load_provider_arguments_json(&provider_args_ref)?["path"],
        "README.md"
    );

    checkpoints.save(checkpoint("checkpoint.sqlite.1", 1), 1)?;
    assert_eq!(
        checkpoints
            .load_latest(&RunId::new("run.sqlite.store"))?
            .expect("latest checkpoint")
            .checkpoint_id,
        "checkpoint.sqlite.1"
    );

    let cursor = archive.append_frame(event_frame(1, EventKind::RunStarted))?;
    archive.append_frame(event_frame(2, EventKind::RunCompleted))?;
    let frames = archive.frames_after(Some(cursor))?;
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].event.envelope.event_kind, EventKind::RunCompleted);

    let pool_id = agent_sdk_core::AgentPoolId::new("pool.sqlite.store");
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
            RunId::new("run.sqlite.store.member"),
            AgentId::new("agent.sqlite.store.member"),
        ),
    )?;
    assert_eq!(agent_pool.snapshot(&pool_id)?.members.len(), 1);

    let tool_record = tool_journal_record(2, "journal.sqlite.tool.2");
    let projection = ToolExecutionStoreRecord::from_journal_record(
        &tool_record,
        Some(JournalCursor::new("journal.2")),
    )
    .expect("tool projection");
    tool_execution.put_tool_execution_record(projection.clone())?;
    assert_eq!(
        tool_execution
            .record_for_tool_call(
                &RunId::new("run.sqlite.store"),
                &ToolCallId::new("tool.call.sqlite.store")
            )?
            .expect("tool call record"),
        projection.clone()
    );
    assert_eq!(
        tool_execution
            .records_for_idempotency_key(&IdempotencyKey::new("idem.sqlite.store.tool"))?
            .len(),
        1
    );
    assert_eq!(
        tool_execution
            .records_for_effect_id(&EffectId::new("effect.sqlite.store.tool"))?
            .len(),
        1
    );
    assert_eq!(
        tool_execution
            .records_in_journal_cursor_range(
                &RunId::new("run.sqlite.store"),
                Some(&JournalCursor::new("journal.1")),
                Some(&JournalCursor::new("journal.2")),
            )?
            .len(),
        1
    );
    assert!(
        tool_execution
            .records_in_journal_cursor_range(
                &RunId::new("run.sqlite.store"),
                Some(&JournalCursor::new("journal.2")),
                None,
            )?
            .is_empty()
    );

    drop(fs::remove_file(db_path));
    Ok(())
}

fn journal_record(journal_seq: u64, record_id: &str) -> JournalRecord {
    let mut intent = EffectIntent::new(
        EffectId::new("effect.sqlite.store"),
        EffectKind::ToolExecution,
        EntityRef::new(EntityKind::ToolCall, "tool.call.sqlite.store"),
        SourceRef::with_kind(SourceKind::Sdk, "source.sqlite.store"),
        "execute sqlite store test tool",
    );
    intent.destination = Some(DestinationRef::with_kind(
        DestinationKind::Tool,
        "destination.sqlite.store.tool",
    ));
    intent.idempotency_key = Some(IdempotencyKey::new("idem.sqlite.store.tool"));

    let mut base = JournalRecordBase::new(
        journal_seq,
        record_id,
        RunId::new("run.sqlite.store"),
        AgentId::new("agent.sqlite.store"),
        SourceRef::with_kind(SourceKind::Sdk, "source.sqlite.store"),
    );
    base.timestamp_millis = 1_780_000_000_000 + journal_seq;
    JournalRecord::effect_intent(base, intent)
}

fn tool_journal_record(journal_seq: u64, record_id: &str) -> JournalRecord {
    let mut intent = EffectIntent::new(
        EffectId::new("effect.sqlite.store.tool"),
        EffectKind::ToolExecution,
        EntityRef::new(EntityKind::ToolCall, "tool.call.sqlite.store"),
        SourceRef::with_kind(SourceKind::Sdk, "source.sqlite.store"),
        "execute sqlite store test tool",
    );
    intent.destination = Some(DestinationRef::with_kind(
        DestinationKind::Tool,
        "destination.sqlite.store.tool",
    ));
    intent.idempotency_key = Some(IdempotencyKey::new("idem.sqlite.store.tool"));

    let record = ToolCallRecord::requested(ToolCallRecordParams {
        tool_call_id: ToolCallId::new("tool.call.sqlite.store"),
        run_id: RunId::new("run.sqlite.store"),
        turn_id: None,
        capability_id: CapabilityId::new("cap.sqlite.store.workspace_read"),
        canonical_tool_name: CanonicalToolName::new("workspace_read"),
        namespace: CapabilityNamespace::new("tool.workspace_read"),
        source: SourceRef::with_kind(SourceKind::Sdk, "source.sqlite.store"),
        destination: DestinationRef::with_kind(
            DestinationKind::Tool,
            "destination.sqlite.store.tool",
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
            "content.args.sqlite.store",
        )],
        redacted_args_summary: "read README".to_string(),
        idempotency_key: Some(IdempotencyKey::new("idem.sqlite.store.tool")),
    })
    .with_intent(intent);

    let mut base = JournalRecordBase::new(
        journal_seq,
        record_id,
        RunId::new("run.sqlite.store"),
        AgentId::new("agent.sqlite.store"),
        SourceRef::with_kind(SourceKind::Sdk, "source.sqlite.store"),
    );
    base.timestamp_millis = 1_780_000_000_000 + journal_seq;
    tool_call_journal_record(base, record, "tool_intent_recorded")
}

fn stored_content_ref() -> StoredContentRef {
    StoredContentRef::new(
        ContentId::new("content.sqlite.store"),
        ContentVersion::new("v1"),
        ContentKind::Text,
        ContentScope::Run,
        EntityRef::run(RunId::new("run.sqlite.store")),
        SourceRef::with_kind(SourceKind::Host, "source.sqlite.content"),
        AdapterRef::new("adapter.sqlite.content"),
        "stored sqlite content",
    )
}

fn checkpoint(checkpoint_id: &str, covers_journal_seq: u64) -> RunCheckpoint {
    RunCheckpoint {
        checkpoint_id: checkpoint_id.to_string(),
        run_id: RunId::new("run.sqlite.store"),
        checkpoint_seq: covers_journal_seq,
        covers_journal_seq,
        loop_state: "running".to_string(),
        turn_id: None,
        attempt_id: None,
        runtime_package_fingerprint: "sha256:sqlite-store-checkpoint".to_string(),
        pending_side_effects: Vec::new(),
        pending_approvals: Vec::new(),
        content_ref_manifest: Vec::new(),
        state_hash: format!("sha256:sqlite-store-{covers_journal_seq}"),
        created_at_millis: 1_780_000_000_000 + covers_journal_seq,
        writer_id: "writer.sqlite.store".to_string(),
    }
}

fn event_frame(seq: u64, kind: EventKind) -> EventFrame {
    let run_id = RunId::new("run.sqlite.store");
    let event = agent_sdk_core::AgentEvent::with_redacted_summary(
        EventEnvelope {
            schema_version: EVENT_SCHEMA_VERSION,
            event_id: EventId::new(format!("event.sqlite.store.{seq}")),
            event_seq: seq,
            event_family: EventFamily::Run,
            event_kind: kind,
            payload_schema_version: 1,
            timestamp: "2026-06-08T00:00:00Z".to_string(),
            recorded_at: "2026-06-08T00:00:00Z".to_string(),
            run_id: run_id.clone(),
            session_id: None,
            agent_id: AgentId::new("agent.sqlite.store"),
            turn_id: None,
            attempt_id: None,
            message_id: None,
            context_item_id: None,
            trace_id: TraceId::new("trace.sqlite.store"),
            span_id: SpanId::new(format!("span.sqlite.store.{seq}")),
            parent_event_id: None,
            caused_by: None,
            subject_ref: EntityRef::run(run_id),
            related_refs: Vec::new(),
            causal_refs: Vec::new(),
            correlation: EventCorrelation::default(),
            tags: vec![EventTag::new("store:sqlite")],
            source: SourceRef::with_kind(SourceKind::Sdk, "source.sqlite.store"),
            destination: Some(DestinationRef::with_kind(
                DestinationKind::EventStream,
                "destination.sqlite.archive",
            )),
            policy_refs: vec![policy_ref()],
            journal_cursor: None,
            state_before: None,
            state_after: None,
            delivery_semantics: EventDeliverySemantics::JournalBacked,
            privacy: PrivacyClass::ContentRefsOnly,
            content_capture: EventContentCaptureMode::Off,
            redaction_policy_id: "redaction.sqlite.store".to_string(),
            runtime_package_fingerprint: "sha256:sqlite-store-event".to_string(),
        },
        "sqlite store event",
    );
    EventFrame {
        cursor: event.envelope.cursor(EventStreamScope::All),
        event,
        archive_cursor: None,
        overflow: None,
    }
}

fn policy_ref() -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::RuntimePackage, "policy.sqlite.store")
}

fn temp_db(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "agent-sdk-store-sqlite-{label}-{}-{nanos}.sqlite3",
        std::process::id()
    ))
}
