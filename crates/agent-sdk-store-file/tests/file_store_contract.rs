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
    EffectIntent, EffectKind, EntityKind, EntityRef, EventArchive, EventEnvelope, EventFamily,
    EventFrame, EventId, EventKind, ExecutorRef, IdempotencyKey, JournalCursor, JournalRecord,
    JournalRecordBase, MessageId, MessageReceipt, MessageStatus, PackageSidecarRef, PolicyKind,
    PolicyRef, PrivacyClass, ProviderArgumentStore, RetentionClass, RunAddress, RunCheckpoint,
    RunId, RunJournal, RunJournalReader, RunMessage, SourceKind, SourceRef, ToolCallId,
    ToolExecutionStore, ToolExecutionStoreRecord, TraceId,
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
use agent_sdk_store_file::{
    FileAgentPoolStore, FileCheckpointStore, FileContentStore, FileEventArchive,
    FileProviderArgumentStore, FileRunJournal, FileStoreBundle, FileToolExecutionStore,
};

#[test]
fn file_journal_rehydrates_records_after_restart() -> Result<(), AgentError> {
    let root = temp_root("journal");
    let journal = FileRunJournal::new(&root);
    journal.append(journal_record(1, "journal.record.file.1"))?;
    journal.append(journal_record(2, "journal.record.file.2"))?;

    let restarted = FileRunJournal::new(&root);
    let records = restarted.records_for_run(&RunId::new("run.file.store"))?;
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].record_id, "journal.record.file.1");
    assert_eq!(records[1].journal_seq, 2);
    let after = restarted.records_after(&RunId::new("run.file.store"), 1)?;
    assert_eq!(after.len(), 1);

    drop(fs::remove_dir_all(root));
    Ok(())
}

#[test]
fn file_content_store_resolves_redacted_and_raw_content() -> Result<(), AgentError> {
    let root = temp_root("content");
    let store = FileContentStore::new(&root);
    let content_ref = stored_content_ref();
    store
        .put_content(&content_ref, b"hello durable content".to_vec())
        .map_err(|error| error.to_agent_error())?;

    let redacted = store
        .resolve(
            ContentResolveRequest::new(content_ref.clone()),
            ContentResolutionPolicy::redacted_context(
                EntityRef::run(RunId::new("run.file.store")),
                DestinationRef::with_kind(DestinationKind::Host, "destination.file.content"),
                policy_ref(),
            ),
        )
        .map_err(|error| error.to_agent_error())?;
    assert_eq!(redacted.bytes, None);
    assert!(!redacted.raw_content_included);

    let raw = store
        .resolve(
            ContentResolveRequest::new(content_ref),
            ContentResolutionPolicy::raw_context(
                EntityRef::run(RunId::new("run.file.store")),
                DestinationRef::with_kind(DestinationKind::Host, "destination.file.content"),
                policy_ref(),
                1024,
            ),
        )
        .map_err(|error| error.to_agent_error())?;
    assert_eq!(raw.bytes, Some(b"hello durable content".to_vec()));
    assert!(raw.raw_content_included);

    drop(fs::remove_dir_all(root));
    Ok(())
}

#[test]
fn file_provider_arguments_persist_raw_payloads_behind_content_ref() -> Result<(), AgentError> {
    let root = temp_root("provider-args");
    let store = FileProviderArgumentStore::new(&root);
    let content_ref = store
        .store_provider_arguments(
            "provider.openai.responses",
            "call_1",
            &CanonicalToolName::new("workspace_read"),
            r#"{"path":"README.md"}"#,
        )?
        .expect("provider argument content ref");

    assert!(
        content_ref
            .as_str()
            .starts_with("content.provider_arguments.")
    );
    assert!(
        root.join("provider_arguments").exists(),
        "raw payload is written outside journals/events and exposed only by ref"
    );
    let loaded = store.load_provider_arguments_json(&content_ref)?;
    assert_eq!(loaded["path"], "README.md");

    drop(fs::remove_dir_all(root));
    Ok(())
}

#[test]
fn file_checkpoints_load_latest_and_prune_old_entries() -> Result<(), AgentError> {
    let root = temp_root("checkpoint");
    let store = FileCheckpointStore::new(&root);
    store.save(checkpoint("checkpoint.file.1", 1), 2)?;
    store.save(checkpoint("checkpoint.file.2", 2), 2)?;

    assert_eq!(
        store
            .load_latest(&RunId::new("run.file.store"))?
            .expect("latest")
            .checkpoint_id,
        "checkpoint.file.2"
    );

    let report = store.prune(
        &RunId::new("run.file.store"),
        agent_sdk_core::CheckpointPrunePolicy {
            prune_covered_before: 2,
            preserve_latest_terminal: true,
        },
    )?;
    assert_eq!(report.pruned_count, 1);
    assert_eq!(
        store
            .load_latest(&RunId::new("run.file.store"))?
            .expect("latest after prune")
            .checkpoint_id,
        "checkpoint.file.2"
    );

    drop(fs::remove_dir_all(root));
    Ok(())
}

#[test]
fn file_event_archive_replays_filtered_frames_after_cursor() -> Result<(), AgentError> {
    let root = temp_root("event-archive");
    let archive = FileEventArchive::new(&root);
    let first = archive.append_frame(event_frame(1, EventKind::RunStarted))?;
    archive.append_frame(event_frame(2, EventKind::RunCompleted))?;

    let filter = agent_sdk_core::EventFilter::terminal_run_events().compile()?;
    let frames = archive
        .replay_filtered_from_cursor(filter, first)?
        .collect::<Vec<_>>();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].event.envelope.event_kind, EventKind::RunCompleted);

    drop(fs::remove_dir_all(root));
    Ok(())
}

#[test]
fn file_agent_pool_store_rehydrates_snapshot_and_watch_records() -> Result<(), AgentError> {
    let root = temp_root("agent-pool");
    let store = FileAgentPoolStore::new(&root);
    let pool_id = agent_sdk_core::AgentPoolId::new("pool.file.store");
    let config = AgentPoolStoreConfig {
        message_policy: AgentPoolMessagePolicy::bounded_defaults(),
        wake_policy: AgentPoolWakePolicy::safe_defaults(),
        policy_refs: Vec::new(),
    };
    store.open_pool(pool_id.clone(), config)?;
    store.record_pool_created(&pool_id)?;
    store.join_member(
        &pool_id,
        AgentPoolMember::new(
            RunId::new("run.file.store.a"),
            AgentId::new("agent.file.store.a"),
        ),
    )?;
    store.record_message(
        &pool_id,
        run_message(),
        MessageReceipt {
            message_id: MessageId::new("message.file.store"),
            status: MessageStatus::Delivered,
            delivered_to: vec![RunId::new("run.file.store.a")],
            journal_cursor: None,
        },
    )?;

    let restarted = FileAgentPoolStore::new(&root);
    let snapshot = restarted.snapshot(&pool_id)?;
    assert!(snapshot.created);
    assert_eq!(snapshot.members.len(), 1);
    assert_eq!(snapshot.messages.len(), 1);

    let records = restarted.watch(&pool_id, None)?.collect::<Vec<_>>();
    assert_eq!(records.len(), 4);

    drop(fs::remove_dir_all(root));
    Ok(())
}

#[test]
fn file_tool_execution_store_projects_journaled_tool_records() -> Result<(), AgentError> {
    let root = temp_root("tool-execution");
    let store = FileToolExecutionStore::new(&root);
    let journal_record = tool_journal_record(2, "journal.tool.file.2");
    let projection = ToolExecutionStoreRecord::from_journal_record(
        &journal_record,
        Some(JournalCursor::new("journal.2")),
    )
    .expect("tool projection");

    let cursor = store.put_tool_execution_record(projection.clone())?;
    assert_eq!(cursor.sequence, 1);

    let run_records = store.records_for_run(&RunId::new("run.file.store"))?;
    assert_eq!(run_records, vec![projection.clone()]);
    assert_eq!(
        store
            .record_for_tool_call(
                &RunId::new("run.file.store"),
                &ToolCallId::new("tool.call.file.store")
            )?
            .expect("tool call record"),
        projection.clone()
    );
    assert_eq!(
        store
            .records_for_idempotency_key(&IdempotencyKey::new("idem.file.store.tool"))?
            .len(),
        1
    );
    assert_eq!(
        store
            .records_for_effect_id(&EffectId::new("effect.file.store.tool"))?
            .len(),
        1
    );
    assert_eq!(
        store
            .records_after_journal_seq(&RunId::new("run.file.store"), 1)?
            .len(),
        1
    );
    assert!(
        store
            .records_after_journal_seq(&RunId::new("run.file.store"), 2)?
            .is_empty()
    );
    assert_eq!(
        store
            .records_in_journal_cursor_range(
                &RunId::new("run.file.store"),
                Some(&JournalCursor::new("journal.1")),
                Some(&JournalCursor::new("journal.2")),
            )?
            .len(),
        1
    );
    assert!(
        store
            .records_in_journal_cursor_range(
                &RunId::new("run.file.store"),
                Some(&JournalCursor::new("journal.2")),
                None,
            )?
            .is_empty()
    );

    drop(fs::remove_dir_all(root));
    Ok(())
}

#[test]
fn file_store_bundle_uses_one_root_for_all_adapters() {
    let root = temp_root("bundle");
    let bundle = FileStoreBundle::new(&root);
    assert_eq!(bundle.root(), root.as_path());
    let _journal = bundle.journal();
    let _checkpoints = bundle.checkpoints();
    let _content = bundle.content();
    let _archive = bundle.event_archive();
    let _provider_arguments = bundle.provider_arguments();
    let _agent_pool = bundle.agent_pool();
    let _tool_execution = bundle.tool_execution();
    drop(fs::remove_dir_all(root));
}

fn journal_record(journal_seq: u64, record_id: &str) -> JournalRecord {
    let mut intent = EffectIntent::new(
        EffectId::new("effect.file.store"),
        EffectKind::ToolExecution,
        EntityRef::new(EntityKind::ToolCall, "tool.call.file.store"),
        SourceRef::with_kind(SourceKind::Sdk, "source.file.store"),
        "execute file store test tool",
    );
    intent.destination = Some(DestinationRef::with_kind(
        DestinationKind::Tool,
        "destination.file.store.tool",
    ));
    intent.idempotency_key = Some(IdempotencyKey::new("idem.file.store.tool"));

    let mut base = JournalRecordBase::new(
        journal_seq,
        record_id,
        RunId::new("run.file.store"),
        AgentId::new("agent.file.store"),
        SourceRef::with_kind(SourceKind::Sdk, "source.file.store"),
    );
    base.timestamp_millis = 1_780_000_000_000 + journal_seq;
    JournalRecord::effect_intent(base, intent)
}

fn tool_journal_record(journal_seq: u64, record_id: &str) -> JournalRecord {
    let effect_id = EffectId::new("effect.file.store.tool");
    let mut intent = EffectIntent::new(
        effect_id,
        EffectKind::ToolExecution,
        EntityRef::new(EntityKind::ToolCall, "tool.call.file.store"),
        SourceRef::with_kind(SourceKind::Sdk, "source.file.store"),
        "execute file store test tool",
    );
    intent.destination = Some(DestinationRef::with_kind(
        DestinationKind::Tool,
        "destination.file.store.tool",
    ));
    intent.idempotency_key = Some(IdempotencyKey::new("idem.file.store.tool"));

    let record = ToolCallRecord::requested(ToolCallRecordParams {
        tool_call_id: ToolCallId::new("tool.call.file.store"),
        run_id: RunId::new("run.file.store"),
        turn_id: None,
        capability_id: CapabilityId::new("cap.file.store.workspace_read"),
        canonical_tool_name: CanonicalToolName::new("workspace_read"),
        namespace: CapabilityNamespace::new("tool.workspace_read"),
        source: SourceRef::with_kind(SourceKind::Sdk, "source.file.store"),
        destination: DestinationRef::with_kind(
            DestinationKind::Tool,
            "destination.file.store.tool",
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
        retention: RetentionClass::Durable,
        requested_args_refs: vec![agent_sdk_core::domain::ContentRef::new(
            "content.args.file.store",
        )],
        redacted_args_summary: "read README".to_string(),
        idempotency_key: Some(IdempotencyKey::new("idem.file.store.tool")),
    })
    .with_intent(intent);

    let mut base = JournalRecordBase::new(
        journal_seq,
        record_id,
        RunId::new("run.file.store"),
        AgentId::new("agent.file.store"),
        SourceRef::with_kind(SourceKind::Sdk, "source.file.store"),
    );
    base.timestamp_millis = 1_780_000_000_000 + journal_seq;
    tool_call_journal_record(base, record, "tool_intent_recorded")
}

fn stored_content_ref() -> StoredContentRef {
    StoredContentRef::new(
        ContentId::new("content.file.store"),
        ContentVersion::new("v1"),
        ContentKind::Text,
        ContentScope::Run,
        EntityRef::run(RunId::new("run.file.store")),
        SourceRef::with_kind(SourceKind::Host, "source.file.content"),
        AdapterRef::new("adapter.file.content"),
        "stored file content",
    )
}

fn checkpoint(checkpoint_id: &str, covers_journal_seq: u64) -> RunCheckpoint {
    RunCheckpoint {
        checkpoint_id: checkpoint_id.to_string(),
        run_id: RunId::new("run.file.store"),
        checkpoint_seq: covers_journal_seq,
        covers_journal_seq,
        loop_state: "running".to_string(),
        turn_id: None,
        attempt_id: None,
        runtime_package_fingerprint: "sha256:file-store-checkpoint".to_string(),
        pending_side_effects: Vec::new(),
        pending_approvals: Vec::new(),
        content_ref_manifest: Vec::new(),
        state_hash: format!("sha256:file-store-{covers_journal_seq}"),
        created_at_millis: 1_780_000_000_000 + covers_journal_seq,
        writer_id: "writer.file.store".to_string(),
    }
}

fn event_frame(seq: u64, kind: EventKind) -> EventFrame {
    let run_id = RunId::new("run.file.store");
    let event = agent_sdk_core::AgentEvent::with_redacted_summary(
        EventEnvelope {
            schema_version: EVENT_SCHEMA_VERSION,
            event_id: EventId::new(format!("event.file.store.{seq}")),
            event_seq: seq,
            event_family: EventFamily::Run,
            event_kind: kind,
            payload_schema_version: 1,
            timestamp: "2026-06-07T00:00:00Z".to_string(),
            recorded_at: "2026-06-07T00:00:00Z".to_string(),
            run_id: run_id.clone(),
            session_id: None,
            agent_id: AgentId::new("agent.file.store"),
            turn_id: None,
            attempt_id: None,
            message_id: None,
            context_item_id: None,
            trace_id: TraceId::new("trace.file.store"),
            span_id: SpanId::new(format!("span.file.store.{seq}")),
            parent_event_id: None,
            caused_by: None,
            subject_ref: EntityRef::run(run_id),
            related_refs: Vec::new(),
            causal_refs: Vec::new(),
            correlation: EventCorrelation::default(),
            tags: vec![EventTag::new("store:file")],
            source: SourceRef::with_kind(SourceKind::Sdk, "source.file.store"),
            destination: Some(DestinationRef::with_kind(
                DestinationKind::EventStream,
                "destination.file.archive",
            )),
            policy_refs: vec![policy_ref()],
            journal_cursor: None,
            state_before: None,
            state_after: None,
            delivery_semantics: EventDeliverySemantics::JournalBacked,
            privacy: PrivacyClass::ContentRefsOnly,
            content_capture: EventContentCaptureMode::Off,
            redaction_policy_id: "redaction.file.store".to_string(),
            runtime_package_fingerprint: "sha256:file-store-event".to_string(),
        },
        "file store event",
    );
    EventFrame {
        cursor: event.envelope.cursor(EventStreamScope::All),
        event,
        archive_cursor: None,
        overflow: None,
    }
}

fn run_message() -> RunMessage {
    RunMessage::new(
        MessageId::new("message.file.store"),
        RunId::new("run.file.store.a"),
        RunAddress::run(RunId::new("run.file.store.a")),
        agent_sdk_core::domain::ContentRef::new("content.message.file.store"),
        IdempotencyKey::new("idem.message.file.store"),
    )
}

fn policy_ref() -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::RuntimePackage, "policy.file.store")
}

fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "agent-sdk-store-file-{label}-{}-{nanos}",
        std::process::id()
    ))
}
