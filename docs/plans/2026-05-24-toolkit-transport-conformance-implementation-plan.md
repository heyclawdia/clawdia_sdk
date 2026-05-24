# Toolkit Transport Conformance Implementation Plan

Date: 2026-05-24

## Objective

Implement the first Rust slice of the toolkit adapter plan for ACP/MCP transport
mocks and isolation-composed conformance tests. This slice should make future
adapter work testable through encoded JSON-RPC frames instead of direct helper
calls, while keeping `agent-sdk-core` product-neutral.

## Relevant Existing Context

- `AGENTS.md`: do not create branches; keep the SDK product-neutral; implementation
  contracts must preserve SDK-owned and host-owned boundaries.
- `coding_standards.md` and `docs/architecture/coding-standards.md`: mockability
  and reusable SDK-consumer conformance helpers are required; optional crates must
  keep small public facades and deterministic fakes.
- `docs/workstreams/validation-gates.md`: implementation goals need tests,
  commands, primitive-lowering evidence, and host-boundary evidence.
- `docs/reference/sdk-review-checklist.md`: reject mini-SDK drift, ambient power,
  live-service-first testing, and hidden product behavior.
- `docs/architecture/primitive-map.md`: ACP/MCP/isolation behavior must layer on
  existing ports, `RuntimePackage`, policy refs, content refs, effect records,
  events, and journals rather than adding new core primitives.
- `docs/agent-sdk-toolkit/adapter-and-runtime-plan.md`: ACP and MCP conformance
  fakes must cross JSON-RPC transport boundaries; isolation runtime adapters must
  fail closed and be fake-testable before live runtimes.
- Official ACP and MCP transport docs: local subprocess transports use UTF-8
  JSON-RPC over stdio, one message per newline-delimited line, with no embedded
  newlines and no non-protocol stdout.
- `crates/agent-sdk-core/src/application/isolation.rs` and existing isolation
  tests already prove core journal-before-adapter behavior, downgrade denial,
  cleanup repair, and redacted I/O.
- `crates/agent-sdk-toolkit` is the optional crate that currently owns concrete
  toolkit helpers and public testing utilities.

## Behavior Contract

New behavior:

- `agent-sdk-toolkit` exposes reusable JSON-RPC line transport test helpers,
  including a stdio-style codec and endpoint that write and read
  newline-delimited bytes.
- JSON-RPC responses always include an `id`, using `null` for parse/invalid
  frames where the request ID is unavailable, and invalid response shapes are
  rejected.
- ACP mock client/agent helpers exchange encoded JSON-RPC frames for
  `initialize`, `session/new`, `session/prompt`, `session/cancel` notification,
  agent-to-client `fs/read_text_file`, `terminal/create`,
  `session/request_permission`, and malformed protocol cases.
- MCP mock client/server helpers exchange encoded JSON-RPC frames for
  `initialize`, `notifications/initialized`, tool discovery, allowed tool calls,
  unselected tool/resource/prompt filtering, resource reads, lifecycle ordering,
  server-to-client sampling/elicitation denial, and malformed protocol cases.
- Isolation-composed tests start from a fake isolation/runtime launch step before
  exercising ACP or MCP protocol frames, proving the protocol mock is not a
  direct in-process helper.
- Tests expose transcript/raw-line evidence that communication crossed serialized
  JSON-RPC frames.
- Scripted protocol fakes live under the toolkit `testing` namespace, while
  production-facing wire primitives live under `protocol`.

Preserved behavior:

- `agent-sdk-core` remains untouched unless a missing primitive is proven.
- The toolkit remains optional and does not own a run loop, package registry,
  event stream, journal, policy path, credential store, or product host adapter.
- No live provider, real editor, real MCP server, real browser, or real container
  is required for conformance.

Removed behavior:

- None.

Tests proving behavior:

- `cargo test -p agent-sdk-toolkit --test adapter_conformance`
- `cargo test -p agent-sdk-toolkit`
- `cargo fmt --check`

## Scope

Writable files for this slice:

- `docs/plans/2026-05-24-toolkit-transport-conformance-implementation-plan.md`
- `crates/agent-sdk-toolkit/src/lib.rs`
- new modules under `crates/agent-sdk-toolkit/src/` for protocol/conformance
- new tests under `crates/agent-sdk-toolkit/tests/`
- `crates/agent-sdk-toolkit/README.md` if public imports change

Out of scope for this slice:

- Live ACP editor integration.
- Live MCP server integration.
- Concrete container/VM runtime adapters.
- Product-specific host adapters, UI, credentials, or memory policy.
- Core primitive changes unless tests prove a missing contract.
- Branch creation, push, publish, or tags.

## Workstreams

1. Add a reusable JSON-RPC line transport with raw transcript capture.
2. Add ACP and MCP scripted conformance helpers over that transport.
3. Add isolation-composed tests using existing core fake isolation runtime types.
4. Update public facade/README for the new optional toolkit testing surface.
5. Run focused tests, formatting, and a review pass against SDK review checks.

## Risk / Gotcha Carry-Forward

- Do not let protocol mocks call server handlers directly; tests must assert raw
  JSON-RPC lines were serialized and parsed, and that line frames reject embedded
  newlines.
- Do not model ACP as a raw model provider or second run loop.
- Do not expose an entire MCP server when one tool/resource was selected.
- Do not return unfiltered MCP discovery responses from the host proxy; sibling
  tools/resources/prompts must be hidden before callers inspect the response.
- Do not put scripted fakes in large catch-all `mod.rs` files; use meaningful
  protocol and testing module files.
- Do not pass raw MCP roots, auth, sockets, session IDs, editor handles, or host
  paths into isolation mocks.
- Do not claim fake host-process launch is secure isolation. It is conformance
  scaffolding only.
- Do not add product-specific command examples or host behavior.
- Do not make `agent-sdk-toolkit` a central toolkit authority.

## Review Packet

Primitive decision:

- Reused kernel primitives: `IsolationRuntime`, `ExecutionEnvironment`,
  `ProcessStartRequest`, `EffectIntent`, `ContentRef`, policy refs, and typed
  IDs where isolation composition is needed.
- New feature-layer primitives: none in core. Toolkit adds optional conformance
  helper structs only.
- New capability variants: none.
- Host-owned behavior kept out: editor UI, MCP server installation, auth,
  concrete runtime installation, credentials, product memory, and approval UX.

Validation evidence:

- Contract/unit tests: toolkit adapter conformance tests.
- Golden fixtures: not added in this slice; raw transcript assertions are the
  current proof surface.
- Smoke/scenario tests: no live smoke; fake transport and fake isolation only.

Reviewer checklist:

- Simplicity: helpers are small and transport-focused.
- Product-neutrality: no product names or host-specific adapters.
- Mockability: SDK consumers can reuse helpers without live services.
- Event/journal durability: no side-effect path is introduced; isolation setup
  composes with existing core fake isolation records.
- Privacy/redaction: transcripts contain protocol metadata and test text only,
  no credentials or raw host paths.
