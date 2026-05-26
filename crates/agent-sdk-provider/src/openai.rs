use std::sync::Arc;

use agent_sdk_core::{
    AgentError, ProviderAdapter, ProviderCapabilities, ProviderRequest, ProviderResponse,
    ProviderStreamChunk,
};
use serde_json::{Value, json};

use crate::{
    ProviderApiKey,
    error::unsupported_response,
    http::{CurlJsonHttpTransport, JsonHttpRequest, JsonHttpTransport},
    openai_compatible::{
        OpenAiCompatibleResponsesAdapter, OpenAiResponsesConfig, OpenAiResponsesRequest,
        OpenAiResponsesResponse, OpenAiResponsesTransport, OpenAiToolArgumentSink,
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
/// Configuration for the live OpenAI Responses adapter.
pub struct OpenAiLiveResponsesConfig {
    /// Stable provider ref exposed through `ProviderCapabilities`.
    pub provider_ref: String,
    /// OpenAI model id.
    pub model: String,
    /// Absolute Responses API endpoint.
    pub endpoint_url: String,
    /// Whether this route advertises streaming support.
    pub supports_streaming: bool,
    /// Maximum input tokens advertised by this route.
    pub max_input_tokens: Option<u32>,
}

impl OpenAiLiveResponsesConfig {
    /// Creates a config for OpenAI's hosted Responses API.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            provider_ref: "provider.openai.responses".to_string(),
            model: model.into(),
            endpoint_url: "https://api.openai.com/v1/responses".to_string(),
            supports_streaming: false,
            max_input_tokens: None,
        }
    }

    /// Sets the stable provider ref used in SDK capability metadata.
    pub fn provider_ref(mut self, provider_ref: impl Into<String>) -> Self {
        self.provider_ref = provider_ref.into();
        self
    }

    /// Sets a custom endpoint URL for hosted-compatible OpenAI deployments.
    pub fn endpoint_url(mut self, endpoint_url: impl Into<String>) -> Self {
        self.endpoint_url = endpoint_url.into();
        self
    }

    /// Sets the maximum input token limit advertised for this route.
    pub fn max_input_tokens(mut self, max_input_tokens: u32) -> Self {
        self.max_input_tokens = Some(max_input_tokens);
        self
    }
}

#[derive(Clone)]
/// Live OpenAI Responses API adapter.
///
/// It implements `ProviderAdapter` and delegates all runtime policy, journaling,
/// event publication, approval, and tool execution back to `agent-sdk-core`.
pub struct OpenAiResponsesAdapter {
    inner: OpenAiCompatibleResponsesAdapter,
}

impl OpenAiResponsesAdapter {
    /// Creates a live adapter using `OPENAI_API_KEY`.
    pub fn from_env(model: impl Into<String>) -> Result<Self, AgentError> {
        Self::new(
            OpenAiLiveResponsesConfig::new(model),
            ProviderApiKey::from_env("OPENAI_API_KEY")?,
        )
    }

    /// Creates a live adapter with a host-resolved API key.
    pub fn new(
        config: OpenAiLiveResponsesConfig,
        api_key: ProviderApiKey,
    ) -> Result<Self, AgentError> {
        Self::with_transport(config, api_key, Arc::new(CurlJsonHttpTransport::new()))
    }

    /// Creates an adapter with an injected JSON transport for deterministic
    /// tests or host-managed HTTP stacks.
    pub fn with_transport(
        config: OpenAiLiveResponsesConfig,
        api_key: ProviderApiKey,
        http: Arc<dyn JsonHttpTransport>,
    ) -> Result<Self, AgentError> {
        let compatible_config =
            OpenAiResponsesConfig::new(config.provider_ref.clone(), config.model.clone())
                .endpoint_ref(config.endpoint_url.clone())
                .supports_streaming(config.supports_streaming);
        let compatible_config = match config.max_input_tokens {
            Some(max_input_tokens) => compatible_config.max_input_tokens(max_input_tokens),
            None => compatible_config,
        };
        let transport = Arc::new(OpenAiLiveResponsesTransport {
            endpoint_url: config.endpoint_url,
            api_key,
            http,
        });
        Ok(Self {
            inner: OpenAiCompatibleResponsesAdapter::new(compatible_config, transport),
        })
    }

    /// Adds an optional host-owned sink for raw tool-call arguments.
    pub fn with_argument_sink(mut self, sink: Arc<dyn OpenAiToolArgumentSink>) -> Self {
        self.inner = self.inner.with_argument_sink(sink);
        self
    }
}

impl ProviderAdapter for OpenAiResponsesAdapter {
    fn capabilities(&self) -> ProviderCapabilities {
        self.inner.capabilities()
    }

    fn project_request(
        &self,
        projection: &agent_sdk_core::ContextProjection,
        policy: &agent_sdk_core::ProviderProjectionPolicy,
    ) -> Result<ProviderRequest, AgentError> {
        self.inner.project_request(projection, policy)
    }

    fn complete(&self, request: &ProviderRequest) -> Result<ProviderResponse, AgentError> {
        self.inner.complete(request)
    }

    fn stream(&self, request: &ProviderRequest) -> Result<Vec<ProviderStreamChunk>, AgentError> {
        self.inner.stream(request)
    }

    fn extract_usage(&self, response: &ProviderResponse) -> agent_sdk_core::ProviderUsage {
        self.inner.extract_usage(response)
    }
}

struct OpenAiLiveResponsesTransport {
    endpoint_url: String,
    api_key: ProviderApiKey,
    http: Arc<dyn JsonHttpTransport>,
}

impl OpenAiResponsesTransport for OpenAiLiveResponsesTransport {
    fn complete(
        &self,
        request: OpenAiResponsesRequest,
    ) -> Result<OpenAiResponsesResponse, AgentError> {
        let body = openai_responses_body(request);
        let http_request = JsonHttpRequest::new(self.endpoint_url.clone(), body)
            .header(
                "Authorization",
                format!("Bearer {}", self.api_key.expose_secret()),
            )
            .header("Content-Type", "application/json");
        let response = self.http.post_json(http_request)?;
        serde_json::from_value(response.body)
            .map_err(|error| unsupported_response("OpenAI Responses", error.to_string()))
    }
}

fn openai_responses_body(request: OpenAiResponsesRequest) -> Value {
    let mut instructions = Vec::new();
    let mut input = Vec::new();
    for message in request.input {
        match message.role.as_str() {
            "system" | "developer" => instructions.push(message.content),
            "assistant" => input.push(json!({
                "role": "assistant",
                "content": message.content,
            })),
            "tool" => input.push(json!({
                "role": "user",
                "content": format!("Tool result:\n{}", message.content),
            })),
            _ => input.push(json!({
                "role": "user",
                "content": message.content,
            })),
        }
    }

    let mut body = json!({
        "model": request.model,
        "input": input,
    });
    if !instructions.is_empty() {
        body["instructions"] = Value::String(instructions.join("\n\n"));
    }
    if let Some(text) = request.text.and_then(openai_text_format) {
        body["text"] = text;
    }
    body
}

fn openai_text_format(text: crate::OpenAiTextFormatHint) -> Option<Value> {
    let schema = text.schema?;
    Some(json!({
        "format": {
            "type": "json_schema",
            "name": text.name,
            "schema": schema,
            "strict": true,
        }
    }))
}
