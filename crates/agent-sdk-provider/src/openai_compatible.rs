//! OpenAI-compatible Responses-style provider adapter.
//! The adapter is transport-injected for compatibility testing and non-OpenAI
//! Responses-style endpoints. Use `OpenAiResponsesAdapter` for the default live
//! OpenAI endpoint.
//!
use std::{fmt, sync::Arc};

use agent_sdk_core::{
    AgentError, AgentErrorKind, ProviderAdapter, ProviderCapabilities, ProviderMessageRole,
    ProviderRequest, ProviderResponse, ProviderStopReason, ProviderToolCall, ProviderUsage,
    RetryClassification, ToolCallId, domain::ContentRef as ContentRefId,
    tool_records::CanonicalToolName,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Configuration for one OpenAI-compatible Responses adapter route.
/// It is data-only and does not contain credentials or live endpoint handles.
pub struct OpenAiResponsesConfig {
    /// Stable provider ref exposed through `ProviderCapabilities`.
    pub provider_ref: String,
    /// Provider-native model id.
    pub model: String,
    /// Host-owned endpoint ref or profile label.
    pub endpoint_ref: String,
    /// Whether the injected transport supports streaming.
    pub supports_streaming: bool,
    /// Maximum input token limit advertised by this route.
    pub max_input_tokens: Option<u32>,
}

impl OpenAiResponsesConfig {
    /// Creates a configuration for an OpenAI-compatible Responses route.
    pub fn new(provider_ref: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider_ref: provider_ref.into(),
            model: model.into(),
            endpoint_ref: "endpoint.host_configured.openai_compatible".to_string(),
            supports_streaming: false,
            max_input_tokens: None,
        }
    }

    /// Sets the host-owned endpoint ref.
    pub fn endpoint_ref(mut self, endpoint_ref: impl Into<String>) -> Self {
        self.endpoint_ref = endpoint_ref.into();
        self
    }

    /// Marks whether streaming is supported by the injected transport.
    pub fn supports_streaming(mut self, supports_streaming: bool) -> Self {
        self.supports_streaming = supports_streaming;
        self
    }

    /// Sets the maximum input token limit advertised for this route.
    pub fn max_input_tokens(mut self, max_input_tokens: u32) -> Self {
        self.max_input_tokens = Some(max_input_tokens);
        self
    }
}

/// Transport boundary for an OpenAI-compatible Responses request.
/// Implementations may perform network I/O; the adapter itself only maps
/// SDK DTOs to and from the transport contract.
pub trait OpenAiResponsesTransport: Send + Sync {
    /// Sends one Responses-style request and returns a decoded response.
    fn complete(
        &self,
        request: OpenAiResponsesRequest,
    ) -> Result<OpenAiResponsesResponse, AgentError>;
}

/// Optional host-owned sink for raw provider tool-call arguments.
/// The adapter never places raw arguments in the `ProviderToolCall` summary.
pub trait OpenAiToolArgumentSink: Send + Sync {
    /// Stores raw function-call arguments, returning a content ref when the
    /// host wants executors to resolve arguments through normal content policy.
    fn store_tool_arguments(
        &self,
        call_id: &str,
        canonical_tool_name: &CanonicalToolName,
        raw_arguments: &str,
    ) -> Result<Option<ContentRefId>, AgentError>;
}

#[derive(Clone)]
/// Provider adapter for OpenAI-compatible Responses-style transports.
pub struct OpenAiCompatibleResponsesAdapter {
    config: OpenAiResponsesConfig,
    transport: Arc<dyn OpenAiResponsesTransport>,
    argument_sink: Option<Arc<dyn OpenAiToolArgumentSink>>,
}

impl OpenAiCompatibleResponsesAdapter {
    /// Creates an adapter over a host-supplied transport.
    pub fn new(
        config: OpenAiResponsesConfig,
        transport: Arc<dyn OpenAiResponsesTransport>,
    ) -> Self {
        Self {
            config,
            transport,
            argument_sink: None,
        }
    }

    /// Adds an optional host-owned sink for raw tool-call arguments.
    pub fn with_argument_sink(mut self, sink: Arc<dyn OpenAiToolArgumentSink>) -> Self {
        self.argument_sink = Some(sink);
        self
    }

    /// Returns the adapter config.
    pub fn config(&self) -> &OpenAiResponsesConfig {
        &self.config
    }

    fn map_response(
        &self,
        response: OpenAiResponsesResponse,
    ) -> Result<ProviderResponse, AgentError> {
        let usage = response.usage.clone().map(ProviderUsage::from);
        let tool_calls = self.tool_calls_from_response(&response)?;
        if !tool_calls.is_empty() {
            let mut mapped = ProviderResponse::tool_use(tool_calls);
            mapped.usage = usage;
            return Ok(mapped);
        }

        Ok(ProviderResponse {
            schema_version: ProviderResponse::SCHEMA_VERSION,
            output_text: response.output_text(),
            stop_reason: response.stop_reason_without_tools(),
            tool_calls: Vec::new(),
            usage,
        })
    }

    fn tool_calls_from_response(
        &self,
        response: &OpenAiResponsesResponse,
    ) -> Result<Vec<ProviderToolCall>, AgentError> {
        let mut calls = Vec::new();
        for item in &response.output {
            if item.kind != "function_call" {
                continue;
            }
            let call_id = item.call_id.as_deref().ok_or_else(|| {
                provider_failure("OpenAI-compatible function_call item missing call_id")
            })?;
            let name = item.name.as_deref().ok_or_else(|| {
                provider_failure("OpenAI-compatible function_call item missing name")
            })?;
            let canonical_tool_name = CanonicalToolName::new(name);
            let mut call = ProviderToolCall::new(
                ToolCallId::new(call_id),
                canonical_tool_name.clone(),
                format!("provider requested tool {name} with arguments stored as content refs"),
            );
            if let (Some(sink), Some(raw_arguments)) =
                (self.argument_sink.as_ref(), item.arguments.as_deref())
            {
                if let Some(args_ref) =
                    sink.store_tool_arguments(call_id, &canonical_tool_name, raw_arguments)?
                {
                    call = call.with_args_ref(args_ref);
                }
            }
            calls.push(call);
        }
        Ok(calls)
    }
}

impl ProviderAdapter for OpenAiCompatibleResponsesAdapter {
    fn capabilities(&self) -> ProviderCapabilities {
        let mut capabilities = ProviderCapabilities::text_only(self.config.provider_ref.clone());
        capabilities.supports_streaming = self.config.supports_streaming;
        capabilities.max_input_tokens = self.config.max_input_tokens;
        capabilities
    }

    fn complete(&self, request: &ProviderRequest) -> Result<ProviderResponse, AgentError> {
        let wire_request = OpenAiResponsesRequest::from_provider_request(&self.config, request);
        let response = self.transport.complete(wire_request)?;
        self.map_response(response)
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
/// Minimal Responses-style request sent to an injected transport.
pub struct OpenAiResponsesRequest {
    /// Provider-native model id.
    pub model: String,
    /// Provider message input.
    pub input: Vec<OpenAiInputMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional structured-output text format hint.
    pub text: Option<OpenAiTextFormatHint>,
    /// Host endpoint/profile label, not a credential or raw client.
    pub endpoint_ref: String,
}

impl OpenAiResponsesRequest {
    /// Builds a Responses-style request from the canonical provider request.
    pub fn from_provider_request(
        config: &OpenAiResponsesConfig,
        request: &ProviderRequest,
    ) -> Self {
        Self {
            model: config.model.clone(),
            input: request
                .messages
                .iter()
                .map(OpenAiInputMessage::from_provider_message)
                .collect(),
            text: request
                .structured_output_hint
                .as_ref()
                .map(OpenAiTextFormatHint::from_provider_hint),
            endpoint_ref: config.endpoint_ref.clone(),
        }
    }
}

impl fmt::Debug for OpenAiResponsesRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenAiResponsesRequest")
            .field("model", &self.model)
            .field("input_count", &self.input.len())
            .field("input", &"<redacted>")
            .field("text", &self.text)
            .field("endpoint_ref", &self.endpoint_ref)
            .finish()
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
/// Minimal OpenAI-compatible message input.
pub struct OpenAiInputMessage {
    /// Provider role string.
    pub role: String,
    /// Redacted provider-visible content.
    pub content: String,
}

impl OpenAiInputMessage {
    fn from_provider_message(message: &agent_sdk_core::ProviderMessage) -> Self {
        Self {
            role: role_name(&message.role).to_string(),
            content: message.content.clone(),
        }
    }
}

impl fmt::Debug for OpenAiInputMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenAiInputMessage")
            .field("role", &self.role)
            .field("content", &"<redacted>")
            .field("content_chars", &self.content.chars().count())
            .finish()
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
/// Structured-output text format hint for Responses-compatible providers.
pub struct OpenAiTextFormatHint {
    #[serde(rename = "type")]
    /// Provider text format type.
    pub kind: String,
    /// Stable schema id.
    pub name: String,
    /// Schema semantic version.
    pub schema_version: String,
    /// SDK-owned schema fingerprint.
    pub schema_fingerprint: String,
    /// Whether the host should include the schema ref in the provider request.
    pub include_schema_ref: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Redacted inline schema body when the SDK output contract made one
    /// available for provider-native structured-output hints.
    pub schema: Option<Value>,
}

impl OpenAiTextFormatHint {
    fn from_provider_hint(hint: &agent_sdk_core::ProviderStructuredOutputHint) -> Self {
        Self {
            kind: "json_schema".to_string(),
            name: hint.schema_id.as_str().to_string(),
            schema_version: format!(
                "{}.{}.{}",
                hint.schema_version.major, hint.schema_version.minor, hint.schema_version.patch
            ),
            schema_fingerprint: hint.schema_fingerprint.as_str().to_string(),
            include_schema_ref: hint.include_schema_ref,
            schema: hint.redacted_schema.clone(),
        }
    }
}

impl fmt::Debug for OpenAiTextFormatHint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenAiTextFormatHint")
            .field("kind", &self.kind)
            .field("name", &self.name)
            .field("schema_version", &self.schema_version)
            .field("schema_fingerprint", &self.schema_fingerprint)
            .field("include_schema_ref", &self.include_schema_ref)
            .field("schema_present", &self.schema.is_some())
            .finish()
    }
}

#[derive(Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Minimal Responses-style response accepted by this adapter.
pub struct OpenAiResponsesResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Provider response id.
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Provider status.
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    /// Convenience output text field.
    pub output_text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Provider output items.
    pub output: Vec<OpenAiWireOutputItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Provider usage accounting.
    pub usage: Option<OpenAiResponsesUsage>,
}

impl OpenAiResponsesResponse {
    /// Creates a text response fixture.
    pub fn text(output_text: impl Into<String>) -> Self {
        Self {
            status: Some("completed".to_string()),
            output_text: output_text.into(),
            ..Self::default()
        }
    }

    /// Creates a function-call response fixture.
    pub fn function_call(
        call_id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl Into<String>,
    ) -> Self {
        Self {
            status: Some("completed".to_string()),
            output: vec![OpenAiWireOutputItem::function_call(
                call_id, name, arguments,
            )],
            ..Self::default()
        }
    }

    fn output_text(&self) -> String {
        if !self.output_text.is_empty() {
            return self.output_text.clone();
        }
        self.output
            .iter()
            .filter(|item| item.kind == "message")
            .flat_map(|item| item.content.iter())
            .filter_map(|part| {
                if part.kind == "output_text" {
                    part.text.clone()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("")
    }

    fn stop_reason_without_tools(&self) -> ProviderStopReason {
        match self.status.as_deref().unwrap_or("completed") {
            "completed" => ProviderStopReason::EndTurn,
            "cancelled" => ProviderStopReason::Cancelled,
            "incomplete" => ProviderStopReason::MaxTokens,
            "failed" => ProviderStopReason::ProviderError,
            _ => ProviderStopReason::Unknown,
        }
    }
}

impl fmt::Debug for OpenAiResponsesResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenAiResponsesResponse")
            .field("id", &self.id)
            .field("status", &self.status)
            .field("output_text", &"<redacted>")
            .field("output_text_chars", &self.output_text.chars().count())
            .field("output_count", &self.output.len())
            .field("output", &self.output)
            .field("usage", &self.usage)
            .finish()
    }
}

#[derive(Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Minimal Responses output item shape.
pub struct OpenAiWireOutputItem {
    #[serde(rename = "type")]
    /// Provider item type.
    pub kind: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Message content parts.
    pub content: Vec<OpenAiContentPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Function-call id.
    pub call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Function/tool name.
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Raw provider arguments. The adapter never puts this in summaries.
    pub arguments: Option<String>,
}

impl OpenAiWireOutputItem {
    /// Creates a function-call output item fixture.
    pub fn function_call(
        call_id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl Into<String>,
    ) -> Self {
        Self {
            kind: "function_call".to_string(),
            call_id: Some(call_id.into()),
            name: Some(name.into()),
            arguments: Some(arguments.into()),
            ..Self::default()
        }
    }

    /// Creates a message output item fixture.
    pub fn message(text: impl Into<String>) -> Self {
        Self {
            kind: "message".to_string(),
            content: vec![OpenAiContentPart {
                kind: "output_text".to_string(),
                text: Some(text.into()),
            }],
            ..Self::default()
        }
    }
}

impl fmt::Debug for OpenAiWireOutputItem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenAiWireOutputItem")
            .field("kind", &self.kind)
            .field("content_count", &self.content.len())
            .field("content", &self.content)
            .field("call_id", &self.call_id)
            .field("name", &self.name)
            .field("arguments", &"<redacted>")
            .field(
                "arguments_chars",
                &self.arguments.as_ref().map(|value| value.chars().count()),
            )
            .finish()
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
/// Minimal Responses content part.
pub struct OpenAiContentPart {
    #[serde(rename = "type")]
    /// Provider content part type.
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Text payload.
    pub text: Option<String>,
}

impl fmt::Debug for OpenAiContentPart {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenAiContentPart")
            .field("kind", &self.kind)
            .field("text", &"<redacted>")
            .field(
                "text_chars",
                &self.text.as_ref().map(|value| value.chars().count()),
            )
            .finish()
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Minimal Responses usage shape.
pub struct OpenAiResponsesUsage {
    /// Provider input tokens.
    pub input_tokens: Option<u32>,
    /// Provider output tokens.
    pub output_tokens: Option<u32>,
    /// Provider total tokens.
    pub total_tokens: Option<u32>,
}

impl From<OpenAiResponsesUsage> for ProviderUsage {
    fn from(value: OpenAiResponsesUsage) -> Self {
        Self {
            input_tokens: value.input_tokens,
            output_tokens: value.output_tokens,
            total_tokens: value.total_tokens,
        }
    }
}

fn role_name(role: &ProviderMessageRole) -> &'static str {
    match role {
        ProviderMessageRole::System => "system",
        ProviderMessageRole::Developer => "developer",
        ProviderMessageRole::User => "user",
        ProviderMessageRole::Assistant => "assistant",
        ProviderMessageRole::Tool => "tool",
        ProviderMessageRole::Context => "user",
    }
}

fn provider_failure(message: impl Into<String>) -> AgentError {
    AgentError::new(
        AgentErrorKind::ProviderFailure,
        RetryClassification::RepairNeeded,
        message,
    )
}
