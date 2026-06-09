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
/// Configuration for the live Gemini generateContent adapter.
pub struct GeminiGenerateContentConfig {
    /// Stable provider ref exposed through `ProviderCapabilities`.
    pub provider_ref: String,
    /// Gemini model id.
    pub model: String,
    /// Gemini API base URL.
    pub endpoint_base: String,
    /// Maximum input tokens advertised by this route.
    pub max_input_tokens: Option<u32>,
}

impl GeminiGenerateContentConfig {
    /// Creates a config for the hosted Gemini generateContent API.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            provider_ref: "provider.gemini.generate_content".to_string(),
            model: model.into(),
            endpoint_base: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            max_input_tokens: None,
        }
    }

    /// Sets the stable provider ref used in SDK capability metadata.
    pub fn provider_ref(mut self, provider_ref: impl Into<String>) -> Self {
        self.provider_ref = provider_ref.into();
        self
    }

    /// Sets a custom API base URL.
    pub fn endpoint_base(mut self, endpoint_base: impl Into<String>) -> Self {
        self.endpoint_base = endpoint_base.into();
        self
    }

    /// Sets the maximum input token limit advertised for this route.
    pub fn max_input_tokens(mut self, max_input_tokens: u32) -> Self {
        self.max_input_tokens = Some(max_input_tokens);
        self
    }

    fn endpoint_url(&self) -> String {
        let model = self.model.trim_start_matches("models/");
        format!(
            "{}/models/{model}:generateContent",
            self.endpoint_base.trim_end_matches('/')
        )
    }
}

#[derive(Clone)]
/// Live Gemini generateContent API adapter.
pub struct GeminiGenerateContentAdapter {
    config: GeminiGenerateContentConfig,
    api_key: ProviderApiKey,
    http: Arc<dyn JsonHttpTransport>,
    argument_sink: Option<Arc<dyn ProviderToolArgumentSink>>,
}

impl GeminiGenerateContentAdapter {
    /// Creates a live adapter using `GEMINI_API_KEY`.
    pub fn from_env(model: impl Into<String>) -> Result<Self, AgentError> {
        Self::new(
            GeminiGenerateContentConfig::new(model),
            ProviderApiKey::from_env("GEMINI_API_KEY")?,
        )
    }

    /// Creates a live adapter with a host-resolved API key.
    pub fn new(
        config: GeminiGenerateContentConfig,
        api_key: ProviderApiKey,
    ) -> Result<Self, AgentError> {
        Self::with_transport(config, api_key, Arc::new(CurlJsonHttpTransport::new()))
    }

    /// Creates an adapter with an injected JSON transport.
    pub fn with_transport(
        config: GeminiGenerateContentConfig,
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
        let mut contents = Vec::new();
        for message in &request.messages {
            match message.role {
                ProviderMessageRole::System | ProviderMessageRole::Developer => {
                    system.push(message.content.clone());
                }
                ProviderMessageRole::Assistant => {
                    contents.push(gemini_text_content("model", message.content.clone()));
                }
                ProviderMessageRole::Tool => {
                    contents.push(gemini_text_content(
                        "user",
                        format!("Tool result:\n{}", message.content),
                    ));
                }
                ProviderMessageRole::Context | ProviderMessageRole::User => {
                    contents.push(gemini_text_content("user", message.content.clone()));
                }
            }
        }

        let mut body = json!({ "contents": contents });
        if !system.is_empty() {
            body["systemInstruction"] = json!({
                "parts": [{ "text": system.join("\n\n") }]
            });
        }
        if let Some(generation_config) = gemini_generation_config(request) {
            body["generationConfig"] = generation_config;
        }
        if !request.tools.is_empty() {
            body["tools"] = Value::Array(vec![json!({
                "functionDeclarations": request
                    .tools
                    .iter()
                    .map(gemini_function_declaration)
                    .collect::<Vec<_>>()
            })]);
        }
        body
    }

    fn map_response(
        &self,
        response: GeminiGenerateContentResponse,
    ) -> Result<ProviderResponse, AgentError> {
        let tool_calls = self.tool_calls_from_response(&response)?;
        let usage = response.usage_metadata.clone().map(ProviderUsage::from);
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
        response: &GeminiGenerateContentResponse,
    ) -> Result<Vec<ProviderToolCall>, AgentError> {
        let mut calls = Vec::new();
        for candidate in &response.candidates {
            if let Some(content) = &candidate.content {
                for part in &content.parts {
                    let Some(function_call) = &part.function_call else {
                        continue;
                    };
                    let name = function_call.name.as_deref().ok_or_else(|| {
                        unsupported_response("Gemini generateContent", "functionCall missing name")
                    })?;
                    let call_id = function_call
                        .id
                        .clone()
                        .unwrap_or_else(|| format!("gemini_call_{}", calls.len()));
                    let canonical_tool_name = CanonicalToolName::new(name);
                    let mut call = ProviderToolCall::new(
                        ToolCallId::new(call_id.clone()),
                        canonical_tool_name.clone(),
                        format!(
                            "provider requested tool {name} with arguments stored as content refs"
                        ),
                    );
                    if let (Some(sink), Some(args)) =
                        (self.argument_sink.as_ref(), function_call.args.as_ref())
                    {
                        let raw_arguments = serde_json::to_string(args).map_err(|error| {
                            provider_failure(
                                RetryClassification::RepairNeeded,
                                format!(
                                    "Gemini functionCall args could not be serialized: {error}"
                                ),
                            )
                        })?;
                        if let Some(args_ref) = sink.store_tool_arguments(
                            &self.config.provider_ref,
                            &call_id,
                            &canonical_tool_name,
                            &raw_arguments,
                        )? {
                            call = call.with_args_ref(args_ref);
                        }
                    }
                    calls.push(call);
                }
            }
        }
        Ok(calls)
    }
}

impl ProviderAdapter for GeminiGenerateContentAdapter {
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
            JsonHttpRequest::new(self.config.endpoint_url(), self.wire_request(request))
                .header("x-goog-api-key", self.api_key.expose_secret())
                .header("Content-Type", "application/json");
        let response = self.http.post_json(http_request)?;
        let message = serde_json::from_value::<GeminiGenerateContentResponse>(response.body)
            .map_err(|error| unsupported_response("Gemini generateContent", error.to_string()))?;
        self.map_response(message)
    }
}

fn gemini_text_content(role: &str, text: String) -> Value {
    json!({
        "role": role,
        "parts": [{ "text": text }],
    })
}

fn gemini_generation_config(request: &ProviderRequest) -> Option<Value> {
    let hint = request.structured_output_hint.as_ref()?;
    let schema = hint.redacted_schema.clone()?;
    Some(json!({
        "responseMimeType": "application/json",
        "responseJsonSchema": schema,
    }))
}

fn gemini_function_declaration(tool: &ProviderToolSpec) -> Value {
    json!({
        "name": tool.name,
        "description": tool.description.clone().unwrap_or_else(|| {
            format!("SDK tool {} governed by package policy refs", tool.name)
        }),
        "parameters": tool.provider_schema(),
    })
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
/// Minimal Gemini generateContent response shape used by the adapter.
pub struct GeminiGenerateContentResponse {
    /// Response candidates.
    #[serde(default)]
    pub candidates: Vec<GeminiCandidate>,
    /// Provider usage accounting.
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: Option<GeminiUsage>,
}

impl GeminiGenerateContentResponse {
    /// Creates a text response fixture.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            candidates: vec![GeminiCandidate {
                content: Some(GeminiContent {
                    role: Some("model".to_string()),
                    parts: vec![GeminiPart::text(text)],
                }),
                finish_reason: Some("STOP".to_string()),
            }],
            usage_metadata: None,
        }
    }

    /// Creates a function-call response fixture.
    pub fn function_call(id: impl Into<String>, name: impl Into<String>, args: Value) -> Self {
        Self {
            candidates: vec![GeminiCandidate {
                content: Some(GeminiContent {
                    role: Some("model".to_string()),
                    parts: vec![GeminiPart::function_call(id, name, args)],
                }),
                finish_reason: Some("STOP".to_string()),
            }],
            usage_metadata: None,
        }
    }

    fn output_text(&self) -> String {
        self.candidates
            .iter()
            .filter_map(|candidate| candidate.content.as_ref())
            .flat_map(|content| content.parts.iter())
            .filter_map(|part| part.text.as_deref())
            .collect::<Vec<_>>()
            .join("")
    }

    fn stop_reason(&self) -> ProviderStopReason {
        let reason = self
            .candidates
            .first()
            .and_then(|candidate| candidate.finish_reason.as_deref())
            .unwrap_or("STOP");
        match reason {
            "STOP" => ProviderStopReason::EndTurn,
            "MAX_TOKENS" => ProviderStopReason::MaxTokens,
            "SAFETY" | "RECITATION" | "MALFORMED_FUNCTION_CALL" => {
                ProviderStopReason::ProviderError
            }
            _ => ProviderStopReason::Unknown,
        }
    }
}

impl fmt::Debug for GeminiGenerateContentResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GeminiGenerateContentResponse")
            .field("candidate_count", &self.candidates.len())
            .field("candidates", &self.candidates)
            .field("usage_metadata", &self.usage_metadata)
            .finish()
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
/// Gemini response candidate.
pub struct GeminiCandidate {
    /// Candidate content.
    pub content: Option<GeminiContent>,
    /// Provider finish reason.
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
}

impl fmt::Debug for GeminiCandidate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GeminiCandidate")
            .field("content", &self.content)
            .field("finish_reason", &self.finish_reason)
            .finish()
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
/// Gemini content block.
pub struct GeminiContent {
    /// Gemini role.
    pub role: Option<String>,
    /// Content parts.
    #[serde(default)]
    pub parts: Vec<GeminiPart>,
}

impl fmt::Debug for GeminiContent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GeminiContent")
            .field("role", &self.role)
            .field("part_count", &self.parts.len())
            .field("parts", &self.parts)
            .finish()
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
/// Gemini content part.
pub struct GeminiPart {
    /// Text part.
    pub text: Option<String>,
    /// Function-call part.
    #[serde(rename = "functionCall")]
    pub function_call: Option<GeminiFunctionCall>,
}

impl GeminiPart {
    /// Creates a text part.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            function_call: None,
        }
    }

    /// Creates a function-call part.
    pub fn function_call(id: impl Into<String>, name: impl Into<String>, args: Value) -> Self {
        Self {
            text: None,
            function_call: Some(GeminiFunctionCall {
                id: Some(id.into()),
                name: Some(name.into()),
                args: Some(args),
            }),
        }
    }
}

impl fmt::Debug for GeminiPart {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GeminiPart")
            .field(
                "text_chars",
                &self.text.as_ref().map(|value| value.chars().count()),
            )
            .field("function_call", &self.function_call)
            .finish()
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
/// Gemini function-call part.
pub struct GeminiFunctionCall {
    /// Provider function-call id.
    pub id: Option<String>,
    /// Tool/function name.
    pub name: Option<String>,
    /// Function-call arguments.
    pub args: Option<Value>,
}

impl fmt::Debug for GeminiFunctionCall {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GeminiFunctionCall")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("args", &"<redacted>")
            .field("args_present", &self.args.is_some())
            .finish()
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Gemini usage accounting.
pub struct GeminiUsage {
    /// Provider input tokens.
    #[serde(rename = "promptTokenCount")]
    pub prompt_token_count: Option<u32>,
    /// Provider output tokens.
    #[serde(rename = "candidatesTokenCount")]
    pub candidates_token_count: Option<u32>,
    /// Provider total tokens.
    #[serde(rename = "totalTokenCount")]
    pub total_token_count: Option<u32>,
}

impl From<GeminiUsage> for ProviderUsage {
    fn from(value: GeminiUsage) -> Self {
        Self {
            input_tokens: value.prompt_token_count,
            output_tokens: value.candidates_token_count,
            total_tokens: value.total_token_count,
        }
    }
}
