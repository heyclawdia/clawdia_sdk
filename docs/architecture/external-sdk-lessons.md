# External SDK Lessons

This note extracts reusable primitives from Strands, Cursor, Claude Agent SDK, Google ADK, OpenAI Agents SDK, Pi, oh-my-pi, Apple Containerization, and OpenTelemetry GenAI conventions. It intentionally avoids cloning any one SDK.

## Source Audit Format

Every source lesson that changes an SDK primitive should be auditable:

| Field | Meaning |
| --- | --- |
| Source URL | Primary documentation or source URL. |
| Date checked | Date the lesson was reviewed. |
| Accepted lesson | Product-neutral primitive or validation rule adopted. |
| Rejected behavior | Product/provider-specific behavior kept out of core. |
| SDK decision | Contract, primitive, workstream, or validation gate changed. |

## Current Source Audit

| Source | URL | Date checked | Accepted lesson | Rejected behavior | SDK decision |
| --- | --- | --- | --- | --- | --- |
| Strands Agents SDK | [session management](https://strandsagents.com/docs/user-guide/concepts/agents/session-management/), [hooks](https://strandsagents.com/docs/user-guide/concepts/agents/hooks/), [swarm](https://strandsagents.com/docs/user-guide/concepts/multi-agent/swarm/) | 2026-05-23 | Typed lifecycle hooks, session/event separation, tool execution visibility, multi-agent lineage. | Python mutability, provider defaults, implicit message mutation, unconstrained agent societies. | Keep typed hooks and subagent lineage, but require package sidecars, mutation rights, policy, events, and journals. |
| Cursor SDK | [SDK release notes](https://cursor.com/changelog/sdk-release) | 2026-05-23 | Agent/run split, run-scoped streaming, cancellation/reconnect, extension surfaces. | Product cloud lifecycle, Cursor workspace/session assumptions. | Split `Agent`/`AgentRuntime`/`RunHandle`; keep deployment/session details host-owned. |
| Claude Agent SDK | [agent loop](https://code.claude.com/docs/en/agent-sdk/agent-loop) | 2026-05-23 | Clear loop lifecycle, tool execution, and context-window pressure. | Claude Code-specific transport, tool vocabulary, and permission defaults. | Keep explicit state machine, typed content refs, and tool policy; do not copy provider vocabulary. |
| Google ADK | [context](https://google.github.io/adk-docs/context/), [sessions](https://adk.dev/sessions/), [events](https://adk.dev/events/), [artifacts](https://adk.dev/artifacts/) | 2026-05-23 | Separate operation context, session/state, memory, events, and artifacts; artifacts are ref-backed content. | Google service integrations, callback names, deployment assumptions. | Add `ArtifactRef` / `ContentRef` and `ContextContribution` -> `ContextItem` -> `ContextProjection`. |
| OpenAI Agents SDK | [context](https://openai.github.io/openai-agents-python/context/), [agents](https://openai.github.io/openai-agents-python/agents/), [running agents](https://openai.github.io/openai-agents-python/running_agents/), [guardrails](https://openai.github.io/openai-agents-python/guardrails/), [streaming](https://openai.github.io/openai-agents-python/streaming/), [results](https://openai.github.io/openai-agents-python/results/), [handoffs](https://openai.github.io/openai-agents-python/handoffs/) | 2026-05-23 | Runtime context differs from LLM-visible context; guardrails are stage-scoped; streams/results/handoffs need explicit terminal and filtering semantics. | Provider-specific API shapes and provider-native policy authority. | Add API tiers, stage-scoped policy checks, terminal stream completion, and explicit `ContextHandoffPolicy`. |
| Pi packages | [package docs](https://pi.dev/docs/latest/packages), [pi-multiagent](https://pi.dev/packages/pi-multiagent) | 2026-05-23 | Package/catalog boundaries, source-qualified package refs, parent-led delegation. | Product harness, install UX, and project package defaults. | Add `CapabilityCatalogSnapshot`; keep package activation through policy and package deltas. |
| oh-my-pi | [repo](https://github.com/can1357/oh-my-pi), [read](https://github.com/can1357/oh-my-pi/blob/main/docs/tools/read.md), [search](https://github.com/can1357/oh-my-pi/blob/main/docs/tools/search.md), [edit](https://github.com/can1357/oh-my-pi/blob/main/docs/tools/edit.md), [stream rules](https://github.com/can1357/oh-my-pi/blob/main/docs/ttsr-injection-lifecycle.md) | 2026-05-23 | Read/search/edit/write/shell/resource tools, anchored edits, hidden discovery, stream interruptions. | Coding-agent UX, prompt recipes, TUI/session UI, marketplace behavior. | Keep optional toolkit packs and stream-rule primitives over package/policy/journal. |
| Apple Containerization | [repo README](https://github.com/apple/containerization) | 2026-05-23 | Explicit image/rootfs/process lifecycle, VM isolation, mount/network capability reports. | macOS/Swift/runtime specifics as core dependencies. | Keep `ExecutionEnvironment` and `IsolationRuntime` portable; concrete runtimes stay adapters. |
| OpenTelemetry GenAI | [GenAI semconv](https://opentelemetry.io/docs/specs/semconv/gen-ai/), [agent spans](https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-agent-spans/), [model/tool spans](https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-spans/), [MCP semconv](https://opentelemetry.io/docs/specs/semconv/gen-ai/mcp/) | 2026-05-23 | Agent/model/tool span vocabulary, usage/cost attributes, MCP conventions, raw content caution. | Treating telemetry as durable run truth or raw content as default. | Telemetry remains a derived projection with content-capture policy and sink failure isolation. |

## Phase 03 Source-Audit Reconciliation

Date: 2026-05-24

Phase 03 reviewed the source-audit rows against the Phase 01 runtime-package spine and Phase 02 primitive-kernel outputs. No source lesson changed the accepted SDK posture: the current rows still support the product-neutral kernel, explicit run/runtime split, context/content-ref separation, event/journal durability split, package/capability sidecar discipline, portable isolation boundary, and telemetry-as-derived-projection rule.

Phase 03 therefore records the source audit as reviewed with no new source rows required. Later feature phases may update the audit only when a source-backed lesson changes a contract, validation gate, or primitive decision.

## Summary Table

| Source | Reusable lessons | Incidental or risky parts |
| --- | --- | --- |
| Strands Agents SDK | Flexible async event loop, typed hooks, hook-driven session management, message metadata stripped before provider calls, tool executors, agent-as-tool, multi-agent events, bidirectional streaming, OTel spans. | Python mutability, product/provider defaults, callback dict compatibility, implicit message mutation, AWS-heavy defaults. |
| Cursor TypeScript SDK | Durable agent/run split, reconnectable streams, MCP/skills/hooks/subagents, local/cloud/self-hosted deployment boundaries, sandbox/session separation. | Product-specific cloud agent lifecycle, Cursor workspace assumptions, VM/session details that belong in hosts. |
| Claude Agent SDK Python | Typed content blocks, session continuity, query/client split, permission modes, hooks, custom MCP tools, subagents, compact hooks, typed errors. | CLI transport coupling, Claude Code-specific permission semantics, provider-specific tool/content vocabulary. |
| Google ADK | Clear separation between session, state, memory, events, artifacts, and operation-scoped context; artifacts keep large/binary/generated content outside context/state by ref. | Google-specific service integrations, framework callback names, and deployment assumptions. |
| OpenAI Agents SDK | Agent/Runner split, local runtime context distinct from LLM-visible context, sessions, guardrail stages, streaming events/results, handoff input filters. | Provider-specific model/tool naming, Python/JS library shapes, and product defaults. |
| Pi Agent Harness | Strong package boundaries between provider API, agent core, coding agent, TUI, and chat automation; harness ergonomics; testable agent loop. | Product harness belongs above SDK core; language/package choices should not drive Rust API shape. |
| oh-my-pi coding-agent tools | Batteries-included read/search/edit/write/shell/resource tools, native search helpers, internal resource URLs, hidden tool discovery, anchored edits, structural edit preview, and streaming rule interruption. | Full coding-agent UX, prompt recipes, TUI/session UI, and marketplace behavior belong above the SDK. |
| Apple Containerization | One-VM-per-container isolation, OCI image/rootfs APIs, typed Linux process lifecycle, mounts, networking, Rosetta, stats, and explicit kernel/runtime setup. | macOS 26/Apple silicon/Xcode 26 constraints, Swift implementation details, and source-instability belong in host adapters, not Rust core. |
| OpenTelemetry GenAI | Shared vocabulary for agent/model/tool spans, usage, MCP, errors, events, and content-capture caution. | Spec is still evolving; raw content attributes are sensitive and should be opt-in. |

## Strands Deep Dive

Strands is the most useful loop-shape inspiration for this proposal.

### Flexible Loop

Strands models the agent as an async event producer. A request becomes:

1. Convert prompt into messages.
2. Invoke `BeforeInvocationEvent`.
3. Append messages and emit `MessageAddedEvent`.
4. Run an event-loop cycle.
5. Stream model chunks into typed events.
6. Convert model stop reason into either final response, structured-output retry, max-token failure, or tool execution.
7. Execute tools with before/after hooks, tracing, retries, interrupts, cancellation, and sequential/concurrent strategies.
8. Append tool results as a user message.
9. Recurse into the loop until stop.
10. Apply conversation management and invoke `AfterInvocationEvent`.

The future Rust SDK should keep this flexibility, but make recursion an explicit state-machine transition. That gives Rust callers a stable `Stream<AgentEvent>` and a final `RunResult` without hiding attempts and retries.

### Hook Model

Strands hook events are typed and selective:

- `BeforeInvocationEvent` can mutate input messages.
- `AfterInvocationEvent` can request resume.
- `BeforeModelCallEvent` can inspect projected tokens and invocation state.
- `AfterModelCallEvent` can request model retry.
- `BeforeToolCallEvent` can modify tool choice/input or cancel.
- `AfterToolCallEvent` can modify result or retry.
- Multi-agent hooks observe initialized orchestrators, node start, node stop, and overall invocation.
- Bidirectional hooks observe realtime session start/stop, message additions, tool calls, interruptions, and connection restarts.

The key lesson is not "make hooks mutable everywhere." The lesson is to make mutation rights explicit per hook type and observable. A Rust design should encode writable fields in event-specific response structs instead of giving hooks arbitrary mutable references.

### Message Metadata And Provider Projection

Strands attaches message metadata for usage, metrics, and custom provenance, and strips metadata before provider calls. This is exactly the right separation:

- Internal transcript can be rich and durable.
- Provider projection is a deliberate lossy step.
- Metadata can support compaction provenance, usage accounting, session sync, and trace linkage without leaking into model input by accident.

The future SDK should make this stricter: metadata should include typed lineage fields, privacy class, retention policy, injection source, and projection status.

### Session Management

Strands session managers are hook providers. They initialize agents, append messages, sync agent state after messages, sync after invocation, and support multi-agent and bidirectional agent state separately.

Reusable primitive:

- `SessionManager` should subscribe to durable lifecycle events, not inspect private loop internals.
- Agent, multi-agent, and realtime sessions need related but separate persistence contracts.
- Session managers should persist message history, state, conversation manager state, interrupt state, and app-owned data through explicit schemas.

### Streaming And Tool Execution

Strands stream processing normalizes provider chunks into text, reasoning, citations, tool-input deltas, final model message, stop reason, usage, metrics, cancellation, and redaction events. Tool execution then supports:

- Sequential or concurrent execution.
- Before/after tool hooks.
- Tool stream events.
- Tool result events.
- Tool cancellation.
- Tool interrupts.
- OTel tool spans.
- Retry requested by hooks.

The Rust SDK should expose this as an ordered canonical event stream where attempts and retries are visible. Concurrent tools should preserve per-tool ordering while allowing interleaved progress, with an explicit final result order policy.

### Agent-As-Tool And Multi-Agent

Strands can wrap an agent as a tool and can orchestrate graph or swarm patterns. It emits node start, node stop, handoff, node stream, node cancel, and node interrupt events.

Reusable lesson:

- Agent-to-agent interactions need first-class lineage. A parent should know which child was invoked, what prompt/context was passed, which tool call caused it, which events came back, and how usage rolled up.
- The parent should own cancellation, depth, context policy, tool policy, and approval inheritance.
- Handoff messages are not just text. They are control decisions with source, destination, policy, and trace edges.

The SDK should not adopt unconstrained agent-as-tool recursion. It should keep parent-owned subagents, depth limits, and recursive-subagent prevention.

### Bidirectional Streaming

Strands' experimental bidirectional layer has separate input and output events for text, audio, images, connection start/restart/close, response start, transcript stream, interruptions, and tool use. It uses a bounded queue and gates sends during connection restart.

Reusable lesson:

- Realtime is not "normal chat but faster." It needs send/receive halves, connection lifecycle, interruption events, media format metadata, restart policy, and backpressure.
- The same tool execution and message persistence contracts should apply, but media events should use references/buffers carefully.

### Telemetry

Strands starts spans for agent invocation, event-loop cycle, model invocation, tool execution, and multi-agent orchestration. It maps messages and tool calls to OpenTelemetry GenAI-style events and includes usage/latency attributes. It also guards telemetry failures so they do not crash the agent.

Reusable lesson:

- Trace shape should mirror the loop shape.
- Telemetry sink failure must not break execution.
- Content capture must be controlled by policy.
- Custom trace attributes are useful, but the SDK should provide typed core lineage attributes instead of relying on arbitrary maps.

## Cursor Lessons

Cursor's SDK points toward durable product automation:

- Agent and run are separate. This lets a caller create an agent, start runs, stream, reconnect, and inspect run state.
- Local, cloud, and self-hosted execution are deployment choices, not loop semantics.
- MCP, hooks, skills, and subagents are first-class extension surfaces.
- Sandboxes and sessions are host/product concepts that need explicit identity and lifecycle.

Rust takeaway: expose `AgentRuntime` and `RunHandle` separately. A `RunHandle` should provide stream subscription, cancellation, checkpoint lookup, final result, and replay.

## Claude Agent SDK Lessons

Claude Agent SDK reinforces:

- Typed content blocks instead of flat strings.
- A simple one-shot API plus a lower-level client API.
- Permission modes as first-class configuration.
- Hooks around tool and prompt lifecycle.
- In-process MCP tools for local extension.
- Session continuity and compaction hooks.
- Typed errors that let callers distinguish transport, permission, model, and tool failures.

Rust takeaway: provide a simple `Agent::run()` and a streaming `AgentRuntime::start_run()`, both backed by the same state machine and policies.

## Google ADK Lessons

Sources checked on 2026-05-23:

- [ADK context](https://google.github.io/adk-docs/context/)
- [ADK sessions](https://adk.dev/sessions/)
- [ADK events](https://adk.dev/events/)
- [ADK artifacts](https://adk.dev/artifacts/)

Reusable lessons:

- Context should not be a universal bag. ADK distinguishes operation-scoped context, session state, memory, events, and artifacts. The SDK should similarly distinguish `ContextContribution`, admitted `ContextItem`, provider `ContextProjection`, journal records, and artifacts/content refs.
- Artifacts are first-class refs for large, binary, generated, or persistent content. The SDK should add `ArtifactRef` / `ContentRef` with scope, version, MIME/type, storage service, policy, privacy, and retention metadata.
- Events are appendable records of what happened, while state and artifacts are separate surfaces. The SDK should keep live events, durable journal records, content stores, and provider context projection distinct.

Rejected behavior:

- Do not adopt Google service assumptions or ADK callback names as Rust API authority.
- Do not let artifact storage become SDK-owned bytes storage; it remains a typed ref/port boundary.

SDK decisions:

- Add the context contribution pipeline.
- Add artifact/content refs as content-bearing primitives.
- Require projection and journal audits to explain selected and omitted candidates.

## OpenAI Agents SDK Lessons

Sources checked on 2026-05-23:

- [OpenAI Agents context](https://openai.github.io/openai-agents-python/context/)
- [OpenAI Agents](https://openai.github.io/openai-agents-python/agents/)
- [Running agents](https://openai.github.io/openai-agents-python/running_agents/)
- [Guardrails](https://openai.github.io/openai-agents-python/guardrails/)
- [Streaming](https://openai.github.io/openai-agents-python/streaming/)
- [Results](https://openai.github.io/openai-agents-python/results/)
- [Handoffs](https://openai.github.io/openai-agents-python/handoffs/)
- [JS sessions](https://openai.github.io/openai-agents-js/guides/sessions/)

Reusable lessons:

- Runtime context and LLM-visible context are different things. The SDK should let tools, policies, and runtime ports use local context without automatically projecting it to the model.
- Agent/run separation and runner-style execution reinforce `Agent`, `AgentRuntime`, `RunRequest`, `RunHandle`, and `RunResult`.
- Guardrails are stage-scoped. The SDK should model input, model-input projection, pre-tool, post-tool, output, handoff, stream, and delivery checks as policy decisions with journal/event evidence.
- Streaming completion is not just final visible text. The event stream, session/journal persistence, approvals, compaction, and output delivery must reach terminal state before `RunHandle::wait()` completes.
- Handoffs need input filtering. Subagents should default to isolated child context and require explicit `ContextHandoffPolicy` for summary, selected refs, or full history.

Rejected behavior:

- Do not copy provider-specific names or Python/JS API shapes as Rust core authority.
- Do not make provider-native guardrails the source of policy truth; local policy/journal remains authoritative.

SDK decisions:

- Split public API tiers into MVP, reserved public contracts, and optional crate APIs.
- Add stage-scoped policy/guardrail validation.
- Add stream completion and subagent handoff validation gates.

## Pi Lessons

Pi is most useful for boundaries:

- Unified LLM/provider API is separate from agent core.
- Agent core is separate from coding-agent harness.
- TUI and chat automation are products built on top.
- Session sharing and real-world workflows are supported without making the core a UI product.
- Tests can target the harness and the core independently.

Rust takeaway: `agent_sdk` should be closer to `pi-agent-core` plus durable runtime/event primitives, not a full coding-agent product.

## oh-my-pi Tooling Lessons

The [can1357/oh-my-pi](https://github.com/can1357/oh-my-pi) repo is especially useful for tool ergonomics. Its docs for [read](https://github.com/can1357/oh-my-pi/blob/main/docs/tools/read.md), [search](https://github.com/can1357/oh-my-pi/blob/main/docs/tools/search.md), [edit](https://github.com/can1357/oh-my-pi/blob/main/docs/tools/edit.md), [write](https://github.com/can1357/oh-my-pi/blob/main/docs/tools/write.md), [ast-edit](https://github.com/can1357/oh-my-pi/blob/main/docs/tools/ast-edit.md), [tool discovery](https://github.com/can1357/oh-my-pi/blob/main/docs/tools/search_tool_bm25.md), and [stream-rule interruption](https://github.com/can1357/oh-my-pi/blob/main/docs/ttsr-injection-lifecycle.md) show how much leverage a good SDK toolkit can provide without forcing every host to build a coding agent.

Reusable lessons:

- Provide tool packs, not a monolithic agent. A host should be able to add read-only workspace tools, mutating edit/write tools, shell tools, resource readers, or tool discovery to a `RuntimePackage` independently.
- Make `read` broad but policy-aware: files, directories, archives, SQLite, notebooks, documents, images, URLs, and internal resource URLs are all useful, but each backing store needs its own source metadata, permission check, truncation policy, and sensitivity class.
- Make `search` and `grep` first-class: regex compile errors, case sensitivity, gitignore behavior, glob handling, context lines, pagination, line truncation, and result limits should be typed tool behavior instead of prompt folklore.
- Make read/search output editable through anchors. Stable line anchors, content hashes, and sparse read caches make later edits more reliable and observable.
- Prefer anchored and structural edits over blind writes. Edits should validate preconditions, preview diffs, preserve line endings where practical, invalidate caches, and record before/after effect metadata.
- Keep broad structural rewrites preview-first. AST search/edit is powerful, but applying a cross-file rewrite should be a separate resolver or approval action.
- Provide `write` as a real primitive for create/overwrite/archive/structured-row cases, but keep it more restricted than anchored edit because it is easier to destroy context accidentally.
- Offer shell/PTY as a policy-gated pack with cwd/env/network/timeout/cancellation and streaming stdout/stderr. It should never be ambient because a host added the SDK dependency.
- Let specialized tools be discoverable instead of always prompt-visible. A tool discovery index can keep the common prompt small while still letting agents find capabilities by name, summary, schema, or source.
- Add stream rule matching as a runtime primitive. Matching assistant text, provider-exposed reasoning summaries, tool-call argument deltas, tool result text, or realtime transcripts can stop, abort-and-retry, request approval, mask, or emit events while generation is still in progress.

The SDK should learn from these concrete tools, but keep product surfaces outside core. A desktop host, coding-agent harness, or CLI may choose these packs by default; the SDK itself only provides typed, policy-aware, observable building blocks.

## Apple Containerization Lessons

Apple's [Containerization](https://github.com/apple/containerization) package is a strong reference for local execution isolation on macOS. Its [README](https://github.com/apple/containerization/blob/main/README.md) describes a Swift package that runs Linux containers using `Virtualization.framework` on Apple silicon, manages OCI images and registries, creates ext4 root filesystems, starts lightweight VMs, runs containerized processes, and supports Rosetta 2 for linux/amd64 containers. The design runs each Linux container in its own lightweight VM and uses `vminitd` as a small guest init that exposes a gRPC API over vsock for runtime configuration and process launch. The source-level anchors worth learning from are [LinuxContainer](https://github.com/apple/containerization/blob/main/Sources/Containerization/LinuxContainer.swift), [LinuxProcess](https://github.com/apple/containerization/blob/main/Sources/Containerization/LinuxProcess.swift), [LinuxProcessConfiguration](https://github.com/apple/containerization/blob/main/Sources/Containerization/LinuxProcessConfiguration.swift), and the [single-file mount note](https://github.com/apple/containerization/blob/main/docs/single-file-mounts.md).

Reusable lessons:

- Isolation should be first-class in the SDK vocabulary. Shell, write, edit, eval, browser, and subagent tools should be able to request an `ExecutionEnvironment`, not just a cwd.
- The core primitive should be portable: `IsolationRuntime` / `ExecutionEnvironment` / `ProcessSpec`. Apple Containerization is one macOS adapter, not a hard dependency.
- Container lifecycle should be observable: image resolution, rootfs creation, environment prepared, process started, stdout/stderr/stdin, signal, wait, timeout, stats, cleanup, and failure.
- Resource and privilege controls are policy data: CPU, memory, rlimits, Linux capabilities, `no_new_privileges`, user, cwd, environment, terminal mode, read-only roots, writable layers, mounts, network, DNS, hosts, socket relays, kernel choice, and emulation.
- First-run behavior matters. Kernel availability, init image fetch, registry credentials, service readiness, and platform support should become adapter health events and user-actionable diagnostics.
- Mount semantics must be auditable. Apple Containerization's single-file mount design shares the parent directory into the guest VM before bind mounting the specific file into the container, so the SDK should record expanded mount exposure rather than only the final container destination.
- One-container-per-lightweight-VM is a useful isolation boundary for agent workloads because it gives each risky run a clear environment ID, lifecycle, resource envelope, network identity, and cleanup edge.

Rust takeaway: add execution-isolation primitives to the SDK, but keep all concrete runtime mechanics in adapters. Hosts can choose Apple Containerization on supported macOS machines, remote sandboxes on unsupported hosts, and cheap mock isolation in tests.

## OpenTelemetry Lessons

OpenTelemetry GenAI conventions suggest a trace vocabulary:

- Agent spans for invocation.
- Model spans for inference.
- Tool spans for execution.
- Usage attributes for input/output/total tokens and cache tokens.
- MCP attributes where tools/resources/prompts are provided by MCP.
- Events for messages and tool definitions.

Rust takeaway:

- Use OTel-compatible names where stable, and keep SDK-specific fields in a namespace until conventions settle.
- Treat raw content capture as opt-in with redaction, truncation, and sampling policy.
- Always include IDs and summaries so traces remain useful when content is omitted.

## What Rust Can Do Better

- Exhaustive `enum AgentEvent` instead of untyped event dictionaries.
- Strongly typed IDs and lineage refs.
- Compile-time ownership boundaries for messages, context items, policies, and adapters.
- Backpressure and cancellation built into stream types.
- Provider projection as a typed transform with tests.
- Append-only journals and deterministic replay from the start.
- Stable error variants and retry classifications.
- Static dispatch in hot paths with dynamic adapters only at external boundaries.
- Content privacy encoded as data, not as logging convention.

## Reusable Primitive Shortlist

- `Agent`, `AgentBuilder`, `AgentRuntime`, `RunHandle`.
- `AgentLoop`, `AgentStateMachine`, `LoopTransition`.
- `RuntimePackage`, `RuntimePackageFingerprint`.
- `ProviderAdapter`, `RealtimeProviderAdapter`, `ExternalAgentAdapter`, `ProviderEventMapper`.
- `ExecutionEnvironment`, `IsolationRuntime`, `ContainerRuntimeAdapter`, `ProcessSpec`, `FilesystemIsolationPolicy`, `NetworkIsolationPolicy`, `IsolationCapabilityReport`.
- `AgentMessage`, `MessagePart`, `ArtifactRef`, `ContentRef`, `ContextContribution`, `ContextItem`, `ContextProjection`, `ContextSelectionDecision`.
- `Lineage`, `EntityRef`, `EventEnvelope`, `AgentEvent`, `TelemetrySink`.
- `EffectIntent`, `EffectResult`, `PolicyDecision`.
- `ToolRegistry`, `ToolRouter`, `ToolExecutor`, `ToolAttempt`, `ToolResultEnvelope`.
- `ToolPack`, `BuiltinToolPack`, `ResourceUriRouter`, `WorkspaceSearch`, `WorkspaceEditPlanner`, `PatchApplier`, `ShellTool`, `ToolDiscoveryIndex`.
- `StreamDelta`, `StreamRule`, `StreamMatcher`, `StreamRuleEngine`, `StreamIntervention`.
- `ApprovalPolicy`, `ApprovalBroker`, `PermissionPolicy`, `SandboxPolicy`, `EscalationPolicy`.
- `SessionManager`, `CheckpointStore`, `RunJournal`.
- `HookBus`, `HookResponse`, `HookMutation`.
- `MemoryPort`, `ContextAssembler`, `CompactionPolicy`.
- `SubagentSupervisor`, `AgentTopology`, `HandoffRecord`.
- `RemoteChannelAdapter`, `OutputSink`.
