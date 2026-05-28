# Changelog

## Unreleased

No unreleased changes yet.

## 0.1.0-alpha.3

Status: third public alpha crates.io release.

### Added

- Added live-provider, typed-output, tool-approval, and memory-compaction quickstarts that show the canonical runtime, output-contract, package, policy, journal, event, effect, context, and projection paths with real provider onboarding first.
- Added provider tool-call DTOs and terminal stream deltas so model-requested tools can be represented by `ProviderAdapter` output without adapter-specific callback shapes.
- Added the first app-facing model-tool continuation path: `run_text` can lower provider `tool_use` responses through `ToolRoute`, policy, journal intent/result, executor output, and a provider tool-result continuation.
- Added toolkit-owned `Tool`, `AsyncTool`, and `ToolPackBuilder::listen*` wrappers that declare tools ergonomically while lowering into core tool-pack snapshots, capabilities, sidecars, and routes without executing outside the canonical coordinator.
- Added toolkit-owned `EnvironmentRuntime` aliases and `AgentWorkspaceEnvironmentProfile::runtime(...)` so common environment profiles can select stable isolation runtime refs while still lowering into core environment contracts without registering or starting adapters.
- Added live OpenAI Responses, Anthropic Messages, and Gemini generateContent adapters in the optional `agent-sdk-provider` crate, plus transport-injected deterministic tests. The adapters map canonical `ProviderRequest`/`ProviderResponse`, usage, text output, structured-output hints, and function-call tool requests without owning runtime policy, journals, events, approval, or tool execution.
- Added a persistence ownership map that separates journal, checkpoint, content, event cursor, agent-pool, provider-argument, and tool-execution storage responsibilities before any durable store crate is added.

### Changed

- Released `0.1.0-alpha.3` because `ProviderStructuredOutputHint` now carries optional provider-projected redacted inline schema material for live structured-output hints.
- Synced current public docs to the `0.1.0-alpha.3` crate family and clarified that optional provider, MCP, browser, OTel, isolation, and workflow work belongs in adapter crates layered over `agent-sdk-core`.

## 0.1.0-alpha.2

Status: second public alpha crates.io release.

### Changed

- Added the Rust API Guidelines review gate to the SDK coding standards and reviewer checklist.
- Reduced large public error payloads so common `Result` APIs stay cheaper to pass: `AgentError` and `ContentResolutionError` now box large internal payloads where needed, and structured-output validation returns boxed validation reports on failure.
- Made the remaining Clippy API-shape decisions explicit with local `#[expect(..., reason = "...")]` annotations for durable serde enums and wide lineage constructors that intentionally keep direct record ergonomics.

### Validation

- `cargo clippy --workspace --all-targets -- -D warnings` now passes as a release gate.
- Public API regression tests cover error-size and serialized JSON shape preservation.

## 0.1.0-alpha.1

Status: first public alpha crates.io release.

### Added

- `agent-sdk-core` Rust crate with the product-neutral primitive kernel: typed IDs/refs, runtime packages, content/context records, events, journals, policy, run control, P0 text runs, P1 typed output, P2 side-effect coordination, replay/recovery, scenario tests, and public API docs.
- `agent-sdk-core::testing` namespace with deterministic fake providers, content resolvers, journal stores, event sinks, scripted approval/tool/output/hook/realtime/isolation/extension/telemetry helpers, and conformance-oriented fixtures.
- `agent-sdk-toolkit` optional helper crate for filesystem workspace tools, resource reads, discovery, and shell helper contracts layered over core policy, content refs, capabilities, and effect lineage.
- Golden fixtures for event, journal, package, replay, OTel, extension, output delivery, scenario, privacy, and typed-output contract surfaces.
- Phase-gated implementation reports under `docs/implementation-workstreams`.
- GitHub Actions publish workflow for release-triggered crates.io publication in dependency order.
- Public-repo release audit for personal/sensitive content and `.gitignore` guardrails.

### Package Boundaries

- `agent-sdk-core` has an empty default feature set and does not depend on the optional toolkit crate.
- `agent-sdk-toolkit` depends on `agent-sdk-core`; core never imports toolkit helpers.
- Release metadata is configured for crates.io publication from the public GitHub repository.

### Unsupported

- No live provider adapters are included.
- No concrete container, VM, Firecracker, Docker, Apple Containerization, or remote sandbox adapter is included.
- No product UI, desktop/window, remote-channel, marketplace, or host-specific approval adapter is included.
- No network telemetry exporter, trace-store service, workflow engine, or product-owned memory backend is included.
- Live-provider, concrete-container, product-UI, and host-adapter support must not be claimed without matching contracts, tests, fixtures, and release notes.
