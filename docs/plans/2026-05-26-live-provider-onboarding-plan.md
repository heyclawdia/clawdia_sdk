# Live Provider Onboarding Plan

Date: 2026-05-26

## Goal

Move SDK adoption to live-provider onboarding by implementing real provider
adapters in the existing aggregate `agent-sdk-provider` crate and making the
first documentation path use live provider setup.

## Current Findings

- `README.md`, `docs/examples/README.md`, `crates/agent-sdk-provider/README.md`,
  and several handoff notes still said the provider surface was only
  deterministic-test oriented.
- `crates/agent-sdk-provider` currently contains only
  `OpenAiCompatibleResponsesAdapter` over a host-injected transport.
- Core already has the right boundary: providers implement `ProviderAdapter`;
  runtime policy, journal, event publication, approval, tool routing, output
  validation, and effects stay outside the adapter.
- Core can parse provider-returned tool calls through `ProviderToolCall`, and
  the P0 runtime can continue after tool execution. Provider-native tool schema
  projection is still thin because provider-visible package projections carry
  capability refs and policy refs, not materialized JSON schemas.

## External API Basis

- OpenAI Responses API: `POST /v1/responses`, `input`, `text.format`, `tools`,
  `tool_choice`, usage fields, and Responses function-calling flow.
- Anthropic Messages API: `POST /v1/messages`, `model`, `max_tokens`,
  `messages`, `system`, usage fields, and top-level `tools` with `input_schema`.
- Gemini generateContent API: `models/{model}:generateContent`, `contents`,
  `systemInstruction`, function declarations, function call parts, and function
  response continuation.

References used:

- https://platform.openai.com/docs/api-reference/responses
- https://developers.openai.com/api/docs/guides/function-calling
- https://platform.claude.com/docs/en/build-with-claude/working-with-messages
- https://platform.claude.com/docs/en/agents-and-tools/tool-use/define-tools
- https://platform.claude.com/docs/en/build-with-claude/structured-outputs
- https://ai.google.dev/gemini-api/docs/function-calling
- https://ai.google.dev/gemini-api/docs/structured-output

## Decisions

- Do not create `agent-sdk-provider-openai`,
  `agent-sdk-provider-anthropic`, or `agent-sdk-provider-gemini` in this slice.
  Use the existing aggregate `agent-sdk-provider` crate with provider-specific
  modules and stable re-exports.
- Do not delete deterministic fake/scripted transports. They are conformance
  and CI tools, not onboarding strategy.
- Do not make live network calls part of default tests. Default validation uses
  scripted HTTP transports and checked response fixtures; optional live smoke
  can be env-gated later.
- Do not move credentials into core, runtime packages, events, journals, or
  fingerprints. Adapters may hold runtime API-key values, but docs must show
  host-owned env resolution and redacted debug behavior.
- Do not claim complete native provider tool schema support unless this slice
  materializes schemas in the provider request contract. If that grows too large,
  implement live text/typed-output first and keep provider-native tool schema
  projection as an explicit follow-up.

## Implementation Scope

### Provider Crate

- Add a small provider HTTP boundary in `crates/agent-sdk-provider`:
  - redacted API-key wrapper;
  - JSON HTTP request/response DTOs;
  - transport trait for deterministic tests;
  - live blocking transport implementation.
- Add actual adapters:
  - OpenAI Responses adapter with live endpoint defaults and request/response
    mapping;
  - Anthropic Messages adapter with live endpoint defaults and request/response
    mapping;
  - Gemini generateContent adapter with live endpoint defaults and
    request/response mapping.
- Preserve the existing OpenAI-compatible/scripted transport path as a
  compatibility and conformance layer, but remove deterministic-test-first
  naming from public docs.
- Map HTTP status failures to `AgentErrorKind::ProviderFailure` with retry
  classification:
  - 408, 409, 429, and 5xx: `Retryable`;
  - 401, 403, 404, malformed JSON, unsupported response shape: `RepairNeeded`
    or `HostConfigurationNeeded` where appropriate;
  - local missing env vars: `HostConfigurationNeeded`.

### Docs And Onboarding

- Replace the README quickstart path with:
  - live provider quickstart;
  - typed-output quickstart using live provider runtime setup;
  - tool-approval quickstart using canonical runtime/tool paths.
- Replace the deterministic provider quickstart with a live provider quickstart
  so deterministic tests are no longer the first adoption story.
- Update `docs/examples/typed-output-quickstart.md`,
  `docs/examples/memory-compaction-quickstart.md`, provider README, toolkit
  roadmap snippets, phase exit notes, and release-readiness matrix to remove
  stale live-provider gap claims.
- Keep docs product-neutral: use generic env variables and no host/product UI.

## Validation

- `cargo test -p agent-sdk-provider --quiet`
- `cargo test --workspace --quiet`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --check`
- `git diff --check`
- `scripts/public-release-audit.sh`

## Review Gate

Before implementation, run an independent plan review against this plan, the
provider contract, and the public-doc constraints. After implementation, run an
independent code/docs review before the final validation pass.

## Review Findings Carry-Forward

- Live transports must not expose API keys through process argv, debug output,
  journals, events, package fingerprints, or docs. The dependency-light curl
  transport passes secret-bearing headers through a private `@file`/FIFO path
  rather than literal `--header` argv values, and hosts can inject their own
  `JsonHttpTransport`.
- Provider DTOs may carry prompt text, schema material, model output, and raw
  tool arguments for provider calls and deterministic fixtures. Public `Debug`
  implementations must summarize these fields and avoid raw content leakage.
- `ProviderStructuredOutputHint` gained optional provider-projected schema
  material, so local manifests and docs must treat this checkout as the next
  `0.1.0-alpha.3` API rather than the published `0.1.0-alpha.2` API.
