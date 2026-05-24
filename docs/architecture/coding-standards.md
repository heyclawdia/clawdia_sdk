# Agent SDK Coding Standards

These standards apply to the future Rust-first `agent_sdk` design. They are derived from the Phase 1 SDK study, TDD, DDD, Rust async practice, and observability requirements.

## Architecture Standards

- Design from domain primitives first: run, runtime package, content/artifact refs, context contribution/admission/projection, model call, effect intent/result, policy, event, journal, and typed ports.
- Keep `agent_sdk` product-neutral. UI shells, coding-agent harnesses, remote messaging products, desktop windows, and cloud sandboxes belong in adapters or host crates.
- Keep product-specific decisions host-owned: provider selection UI, external-runtime session caches, restore keys, trace ingestion policy, extension runtime packaging, and approval transport.
- Prefer typed snapshots over ambient lookup. `RuntimePackage` should be the source of truth for model-visible tools, executable tool registry, hooks, policies, MCP exposure, subagent definitions, and extension capabilities.
- Prefer explicit state machines over hidden recursion. Recursive continuation is allowed, but it must be represented as loop state and observable events.
- Keep lossless internal records separate from provider projection. Provider adapters may drop or transform fields only through explicit projection steps.
- Treat APIs as contracts, not convenience helpers. Every public type needs a clear owner, methods, invariants, and "must not own" boundaries.
- Model failure as part of the domain: cancellation, timeout, max iterations, context overflow, interrupted tools, rejected approvals, bad provider chunks, partial streams, replay conflicts, and recovery attempts.

## Test-First Standards

Phase 1 is documentation-only, but phase 2 should start with tests before implementation.

- Start with small Rust unit tests for state transitions: input accepted, model requested, tool requested, approval required, tool result appended, continue, stop, cancel, compact, retry, and recover.
- Add golden event tests for every `AgentEvent` variant so event names, causal IDs, and redaction rules remain stable.
- Add provider-projection tests that prove internal metadata is stripped or transformed exactly once before a provider call.
- Add structured-output tests that pass user/host schemas, validate locally, retry with repair prompts on invalid output, and commit only validated typed results.
- Add stream-rule tests for literal/regex matching over assistant text, provider-exposed reasoning summaries, tool-call arguments, tool results, and realtime transcript channels. Tests should prove stop, abort-and-retry, mask, approval, repeat policy, resume state, and redacted match events.
- Add built-in tool-pack contract tests with mock workspaces: read/search anchors, truncation metadata, glob handling, edit precondition failures, reversible-effect metadata, write approval, shell sandbox denial, resource URI resolution, and tool discovery activation.
- Add isolation-runtime contract tests with fake adapters: environment capability negotiation, image/rootfs resolution, mount expansion audits, network denial, process I/O redaction, signal/timeout handling, stats, cleanup, and resume after partial lifecycle states.
- Add session/replay tests using append-only journals, deterministic IDs, and mock providers.
- Add policy tests for approval modes, permission modes, sandbox modes, MCP allowlists, source-scoped approvals, and escalation timeouts.
- Add subagent tests for bounded depth, no recursive subagent tools by default, parent-owned cancellation, event wrapping, and trace lineage.
- Add realtime tests with bounded channels and fake audio/image/text streams to prove backpressure and restart behavior.
- Add extension SDK smoke tests for public subpath exports, packaged fallback ordering, browser-safe helpers, and temp-directory execution.
- Keep mock providers fast and deterministic. Live-provider tests should be opt-in and never required for local unit confidence.

## DDD Standards

- Domain layer owns the vocabulary: `RunId`, `TurnId`, `MessageId`, `ContextItemId`, `ToolCallId`, `SpanId`, `AgentId`, `SessionId`, `RuntimePackageId`, and `LineageRef`.
- Application layer coordinates a run using domain services and ports. It should not parse strings to infer tool risk, provider routing, message source, or approval state.
- Infrastructure implements provider adapters, MCP clients, file stores, remote channels, OTLP exporters, process runners, and extension hosts.
- Host/application crates decide which adapters, policies, and sinks are active.
- Extension APIs expose bounded capabilities. Extensions can observe and propose, but cannot silently become approval, memory, provider-routing, or telemetry owners.

## Rust API Standards

- Use `Result<T, AgentError>` for fallible SDK operations. Error variants should preserve typed context and causal IDs.
- Prefer enums for finite state: stop reasons, approval decisions, permission outcomes, stream item kinds, recovery actions, and policy decisions.
- Use newtype IDs instead of raw strings at public boundaries.
- Use `serde` for durable contracts and maintain schema versions for journals, events, snapshots, and runtime packages.
- Prefer `Arc<dyn Trait + Send + Sync>` only at stable dynamic boundaries such as providers, stores, hooks, and telemetry sinks. Use generic/static composition inside hot paths where practical.
- Avoid cloning large message/media payloads in loops. Use IDs, `Arc`, borrowed projections, or media references where possible.
- Separate hot event emission from slow sink delivery. The loop should enqueue lightweight events and never block on network telemetry export.
- Make cancellation explicit and cheap to check. Long-running provider streams, tools, subagents, and realtime tasks must receive cancellation handles.
- Require bounded channels for streaming and realtime paths unless a host explicitly opts into an unbounded sink for testing.

## Observability Standards

Observability is a core SDK feature, not a plugin.

- Every run has a `RunId`, trace context, root agent identity, runtime package fingerprint, and host source.
- Every turn has a `TurnId`, parent `RunId`, input summary, context projection ID, and outcome.
- Every message has a `MessageId`, role, parts, lineage, sensitivity, retention, and provider-projection status.
- Every context item comes from a `ContextContribution` and carries source, injection path, selection decision, policy refs, privacy, retention, trust, and lineage: user, system, developer, memory, compaction, hook, extension, tool result, remote channel, scheduled task, subagent, external runtime, or replay.
- Every event has causal and filterable links: `run_id`, `agent_id`, optional `turn_id`, optional `attempt_id`, optional `message_id`, optional `context_item_id`, `span_id`, optional `parent_event_id`, `subject_ref`, `related_refs`, `causal_refs`, source, destination, correlation keys, tags, privacy class, and delivery semantics. Tool calls, hooks, approvals, subagents, isolated processes, extension actions, output deliveries, and effects use `EntityRef` rather than feature-specific envelope IDs.
- Events should support content elision by default. The event must still be useful when raw content is absent.
- Telemetry must align with OpenTelemetry GenAI concepts for agent, model, tool, usage, MCP, and errors while retaining SDK-specific lineage fields.
- Tool and model retries must be observable as attempts, not overwritten history.
- Multi-agent handoffs must preserve who instructed whom, what message/context was passed, and which policy allowed the handoff.
- Message metadata must never leak to provider adapters unless a projection explicitly maps a safe subset.
- Structured output validation must happen inside the SDK after provider output, even when the provider claims native schema support. Validation failures and repair retries must be observable as attempts.
- Stream-rule matching and interventions must be observable as their own events. A stopped or retried model attempt should explain which rule matched, which channel it watched, which action was applied, and how matched content was redacted.
- Isolated execution must emit environment lifecycle and process events. A shell/code-execution result should explain the adapter, capability report, image/rootfs refs, mount/network policy, process status, stats, and cleanup status without logging raw process I/O by default.

## Approval And Policy Standards

- Approval is a broker/policy decision, not a UI event.
- Permission, sandbox, MCP allowlist, allowed tools, YOLO/autonomy, and escalation are separate policy layers.
- Decisions are finite: allow, deny, ask, modify, defer, or interrupt.
- Desktop, CLI, external-runtime, headless, extension-submitted, and source-scoped contexts use the same decision model with different dispatchers.
- Headless approval parks the broker receiver, uses explicit host-owned escalation channels, accepts exact finite tokens, and denies on timeout.
- Extensions can request tools and observe decisions only through host-owned APIs. They cannot approve themselves.

## Performance And Robustness Standards

- Event emission must be low allocation and non-blocking on slow observers.
- Streaming must be backpressure-aware. If a sink is slow, apply a declared overflow policy: block, drop noncritical progress, summarize, or fail.
- Context assembly must be bounded by token, byte, item count, media budget, and time budget.
- Tool execution must have explicit concurrency, ordering, timeout, retry, and cancellation policies.
- Realtime media streams must use references or bounded buffers instead of copying large base64 payloads through every layer.
- Streaming regex/literal matchers must use bounded rolling windows, compile-time validation, timeout/backtracking protection, and channel-specific privacy controls.
- Built-in tool packs must be bounded by workspace, file size, match count, byte limits, process timeout, and cancellation policy. Mutating packs must journal intent and before/after effect metadata before applying changes.
- Isolation adapters must declare health and capabilities before use. Environment preparation, image pulls, rootfs creation, first-run kernel/init artifact fetches, process startup, stats collection, and cleanup must be bounded, cancellable where possible, and journaled.
- Journals and checkpoints should be append-first and compact later.
- Recovery should be explicit: replay from journal, resume from checkpoint, regenerate provider projection, retry safe step, or surface repair-needed.
- Invariant checking and anti-entropy jobs should be separate from the hot loop.

## Documentation Standards

- Every future implementation slice should update architecture docs and risk notes before code lands.
- Conceptual Rust examples in docs must be labeled non-compiling sketches until they become crate APIs.
- Any change to public events, journals, snapshots, or extension contracts needs a migration note and compatibility strategy.
- Source links in docs should prefer primary docs, repos, or specifications.
