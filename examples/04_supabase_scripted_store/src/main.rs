use std::sync::{Arc, Mutex};

use clawdia_sdk::{
    core::{AgentError, CanonicalToolName, ProviderArgumentStore},
    stores::{
        SupabaseAuth, SupabaseClient, SupabaseHttpRequest, SupabaseHttpResponse,
        SupabaseHttpTransport, SupabaseProviderArgumentStore, SupabaseStoreConfig,
    },
};

#[derive(Clone, Default)]
struct ScriptedTransport {
    requests: Arc<Mutex<Vec<SupabaseHttpRequest>>>,
    responses: Arc<Mutex<Vec<SupabaseHttpResponse>>>,
}

impl ScriptedTransport {
    fn new() -> Self {
        Self {
            requests: Arc::new(Mutex::new(Vec::new())),
            responses: Arc::new(Mutex::new(
                vec![
                    SupabaseHttpResponse::empty(201),
                    SupabaseHttpResponse::json(
                        200,
                        serde_json::json!([{ "raw_arguments": r#"{"path":"README.md"}"# }]),
                    ),
                ]
                .into_iter()
                .rev()
                .collect(),
            )),
        }
    }
}

impl SupabaseHttpTransport for ScriptedTransport {
    fn send(&self, request: SupabaseHttpRequest) -> Result<SupabaseHttpResponse, AgentError> {
        self.requests.lock().expect("requests lock").push(request);
        self.responses
            .lock()
            .expect("responses lock")
            .pop()
            .ok_or_else(|| AgentError::contract_violation("scripted transport exhausted"))
    }
}

fn main() -> Result<(), AgentError> {
    let transport = ScriptedTransport::new();
    let client = SupabaseClient::new(
        SupabaseStoreConfig::new(
            "https://example.supabase.co",
            "agent_sdk",
            "example-scope",
            SupabaseAuth::service_role("service-role-secret"),
        )?,
        transport.clone(),
    );
    let store = SupabaseProviderArgumentStore::new(client);
    let content_ref = store
        .store_provider_arguments(
            "provider.example",
            "call_example",
            &CanonicalToolName::new("workspace_read"),
            r#"{"path":"README.md"}"#,
        )?
        .expect("argument content ref");
    let loaded = store.load_provider_arguments_json(&content_ref)?;
    let request = transport
        .requests
        .lock()
        .expect("requests lock")
        .first()
        .expect("request captured")
        .clone();
    println!(
        "{} {} {}",
        content_ref.as_str(),
        loaded["path"],
        request.url
    );
    Ok(())
}
