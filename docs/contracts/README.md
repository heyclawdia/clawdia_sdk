# Agent SDK Contracts

These files define the current implementation contracts for the Agent SDK.

They are still documentation. They are not Rust source, executable tests, or fixtures. Their job is to remove ambiguity before a coding agent starts.

## Reading Order

| Contract | Purpose |
| --- | --- |
| [api-contracts.md](api-contracts.md) | Public crate/module/API boundary, host-owned boundaries, and first implementation slices. |
| [event-schema.md](event-schema.md) | Stable event envelope, event frames, event filters/subscriptions, payload requirements, redaction, and golden tests. |
| [run-handle-reconnect-contract.md](run-handle-reconnect-contract.md) | Run registry, runtime-wide subscriptions, reconnectable streams, event cursors, wait/status idempotency, and terminal result consistency. |
| [loop-state-machine.md](loop-state-machine.md) | Agent loop states, transitions, cancellation, max iteration, tool denial, and recovery edges. |
| [runtime-package-schema.md](runtime-package-schema.md) | Runtime package snapshot, typed capability/sidecar boundaries, catalog provenance, canonical fingerprinting, package deltas, and projection/execution alignment. |
| [content-artifact-ref-contract.md](content-artifact-ref-contract.md) | Artifact/content ref lifecycle, resolver boundaries, missing-ref recovery, retention, privacy, and no-raw-content defaults. |
| [context-memory-contract.md](context-memory-contract.md) | Messages, artifacts/content refs, context contributions, admitted context items, context projection, memory ports, compaction, and projection audit boundaries. |
| [hook-lifecycle-contract.md](hook-lifecycle-contract.md) | First-class lifecycle hooks, config/code registration, typed mutation rights, ordering, timeout, cancellation, and extension boundaries. |
| [journal-replay-schema.md](journal-replay-schema.md) | Append-only journal record schemas, replay modes, checkpoints, side-effect atomicity, resume, cancel, and anti-entropy. |
| [tool-approval-contract.md](tool-approval-contract.md) | Permission, sandbox, approval, escalation, autonomy, source-scoped approval, and compatibility rules. |
| [structured-output-contract.md](structured-output-contract.md) | User-provided output schemas, validation, repair retries, streaming candidates, and typed results. |
| [stream-rule-contract.md](stream-rule-contract.md) | Literal/regex stream matching, channels, cursor semantics, interventions, privacy, and resume behavior. |
| [tool-pack-contract.md](tool-pack-contract.md) | Built-in optional tool packs for read/search/edit/write/shell/resource/tool discovery and effect lineage. |
| [isolation-runtime-contract.md](isolation-runtime-contract.md) | Portable isolated execution contract, adapter capabilities, mounts, network, secrets, process lifecycle, cleanup, and fallback. |
| [subagent-contract.md](subagent-contract.md) | Parent-owned subagent supervision, package stripping, route validation, event wrapping, usage rollup, and no-chat promotion rules. |
| [extension-sdk-contract.md](extension-sdk-contract.md) | Host manifest/JSON-RPC contracts, SDK-facing core capability boundaries, browser-safe helper exports, and packaging smoke tests. |
| [output-delivery-contract.md](output-delivery-contract.md) | Destination refs, output sink dispatch, dedupe, intent-before-delivery, and host channel boundaries. |
| [otel-mapping-contract.md](otel-mapping-contract.md) | SDK-to-OpenTelemetry mapping, semconv stability, MCP dedupe, content opt-in, and exporter failure behavior. |
| [telemetry-privacy-contract.md](telemetry-privacy-contract.md) | Telemetry/cost/content-capture sink authority, redaction limits, and trace-sink boundaries. |
| [review-matrix.md](review-matrix.md) | Contract-to-source review matrix for tomorrow's human pass and future coding-agent handoff. |

## Contract Rules

- These contracts are normative for implementation planning.
- If implementation discovers a mismatch, update the contract before changing code.
- Each contract implementation must satisfy the relevant workstream validation gates in [../workstreams/validation-gates.md](../workstreams/validation-gates.md).
- Contract reviews should use [../reference/sdk-review-checklist.md](../reference/sdk-review-checklist.md), including the simplicity, product-neutrality, event/journal, privacy, and boundary passes.
- Simplicity guidance lives in [../reference/simplicity-audit.md](../reference/simplicity-audit.md). Simplify through defaults, presets, builders, and canonical lowering before adding or removing primitives.
- The primitive kernel lives in [../architecture/primitive-map.md](../architecture/primitive-map.md). New contracts must say whether they add a kernel primitive, a feature layer, an optional adapter, or host-owned behavior.
- The feature-to-primitive matrix lives in [../reference/feature-to-primitive-matrix.md](../reference/feature-to-primitive-matrix.md). New work must update or cite it when adding behavior.
- Public event, journal, runtime package, or extension wire fields need compatibility notes.
- Do not turn host-owned behavior into SDK-owned behavior just because it appears in a diagram.
- Examples live under [../examples](../examples) and should be updated when these contracts change.
- Normative contract docs end with `## Complete Example` sections. Those examples must name typed shapes, replaceable ports, wiring, events, journal records, policies/failures, SDK-owned boundaries, host-owned boundaries, and tests.
- Ergonomic helpers and presets are allowed only as thin lowering layers over these same contracts. A simple API must produce the same canonical DTOs, events, journal records, policy checks, telemetry, and failures as the explicit advanced path.

## Scenario References

Scenario references are not normative SDK contracts. They are coverage examples for products built on the SDK. Keep them product-neutral; host-specific adapters belong outside this active handoff unless the user explicitly asks for a separate external task.

| Reference | Purpose |
| --- | --- |
| [../examples/README.md](../examples/README.md) | Generic desktop, CLI/headless, realtime, remote, external-runtime, telemetry, isolation, and subagent scenarios. |

## Host-Owned Boundaries

The SDK exposes ports, schemas, events, and optional toolkit contracts. These remain host-owned:

- Desktop/window routing.
- Host live-event ingestion and retention.
- Durable trace-store schema and dashboard query policy.
- External runtime session cache, restore keys, prewarm, retirement, and process ownership.
- Approval transport UI and out-of-band channels.
- Extension install/marketplace UX and packaged runtime resource placement.
- Concrete container runtimes such as Apple Containerization, Docker, Firecracker, or remote sandboxes.
- Concrete process control, detached child/process inspectors, and reclaim schedulers.
- Proposal scoring, benchmark UI, product undo/revert UX, and automatic self-improvement policy.

## External Patterns To Learn From

- Strands: [agent hooks](https://strandsagents.com/docs/user-guide/concepts/agents/hooks/), [streaming](https://strandsagents.com/docs/user-guide/concepts/streaming/), and [bidirectional hooks](https://strandsagents.com/docs/user-guide/concepts/bidirectional-streaming/hooks/) for typed lifecycle, strongly named stream events, and connection/interruption/restart events.
- Cursor SDK: [SDK release notes](https://cursor.com/changelog/sdk-release) for the simple agent/run split, inspectable runs, cancellation, SSE streaming, and `Last-Event-ID` reconnect.
- Google ADK: [context](https://google.github.io/adk-docs/context/), [sessions](https://adk.dev/sessions/), [events](https://adk.dev/events/), and [artifacts](https://adk.dev/artifacts/) for separating context, session/state, memory, events, and artifact refs.
- OpenAI Agents SDK: [context](https://openai.github.io/openai-agents-python/context/), [running agents](https://openai.github.io/openai-agents-python/running_agents/), [guardrails](https://openai.github.io/openai-agents-python/guardrails/), [streaming](https://openai.github.io/openai-agents-python/streaming/), and [handoffs](https://openai.github.io/openai-agents-python/handoffs/) for local-vs-LLM context, agent/run execution, stage-scoped policy checks, terminal streaming, and handoff filtering.
- Pi: [package docs](https://pi.dev/docs/latest/packages) and [pi-multiagent](https://pi.dev/packages/pi-multiagent) for package layering, source-qualified subagent roles, parent-led delegation, and trust/install boundaries.
- oh-my-pi: [repo README](https://github.com/can1357/oh-my-pi) for concrete read/search/edit/write ergonomics, anchored edits, hidden tool discovery, and a batteries-included coding harness that the SDK should not absorb.
- Apple Containerization: [repo README](https://github.com/apple/containerization) for explicit image/rootfs/process lifecycle, lightweight VM isolation, capability requirements, and unsupported-host detection.
- OpenTelemetry GenAI/MCP: [GenAI semconv](https://opentelemetry.io/docs/specs/semconv/gen-ai/), [agent spans](https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-agent-spans/), [model/tool spans](https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-spans/), and [MCP semconv](https://opentelemetry.io/docs/specs/semconv/gen-ai/mcp/) for traces, metrics, events, usage, MCP dedupe, and raw-content opt-in.
