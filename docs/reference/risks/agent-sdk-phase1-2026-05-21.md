# Agent SDK Phase 1 Risk Note

Date: 2026-05-21

## Context

Phase 1 produced a documentation-only proposal for a future Rust-first `agent_sdk` crate and extension SDK layer. No code, Rust source files, executable tests, fixtures, manifests, or runtime behavior changes are part of this phase.

## 2026-05-23 Phase 1 Completion Audit

The Phase 1 docs now include explicit answers for the prior open questions, stable event family/kind/envelope guarantees, journal/replay behavior for resume/cancel/failure paths, stricter headless/source-scoped approval semantics, projection audit and metadata limits, minimal telemetry/cost accounting guarantees, and extension SDK compatibility rules.

The docs also incorporate oh-my-pi-inspired SDK tool packs for read/search/edit/write/shell/resource-reader/tool-discovery behavior, plus a stream-rule primitive for stop/retry/mask/approval interventions while model output is still streaming.

The docs now also incorporate Apple Containerization lessons as portable execution-isolation primitives: `ExecutionEnvironment`, `IsolationRuntime`, container/VM/remote sandbox adapters, environment lifecycle events, mount/network/resource policy, and cleanup/recovery semantics.

The later Phase 2 handoff review found that the Phase 1 docs are strong enough for architecture review but not sufficient as a direct coding handoff. Implementation must first freeze event payload schemas, loop transitions, runtime-package canonicalization, journal record shapes, structured-output semantics, stream-rule safety, tool-pack boundaries, isolation fallback/security behavior, extension packaging smoke tests, and generic host-flow mappings as testable contracts.

The follow-up contract examples pass requires every normative contract doc to end with a complete typed example. Each example must show structs/specs/enums, replaceable ports/adapters, runtime wiring, emitted event kinds, journal records, policy/failure behavior, explicit `SDK owns / Host owns` boundaries, and tests/golden fixtures. This is now part of the anti-entropy guardrail for future coding agents: do not implement from a contract whose example is missing or thinner than that template.

The ergonomics pass adds simple one-liners, presets, and defaulted builders for normal SDK use, especially Pydantic-like typed output via `run_typed::<T>()`. These helpers must stay thin: they lower into canonical `RunRequest`, `OutputContract`, `RuntimePackage`, `StreamRule`, `EnvironmentSpec`, `SubagentRequest`, `CoreExtensionCapabilities`, or telemetry specs and must emit the same events, journal records, retries, policies, and failures as explicit advanced usage.

The lifecycle hooks and child ownership pass adds first-class `HookSpec`/`HookPoint` primitives plus `RunChildLifecyclePolicy`. Code hooks such as `agent.on(...)` and declarative hook config must lower into the same runtime-package capabilities. Manual run cancellation must cascade to agent-owned child work by default. Long-running processes or child runs can outlive a parent only through explicit detach intent, acknowledgement, durable records, and reclaim policy.

Node ESM was explicitly checked before documenting support:

- Node v25.9.0 does not resolve the extension SDK from the packaged fallback through `NODE_PATH` when run from `/tmp`; it fails with `ERR_MODULE_NOT_FOUND`.
- Node ESM succeeds when the SDK is reachable through normal local `node_modules` resolution; root, browser-safe helper, and process-only media subpaths need continuing smoke coverage.
- Bun fallback support must stay tied to smoke coverage for root, browser-safe helper, and process-only media subpaths.

Do not document Node ESM `NODE_PATH` fallback support unless a future loader, import-map, or installation strategy is added and verified from a temp directory outside the repo.

## Key Risks

1. **Overfitting to product internals.** Host behavior is coverage input, not the architecture to copy. Future implementation should keep host-specific runtime selection, external-runtime session lifecycle, UI routing, and trace ingestion out of the reusable core.

2. **Under-observing message and context lineage.** The user's explicit concern is that messages can come from many places and be sent to many destinations. If source, destination, policy, injection path, run/turn/message/span IDs, sensitivity, and retention are optional afterthoughts, the SDK will be hard to debug and unsafe to extend.

3. **Telemetry content leakage.** OTel GenAI conventions include message and tool event concepts, but raw prompt/model/tool/memory content is sensitive. Default telemetry should capture IDs, summaries, sizes, usage, timing, status, and policy decisions, not raw content.

4. **Runtime package drift.** Provider-visible tool schemas and executable tool registries must share a deterministic snapshot. Dynamic discovery after projection can create approvals and traces that do not match what the model saw.

5. **Approval boundary erosion.** Approval must remain broker/policy-owned. UI events, extension hooks, or remote channels must not become raw approval APIs. Extensions cannot approve their own requested actions.

6. **Headless/source-scoped approval gaps.** Headless approval parks the broker receiver and resumes after a finite host-owned response or timeout. Future runtimes need explicit dispatchers/custom handlers. If none is available, source-scoped approval should deny rather than silently falling back to desktop UI.

7. **Extension SDK runtime fallback drift.** The packaged fallback for the extension SDK is host-owned. Public subpath exports, resource/package drift, fallback ordering, browser-safe boundaries, Node ESM assumptions, and temp-directory smoke tests must stay explicit.

8. **Subagent recursion and ownership drift.** Subagents should remain parent-owned, depth-bounded, and prevented from receiving tools that create further subagents by default. Child transcripts should not silently become normal user conversations.

9. **Realtime backpressure underdesign.** Bidirectional text/audio/image paths need bounded queues, connection lifecycle events, send gates during restart, media references, and interruption semantics. Treating realtime as normal chat streaming will not be enough.

10. **Event schema sprawl.** A rich event model is required, but it can become unusable if every adapter invents new fields. Keep a stable envelope and versioned payloads.

11. **Built-in tools becoming a product harness.** Read/search/edit/write/shell tools are useful SDK utilities, but a bundled coding agent, TUI, marketplace, prompts, and workflow UX belong above the core. Tool packs must be opt-in through `RuntimePackage`, not globally available.

12. **Streaming matcher overreach.** Stop-on-regex over assistant text, tool arguments, tool results, or provider-exposed reasoning is powerful. It can also leak sensitive content or interrupt incorrectly if match windows, redaction, repeat state, and channel boundaries are vague. Hidden chain-of-thought must not become observable just because stream rules exist.

13. **Mutating-tool reversibility overpromise.** The SDK can record before/after hashes, diffs, idempotency keys, and inverse patch candidates. It should not promise all side effects are automatically reversible, and it must not become a self-improvement engine.

14. **Containerization hard-dependency drift.** Apple Containerization is a strong macOS adapter candidate, but it currently carries platform and toolchain constraints. The SDK should define portable isolation contracts and let hosts provide adapters. Do not make Rust core depend on Swift, macOS 26, Apple silicon, Xcode 26, a local kernel, or the Apple `container` service.

15. **Container security overclaiming.** Containers and lightweight VMs reduce blast radius, but they do not remove the need for approval, permission, mount, network, secret, registry, and cleanup policy. Treat isolation as one policy layer, not a blanket safe mode.

16. **Mount exposure surprises.** Single-file mount support can expose a parent directory inside the guest VM even when the final container path is a single file. Future adapters must record expanded mount exposure and let policy deny it when unacceptable.

17. **Implicit orphan process drift.** Shell tools, isolated processes, child agents, realtime sessions, approval waits, and hook invocations must be agent-owned by default. Do not let a successful run seal while child work is still running unless detach policy, intent, acknowledgement, owner transfer, and reclaim metadata are durable.

18. **Hook callback sprawl.** Hooks are useful only if they stay typed and bounded. Do not introduce arbitrary mutable callbacks, raw transcript access by default, slow observe hooks on the hot path, or security hooks that fail open.

## Watchpoints For Phase 2

- Start with the active contracts, primitive map, and workstreams before code. Do not let a coding agent infer missing contracts from prose or historical plans.
- Start with tests that prove metadata is stripped before provider calls.
- Make `AgentEvent` schema reviewable before adapter implementation.
- Add runtime package projection/execution invariant tests.
- Add approval broker tests for desktop, CLI, external-runtime, headless, extension-submitted, and source-scoped contexts.
- Add subagent depth and no-recursive-tools tests before child execution is wired.
- Add OTel sink failure tests so telemetry cannot crash a run.
- Add extension SDK packaged fallback smoke tests for every public subpath.
- Keep Node ESM `NODE_PATH` fallback support documented as unsupported until a real loader/import-map/install strategy is implemented and smoked.
- Keep raw content capture behind an explicit policy and document retention.
- Add built-in tool-pack contract tests for read/search anchors, edit preconditions, write approval, shell sandboxing, resource URI privacy, and discovery activation.
- Add stream-rule tests for regex compile failures, bounded windows, stop/retry/mask/approval actions, provider-exposed reasoning channels, resume repeat-state restoration, and default redaction of matched content.
- Add isolation-runtime contract tests for adapter capability negotiation, Apple Containerization unsupported-host fallback, image/rootfs readiness, mount expansion audit, network denial, process I/O redaction, signal/timeout handling, stats collection, cleanup, and recovery after partial environment preparation.
- Before coding from any Agent SDK contract, check that the contract's `## Complete Example` section follows the full template and that host-owned behavior is not pulled into `agent-sdk-core`.
- When adding ergonomic helpers or presets, add equivalence tests proving the helper lowers into the same canonical contract and does not bypass local validation, approval, redaction, lineage, telemetry, or side-effect intent records.
- Add lifecycle ownership tests before wiring shell/isolation/subagent execution: manual cancel cascades to agent-owned children, `wait_with_timeout` does not cancel, normal completion denies implicit orphans, and explicit detach requires intent/ack/reclaim.
- Add hook tests before extension bridge work: code/config lowering equivalence, nonblocking slow-hook behavior, content-ref inputs by default, blocking security timeout deny/interrupt, mutation-right rejection, cancel interruption, and audit replay not reinvoking hooks.

## Rollback

This phase is documentation-only. In the standalone workspace, rollback is removing:

- `/Users/clawdia/clawdia_sdk`

For any older product-checkout pointer, rollback is restoring the previous in-repo packet if the standalone move is abandoned:

- `docs/architecture/agent-sdk/`
- `docs/reference/risks/agent-sdk-phase1-2026-05-21.md`
- The `docs/README.md` index entries for this doc set
- The plan doc if the proposal is abandoned
