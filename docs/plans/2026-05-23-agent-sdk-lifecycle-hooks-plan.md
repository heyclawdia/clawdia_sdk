# Agent SDK Lifecycle Ownership And Hooks Plan

Date: 2026-05-23

## Objective

Make lifecycle ownership explicit before Phase 2 coding:

- manual run stop/cancel must cascade to agent-owned child work by default;
- normal run completion may preserve intentionally detached child work only when policy allows it;
- process/subagent/realtime/approval/isolation cleanup must be observable, journaled, replayable, and configurable;
- hooks must be first-class lifecycle primitives in `agent-sdk-core`, while extension processes remain optional adapters.

## Relevant Existing Context

- `AGENTS.md`: keep the packet product-neutral, edit contracts rather than product host adapters, and avoid code or fixture creation in this documentation-only phase.
- `coding_standards.md` and `docs/architecture/coding-standards.md`: cancellation is explicit, long-running providers/tools/subagents/realtime tasks receive cancellation handles, isolated execution emits process lifecycle events, and hooks are stable dynamic boundaries.
- `docs/contracts/run-handle-reconnect-contract.md`: `RunHandle::cancel` exists but needs child lifecycle semantics.
- `docs/contracts/journal-replay-schema.md`: cancel already records provider/tool/realtime/approval/child/isolation cancellation but needs detached-process and shutdown reconciliation records.
- `docs/contracts/runtime-package-schema.md`: hooks are already runtime package capabilities and fingerprint inputs but need a canonical hook contract.
- `docs/contracts/extension-sdk-contract.md`: extensions can provide hooks through optional bridge/runtime code, but core must see only typed hook capabilities and ports.

## Behavior Contract

New behavior:

- A run has a `RunChildLifecyclePolicy` with conservative defaults.
- `RuntimePackage` owns the default/allowed child lifecycle policy refs; `RunRequest` may select or tighten one before the run starts. The effective policy is immutable for the run, recorded in `RunRecord`, and exposed on event envelopes through policy refs.
- Manual stop/cancel cascades to agent-owned child runs, tool processes, isolated processes, realtime sessions, approval waits, and hook invocations.
- Normal completion preserves only explicitly detached work with a recorded intent, host/policy acknowledgement, retention/reclaim policy, and terminal visibility.
- Hooks attach to named lifecycle points through `HookPoint` and return typed `HookResponse` values with bounded mutation rights.
- Hook inputs default to envelope/index fields, content refs, hashes, sizes, statuses, and bounded redacted summaries. Raw content is opt-in through hook privacy/content-capture policy.
- Hook delivery is non-blocking by default. Only declared blocking hooks can hold a lifecycle transition, and security-critical blocking hooks must fail by deny, interrupt, or fail-run rather than fail open.
- Hook execution mode, queue capacity, and overflow policy are part of `HookSpec` and the runtime package fingerprint.
- Hook responses are a closed typed enum for the first Rust slice; any SDK effect request is also a closed enum, not a generic host-action escape hatch.
- Cleanup, process signaling, child cancellation, hook cancellation, detach transfer, and reclaim each follow intent-before-effect ordering: append intent, perform bounded effect, append terminal result or recovery record.

Preserved behavior:

- `wait_with_timeout()` still never cancels a run.
- Live subscribers still cannot cause durable facts.
- Extensions cannot own approval, memory, provider routing, telemetry, or durable run state.
- Concrete process/container runtimes stay adapter-owned.

Removed behavior:

- No implicit orphan processes.
- No ad hoc lifecycle callbacks with ambient powers.
- No security-critical fail-open hooks.

## Workstreams

- Core API/runtime: add child lifecycle policy to run handles and advanced run config.
- Events/journal: add shutdown/detach records, hook records, replay and anti-entropy rules.
- Loop state machine: add hook points and cancellation overlay semantics.
- Runtime package: keep hooks as capabilities with ordering, timeout, mutation rights, and lifecycle point fingerprint inputs.
- Tools/isolation/subagents: align process ownership and detach semantics with the shared lifecycle policy.
- Scenario coverage: add generic examples for long-running scripts, manual cancellation, and hook attachment.

## Validation Plan

- Docs audits: link audit, contract index audit, product-neutrality grep.
- Contract tests to name in docs: manual cancel cascades by default, timeout wait does not cancel, detached process survives only with policy/intent/ack, non-detached child work is cleaned up, hook ordering and fail behavior are deterministic, hook helper lowering equals explicit hook sidecar.
- Golden fixtures: versioned JSON fixtures for every new hook, child-shutdown, detach, process-signal, and reclaim event kind plus every new journal record kind.
- Redaction fixtures: hook inputs/events and child lifecycle events show no raw prompt/model/tool/file/process content by default.
- Performance fixtures: slow observe hooks and slow lifecycle event subscribers cannot block the agent loop.
- Review: run SDK review checklist against simplicity, product-neutrality, event/journal durability, privacy, and boundaries.

## Risk/Gotcha Carry-Forward

- Do not model this as a generic process manager product. Core owns typed lifecycle policy and journal/event contracts; hosts/adapters own concrete process control.
- Do not let a detached process disappear from observability. Detach means lifecycle ownership transferred or was explicitly preserved, not forgotten.
- Do not let hooks mutate arbitrary runtime state. Each hook point must define its allowed responses.
- Do not let extension hooks become approval/security authorities. Security hooks must fail deny/interrupt, never allow.
- Do not rely on live events as ownership proof for detached work. The durable journal must contain the detach intent, acknowledgement, owner transfer or preservation record, reclaim policy, and recovery path.
