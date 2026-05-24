# Coverage And Gap Matrix

This file is a coverage audit, not an implementation-ready contract. Current coding handoff authority lives in [../contracts/README.md](../contracts/README.md) and [../workstreams/README.md](../workstreams/README.md).

## Goal Acceptance Coverage

| Acceptance item | Covered in | Status |
| --- | --- | --- |
| Markdown documentation only. | This doc set. | Covered. |
| No code, Rust source files, executable tests, or fixtures. | All Rust is fenced conceptual sketch text. | Covered. |
| Demanding host scenarios studied. | [../examples/README.md](../examples/README.md) and external SDK lessons. | Covered as product-neutral scenario coverage, not SDK ownership. |
| External SDK lessons incorporated without cloning. | [external-sdk-lessons.md](external-sdk-lessons.md). | Covered. |
| Future `agent_sdk` crate clearly described. | [architecture-proposal.md](architecture-proposal.md). | Covered. |
| Major types, methods, responsibilities, relationships. | [primitive-map.md](primitive-map.md) and [architecture-proposal.md](architecture-proposal.md). | Covered with an MVP kernel plus feature layers. |
| Diagrams for important flows. | [architecture-proposal.md](architecture-proposal.md) and [observability-and-lineage.md](observability-and-lineage.md). | Covered. |
| Caveats, risks, and tradeoffs. | [architecture-proposal.md](architecture-proposal.md) and active contracts. | Covered. |
| Stop for user review before implementation. | Implementation waits for accepted contracts and workstreams. | Covered. |

## Generic Host Scenario Coverage

| Scenario | Proposed support | Gaps before implementation |
| --- | --- | --- |
| Stateful external runtime path. | `ExternalAgentAdapter` plus provider-native runtime behind host dispatch boundaries. | Need adapter test contracts and compatibility plan. |
| Runtime package assembly and fingerprinting. | `RuntimePackage` typed snapshot and projection/execution invariant. | Need exact fingerprint fields and stable serde schema. |
| Model calls, tool calls, approvals, events, usage, cancellation, max iterations. | State machine, provider adapters, tool executor, approval broker, event stream, usage extractor, stop policy. | Need executable state-transition tests. |
| History replay, images, context windows, compaction. | Lossless messages, `ArtifactRef` / `ContentRef`, `ContextAssembler`, `ConversationManager`, `CompactionPolicy`. | Need content/artifact storage refs and projection spec. |
| External runtime sessions, restore keys, prewarm, retirement, compacted replay. | External adapter contract while host owns live-session policy. | Need generic external-runtime adapter tests. |
| Memory prompt injection and memory tools. | `MemoryPort` and context item lineage. | Need memory capability schema and privacy policy. |
| Remote messages from arbitrary channels. | `RemoteChannelAdapter`, `OutputSink`, source/destination metadata. | Need channel adapter contracts and dedupe IDs. |
| Scheduled, headless, CLI, memory ingestion. | Same run request model with host source and output sinks. | Need headless run examples and approval dispatcher tests. |
| Tool approvals in all contexts. | Policy/broker/escalation split with source-scoped metadata. | Need exact decision enum and timeout behavior tests. |
| MCP discovery, naming, exposure, filtering, approval, execution. | `McpRegistry`, tool router, runtime package snapshot, policy layers. | Need MCP namespace/versioning design. |
| App, macOS, MCP, extension, subagent tools. | Tool source and routing primitives. | Need source-specific tool envelope details. |
| Skills, plugins, extension core capabilities, SDK processes, hook merging. | Extension SDK layer, `HookBus`, runtime-package sidecars, and callable capability refs where applicable. | Need core-capability and host-manifest schema evolution plan. |
| Parent-owned subagents and recursive prevention. | `SubagentSupervisor`, depth budget, child tool policy, event wrapping. | Need exact child package stripping rules. |
| Conversation, trace, app event, session snapshot, and runtime event relationships. | Distinct IDs and lineage docs. | Need concrete schema mapping to generic stores. |

## External SDK Coverage

| Source | Lessons included | Follow-up needed |
| --- | --- | --- |
| Strands | Deep loop, hooks, sessions, metadata, streaming, tools, agent-as-tool, multi-agent, bidi, OTel. | Implementation should turn chosen loop states into tests. |
| Cursor | Agent/run split, reconnectable streams, local/cloud/self-hosted boundaries, MCP/skills/hooks/subagents. | Need decide how much `RunHandle` should mirror reconnect semantics. |
| Claude Agent SDK | Typed content, query/client split, permissions, hooks, MCP tools, sessions, compaction. | Need map permission modes to SDK approval/autonomy policies. |
| Pi | Package boundaries, harness separation, provider API vs agent core vs products. | Need harness example doc before implementation. |
| OpenTelemetry | GenAI span vocabulary, usage, tool/model/agent events, content sensitivity. | Need stable SDK namespace while OTel conventions evolve. |

## Implementation Test Candidates

1. `AgentLoop` state transition tests.
2. `AgentEvent` golden schema tests.
3. Provider projection strips metadata and preserves lineage.
4. Runtime package projection/execution invariant.
5. Tool execution attempts and retry visibility.
6. Approval broker desktop, CLI, headless, source-scoped, extension-submitted paths.
7. Context assembly budget and compaction preservation.
8. Realtime bounded queue and restart behavior.
9. Subagent depth, parent-owned cancellation, event wrapping, and no recursive child tools.
10. Journal replay with deterministic IDs.
11. OTel sink failure isolation.
12. Extension SDK runtime fallback public-subpath smoke.

## Active Contract Arrangement

Active implementation authority is:

- [../contracts/README.md](../contracts/README.md) for normative contracts.
- [../workstreams/README.md](../workstreams/README.md) for parallel ownership and validation.
- [primitive-map.md](primitive-map.md) for the MVP primitive kernel and feature-layer discipline.
- [../reference/open-questions-and-ambiguities.md](../reference/open-questions-and-ambiguities.md) for decisions and deferrals.

## Remaining Design Gaps

- Exact public type names and module names are provisional.
- The event schema needs golden tests and payload schemas before implementation.
- The runtime package fingerprint fields need implementation fixtures for the MVP profile and typed sidecar rules for reserved variants.
- Provider adapter trait signatures need live Rust prototyping with fake providers.
- The session/journal/checkpoint storage split needs concrete schemas, atomicity rules, replay tests, and explicit ownership diagrams before implementation.
- Extension SDK packaging needs smoke tests to avoid outdated packaged fallback resources.
- OTel GenAI conventions may change; SDK-specific namespacing must absorb drift.
- Any host integration needs staged compatibility plans.

## Current Stop Point

The next useful step is review of the active contracts/workstreams. Implementation should wait until the MVP primitive kernel, feature-layer boundaries, observability model, and risk posture are accepted or revised.
