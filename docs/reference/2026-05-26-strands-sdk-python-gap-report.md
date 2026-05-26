# Strands SDK Python Gap Report

External target: `strands-agents/sdk-python` at commit `f6c3b571eda8e5ae2eeb3c997db5d1f7bc2ed986`.

Local target: this Rust-first Agent SDK packet and current implementation docs.

Current implementation note: the first adoption slice after this report added
provider tool-call DTOs, `ProviderStopReason::ToolUse`, terminal stream tool-call
deltas, and a non-streaming `run_text` continuation path that lowers provider
tool requests through `ToolRoute`, policy, journal intent/result records,
`ToolExecutor`, and a provider tool-result continuation message. The remaining
tool-loop gaps are streaming tool-call input deltas, concurrent execution,
provider-native function-result replay, richer model/tool hooks in the everyday
loop, and streaming provider adapters.

Toolkit ergonomics note: `agent-sdk-toolkit` now includes data-only `Tool`,
`AsyncTool`, and `ToolPackBuilder::listen*` wrappers. They improve declaration
ergonomics while still lowering into core `ToolPackSnapshot`, capabilities,
sidecars, and `ToolRoute` values; they do not execute tools directly.

Memory ergonomics note: the examples now include a small memory-compaction
quickstart that treats memory as a host-owned port returning
`ContextContribution` candidates, not as a shadow transcript or second context
store.

Provider adapter note: the repository now includes an unreleased optional
`agent-sdk-provider` crate with live OpenAI Responses, Anthropic Messages, and
Gemini generateContent adapters, plus transport-injected deterministic tests.
It maps canonical provider requests, text/usage responses, structured-output
hints, and function-call tool requests through `ProviderAdapter`; it
deliberately does not own runtime policy, journals, events, approval, or tool
execution.

## Executive Summary

This SDK has the stronger long-term kernel: explicit events, durable journals, typed IDs, policy boundaries, privacy classes, package fingerprints, deterministic fakes, and a cleaner product-neutral split between core primitives and optional adapters. That is the right foundation for a standalone SDK.

The critical gap is that the current implementation still reads more like a well-specified kernel plus partial feature ports than a complete, batteries-usable agent runtime. Strands is less cleanly neutral and more provider/product shaped, but it already has a complete agent loop: model stream, tool-use parsing, concurrent tool execution, tool-result recursion, structured-output-as-tool, session persistence, MCP tools, provider adapters, hooks, multi-agent graph/swarm helpers, and telemetry spans.

The shortest useful critique is this: this SDK has better contracts, but Strands has more runnable surface area. To win, this SDK should keep the kernel discipline while adding thin optional crates that make the canonical contracts actually operable.

## Highest-Priority Gaps

### 1. Model-Driven Tool Use Has a First Bridge, Not Yet the Full Loop

Strands centers its runtime around a recursive event-loop cycle: call the model, stream deltas, detect tool use, execute tools, append tool results, and recurse until stop. Tool calls can run concurrently, stream tool events, retry through hooks, interrupt, or cancel.

This SDK has strong side-effect primitives: `ToolExecutionCoordinator`, tool intents/results, approval policy, hook records, journal records, and event framing. The first bridge now exists for non-streaming runs: provider output can carry canonical tool calls, `run_text` can execute them through the SDK tool spine, append durable tool records, add a tool-result continuation message, and call the provider again. The remaining gap is the Strands-level loop depth: streamed tool-call input deltas, concurrent tool execution, provider-native structured-output/tool strategies, richer model/tool hooks, and cancellation/interruption during the loop.

Recommendation: harden the P2 loop into the canonical advanced path by adding streaming support, bounded concurrency, hook activation, cancellation/retry policy, and richer tool-result projection. Keep ergonomic helpers thin, and make each helper visibly lower into this same route, policy, journal, event, and effect path.

### 2. Provider Adapters Have a First Live Surface, Not a Full Ecosystem

Strands ships model adapters for Bedrock, Anthropic, Gemini, LiteLLM, Llama API, llama.cpp, Mistral, Ollama, OpenAI, OpenAI Responses, SageMaker, Writer, and custom providers. Its model layer handles content blocks, tool specs, usage metadata, streaming, token counting, and provider-specific message conversion.

This SDK has a provider port, model capability descriptions, deterministic fakes, and now a first optional aggregate provider crate with live OpenAI, Anthropic, and Gemini adapters. The shape is right because each adapter still lowers function calls into `ProviderToolCall` without bypassing core policy or tool execution. The remaining adoption gap is still large: there is no provider model catalog, streaming compatibility profile, token counting, Bedrock/local adapter set, or provider-native function-result replay.

Recommendation: harden `agent-sdk-provider` with a model catalog, streaming/tool-call fixtures, retry classification, redaction behavior, optional live smoke tests, and provider-native function-result continuation support. Add one local/offline adapter next. Do not put provider dependencies in the core crate.

### 3. Live MCP Tooling Is Missing

Strands has a concrete MCP client that acts as a tool provider, manages lifecycle, lists tools with filters/prefixes, reads resources, lists prompts/templates, calls tools, handles output schemas, and supports task-oriented MCP flows.

This SDK has protocol test helpers and capability boundaries in the toolkit, but not a live MCP client/tool provider. That leaves a major practical tooling gap because MCP is now table stakes for agent SDK adoption.

Recommendation: add an optional MCP adapter crate that lowers remote tool calls into SDK effect intents and tool results. It should expose filtering, namespacing, prompt/resource refs, task polling, timeout/cancellation, elicitation handling, and exact journal/event evidence.

### 4. Session and Conversation Persistence Need an Ergonomic Layer

Strands treats sessions as hook providers. Its file and S3 session managers persist agents, messages, multi-agent state, and invocation state. It also repairs malformed histories such as orphaned tool-use/tool-result pairs before sending them back to a model.

This SDK has the better durable truth model in `RunJournal`, checkpoints, trace records, IDs, and session/turn fields. What is missing is a product-neutral session manager/repository API that ordinary users can turn on without designing their own journal projection layer.

The new [persistence ownership map](persistence-ownership-map.md) now makes the store split explicit: journals are truth; checkpoints, content refs, event archives, agent-pool stores, provider argument sinks, and tool-execution caches are separate projections or adapters. Current concrete support is still uneven: checkpoint and agent-pool stores exist in memory/SQLite forms, but durable journal, content, event archive, and session repositories are still missing.

Recommendation: add optional file and SQLite store crates first, then a session crate over those stores. The session layer should project from journals, never replace journals as truth, and provide conversation-window repair, pagination, snapshot/restore, and context projection hooks.

### 5. Context Compaction Is More Specified Than Usable

Strands includes conversation managers and context-window overflow recovery. The loop can catch context overflow, reduce context, and retry. This is not as architecturally clean as this SDK's projection model, but it is usable.

This SDK has `ContextContribution`, `ContextProjection`, privacy-aware summaries, compaction policy concepts, and reference-based context. It now has a small memory-compaction quickstart that shows the right mental model, but the current common run shape is still text-input oriented and does not yet expose a concrete history window, summarizer, overflow recovery loop, or model-facing content-block projection.

Recommendation: implement a canonical context assembler that can read session history, content refs, tool results, and summaries; apply a token budget; emit a deterministic projection; and journal what was included, omitted, summarized, or redacted.

### 6. Streaming and Realtime Are Not Yet First-Class in the Canonical Run Path

Strands normalizes text, reasoning, citations, tool-use deltas, usage, and final messages into a streaming event surface. It also has experimental bidirectional streaming.

This SDK has stream rules, event frames, realtime records, subscriptions, and provider stream ports. The provider stream delta type and P0 loop are still too narrow for Strands-level behavior: text/usage/error deltas are not enough for modern tool-calling agents, and the common run path does not visibly consume provider streaming.

Recommendation: add a canonical `run_stream` path that emits typed model deltas, tool-call input deltas, reasoning/citation metadata where supported, rule interventions, terminal events, and journal cursors. The stream should be live convenience; the journal remains durable truth.

### 7. Hooks Are Stronger on Paper Than in the Active Loop

Strands has straightforward hooks for invocation, model call, tool call, message-added, multi-agent nodes, and cleanup. Hook events expose writable fields through typed event objects.

This SDK has a more rigorous hook contract: hook specs, mutation rights, package fingerprints, journal-before-apply behavior, and policy-scoped mutation. The active P0/P1 path only supports a small subset of those hook points. Model-call and tool-call hooks are not yet part of the everyday canonical path.

Recommendation: stage hook activation with explicit readiness levels: P0 terminal/context hooks, P1 validation hooks, P2 model/tool hooks, and feature-layer multi-agent hooks. Add a simple builder API for registering hooks while preserving the canonical `HookSpec` underneath.

### 8. Tool Ergonomics Have a Declaration Layer, But Dynamic Loading Is Still Thin

Strands makes simple tools easy: `@tool`, function tools, module/file tools, directory hot reload, tool name normalization, conflict checks, direct tool invocation, and agent-as-tool.

Rust should not copy Python's ambient dynamic execution model into core. The toolkit now has an initial declaration layer for `Tool`, `AsyncTool`, and pack `listen*` registration, which is the right scope because it stays data-only and lowers to core routes. The remaining gap is typed function adapters, schema derivation, executor registration ergonomics, and dev-time reload through an explicit host/toolkit policy.

Recommendation: add typed function-tool builders or macros in an optional ergonomics crate or toolkit feature, plus a dev-only tool-pack loader that records package deltas and requires explicit host policy. Agent-as-tool should lower into subagent/AgentPool primitives with journaled invocation and cancellation.

### 9. Multi-Agent Helpers Are Lower-Level Than Users Expect

Strands provides graph orchestration, swarm collaboration, shared context, handoff tools, multi-agent hooks, session persistence, and A2A executor support.

This SDK has the better primitive direction: `AgentPool`, messages, wake conditions, subscriptions, typed addresses, and subagent lowering. That is more durable than baking graph/swarm semantics into core. The missing layer is a small optional orchestration package that proves those primitives can support real graph, handoff, and A2A flows.

Recommendation: keep `AgentPool` primitive and product-neutral. Add optional graph/handoff helpers as lowering layers over messages, wakes, journals, and pool-scoped policy. Treat A2A/ACP adapters as edge crates, not core behavior.

### 10. Public Code Organization Needs Sharper Boundaries

Strands is large, but its major concepts are easy to find: `agent`, `event_loop`, `models`, `tools`, `hooks`, `session`, `multiagent`, `telemetry`, and `plugins`.

This SDK is domain-organized, but some files and public facades are too large for future contributors to navigate. The core facade is long, and several implementation files combine too many responsibilities. This clashes with the repo's own rule that `mod.rs` and facades should stay small and behavior should live in meaningfully named files.

Recommendation: split the largest runtime files by responsibility. Good first targets are the run loop, agent pool, stream records, event records, isolation package logic, and package hook logic. Preserve stable public exports, but make the internal paths match the operation a future agent will search for.

## Secondary Gaps

- Human-in-the-loop interrupts and resume should become a visible runtime state, not only an approval/tooling concern.
- Structured output should support both local validation/repair and provider/tool-native structured-output strategies.
- Tool execution should support streaming tool output and partial progress, not only final tool results.
- Telemetry needs an optional concrete OpenTelemetry exporter and content-capture presets.
- Built-in example coverage should include one complete provider-plus-tool-plus-session scenario once provider adapters exist.
- Snapshot/restore should have a one-line ergonomic path that projects from journal/checkpoint truth.
- Token counting should be a real provider capability with deterministic fallback, not just a future adapter concern.
- Runtime errors need a user-facing classification guide that maps retryable, policy-denied, provider, parser, cancellation, and replay failures.

## Things This SDK Is Already Doing Better

- Product neutrality: the core is not tied to AWS, Bedrock, a UI product, a cloud store, or a default provider dependency tree.
- Durability: events, journals, effect intents/results, typed IDs, and replay posture are much stronger than a callback-first loop.
- Privacy: content refs, redacted summaries, privacy classes, and opt-in raw content are a better default than provider-facing message mutation.
- Policy boundaries: host-owned authority, approval brokers, isolation requirements, and package fingerprints are clearer than Strands' more permissive dynamic runtime.
- Testability: deterministic fakes, conformance fixtures, and public testing helpers are a stronger foundation for adapter compatibility.
- Toolkit file reading: the format-aware workspace reader pipeline is more disciplined than a generic text-read tool surface.
- Multi-agent foundation: `AgentPool` as a coordination scope over events and journals is cleaner than making graph/swarm semantics a core primitive.

## Suggested Roadmap

1. Harden the new P2 model-tool bridge into the canonical advanced loop with streaming, concurrency, hooks, cancellation, and richer projection.
2. Add provider streaming/tool-call deltas and a `run_stream` API backed by journal cursors.
3. Harden the live provider adapters with a model catalog, complete projection, streaming support, retry classification, and opt-in live smoke.
4. Ship a live MCP adapter crate that lowers into effect intents and tool results.
5. Add a session repository layer that projects from journals and supports file/SQLite persistence.
6. Add context-window assembly, compaction, overflow retry, and projection evidence.
7. Activate model/tool hooks in the canonical loop with clear mutation rights.
8. Add typed tool ergonomics and dev-only explicit tool-pack loading.
9. Split oversized runtime modules while preserving public exports.
10. Add optional graph/handoff/A2A helpers over `AgentPool`, not inside the kernel.

## Bottom Line

This SDK should not chase Strands by becoming a Python-style dynamic runtime or by putting provider/tool dependencies into core. The better move is to keep the kernel strict and make the missing operational layers real: provider adapters, live MCP, session repositories, streaming, context compaction, and model-driven tool recursion.

The most critical issue to fix is not naming or docs polish. It is making the SDK's strongest primitives feel runnable on day one: provider adapters, the new model-tool bridge, session persistence, context compaction, MCP/web tooling, and streaming must become small, well-lit paths rather than architecture-page discoveries.
