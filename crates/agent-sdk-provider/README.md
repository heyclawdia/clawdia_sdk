# agent-sdk-provider

`agent-sdk-provider` is the optional aggregate crate for model-provider adapters
that should not live in `agent-sdk-core`.

Current live adapters:

- `OpenAiResponsesAdapter`
- `AnthropicMessagesAdapter`
- `GeminiGenerateContentAdapter`

Compatibility and test support:

- `OpenAiCompatibleResponsesAdapter`
- `JsonHttpTransport`
- `ProviderToolArgumentSink`

All adapters implement `agent_sdk_core::ProviderAdapter`. They map provider
request/response DTOs, usage, text, structured-output hints, and model-requested
tool calls, but they do not own runtime policy, journals, events, approval,
tool execution, billing, provider-selection UI, or product routing decisions.

## Quick Start

```rust
use agent_sdk_provider::{
    AnthropicMessagesAdapter, GeminiGenerateContentAdapter, OpenAiResponsesAdapter,
};

let openai = OpenAiResponsesAdapter::from_env("gpt-4.1")?;
let anthropic = AnthropicMessagesAdapter::from_env("claude-sonnet-4-5")?;
let gemini = GeminiGenerateContentAdapter::from_env("gemini-2.5-flash")?;
```

Environment variables:

- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `GEMINI_API_KEY`

The default live transport uses the system `curl` executable and remains
synchronous. Hosts that need a different HTTP stack can inject a
`JsonHttpTransport` through each adapter's `with_transport` constructor.

## Boundary Rules

- API keys are resolved by the host and kept out of runtime package
  fingerprints, journals, events, and content refs.
- Raw provider tool-call arguments should go through `ProviderToolArgumentSink`
  so executors resolve them via normal content policy.
- Provider-native structured-output hints use redacted inline schemas when the
  `OutputContract` exposes one. Local SDK validation remains authoritative.
- Provider adapters may call model APIs. They must not bypass policy, approval,
  tool routing, journal append, event publication, telemetry redaction, or
  output-delivery contracts.

## Not Included

This crate does not include product-specific provider selection UI, managed
credentials, remote MCP servers, concrete isolation runtimes, marketplace
installers, durable workflow engines, or telemetry exporters.
