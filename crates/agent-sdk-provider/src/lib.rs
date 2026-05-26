//! Optional provider adapter helpers for the Agent SDK.
//!
//! This crate implements provider-facing adapters over `agent-sdk-core`
//! contracts. It must not own credentials, live endpoint policy, journals,
//! events, tool execution, or product routing decisions.

mod anthropic;
mod arguments;
mod auth;
mod error;
mod gemini;
mod http;
mod openai;
mod openai_compatible;

pub use anthropic::{
    AnthropicContentBlock, AnthropicMessagesAdapter, AnthropicMessagesConfig,
    AnthropicMessagesResponse, AnthropicUsage,
};
pub use arguments::ProviderToolArgumentSink;
pub use auth::ProviderApiKey;
pub use gemini::{
    GeminiCandidate, GeminiContent, GeminiFunctionCall, GeminiGenerateContentAdapter,
    GeminiGenerateContentConfig, GeminiGenerateContentResponse, GeminiPart, GeminiUsage,
};
pub use http::{CurlJsonHttpTransport, JsonHttpRequest, JsonHttpResponse, JsonHttpTransport};
pub use openai::{OpenAiLiveResponsesConfig, OpenAiResponsesAdapter};
pub use openai_compatible::{
    OpenAiCompatibleResponsesAdapter, OpenAiContentPart, OpenAiInputMessage, OpenAiResponsesConfig,
    OpenAiResponsesRequest, OpenAiResponsesResponse, OpenAiResponsesTransport,
    OpenAiResponsesUsage, OpenAiTextFormatHint, OpenAiToolArgumentSink, OpenAiWireOutputItem,
};
