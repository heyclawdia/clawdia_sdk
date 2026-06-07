use std::sync::{Arc, Mutex};

use agent_sdk_core::{
    AgentError, CapabilityId, CapabilityNamespace, PackageSidecarRef, PolicyKind, PolicyRef,
    PrivacyClass, ProviderAdapter, ProviderMessage, ProviderMessageRole, ProviderRequest,
    ProviderStopReason, ProviderToolSpec, SchemaVersion, domain::ContentRef as ContentRefId,
    domain::OutputSchemaId, output::OutputContract, tool_records::CanonicalToolName,
};
use agent_sdk_provider::{
    AnthropicMessagesAdapter, AnthropicMessagesConfig, AnthropicMessagesResponse,
    GeminiGenerateContentAdapter, GeminiGenerateContentConfig, GeminiGenerateContentResponse,
    JsonHttpRequest, JsonHttpResponse, JsonHttpTransport, OpenAiInputMessage,
    OpenAiLiveResponsesConfig, OpenAiResponsesAdapter, OpenAiResponsesRequest,
    OpenAiResponsesResponse, OpenAiTextFormatHint, OpenAiWireOutputItem, ProviderApiKey,
    ProviderToolArgumentSink,
};
use serde_json::{Value, json};

#[test]
fn provider_api_key_debug_redacts_secret_material() {
    let key = ProviderApiKey::new("secret-key").expect("key builds");

    let debug = format!("{key:?}");

    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("secret-key"));
}

#[test]
fn json_http_debug_redacts_headers_and_body() {
    let request = JsonHttpRequest::new(
        "https://provider.test",
        json!({"prompt": "do not log", "api_key": "body-secret"}),
    )
    .header("Authorization", "Bearer header-secret");
    let response = JsonHttpResponse {
        status: 200,
        body: json!({"output": "private model output"}),
    };

    let request_debug = format!("{request:?}");
    let response_debug = format!("{response:?}");

    assert!(request_debug.contains("Authorization: <redacted>"));
    assert!(!request_debug.contains("header-secret"));
    assert!(!request_debug.contains("body-secret"));
    assert!(!request_debug.contains("do not log"));
    assert!(!response_debug.contains("private model output"));
}

#[test]
fn provider_wire_debug_redacts_prompt_output_schema_and_tool_arguments() {
    let openai_request = OpenAiResponsesRequest {
        model: "gpt-test".to_string(),
        input: vec![OpenAiInputMessage {
            role: "user".to_string(),
            content: "openai-prompt-secret".to_string(),
        }],
        text: Some(OpenAiTextFormatHint {
            kind: "json_schema".to_string(),
            name: "schema.debug".to_string(),
            schema_version: "1.0.0".to_string(),
            schema_fingerprint: "hash.debug".to_string(),
            include_schema_ref: true,
            schema: Some(json!({"secret": "openai-schema-secret"})),
        }),
        tools: Vec::new(),
        endpoint_ref: "endpoint.debug".to_string(),
    };
    let openai_response = OpenAiResponsesResponse {
        output_text: "openai-output-secret".to_string(),
        output: vec![
            OpenAiWireOutputItem::message("openai-message-secret"),
            OpenAiWireOutputItem::function_call(
                "call_openai",
                "workspace_read",
                r#"{"secret":"openai-args-secret"}"#,
            ),
        ],
        ..OpenAiResponsesResponse::default()
    };
    let anthropic_response = AnthropicMessagesResponse::tool_use(
        "toolu_debug",
        "workspace_read",
        json!({"secret": "anthropic-args-secret"}),
    );
    let gemini_response = GeminiGenerateContentResponse::function_call(
        "fn_debug",
        "workspace_grep",
        json!({"secret": "gemini-args-secret"}),
    );
    let gemini_text = GeminiGenerateContentResponse::text("gemini-text-secret");

    let debug = format!(
        "{openai_request:?}\n{openai_response:?}\n{anthropic_response:?}\n{gemini_response:?}\n{gemini_text:?}"
    );

    for secret in [
        "openai-prompt-secret",
        "openai-schema-secret",
        "openai-output-secret",
        "openai-message-secret",
        "openai-args-secret",
        "anthropic-args-secret",
        "gemini-args-secret",
        "gemini-text-secret",
    ] {
        assert!(
            !debug.contains(secret),
            "provider wire debug leaked {secret}: {debug}"
        );
    }
}

#[test]
fn openai_responses_adapter_calls_live_responses_shape() {
    let transport = Arc::new(ScriptedJsonTransport::with_body(json!({
        "status": "completed",
        "output_text": "hello from openai",
        "usage": {
            "input_tokens": 3,
            "output_tokens": 4,
            "total_tokens": 7
        }
    })));
    let adapter = OpenAiResponsesAdapter::with_transport(
        OpenAiLiveResponsesConfig::new("gpt-test")
            .endpoint_url("https://api.openai.test/v1/responses"),
        ProviderApiKey::new("openai-test-key").expect("api key"),
        transport.clone(),
    )
    .expect("adapter builds");

    let response = adapter
        .complete(&structured_provider_request())
        .expect("provider response maps");

    assert_eq!(response.output_text, "hello from openai");
    assert_eq!(response.usage.unwrap().total_tokens, Some(7));
    let requests = transport.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url, "https://api.openai.test/v1/responses");
    assert_eq!(requests[0].body["model"], "gpt-test");
    assert_eq!(requests[0].body["instructions"], "follow SDK policy");
    assert_eq!(requests[0].body["input"][0]["role"], "user");
    assert_eq!(requests[0].body["text"]["format"]["type"], "json_schema");
    assert_eq!(
        requests[0].body["text"]["format"]["name"],
        "schema.provider.todo"
    );
    assert_eq!(
        requests[0].body["text"]["format"]["schema"]["required"][0],
        "title"
    );
    assert_eq!(requests[0].body["tools"][0]["type"], "function");
    assert_eq!(requests[0].body["tools"][0]["name"], "workspace_read");
    assert_eq!(
        requests[0].body["tools"][0]["parameters"]["x-agent-sdk-schema-ref"],
        "schema.workspace_read"
    );
}

#[test]
fn anthropic_messages_adapter_calls_live_messages_shape_and_maps_tool_use() {
    let transport = Arc::new(ScriptedJsonTransport::with_body(
        serde_json::to_value(AnthropicMessagesResponse::tool_use(
            "toolu_123",
            "workspace_read",
            json!({"path": "README.md", "secret": "do-not-project"}),
        ))
        .expect("fixture serializes"),
    ));
    let sink = Arc::new(CapturingArgumentSink::new(
        "content.args.anthropic.toolu_123",
    ));
    let adapter = AnthropicMessagesAdapter::with_transport(
        AnthropicMessagesConfig::new("claude-test")
            .endpoint_url("https://api.anthropic.test/v1/messages")
            .max_tokens(256),
        ProviderApiKey::new("anthropic-test-key").expect("api key"),
        transport.clone(),
    )
    .expect("adapter builds")
    .with_argument_sink(sink.clone());

    let response = adapter
        .complete(&structured_provider_request())
        .expect("provider response maps");

    assert_eq!(response.stop_reason, ProviderStopReason::ToolUse);
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(
        response.tool_calls[0].canonical_tool_name.as_str(),
        "workspace_read"
    );
    assert_eq!(
        response.tool_calls[0].requested_args_refs,
        vec![ContentRefId::new("content.args.anthropic.toolu_123")]
    );
    assert!(
        !response.tool_calls[0]
            .redacted_args_summary
            .contains("do-not-project")
    );
    let requests = transport.requests();
    assert_eq!(requests[0].body["model"], "claude-test");
    assert_eq!(requests[0].body["system"], "follow SDK policy");
    assert_eq!(requests[0].body["max_tokens"], 256);
    assert_eq!(
        requests[0].body["output_config"]["format"]["type"],
        "json_schema"
    );
    assert_eq!(requests[0].body["tools"][0]["name"], "workspace_read");
    assert_eq!(
        requests[0].body["tools"][0]["input_schema"]["x-agent-sdk-schema-ref"],
        "schema.workspace_read"
    );
    assert_eq!(
        sink.calls(),
        vec![(
            "provider.anthropic.messages".to_string(),
            "toolu_123".to_string(),
            "workspace_read".to_string(),
            r#"{"path":"README.md","secret":"do-not-project"}"#.to_string()
        )]
    );
}

#[test]
fn gemini_generate_content_adapter_calls_live_shape_and_maps_function_call() {
    let transport = Arc::new(ScriptedJsonTransport::with_body(
        serde_json::to_value(GeminiGenerateContentResponse::function_call(
            "fn_123",
            "workspace_grep",
            json!({"query": "ProviderAdapter"}),
        ))
        .expect("fixture serializes"),
    ));
    let sink = Arc::new(CapturingArgumentSink::new("content.args.gemini.fn_123"));
    let adapter = GeminiGenerateContentAdapter::with_transport(
        GeminiGenerateContentConfig::new("gemini-test")
            .endpoint_base("https://generativelanguage.test/v1beta"),
        ProviderApiKey::new("gemini-test-key").expect("api key"),
        transport.clone(),
    )
    .expect("adapter builds")
    .with_argument_sink(sink.clone());

    let response = adapter
        .complete(&structured_provider_request())
        .expect("provider response maps");

    assert_eq!(response.stop_reason, ProviderStopReason::ToolUse);
    assert_eq!(
        response.tool_calls[0].canonical_tool_name.as_str(),
        "workspace_grep"
    );
    assert_eq!(
        response.tool_calls[0].requested_args_refs,
        vec![ContentRefId::new("content.args.gemini.fn_123")]
    );
    let requests = transport.requests();
    assert_eq!(
        requests[0].url,
        "https://generativelanguage.test/v1beta/models/gemini-test:generateContent"
    );
    assert_eq!(
        requests[0].body["systemInstruction"]["parts"][0]["text"],
        "follow SDK policy"
    );
    assert_eq!(
        requests[0].body["generationConfig"]["responseMimeType"],
        "application/json"
    );
    assert_eq!(
        requests[0].body["tools"][0]["functionDeclarations"][0]["name"],
        "workspace_read"
    );
    assert_eq!(
        requests[0].body["tools"][0]["functionDeclarations"][0]["parameters"]["x-agent-sdk-schema-ref"],
        "schema.workspace_read"
    );
    assert_eq!(
        requests[0].body["generationConfig"]["responseJsonSchema"]["required"][0],
        "title"
    );
    assert_eq!(
        sink.calls(),
        vec![(
            "provider.gemini.generate_content".to_string(),
            "fn_123".to_string(),
            "workspace_grep".to_string(),
            r#"{"query":"ProviderAdapter"}"#.to_string()
        )]
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

#[derive(Clone)]
struct CapturingArgumentSink {
    next_ref: ContentRefId,
    calls: Arc<Mutex<Vec<CapturedArgumentCall>>>,
}

type CapturedArgumentCall = (String, String, String, String);

impl CapturingArgumentSink {
    fn new(next_ref: impl Into<String>) -> Self {
        Self {
            next_ref: ContentRefId::new(next_ref),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn calls(&self) -> Vec<CapturedArgumentCall> {
        self.calls.lock().expect("argument calls").clone()
    }
}

impl ProviderToolArgumentSink for CapturingArgumentSink {
    fn store_tool_arguments(
        &self,
        provider_ref: &str,
        call_id: &str,
        canonical_tool_name: &CanonicalToolName,
        raw_arguments: &str,
    ) -> Result<Option<ContentRefId>, AgentError> {
        self.calls.lock().expect("argument calls").push((
            provider_ref.to_string(),
            call_id.to_string(),
            canonical_tool_name.as_str().to_string(),
            raw_arguments.to_string(),
        ));
        Ok(Some(self.next_ref.clone()))
    }
}

fn structured_provider_request() -> ProviderRequest {
    let contract = OutputContract::inline_json_schema(
        OutputSchemaId::new("schema.provider.todo"),
        SchemaVersion::new(1, 0, 0),
        json!({
            "type": "object",
            "required": ["title"],
            "properties": {
                "title": { "type": "string" }
            },
            "additionalProperties": false
        }),
    );
    provider_request()
        .with_structured_output_hint(&contract)
        .with_tools([workspace_read_tool_spec()])
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
                content: "extract the todo".to_string(),
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
}
