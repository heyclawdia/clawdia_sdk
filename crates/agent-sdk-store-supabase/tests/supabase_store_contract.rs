use std::sync::{Arc, Mutex};

use agent_sdk_core::{
    AdapterRef, AgentError, AgentId, AgentPoolId, AgentPoolMessagePolicy, AgentPoolStore,
    AgentPoolStoreConfig, CanonicalToolName, ContentId, ContentKind, ContentScope, ContentStore,
    ContentVersion, DestinationKind, DestinationRef, EffectId, EffectIntent, EffectKind,
    EntityKind, EntityRef, IdempotencyKey, JournalRecord, JournalRecordBase, ProviderArgumentStore,
    RunId, RunJournal, RunJournalReader, SourceKind, SourceRef, agent_pool::AgentPoolWakePolicy,
    content::ContentRef as StoredContentRef,
};
use agent_sdk_store_supabase::{
    SupabaseAgentPoolStore, SupabaseAuth, SupabaseClient, SupabaseContentStore,
    SupabaseHttpRequest, SupabaseHttpResponse, SupabaseHttpTransport,
    SupabaseProviderArgumentStore, SupabaseRunJournal, SupabaseStoreConfig,
};

#[test]
fn supabase_client_adds_auth_schema_profile_and_json_headers() -> Result<(), AgentError> {
    let transport = ScriptedTransport::new(vec![SupabaseHttpResponse::empty(201)]);
    let client = SupabaseClient::new(config()?, transport.clone());
    client.insert("agent_sdk_content", &serde_json::json!({"ok": true}))?;

    let request = transport.single_request();
    assert_eq!(request.method, "POST");
    assert_eq!(
        request.url,
        "https://example.supabase.co/rest/v1/agent_sdk_content"
    );
    assert_eq!(request.header("apikey"), Some("service-role-secret"));
    assert_eq!(
        request.header("Authorization"),
        Some("Bearer service-role-secret")
    );
    assert_eq!(request.header("Accept-Profile"), Some("agent_sdk"));
    assert_eq!(request.header("Content-Profile"), Some("agent_sdk"));
    assert_eq!(request.header("Content-Type"), Some("application/json"));
    Ok(())
}

#[test]
fn supabase_journal_append_uses_rpc_and_never_logs_raw_arguments() -> Result<(), AgentError> {
    let transport = ScriptedTransport::new(vec![SupabaseHttpResponse::empty(204)]);
    let journal = SupabaseRunJournal::new(SupabaseClient::new(config()?, transport.clone()));
    let cursor = journal.append(journal_record(1, "journal.record.supabase.1"))?;
    assert_eq!(cursor.as_str(), "journal.1");

    let request = transport.single_request();
    assert_eq!(request.method, "POST");
    assert_eq!(
        request.url,
        "https://example.supabase.co/rest/v1/rpc/agent_sdk_append_journal_record"
    );
    let body: serde_json::Value =
        serde_json::from_slice(request.body.as_deref().expect("request body"))
            .expect("rpc body is json");
    assert_eq!(body["p_store_scope"], "test-scope");
    assert_eq!(body["p_run_id"], "run.supabase.store");
    assert_eq!(body["p_journal_seq"], 1);
    assert_eq!(body["p_record"]["record_id"], "journal.record.supabase.1");
    Ok(())
}

#[test]
fn supabase_journal_reader_decodes_records_from_postgrest_rows() -> Result<(), AgentError> {
    let record = journal_record(7, "journal.record.supabase.7");
    let transport = ScriptedTransport::new(vec![SupabaseHttpResponse::json(
        200,
        serde_json::json!([{ "record": record }]),
    )]);
    let journal = SupabaseRunJournal::new(SupabaseClient::new(config()?, transport.clone()));
    let records = journal.records_for_run(&RunId::new("run.supabase.store"))?;

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].record_id, "journal.record.supabase.7");
    let request = transport.single_request();
    assert_eq!(request.method, "GET");
    assert!(request.url.contains("/rest/v1/agent_sdk_journal_records?"));
    assert!(request.url.contains("store_scope=eq.test-scope"));
    assert!(request.url.contains("run_id=eq.run.supabase.store"));
    Ok(())
}

#[test]
fn supabase_provider_arguments_store_raw_payload_by_ref() -> Result<(), AgentError> {
    let transport = ScriptedTransport::new(vec![
        SupabaseHttpResponse::empty(201),
        SupabaseHttpResponse::json(
            200,
            serde_json::json!([{ "raw_arguments": r#"{"path":"README.md"}"# }]),
        ),
    ]);
    let store =
        SupabaseProviderArgumentStore::new(SupabaseClient::new(config()?, transport.clone()));
    let content_ref = store
        .store_provider_arguments(
            "provider.openai.responses",
            "call_123",
            &CanonicalToolName::new("workspace_read"),
            r#"{"path":"README.md"}"#,
        )?
        .expect("content ref");

    assert!(
        content_ref
            .as_str()
            .starts_with("content.provider_arguments.")
    );
    let request = transport.single_request();
    assert_eq!(
        request.url,
        "https://example.supabase.co/rest/v1/agent_sdk_provider_arguments"
    );
    let body: serde_json::Value =
        serde_json::from_slice(request.body.as_deref().expect("request body"))
            .expect("insert body is json");
    assert_eq!(body["store_scope"], "test-scope");
    assert_eq!(body["provider_ref"], "provider.openai.responses");
    assert_eq!(body["call_id"], "call_123");
    assert_eq!(body["canonical_tool_name"], "workspace_read");
    assert_eq!(body["raw_arguments"], r#"{"path":"README.md"}"#);
    assert_eq!(body["content_ref"], content_ref.as_str());
    let loaded = store.load_provider_arguments_json(&content_ref)?;
    assert_eq!(loaded["path"], "README.md");
    let requests = transport.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].method, "GET");
    assert!(
        requests[1]
            .url
            .contains("/rest/v1/agent_sdk_provider_arguments?")
    );
    assert!(requests[1].url.contains("content_ref=eq."));
    Ok(())
}

#[test]
fn supabase_agent_pool_store_opens_pool_and_allocates_sequence() -> Result<(), AgentError> {
    let transport = ScriptedTransport::new(vec![
        SupabaseHttpResponse::json(200, serde_json::json!([])),
        SupabaseHttpResponse::empty(204),
        SupabaseHttpResponse::json(200, serde_json::json!([{ "next_sequence": 42 }])),
    ]);
    let store = SupabaseAgentPoolStore::new(SupabaseClient::new(config()?, transport.clone()));
    let pool_id = AgentPoolId::new("pool.supabase.store");
    let config = AgentPoolStoreConfig {
        message_policy: AgentPoolMessagePolicy::bounded_defaults(),
        wake_policy: AgentPoolWakePolicy::safe_defaults(),
        policy_refs: Vec::new(),
    };

    let snapshot = store.open_pool(pool_id.clone(), config)?;
    let sequence = store.next_event_sequence(&pool_id)?;

    assert!(!snapshot.created);
    assert_eq!(sequence, 42);
    let requests = transport.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].method, "GET");
    assert!(requests[0].url.contains("/rest/v1/agent_sdk_agent_pools?"));
    assert_eq!(
        requests[1].url,
        "https://example.supabase.co/rest/v1/rpc/agent_sdk_upsert_agent_pool_state"
    );
    let body: serde_json::Value =
        serde_json::from_slice(requests[1].body.as_deref().expect("request body"))
            .expect("rpc body is json");
    assert_eq!(body["p_store_scope"], "test-scope");
    assert_eq!(body["p_pool_id"], "pool.supabase.store");
    assert_eq!(
        body["p_state"]["config"]["message_policy"]["include_sender_in_pool_broadcast"],
        false
    );
    assert_eq!(body["p_state"]["records"].as_array().unwrap().len(), 1);
    assert_eq!(
        requests[2].url,
        "https://example.supabase.co/rest/v1/rpc/agent_sdk_next_agent_pool_event_sequence"
    );
    Ok(())
}

#[test]
fn supabase_content_store_writes_bytes_as_base64() -> Result<(), AgentError> {
    let transport = ScriptedTransport::new(vec![SupabaseHttpResponse::empty(201)]);
    let store = SupabaseContentStore::new(SupabaseClient::new(config()?, transport.clone()));
    store
        .put_content(&stored_content_ref(), b"hello".to_vec())
        .map_err(|error| error.to_agent_error())?;

    let request = transport.single_request();
    assert_eq!(
        request.url,
        "https://example.supabase.co/rest/v1/agent_sdk_content"
    );
    let body: serde_json::Value =
        serde_json::from_slice(request.body.as_deref().expect("request body"))
            .expect("insert body is json");
    assert_eq!(body["content_id"], "content.supabase.store");
    assert_eq!(body["bytes_base64"], "aGVsbG8=");
    assert_eq!(body["byte_len"], 5);
    Ok(())
}

#[derive(Clone, Debug)]
struct ScriptedTransport {
    responses: Arc<Mutex<Vec<SupabaseHttpResponse>>>,
    requests: Arc<Mutex<Vec<SupabaseHttpRequest>>>,
}

impl ScriptedTransport {
    fn new(responses: Vec<SupabaseHttpResponse>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses.into_iter().rev().collect())),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn single_request(&self) -> SupabaseHttpRequest {
        let requests = self.requests.lock().expect("requests lock");
        assert_eq!(requests.len(), 1);
        requests[0].clone()
    }

    fn requests(&self) -> Vec<SupabaseHttpRequest> {
        self.requests.lock().expect("requests lock").clone()
    }
}

impl SupabaseHttpTransport for ScriptedTransport {
    fn send(&self, request: SupabaseHttpRequest) -> Result<SupabaseHttpResponse, AgentError> {
        self.requests.lock().expect("requests lock").push(request);
        self.responses
            .lock()
            .expect("responses lock")
            .pop()
            .ok_or_else(|| AgentError::contract_violation("missing scripted response"))
    }
}

fn config() -> Result<SupabaseStoreConfig, AgentError> {
    SupabaseStoreConfig::new(
        "https://example.supabase.co",
        "agent_sdk",
        "test-scope",
        SupabaseAuth::service_role("service-role-secret"),
    )
}

fn journal_record(journal_seq: u64, record_id: &str) -> JournalRecord {
    let mut intent = EffectIntent::new(
        EffectId::new("effect.supabase.store"),
        EffectKind::ToolExecution,
        EntityRef::new(EntityKind::ToolCall, "tool.call.supabase.store"),
        SourceRef::with_kind(SourceKind::Sdk, "source.supabase.store"),
        "execute supabase store test tool",
    );
    intent.destination = Some(DestinationRef::with_kind(
        DestinationKind::Tool,
        "destination.supabase.store.tool",
    ));
    intent.idempotency_key = Some(IdempotencyKey::new("idem.supabase.store.tool"));

    let mut base = JournalRecordBase::new(
        journal_seq,
        record_id,
        RunId::new("run.supabase.store"),
        AgentId::new("agent.supabase.store"),
        SourceRef::with_kind(SourceKind::Sdk, "source.supabase.store"),
    );
    base.timestamp_millis = 1_780_000_000_000 + journal_seq;
    JournalRecord::effect_intent(base, intent)
}

fn stored_content_ref() -> StoredContentRef {
    StoredContentRef::new(
        ContentId::new("content.supabase.store"),
        ContentVersion::new("v1"),
        ContentKind::Text,
        ContentScope::Run,
        EntityRef::run(RunId::new("run.supabase.store")),
        SourceRef::with_kind(SourceKind::Host, "source.supabase.store"),
        AdapterRef::new("adapter.supabase.content"),
        "supabase content",
    )
}
