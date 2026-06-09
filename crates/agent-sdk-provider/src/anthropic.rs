use std::{fmt, sync::Arc};

use agent_sdk_core::{
    AgentError, ProviderAdapter, ProviderCapabilities, ProviderMessageRole,
    ProviderProjectionPolicy, ProviderRequest, ProviderResponse, ProviderStopReason,
    ProviderToolCall, ProviderToolSpec, ProviderUsage, RetryClassification, ToolCallId,
    tool_records::CanonicalToolName,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    ProviderApiKey, ProviderToolArgumentSink,
    error::{provider_failure, unsupported_response},
    http::{CurlJsonHttpTransport, JsonHttpRequest, JsonHttpTransport},
};

#[derive(Clone, Debug, Eq, PartialEq)]
/// Configuration for the live Anthropic Messages adapter.
pub struct AnthropicMessagesConfig {
    /// Stable provider ref exposed through `ProviderCapabilities`.
    pub provider_ref: String,
    /// Anthropic model id.
    pub model: String,
    /// Absolute Messages API endpoint.
    pub endpoint_url: String,
    /// Anthropic API version header value.
    pub api_version: String,
    /// Maximum output tokens for one request.
    pub max_tokens: u32,
    /// Maximum input tokens advertised by this route.
    pub max_input_tokens: Option<u32>,
}

impl AnthropicMessagesConfig {
    /// Creates a config for Anthropic's hosted Messages API.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            provider_ref: "provider.anthropic.messages".to_string(),
            model: model.into(),
            endpoint_url: "https://api.anthropic.com/v1/messages".to_string(),
            api_version: "2023-06-01".to_string(),
            max_tokens: 1024,
            max_input_tokens: None,
        }
    }

    /// Sets the stable provider ref used in SDK capability metadata.
    pub fn provider_ref(mut self, provider_ref: impl Into<String>) -> Self {
        self.provider_ref = provider_ref.into();
        self
    }

    /// Sets a custom endpoint URL.
    pub fn endpoint_url(mut self, endpoint_url: impl Into<String>) -> Self {
        self.endpoint_url = endpoint_url.into();
        self
    }

    /// Sets the Anthropic API version header.
    pub fn api_version(mut self, api_version: impl Into<String>) -> Self {
        self.api_version = api_version.into();
        self
    }

    /// Sets the maximum output token budget.
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Sets the maximum input token limit advertised for this route.
    pub fn max_input_tokens(mut self, max_input_tokens: u32) -> Self {
        self.max_input_tokens = Some(max_input_tokens);
        self
    }
}

#[derive(Clone)]
/// Live Anthropic Messages API adapter.
pub struct AnthropicMessagesAdapter {
    config: AnthropicMessagesConfig,
    api_key: ProviderApiKey,
    http: Arc<dyn JsonHttpTransport>,
    argument_sink: Option<Arc<dyn ProviderToolArgumentSink>>,
}

impl AnthropicMessagesAdapter {
    /// Creates a live adapter using `ANTHROPIC_API_KEY`.
    pub fn from_env(model: impl Into<String>) -> Result<Self, AgentError> {
        Self::new(
            AnthropicMessagesConfig::new(model),
            ProviderApiKey::from_env("ANTHROPIC_API_KEY")?,
        )
    }

    /// Creates a live adapter with a host-resolved API key.
    pub fn new(
        config: AnthropicMessagesConfig,
        api_key: ProviderApiKey,
    ) -> Result<Self, AgentError> {
        Self::with_transport(config, api_key, Arc::new(CurlJsonHttpTransport::new()))
    }

    /// Creates an adapter with an injected JSON transport.
    pub fn with_transport(
        config: AnthropicMessagesConfig,
        api_key: ProviderApiKey,
        http: Arc<dyn JsonHttpTransport>,
    ) -> Result<Self, AgentError> {
        Ok(Self {
            config,
            api_key,
            http,
            argument_sink: None,
        })
    }

    /// Adds an optional host-owned sink for raw tool-call arguments.
    pub fn with_argument_sink(mut self, sink: Arc<dyn ProviderToolArgumentSink>) -> Self {
        self.argument_sink = Some(sink);
        self
    }

    fn wire_request(&self, request: &ProviderRequest) -> Value {
        let mut system = Vec::new();
        let mut messages = Vec::new();
        for message in &request.messages {
            match message.role {
                ProviderMessageRole::System | ProviderMessageRole::Developer => {
                    system.push(message.content.clone());
                }
                ProviderMessageRole::Assistant => {
                    messages.push(json!({"role": "assistant", "content": message.content}));
                }
                ProviderMessageRole::Tool => {
                    messages.push(json!({
                        "role": "user",
                        "content": format!("Tool result:\n{}", message.content),
                    }));
                }
                ProviderMessageRole::Context | ProviderMessageRole::User => {
                    messages.push(json!({"role": "user", "content": message.content}));
                }
            }
        }

        let mut body = json!({
            "model": self.config.model.clone(),
            "max_tokens": self.config.max_tokens,
            "messages": messages,
        });
        if !system.is_empty() {
            body["system"] = Value::String(system.join("\n\n"));
        }
        if let Some(output_config) = anthropic_output_config(request) {
            body["output_config"] = output_config;
        }
        if !request.tools.is_empty() {
            body["tools"] = Value::Array(
                request
                    .tools
                    .iter()
                    .map(anthropic_tool_declaration)
                    .collect(),
            );
        }
        body
    }

    fn map_response(
        &self,
        response: AnthropicMessagesResponse,
    ) -> Result<ProviderResponse, AgentError> {
        let tool_calls = self.tool_calls_from_response(&response)?;
        let usage = response.usage.clone().map(ProviderUsage::from);
        if !tool_calls.is_empty() {
            let mut mapped = ProviderResponse::tool_use(tool_calls);
            mapped.usage = usage;
            return Ok(mapped);
        }

        Ok(ProviderResponse {
            schema_version: ProviderResponse::SCHEMA_VERSION,
            output_text: response.output_text(),
            stop_reason: response.stop_reason(),
            tool_calls: Vec::new(),
            usage,
        })
    }

    fn tool_calls_from_response(
        &self,
        response: &AnthropicMessagesResponse,
    ) -> Result<Vec<ProviderToolCall>, AgentError> {
        let mut calls = Vec::new();
        for item in &response.content {
            if item.kind != "tool_use" {
                continue;
            }
            let call_id = item.id.as_deref().ok_or_else(|| {
                unsupported_response("Anthropic Messages", "tool_use block missing id")
            })?;
            let name = item.name.as_deref().ok_or_else(|| {
                unsupported_response("Anthropic Messages", "tool_use block missing name")
            })?;
            let canonical_tool_name = CanonicalToolName::new(name);
            let mut call = ProviderToolCall::new(
                ToolCallId::new(call_id),
                canonical_tool_name.clone(),
                format!("provider requested tool {name} with arguments stored as content refs"),
            );
            if let (Some(sink), Some(input)) = (self.argument_sink.as_ref(), item.input.as_ref()) {
                let raw_arguments = serde_json::to_string(input).map_err(|error| {
                    provider_failure(
                        RetryClassification::RepairNeeded,
                        format!("Anthropic tool input could not be serialized: {error}"),
                    )
                })?;
                if let Some(args_ref) = sink.store_tool_arguments(
                    &self.config.provider_ref,
                    call_id,
                    &canonical_tool_name,
                    &raw_arguments,
                )? {
                    call = call.with_args_ref(args_ref);
                }
            }
            calls.push(call);
        }
        Ok(calls)
    }
}

impl ProviderAdapter for AnthropicMessagesAdapter {
    fn capabilities(&self) -> ProviderCapabilities {
        let mut capabilities = ProviderCapabilities::text_only(self.config.provider_ref.clone());
        capabilities.supports_usage = true;
        capabilities.max_input_tokens = self.config.max_input_tokens;
        capabilities
    }

    fn project_request(
        &self,
        projection: &agent_sdk_core::ContextProjection,
        policy: &ProviderProjectionPolicy,
    ) -> Result<ProviderRequest, AgentError> {
        agent_sdk_core::projection::project_context_projection(projection, policy)
    }

    fn complete(&self, request: &ProviderRequest) -> Result<ProviderResponse, AgentError> {
        let http_request =
            JsonHttpRequest::new(self.config.endpoint_url.clone(), self.wire_request(request))
                .header("x-api-key", self.api_key.expose_secret())
                .header("anthropic-version", self.config.api_version.clone())
                .header("Content-Type", "application/json");
        let response = self.http.post_json(http_request)?;
        let message = serde_json::from_value::<AnthropicMessagesResponse>(response.body)
            .map_err(|error| unsupported_response("Anthropic Messages", error.to_string()))?;
        self.map_response(message)
    }
}

fn anthropic_output_config(request: &ProviderRequest) -> Option<Value> {
    let hint = request.structured_output_hint.as_ref()?;
    let schema = hint.redacted_schema.clone()?;
    Some(json!({
        "format": {
            "type": "json_schema",
            "schema": schema,
        }
    }))
}

fn anthropic_tool_declaration(tool: &ProviderToolSpec) -> Value {
    json!({
        "name": tool.name,
        "description": tool.description.clone().unwrap_or_else(|| {
            format!("SDK tool {} governed by package policy refs", tool.name)
        }),
        "input_schema": tool.provider_schema(),
    })
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
/// Minimal Anthropic Messages response shape used by the adapter.
pub struct AnthropicMessagesResponse {
    /// Provider response id.
    pub id: Option<String>,
    /// Content blocks returned by Claude.
    #[serde(default)]
    pub content: Vec<AnthropicContentBlock>,
    /// Provider stop reason.
    pub stop_reason: Option<String>,
    /// Provider usage accounting.
    pub usage: Option<AnthropicUsage>,
}

impl AnthropicMessagesResponse {
    /// Creates a text response fixture.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            id: Some("msg_test".to_string()),
            content: vec![AnthropicContentBlock::text(text)],
            stop_reason: Some("end_turn".to_string()),
            usage: None,
        }
    }

    /// Creates a tool-use response fixture.
    pub fn tool_use(id: impl Into<String>, name: impl Into<String>, input: Value) -> Self {
        Self {
            id: Some("msg_tool".to_string()),
            content: vec![AnthropicContentBlock::tool_use(id, name, input)],
            stop_reason: Some("tool_use".to_string()),
            usage: None,
        }
    }

    fn output_text(&self) -> String {
        self.content
            .iter()
            .filter(|item| item.kind == "text")
            .filter_map(|item| item.text.as_deref())
            .collect::<Vec<_>>()
            .join("")
    }

    fn stop_reason(&self) -> ProviderStopReason {
        match self.stop_reason.as_deref().unwrap_or("end_turn") {
            "end_turn" => ProviderStopReason::EndTurn,
            "max_tokens" => ProviderStopReason::MaxTokens,
            "tool_use" => ProviderStopReason::ToolUse,
            "stop_sequence" => ProviderStopReason::EndTurn,
            _ => ProviderStopReason::Unknown,
        }
    }
}

impl fmt::Debug for AnthropicMessagesResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AnthropicMessagesResponse")
            .field("id", &self.id)
            .field("content_count", &self.content.len())
            .field("content", &self.content)
            .field("stop_reason", &self.stop_reason)
            .field("usage", &self.usage)
            .finish()
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
/// Anthropic content block.
pub struct AnthropicContentBlock {
    #[serde(rename = "type")]
    /// Content block type.
    pub kind: String,
    /// Text content for text blocks.
    pub text: Option<String>,
    /// Tool-use id.
    pub id: Option<String>,
    /// Tool name.
    pub name: Option<String>,
    /// Tool input arguments.
    pub input: Option<Value>,
}

impl AnthropicContentBlock {
    /// Creates a text block.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            kind: "text".to_string(),
            text: Some(text.into()),
            id: None,
            name: None,
            input: None,
        }
    }

    /// Creates a tool-use block.
    pub fn tool_use(id: impl Into<String>, name: impl Into<String>, input: Value) -> Self {
        Self {
            kind: "tool_use".to_string(),
            text: None,
            id: Some(id.into()),
            name: Some(name.into()),
            input: Some(input),
        }
    }
}

impl fmt::Debug for AnthropicContentBlock {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AnthropicContentBlock")
            .field("kind", &self.kind)
            .field(
                "text_chars",
                &self.text.as_ref().map(|value| value.chars().count()),
            )
            .field("id", &self.id)
            .field("name", &self.name)
            .field("input", &"<redacted>")
            .field("input_present", &self.input.is_some())
            .finish()
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Anthropic usage accounting.
pub struct AnthropicUsage {
    /// Provider input tokens.
    pub input_tokens: Option<u32>,
    /// Provider output tokens.
    pub output_tokens: Option<u32>,
}

impl From<AnthropicUsage> for ProviderUsage {
    fn from(value: AnthropicUsage) -> Self {
        let total_tokens = match (value.input_tokens, value.output_tokens) {
            (Some(input), Some(output)) => Some(input + output),
            _ => None,
        };
        Self {
            input_tokens: value.input_tokens,
            output_tokens: value.output_tokens,
            total_tokens,
        }
    }
}
