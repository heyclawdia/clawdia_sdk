# Agent SDK Contract Examples Expansion Plan

> Historical plan only. It may contain stale product-specific paths or superseded workstream names. Do not use it as implementation authority; start from [../../start-here.md](../../start-here.md), [../../contracts/README.md](../../contracts/README.md), and [../../workstreams/README.md](../../workstreams/README.md).

## Objective

Make the Agent SDK contract packet concrete enough that a coding agent can start with typed, replaceable building blocks and complete usage examples instead of filling gaps ad hoc.

## Behavior Contract

New behavior:

- Every contract document under `docs/contracts/` that defines runtime behavior ends with a `## Complete Example` section.
- Each complete example shows the relevant structs, replaceable ports/adapters, runtime wiring, events, journal records, and validation expectations for that contract.
- Contracts spell out extension points as typed traits/spec structs so SDK pieces can be swapped like Lego blocks without turning host behavior into SDK behavior.
- Structured output is expanded beyond placeholders: schema version, schema dialect, schema ref, validation policy, repair policy, result envelope, validator ports, repair ports, and retry behavior are explicitly named.

Preserved behavior:

- This remains documentation-only. No Rust source, executable tests, package manifests, or runtime behavior changes.
- `agent-sdk-core` stays the first concrete crate target.
- Host-owned boundaries stay host-owned: UI, display events, trace stores, external runtime sessions, extension marketplace, approval transport, recommendation UX, and concrete container runtimes.
- Existing scenario docs under `docs/examples/` remain workflow examples; contract docs own normative API and invariant details.
- Product-specific host-adapter docs are not part of the active SDK handoff. Generic examples under `docs/examples/` provide scenario coverage.

Removed behavior:

- No contract doc should end immediately after acceptance tests without showing how the contract is actually used.
- No placeholder “policy” or “schema” field should remain unexplained when it is central to implementation.

Tests proving this behavior:

- `git diff --check`.
- A structural audit verifying every normative contract doc has `## Complete Example`.
- A structural audit verifying every complete example includes the required template:
  - typed structs/specs/enums
  - replaceable ports/adapters
  - runtime or package wiring
  - emitted event kinds
  - durable journal records
  - policies, retries, cancellation, timeout, or failure behavior where relevant
  - explicit `SDK owns / Host owns` boundary block
  - acceptance tests or golden fixtures proving the example
- A contract coverage matrix verifying every normative contract doc expands its central fields/policies/specs, not only structured output.
- A structural audit verifying the structured-output contract defines schema, validation, repair, result, validator/repair adapter, retry, event, and journal shapes.
- Local Markdown link audit over the Agent SDK packet.

## Relevant Existing Context

- `coding-standards.md`: preserve source-of-truth boundaries, typed contracts, extensibility-first design, and no ad hoc fallback behavior.
- `docs/architecture/coding-standards.md`: future SDK pieces must be DDD-oriented, test-first, observable, and explicit about structured output validation/repair.
- `docs/contracts/README.md`: contracts are normative for Phase 2 planning and must not convert host-owned features into SDK-owned behavior.
- `docs/contracts/review-matrix.md`: each contract must line up with external source lessons, host-adapter coverage notes, records/events/tests, and a first workstream.
- `docs/reference/plans/2026-05-23-agent-sdk-phase2-implementation-handoff-plan.md`: Phase 2 coding starts from `agent-sdk-core`, contract tests, golden records, fake adapters, and explicit source-of-truth rules.
- `docs/reference/risks/agent-sdk-phase1-2026-05-21.md`: watchpoints include event taxonomy drift, journal/replay ambiguity, approval fail-open behavior, structured output retries, and host/SDK boundary creep.

## Workstreams

1. Contract-wide example pattern:
   Add `## Complete Example` sections to API, event, run handle, loop, runtime package, journal/replay, tool approval, stream rule, tool pack, isolation, subagent, extension SDK, OTel, and telemetry/privacy contracts.

   Every complete example must follow this template:

   - `Typed shape`: concrete contract structs/specs/enums with the extra fields needed for implementation.
   - `Replaceable ports`: traits/adapters/sinks/registries that can be swapped without changing the core loop.
   - `Wiring`: how the host or runtime package composes the pieces.
   - `Events`: exact event kinds emitted by the example.
   - `Journal`: exact durable record kinds written by the example.
   - `Policies and failures`: timeout/retry/cancel/escalation/downgrade/repair behavior where relevant.
   - `SDK owns / Host owns`: explicit boundary block when the example mentions host UI, external runtimes, trace stores, display events, extensions, recommendation products, or concrete container runtimes.
   - `Tests`: acceptance tests/golden fixtures that make the example executable later.

2. Structured output deepening:
   Expand `structured-output-contract.md` with the concrete spec structs, schema refs, validator/repair adapter traits, bounded policies, result envelope, journal records, and a complete typed extraction example.

3. Lego-style extensibility:
   Add typed component IDs, capability declarations, port/adapter traits, unknown-field/versioning posture, and package-delta examples where the contract currently implies swapability but does not name the shape.

4. Validation and review:
   Run structural audits and ask an independent reviewer to check whether the packet is clear enough for coding after this pass.

## Contract Coverage Matrix

| Contract | Central fields/policies/specs that must be expanded |
| --- | --- |
| `api-contracts.md` | crate family, component IDs, builder/runtime/handle traits, replaceable registries, host boundary |
| `event-schema.md` | envelope fields, event kind payload DTOs, cursor/delivery semantics, redaction/privacy, golden fixture mapping |
| `run-handle-reconnect-contract.md` | `EventCursor`, `JournalCursor`, run registry, wait/status/cancel semantics, SSE/CLI/UI adapters |
| `loop-state-machine.md` | state enum, transition input/output structs, transition guards, cancellation overlay, max-iteration policy |
| `runtime-package-schema.md` | capability declarations, package snapshot, canonical fingerprint, package delta, projection/execution invariant |
| `journal-replay-schema.md` | record envelope, record kinds, side-effect intent/result, checkpoint, replay reducer, anti-entropy plan |
| `tool-approval-contract.md` | policy layers, precedence, approval request, dispatcher port, timeout/escalation/headless semantics |
| `structured-output-contract.md` | schema version, dialect, schema ref, validation policy, repair policy, validator/repair ports, result envelope |
| `stream-rule-contract.md` | matcher spec, channel, cursor, action/intervention, repeat/partial-output policy, injection authority |
| `tool-pack-contract.md` | tool spec fields, pack contracts, resource permissions, effect metadata, reversible/non-reversible lineage |
| `isolation-runtime-contract.md` | environment spec, adapter capability report, security class, downgrade rules, process/mount/secret policy |
| `subagent-contract.md` | supervisor, child package rules, parent mailbox, clarification, event wrapping, usage rollup, promotion |
| `extension-sdk-contract.md` | manifest capabilities, JSON-RPC runtime, hook mutation rights, browser-safe subpaths, app-event boundaries |
| `otel-mapping-contract.md` | semconv pin, schema URL, attribute map, golden spans, MCP dedupe, content opt-in |
| `telemetry-privacy-contract.md` | sink capabilities, telemetry fields, cost accounting, content capture, redaction/export failure behavior |

Generic scenario docs are checked separately:

| Scenario reference | Coverage fields that must stay aligned |
| --- | --- |
| `docs/examples/README.md` | desktop, CLI/headless, realtime, remote, external-runtime, telemetry, isolation, and subagent coverage |
| `docs/examples/live-vs-durable-event-flow.md` | live event, display event, journal, telemetry, and trace-store boundaries |
| `docs/examples/external-runtime-session-lifecycle.md` | external runtime restore/prewarm/retirement/fingerprint boundaries |

## Risk / Gotcha Carry-Forward

- If examples become implementation promises outside the contract, coding agents may overfit to sample names. Keep examples concrete but label them as contract-shaped sketches.
- If extension points are too generic, they become untestable. Every pluggable port must still name required events, journal records, and acceptance tests.
- If structured output relies only on provider-native JSON mode, validation bugs will escape. SDK validation and repair events must remain mandatory.
- If host-owned behavior appears in examples without a boundary note, future implementation may pull product behavior into `agent-sdk-core`.
