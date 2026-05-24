# SDK Review Checklist

Use this checklist when reviewing standalone Agent SDK docs, contracts, APIs, crate boundaries, event streams, journals, tools, isolation, telemetry, extension SDK, and host-facing adapter boundaries.

The review goal is to protect the SDK as a long-lived developer platform: simple to use, hard to misuse, observable from day one, durable where it matters, privacy-preserving by default, and product-neutral.

## Core Principles

- SDK, not product: core defines reusable primitives, contracts, and ports. Hosts and optional crates own product workflows.
- Simplicity is a design requirement: one canonical path with ergonomic wrappers beats overlapping concepts.
- Explicit agent loop: stop, retry, cancel, compact, spawn, approve, and recover transitions are modeled.
- Events from day one: lifecycle subscriptions are core infrastructure.
- Fast hot path: no live filter payload parsing, content-store lookup, journal scan, or slow-subscriber blocking.
- Live events are not durable truth: durable facts belong in the run journal.
- Lineage everywhere: source, destination, cause, policy, privacy, retention, run, turn, message, tool, and subagent IDs are typed.
- Host owns policy/product behavior: UI, approval transport, storage, credentials, runtimes, dashboards, marketplace, and product-specific behavior stay outside core.
- Privacy by default: raw content is opt-in and bounded by policy, retention, and sink permission.
- Ergonomic first, explicit underneath: simple APIs lower into canonical contracts.
- Composable pieces: providers, tools, memory, output validation, stream rules, isolation, telemetry, and subagents are typed ports, not untyped plugin soup.
- Isolation is a first-class boundary: no silent downgrade from container/VM to host process.
- No ambient power: filesystem, shell, network, MCP, extensions, subagents, credentials, secrets, and host UI actions come through runtime packages, policy, approval, and adapters.
- Provider-neutral core: provider-native features are optimizations, not authority.
- Replayable and recoverable: idempotency, dedupe, cursors, checkpoints, recovery states, and anti-entropy are explicit.
- Optional orchestration over core events: core exposes events, filters, cursors, lineage, and idempotent starts; workflow/DAG/barrier engines stay optional or host-owned.
- Parallelizable implementation: one stitching owner keeps shared names, IDs, event/journal alignment, package fingerprints, and public API coherence.
- Primitive discipline: feature layers reuse the kernel primitives before adding new concepts, and any new capability variant has a typed sidecar contract and owner.
- Context discipline: context is an admission/projection pipeline over content refs, not a universal bag for tools, memory, skills, events, or host state.
- Effect discipline: side effects share intent/result, policy, idempotency/dedupe, journal, event, and reconciliation semantics.

## Required Review Checks

| Check | Questions |
| --- | --- |
| Simplicity | Is the common path one or two lines? Can concepts be removed, merged, renamed, or defaulted? Does complexity buy real power? |
| Primitive fit | Does this reuse `Agent`, `RunRequest`, `RuntimePackage`, `AgentEvent`, `RunJournal`, policy refs, source/destination refs, and typed ports instead of creating a parallel concept? |
| Context fit | Does content stay in `ArtifactRef` / `ContentRef` until `ContextAssembler` admits it as `ContextItem` for `ContextProjection`? |
| Effect fit | Does every mutating/external action use shared effect intent/result semantics before and after execution? |
| Capability fit | Is `CapabilitySpec` limited to callable/discoverable capabilities with typed sidecars, while provider route/output/delivery/telemetry/lifecycle concerns stay as package fields or sidecars? |
| Product-neutrality | Does core remain free of product behavior? Are scenario examples generic and non-authoritative? |
| Canonical lowering | Do helpers lower into canonical DTOs, events, journals, policy checks, telemetry, and failures? |
| Event stream quality | Are events stable, typed, subscribable by indexed envelope fields, and useful without raw content? |
| Event performance | Are filters envelope/index based? Are queues bounded? Do slow observers avoid blocking the loop? |
| Journal durability | Are side-effect intents journaled before execution? Are terminal states durable? Are resume/cancel/failure paths explicit? |
| Lineage | Can the SDK explain source, destination, cause, policy, privacy, and retention? |
| Privacy | Is raw content opt-in? Are redaction and metadata limits explicit? |
| Policy and approval | Does missing policy/dispatcher/adapter fail closed? Is approval broker/policy, not UI? |
| Isolation | Are risky executions tied to explicit `ExecutionEnvironment` and policy-gated fallback? |
| Idempotency and replay | Are duplicate subscribers safe? Are retries safe? Are event, journal, and archive cursors distinct? |
| Optional layers | Should the behavior live in toolkit, isolation, OTel, workflow, or host adapter instead of core? |
| Public API stability | Are public Rust types future-proof and SemVer-conscious? |
| Testability | Are fake adapters, golden events, journal fixtures, smoke tests, and acceptance tests named? |
| Package topology | Does the code follow the SDK responsibility layout (`domain`, `package`, `records`, `ports`, `application`/`runtime`, `testing`), with only facade/shim files at package roots? Are generated/spec-derived surfaces separated from hand-written runtime logic? |
| Public facade | Are new public modules, deep-import paths, and re-exports reviewed for SemVer/API stability? Is downstream test-kit support exposed through `agent_sdk_core::testing`? |
| Documentation | Does the contract say SDK owns and host owns? Are open questions decisions or deferrals? |

## Review Output Format

```text
## Findings

[P0/P1/P2] Title
File/section:
Violated principle:
Why it matters:
Suggested fix:

## Simplicity Pass

- What can be removed, merged, renamed, or defaulted?
- What should stay advanced-only?
- What should become a one-liner helper?
- What is a kernel primitive, feature layer, optional adapter, or host-owned behavior?
- Any new `CapabilitySpec` variant? If yes, where is its typed sidecar contract, fingerprint impact, event/journal set, owner, and tests?

## Boundary Pass

- Core:
- Optional crate:
- Host-owned:
- Package topology / DDD ownership:
- Public facade / test-kit namespace:

## Event/Journal Pass

- Missing events:
- Missing journal records:
- Cursor/replay concerns:
- Performance concerns:

## Open Questions

- Must answer before coding:
- Can defer:

## Verdict

PASS / PASS WITH NOTES / BLOCKED
```

## Severity Guide

- P0: breaks product-neutral core, privacy, durability, replay safety, or can cause unsafe side effects.
- P1: public API or contract shape likely to age badly, block implementation, or create drift.
- P2: clarity, ergonomics, naming, missing examples, or test coverage gap.

## Common Smells

- "Manager" owns unrelated things.
- Helper API bypasses the real contract.
- Event payload is required for filtering.
- Raw content appears in default telemetry.
- Tool execution happens before journal intent.
- Missing dispatcher falls back to permissive behavior.
- Isolation request silently runs on host.
- Provider-native feature becomes SDK authority.
- Product-specific behavior appears in core.
- Workflow orchestration sneaks into core.
- Public enum lacks future extension strategy.
- Retry is mentioned without idempotency or replay semantics.
- Example has no SDK-owned / host-owned block.
- Lots of knobs, no simple path.
