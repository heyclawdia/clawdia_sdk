use std::sync::{Arc, Mutex};

use agent_sdk_core::{
    AgentError, CapabilityId, CapabilityNamespace, PackageSidecarRef, PolicyKind, PolicyRef,
    PrivacyClass, ProviderAdapter, ProviderMessage, ProviderMessageRole, ProviderRequest,
    ProviderToolSpec,
};
use agent_sdk_provider::{
    AnthropicMessagesAdapter, AnthropicMessagesConfig, AnthropicMessagesResponse,
    GeminiGenerateContentAdapter, GeminiGenerateContentConfig, GeminiGenerateContentResponse,
    JsonHttpRequest, JsonHttpResponse, JsonHttpTransport, OpenAiLiveResponsesConfig,
    OpenAiResponsesAdapter, OpenAiResponsesConfig, OpenAiResponsesRequest, ProviderApiKey,
};
use serde_json::{Value, json};

#[test]
fn openai_compatible_responses_request_projects_provider_tools() {
    let wire = OpenAiResponsesRequest::from_provider_request(
        &OpenAiResponsesConfig::new("provider.openai_compatible.responses", "gpt-test"),
        &provider_request().with_tools([workspace_read_tool_spec()]),
    );

    assert_eq!(wire.tools.len(), 1);
    assert_eq!(wire.tools[0].kind, "function");
    assert_eq!(wire.tools[0].name, "workspace_read");
    assert_eq!(wire.tools[0].description, "Read a file from the workspace");
    assert_eq!(
        wire.tools[0].parameters["x-agent-sdk-schema-ref"],
        "schema.workspace_read"
    );
}

#[test]
fn openai_compatible_responses_request_projects_inline_tool_schema() {
    let wire = OpenAiResponsesRequest::from_provider_request(
        &OpenAiResponsesConfig::new("provider.openai_compatible.responses", "gpt-test"),
        &provider_request().with_tools([workspace_read_tool_spec().with_redacted_schema(json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": { "type": "string" }
            },
            "additionalProperties": false
        }))]),
    );

    assert_eq!(
        wire.tools[0].parameters["properties"]["path"]["type"],
        "string"
    );
    assert_eq!(wire.tools[0].parameters["additionalProperties"], false);
    assert!(wire.tools[0].parameters["x-agent-sdk-schema-ref"].is_null());
}

#[test]
fn openai_live_adapter_projects_provider_tools() {
    let transport = Arc::new(ScriptedJsonTransport::with_body(json!({
        "status": "completed",
        "output_text": "ok"
    })));
    let adapter = OpenAiResponsesAdapter::with_transport(
        OpenAiLiveResponsesConfig::new("gpt-test")
            .endpoint_url("https://api.openai.test/v1/responses"),
        ProviderApiKey::new("openai-test-key").expect("api key"),
        transport.clone(),
    )
    .expect("adapter builds");

    adapter
        .complete(&provider_request().with_tools([workspace_read_tool_spec()]))
        .expect("provider response maps");

    let requests = transport.requests();
    assert_eq!(requests[0].body["tools"][0]["type"], "function");
    assert_eq!(requests[0].body["tools"][0]["name"], "workspace_read");
    assert_eq!(
        requests[0].body["tools"][0]["description"],
        "Read a file from the workspace"
    );
    assert_eq!(
        requests[0].body["tools"][0]["parameters"]["x-agent-sdk-schema-ref"],
        "schema.workspace_read"
    );
}

#[test]
fn anthropic_adapter_projects_provider_tools() {
    let transport = Arc::new(ScriptedJsonTransport::with_body(
        serde_json::to_value(AnthropicMessagesResponse::text("ok")).expect("response serializes"),
    ));
    let adapter = AnthropicMessagesAdapter::with_transport(
        AnthropicMessagesConfig::new("claude-test")
            .endpoint_url("https://api.anthropic.test/v1/messages"),
        ProviderApiKey::new("anthropic-test-key").expect("api key"),
        transport.clone(),
    )
    .expect("adapter builds");

    adapter
        .complete(&provider_request().with_tools([workspace_read_tool_spec()]))
        .expect("provider response maps");

    let requests = transport.requests();
    assert_eq!(requests[0].body["tools"][0]["name"], "workspace_read");
    assert_eq!(
        requests[0].body["tools"][0]["description"],
        "Read a file from the workspace"
    );
    assert_eq!(
        requests[0].body["tools"][0]["input_schema"]["x-agent-sdk-schema-ref"],
        "schema.workspace_read"
    );
}

#[test]
fn gemini_adapter_projects_provider_tools() {
    let transport = Arc::new(ScriptedJsonTransport::with_body(
        serde_json::to_value(GeminiGenerateContentResponse::text("ok"))
            .expect("response serializes"),
    ));
    let adapter = GeminiGenerateContentAdapter::with_transport(
        GeminiGenerateContentConfig::new("gemini-test")
            .endpoint_base("https://generativelanguage.test/v1beta"),
        ProviderApiKey::new("gemini-test-key").expect("api key"),
        transport.clone(),
    )
    .expect("adapter builds");

    adapter
        .complete(&provider_request().with_tools([workspace_read_tool_spec()]))
        .expect("provider response maps");

    let requests = transport.requests();
    assert_eq!(
        requests[0].body["tools"][0]["functionDeclarations"][0]["name"],
        "workspace_read"
    );
    assert_eq!(
        requests[0].body["tools"][0]["functionDeclarations"][0]["description"],
        "Read a file from the workspace"
    );
    assert_eq!(
        requests[0].body["tools"][0]["functionDeclarations"][0]["parameters"]["x-agent-sdk-schema-ref"],
        "schema.workspace_read"
    );
}

#[derive(Clone, Default)]
struct ScriptedJsonTransport {
    responses: Arc<Mutex<Vec<JsonHttpResponse>>>,
    requests: Arc<Mutex<Vec<JsonHttpRequest>>>,
}

impl ScriptedJsonTransport {
    fn with_body(body: Value) -> Self {
        Self {
            responses: Arc::new(Mutex::new(vec![JsonHttpResponse { status: 200, body }])),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn requests(&self) -> Vec<JsonHttpRequest> {
        self.requests.lock().expect("requests lock").clone()
    }
}

impl JsonHttpTransport for ScriptedJsonTransport {
    fn post_json(&self, request: JsonHttpRequest) -> Result<JsonHttpResponse, AgentError> {
        self.requests.lock().expect("requests lock").push(request);
        self.responses
            .lock()
            .expect("responses lock")
            .pop()
            .ok_or_else(|| AgentError::contract_violation("scripted transport exhausted"))
    }
}

fn provider_request() -> ProviderRequest {
    ProviderRequest {
        schema_version: ProviderRequest::SCHEMA_VERSION,
        projection_policy_ref: "policy.provider.test".to_string(),
        messages: vec![
            ProviderMessage {
                role: ProviderMessageRole::Developer,
                content: "follow SDK policy".to_string(),
                privacy: PrivacyClass::ContentRefsOnly,
                projected_metadata: None,
            },
            ProviderMessage {
                role: ProviderMessageRole::User,
                content: "read README".to_string(),
                privacy: PrivacyClass::ContentRefsOnly,
                projected_metadata: None,
            },
        ],
        projection_item_count: 2,
        structured_output_hint: None,
        tools: Vec::new(),
    }
}

fn workspace_read_tool_spec() -> ProviderToolSpec {
    ProviderToolSpec::new(
        "workspace_read",
        CapabilityId::new("cap.tool.workspace_read"),
        CapabilityNamespace::new("tool.workspace_read"),
        PackageSidecarRef::new("schema.workspace_read", "json_schema", "v1"),
        vec![PolicyRef::with_kind(
            PolicyKind::Approval,
            "policy.approval.workspace_read",
        )],
    )
    .with_description("Read a file from the workspace")
}
