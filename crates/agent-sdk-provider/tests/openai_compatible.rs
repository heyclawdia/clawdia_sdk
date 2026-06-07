use std::sync::{Arc, Mutex};

use agent_sdk_core::{
    AgentError, AgentErrorKind, CapabilityId, CapabilityNamespace, PackageSidecarRef, PolicyKind,
    PolicyRef, PrivacyClass, ProviderAdapter, ProviderMessage, ProviderMessageRole,
    ProviderRequest, ProviderStopReason, ProviderStreamDelta, ProviderToolSpec,
    RetryClassification, SchemaVersion, domain::ContentRef as ContentRefId, domain::OutputSchemaId,
    output::OutputContract,
};
use agent_sdk_provider::{
    OpenAiCompatibleResponsesAdapter, OpenAiResponsesConfig, OpenAiResponsesRequest,
    OpenAiResponsesResponse, OpenAiResponsesTransport, OpenAiResponsesUsage,
    OpenAiToolArgumentSink, OpenAiWireOutputItem,
};
use serde_json::json;

#[test]
fn responses_adapter_maps_provider_request_to_wire_and_text_response() {
    let transport = Arc::new(ScriptedTransport::with_response(
        OpenAiResponsesResponse::text("hello from provider").with_usage(1, 2, 3),
    ));
    let adapter = OpenAiCompatibleResponsesAdapter::new(
        OpenAiResponsesConfig::new("provider.openai_compatible.responses", "gpt-test")
            .endpoint_ref("endpoint.test")
            .supports_streaming(true)
            .max_input_tokens(128),
        transport.clone(),
    );

    let response = adapter
        .complete(&provider_request())
        .expect("provider response maps");

    assert_eq!(response.output_text, "hello from provider");
    assert_eq!(response.stop_reason, ProviderStopReason::EndTurn);
    assert_eq!(response.usage.as_ref().unwrap().total_tokens, Some(3));
    let capabilities = adapter.capabilities();
    assert!(capabilities.supports_streaming);
    assert_eq!(capabilities.max_input_tokens, Some(128));

    let requests = transport.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].model, "gpt-test");
    assert_eq!(requests[0].endpoint_ref, "endpoint.test");
    assert_eq!(
        requests[0]
            .input
            .iter()
            .map(|message| message.role.as_str())
            .collect::<Vec<_>>(),
        vec!["developer", "user", "tool"]
    );
}

#[test]
fn responses_adapter_projects_structured_output_hint() {
    let contract = OutputContract::inline_json_schema(
        OutputSchemaId::new("schema.provider.todo"),
        SchemaVersion::new(1, 2, 3),
        json!({
            "type": "object",
            "required": ["title"],
            "properties": {
                "title": {"type": "string"}
            },
            "additionalProperties": false
        }),
    );
    let request = provider_request().with_structured_output_hint(&contract);

    let wire = OpenAiResponsesRequest::from_provider_request(
        &OpenAiResponsesConfig::new("provider.openai_compatible.responses", "gpt-test"),
        &request,
    );

    let text = wire.text.expect("structured output hint projects");
    assert_eq!(text.kind, "json_schema");
    assert_eq!(text.name, "schema.provider.todo");
    assert_eq!(text.schema_version, "1.2.3");
    assert_eq!(
        text.schema_fingerprint,
        contract.schema_fingerprint().as_str()
    );
    assert!(text.include_schema_ref);
}

#[test]
fn responses_adapter_projects_provider_tool_specs() {
    let request = provider_request().with_tools([workspace_read_tool_spec()]);

    let wire = OpenAiResponsesRequest::from_provider_request(
        &OpenAiResponsesConfig::new("provider.openai_compatible.responses", "gpt-test"),
        &request,
    );

    assert_eq!(wire.tools.len(), 1);
    assert_eq!(wire.tools[0].kind, "function");
    assert_eq!(wire.tools[0].name, "workspace_read");
    assert_eq!(
        wire.tools[0].parameters["x-agent-sdk-schema-ref"],
        "schema.workspace_read"
    );
}

#[test]
fn responses_adapter_maps_function_call_to_provider_tool_use_with_argument_sink() {
    let transport = Arc::new(ScriptedTransport::with_response(
        OpenAiResponsesResponse::function_call(
            "call_123",
            "workspace_read",
            r#"{"path":"README.md","secret":"do-not-project"}"#,
        ),
    ));
    let sink = Arc::new(CapturingArgumentSink::new("content.args.openai.call_123"));
    let adapter = OpenAiCompatibleResponsesAdapter::new(
        OpenAiResponsesConfig::new("provider.openai_compatible.responses", "gpt-test"),
        transport,
    )
    .with_argument_sink(sink.clone());

    let response = adapter
        .complete(&provider_request())
        .expect("function call maps");

    assert_eq!(response.stop_reason, ProviderStopReason::ToolUse);
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(
        response.tool_calls[0].canonical_tool_name.as_str(),
        "workspace_read"
    );
    assert_eq!(
        response.tool_calls[0].requested_args_refs,
        vec![ContentRefId::new("content.args.openai.call_123")]
    );
    assert!(
        !response.tool_calls[0]
            .redacted_args_summary
            .contains("do-not-project")
    );
    assert_eq!(
        sink.calls(),
        vec![(
            "call_123".to_string(),
            "workspace_read".to_string(),
            r#"{"path":"README.md","secret":"do-not-project"}"#.to_string()
        )]
    );
}

#[test]
fn responses_adapter_collects_message_output_parts_when_output_text_is_absent() {
    let transport = Arc::new(ScriptedTransport::with_response(OpenAiResponsesResponse {
        status: Some("completed".to_string()),
        output: vec![
            OpenAiWireOutputItem::message("hello "),
            OpenAiWireOutputItem::message("world"),
        ],
        ..OpenAiResponsesResponse::default()
    }));
    let adapter = OpenAiCompatibleResponsesAdapter::new(
        OpenAiResponsesConfig::new("provider.openai_compatible.responses", "gpt-test"),
        transport,
    );

    let response = adapter
        .complete(&provider_request())
        .expect("message output maps");

    assert_eq!(response.output_text, "hello world");
    assert_eq!(response.stop_reason, ProviderStopReason::EndTurn);
}

#[test]
fn responses_adapter_rejects_malformed_function_call() {
    let transport = Arc::new(ScriptedTransport::with_response(OpenAiResponsesResponse {
        status: Some("completed".to_string()),
        output: vec![OpenAiWireOutputItem {
            kind: "function_call".to_string(),
            name: Some("workspace_read".to_string()),
            ..OpenAiWireOutputItem::default()
        }],
        ..OpenAiResponsesResponse::default()
    }));
    let adapter = OpenAiCompatibleResponsesAdapter::new(
        OpenAiResponsesConfig::new("provider.openai_compatible.responses", "gpt-test"),
        transport,
    );

    let error = adapter
        .complete(&provider_request())
        .expect_err("malformed function call fails");

    assert_eq!(error.kind(), AgentErrorKind::ProviderFailure);
    assert!(error.context().message.contains("missing call_id"));
}

#[test]
fn responses_adapter_stream_uses_core_terminal_tool_call_delta() {
    let transport = Arc::new(ScriptedTransport::with_response(
        OpenAiResponsesResponse::function_call("call_stream", "workspace_read", r#"{}"#),
    ));
    let adapter = OpenAiCompatibleResponsesAdapter::new(
        OpenAiResponsesConfig::new("provider.openai_compatible.responses", "gpt-test"),
        transport,
    );

    let chunks = adapter.stream(&provider_request()).expect("stream maps");

    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].is_terminal);
    assert_eq!(
        chunks[0].delta,
        ProviderStreamDelta::ToolCalls {
            tool_calls: chunks[0].delta.delta_tool_calls_for_test(),
            stop_reason: Some(ProviderStopReason::ToolUse),
        }
    );
}

#[derive(Clone, Default)]
struct ScriptedTransport {
    responses: Arc<Mutex<Vec<OpenAiResponsesResponse>>>,
    requests: Arc<Mutex<Vec<OpenAiResponsesRequest>>>,
}

impl ScriptedTransport {
    fn with_response(response: OpenAiResponsesResponse) -> Self {
        Self {
            responses: Arc::new(Mutex::new(vec![response])),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn requests(&self) -> Vec<OpenAiResponsesRequest> {
        self.requests.lock().expect("requests lock").clone()
    }
}

impl OpenAiResponsesTransport for ScriptedTransport {
    fn complete(
        &self,
        request: OpenAiResponsesRequest,
    ) -> Result<OpenAiResponsesResponse, AgentError> {
        self.requests.lock().expect("requests lock").push(request);
        self.responses
            .lock()
            .expect("responses lock")
            .pop()
            .ok_or_else(|| {
                AgentError::new(
                    AgentErrorKind::ProviderFailure,
                    RetryClassification::RepairNeeded,
                    "scripted transport exhausted",
                )
            })
    }
}

#[derive(Clone)]
struct CapturingArgumentSink {
    next_ref: ContentRefId,
    calls: Arc<Mutex<Vec<(String, String, String)>>>,
}

impl CapturingArgumentSink {
    fn new(next_ref: impl Into<String>) -> Self {
        Self {
            next_ref: ContentRefId::new(next_ref),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn calls(&self) -> Vec<(String, String, String)> {
        self.calls.lock().expect("argument calls").clone()
    }
}

impl OpenAiToolArgumentSink for CapturingArgumentSink {
    fn store_tool_arguments(
        &self,
        call_id: &str,
        canonical_tool_name: &agent_sdk_core::tool_records::CanonicalToolName,
        raw_arguments: &str,
    ) -> Result<Option<ContentRefId>, AgentError> {
        self.calls.lock().expect("argument calls").push((
            call_id.to_string(),
            canonical_tool_name.as_str().to_string(),
            raw_arguments.to_string(),
        ));
        Ok(Some(self.next_ref.clone()))
    }
}

trait ResponseUsageExt {
    fn with_usage(self, input: u32, output: u32, total: u32) -> Self;
}

impl ResponseUsageExt for OpenAiResponsesResponse {
    fn with_usage(mut self, input: u32, output: u32, total: u32) -> Self {
        self.usage = Some(OpenAiResponsesUsage {
            input_tokens: Some(input),
            output_tokens: Some(output),
            total_tokens: Some(total),
        });
        self
    }
}

trait ProviderStreamDeltaTestExt {
    fn delta_tool_calls_for_test(&self) -> Vec<agent_sdk_core::ProviderToolCall>;
}

impl ProviderStreamDeltaTestExt for ProviderStreamDelta {
    fn delta_tool_calls_for_test(&self) -> Vec<agent_sdk_core::ProviderToolCall> {
        match self {
            ProviderStreamDelta::ToolCalls { tool_calls, .. } => tool_calls.clone(),
            _ => Vec::new(),
        }
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
                content: "read the repo".to_string(),
                privacy: PrivacyClass::ContentRefsOnly,
                projected_metadata: None,
            },
            ProviderMessage {
                role: ProviderMessageRole::Tool,
                content: "workspace_read: refs only".to_string(),
                privacy: PrivacyClass::ContentRefsOnly,
                projected_metadata: None,
            },
        ],
        projection_item_count: 3,
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
