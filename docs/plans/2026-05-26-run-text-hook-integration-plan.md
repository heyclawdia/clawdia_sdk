# Run Text Hook Integration Plan

Date: 2026-05-26

## Objective

Wire the existing hook lifecycle contract into the active `AgentRuntime::run_text`
path so package-declared hooks are validated before a run starts, invoked at P0
text lifecycle points, and able to lower accepted responses into the canonical
context, model-attempt, run-terminal, journal, and event path.

## Current Problem Shape

Phase 09 implemented the hook data model, package sidecar hash, executor port,
coordinator, mutation-right validation, timeout/cancel handling, and
journal-before-apply behavior. The active P0/P1 run driver never calls that
coordinator, and `RuntimePackage` currently stores only opaque hook sidecar
snapshots, not recoverable `HookSpec` values that `run_text` can invoke.

The result is a contract/runtime split: hook points such as
`BeforeRunComplete` and `AfterRunTerminal` exist and can be unit-tested through
`HookLifecycleCoordinator`, but a normal `AgentRuntime::run_text(...)` cannot
observe or apply them.

## Authoritative Source Of Truth

- `docs/contracts/hook-lifecycle-contract.md` owns hook points, mutation rights,
  response lowering, journal-before-apply, and host-owned executor boundaries.
- `docs/implementation-workstreams/09-p2-side-effects/09d-hook-lifecycle.md`
  owns the first Rust hook lifecycle implementation surface.
- `crates/agent-sdk-core/src/application/loop_driver.rs` owns the canonical P0
  text run path and must remain the only provider-call path for `run_text`.
- `crates/agent-sdk-core/src/application/hooks.rs` owns hook ordering,
  executor invocation, response validation, and hook mutation journal evidence.
- `crates/agent-sdk-core/src/package/hooks.rs` owns the closed hook DTOs and
  mutation-right matrix.

## Relevant Existing Context

- `AGENTS.md`: do not create branches; keep the SDK product-neutral; hooks must
  layer on primitive contracts rather than host/product behavior.
- `README.md` and `docs/start-here.md`: implementation must keep the standalone
  Rust SDK packet coherent and use docs/contracts as source of truth.
- `coding_standards.md` and `docs/architecture/coding-standards.md`: preserve
  deterministic fakes, journal/event durability, privacy, policy, and typed
  boundaries.
- `docs/workstreams/validation-gates.md`: behavior needs tests, commands,
  primitive-lowering evidence, and host-boundary evidence.
- `docs/reference/sdk-review-checklist.md`: reject ambient callbacks, hidden
  runtime state, mini-SDK drift, and unmockable live-service paths.
- `docs/architecture/primitive-map.md`: hooks are a feature layer over
  `RuntimePackage`, ports, events, journals, policy refs, and context/model
  primitives, not a new agent coordination system.
- `docs/implementation-workstreams/09-p2-side-effects/_phase/phase-exit-report.md`:
  Phase 09 passed for hook coordinator behavior but left active loop stitching
  outside the evidence.
- `docs/plans/2026-05-25-agent-pool-store-plan.md`: cross-process messaging
  belongs to `AgentPool`; same-run nudges belong to lifecycle hooks and context
  or retry lowering.

## Behavior Contract

New behavior:

- `RuntimePackageBuilder::hook(...)` records recoverable `HookSpec` values and
  derives their canonical hook sidecar snapshots before package validation.
- `RuntimePackage::validate()` rejects hook specs whose sidecar snapshot is
  missing or does not match the spec hash.
- `AgentRuntime` owns a hook executor registry port, defaults to an empty
  in-memory registry, and validates package hooks against it before
  `start_run` registers a run.
- `run_text` invokes exactly these P0 lifecycle hook points in this slice:
  `BeforeContextAssembly`, `BeforeRunComplete`, and `AfterRunTerminal`.
- `BeforeContextAssembly` accepted `InjectContext` responses create bounded
  `ContextContribution` candidates, admit them into `ContextItem` values with
  hook/source/policy lineage, append the existing P0 context projection journal
  evidence before provider projection, and only then reach provider messages.
  In the P0 slice, a behavior-changing `BeforeContextAssembly` hook must be the
  only hook at that point so an accepted injection cannot be stranded by a later
  same-point failure before projection.
- `BeforeRunComplete` also accepts `RequestRetry` so completion guards can say
  "do not stop yet; retry with this redacted nudge" without an out-of-band
  session message. The same bounded retry budget applies.
- `BeforeRunComplete` accepted `StopCompletionWithRepairNeeded` stops normal
  completion and surfaces a repair-needed failure path through existing run
  terminal/error records until a dedicated repair-needed terminal state is
  introduced.
- P0 guard rejections, including retry-budget exhaustion and payload bounds,
  append durable rejected hook response decisions before returning the guarded
  failure.
- Before invoking `BeforeRunComplete`, the P0 loop rejects specs for that point
  whose mutation rights include unsupported classes such as `ValidateDetach`.
  This keeps the closed mutation matrix honest until detach lowering exists in a
  child/process lifecycle slice.
- `AfterRunTerminal` observe hooks run best-effort after terminal sealing and
  cannot change the returned `RunResult`.

Preserved behavior:

- No hook can mutate ambient transcript, provider state, package state, or host
  process internals directly.
- Behavior-changing hook responses still journal hook response, effect intent,
  and effect result before the loop applies the change.
- Hook executors remain host-owned ports; core does not store function pointers,
  spawn hook processes, or add product-specific adapters.
- Existing P0 text runs with no hooks keep their current journal/event fixture
  shape.
- `AgentPool` remains the cross-process/message-delivery primitive; hooks are
  same-run lifecycle guards and context/retry proposals only.
- Hook points outside the explicit P0 set, including `RunStarting`,
  `BeforeModelCall`, and `AfterModelCall`, remain contract DTO/coordinator
  surfaces only in this slice.

Removed behavior:

- None.

Tests proving behavior:

- Hook lifecycle unit test updates for the `BeforeRunComplete` request-retry
  mutation matrix.
- Package tests proving `RuntimePackageBuilder::hook(...)` fingerprints through
  a matching sidecar and validation rejects missing/mismatched hook sidecars.
- P0 runtime tests proving hook context injection reaches the provider request
  and the context projection journal record shows the injected item count plus
  hook/source/policy lineage on the admitted context item.
- P0 runtime tests proving a `BeforeRunComplete` retry hook causes a second
  provider attempt and returns the second output.
- P0 runtime tests proving `BeforeRunComplete` retry budget exhaustion fails
  closed before an unbounded provider loop and records a rejected hook response.
- P0 runtime tests proving `BeforeRunComplete`
  `StopCompletionWithRepairNeeded` stops normal completion and records a failed
  terminal result with repair-needed summary evidence.
- P0 runtime tests proving `AfterRunTerminal` observe hooks are invoked after the
  run result is sealed.
- P0/runtime tests proving missing hook executors fail before provider calls.
- P0/runtime tests proving unsupported P0 hook mutation rights fail before
  provider calls rather than being accepted and ignored.
- P0/runtime tests proving a behavior-changing `BeforeContextAssembly` hook
  cannot be mixed with another same-point hook in this slice.
- P0/runtime tests proving oversized hook payloads fail closed with rejected hook
  response evidence.
- Existing no-hook P0 fixture tests must continue passing unchanged.

## Scope

Writable files for this slice:

- `docs/plans/2026-05-26-run-text-hook-integration-plan.md`
- `docs/contracts/hook-lifecycle-contract.md`
- `crates/agent-sdk-core/src/package/mod.rs`
- `crates/agent-sdk-core/src/package/hooks.rs`
- `crates/agent-sdk-core/src/application/runtime.rs`
- `crates/agent-sdk-core/src/application/hooks.rs`
- `crates/agent-sdk-core/src/application/loop_driver.rs`
- `crates/agent-sdk-core/src/lib.rs` only if new public exports are required
- `crates/agent-sdk-core/tests/feature_layers/hook_lifecycle_contract.rs`
- `crates/agent-sdk-core/tests/p0/p0_text_run.rs`
- new or updated fixtures only if existing golden summaries intentionally change

Out of scope:

- Cross-process agent messaging, session injection APIs, or agent-pool transport.
- Hook invocation for tools, approvals, isolation, subagents, compaction, or
  streaming deltas beyond existing coordinator tests.
- Extension hook subprocess/JSON-RPC adapters.
- A new terminal status schema for `RepairNeeded`.
- Branch creation, publish, push, or product-specific examples.

## Workstreams

1. Package/runtime authority:
   - Add recoverable hook specs to runtime packages.
   - Keep hook specs tied to sidecar hashes so fingerprints still flow through
     the canonical sidecar group.
   - Add a runtime hook executor registry port and pre-start validation.

2. Hook coordinator application surface:
   - Return accepted hook responses to callers after journal-before-apply.
   - Allow callers to synchronize journal sequence allocation without gaps.
   - Extend `BeforeRunComplete` to allow `RequestRetry`.
   - Keep unsupported response classes out of the invoked P0 point set or reject
     them before execution.

3. P0 run-loop lowering:
   - Invoke only `BeforeContextAssembly`, `BeforeRunComplete`, and
     `AfterRunTerminal`.
   - Lower context injection into admitted context items before provider
     projection; the existing context projection journal record must show the
     injected item in the selected item count.
   - Lower retry requests into bounded provider retries with redacted developer
     nudges.
   - Lower repair-needed stops into a failed terminal result and returned error
     until the run terminal schema grows a repair-needed terminal case.
   - Invoke terminal observe hooks best-effort after sealing.

4. Tests and documentation:
   - Add focused P0 runtime tests for injection, retry, terminal observe, and
     fail-before-provider behavior.
   - Update hook lifecycle contract tests and docs for the new
     `BeforeRunComplete` retry guard.
   - Run focused and package-level verification.

## Risk / Gotcha Carry-Forward

- Do not make hooks a general message-delivery API. If another process needs to
  talk to an agent, use `AgentPool`; if the same run needs a nudge, lower it
  through context injection or bounded retry.
- Do not let `RuntimePackage` carry hook specs that are not matched by the
  canonical hook sidecar hash. The sidecar remains the fingerprint authority.
- Do not add an ambient callback table to active runs. Executors are resolved
  from the runtime hook registry before run start and invoked only at declared
  lifecycle points.
- Do not retry indefinitely. Hook-driven model retries must be bounded and
  journaled before the second provider call.
- Do not let `AfterRunTerminal` change terminal result or fail a sealed run.
- Do not silently ignore accepted mutation responses in invoked P0 hook points.
  Each invoked behavior-changing class must either lower into a domain operation
  in this slice or remain outside the invoked P0 point set.
- Do not widen the invoked P0 point set without adding one-to-one lowering and
  tests for every allowed mutation class at that point.
- Do not direct-insert hook text into provider requests. Hook context must pass
  through `ContextContribution`/`ContextItem` admission and context journal
  evidence before projection.
- Do not change no-hook P0 journal/event golden behavior.

## Public API / SemVer Note

This slice adds `RuntimePackage::hooks` and `RuntimePackageBuilder::hook(...)`.
The workspace is still in alpha, but this is still a public DTO surface change:
downstream struct literals for `RuntimePackage` must add `hooks: Vec::new()` or
prefer the builder. The tradeoff is intentional because active runtime hook
invocation needs recoverable typed `HookSpec` values while hook sidecars remain
the canonical fingerprint authority.

## Review Packet

Primitive decision:

- Reused kernel primitives: `RuntimePackage`, hook sidecars, hook executor port,
  `RunJournal`, `AgentEventBus`, context projection, provider attempts, policy
  refs, effect intent/result, typed IDs, and run terminal records.
- New feature-layer primitive: none.
- New capability variants: none.
- Host-owned behavior kept out: concrete hook executors, extension processes,
  product UI, cross-process messaging, credentials, and session injection APIs.

Validation evidence to collect:

- `cargo fmt --check`
- `cargo test -p agent-sdk-core --test hook_lifecycle_contract`
- `cargo test -p agent-sdk-core --test p0_text_run`
- `cargo clippy --workspace --all-targets -- -D warnings`
- Source-layout audit commands:
  - `find crates/agent-sdk-core/src -maxdepth 1 -type f -not -name lib.rs -not -name README.md`
  - `find crates/agent-sdk-core/tests -maxdepth 1 -type f -name '*.rs' -print -exec sh -c 'wc -l "$1"' sh {} \;`
  - `find crates -path '*/src/*.rs' -maxdepth 3 -type f`
  - `rg -n '#\\[path = .*\\]\\s*pub mod|pub mod [a-zA-Z0-9_]+;' crates/agent-sdk-core/src/lib.rs`
  - `rg -n '\\b(Fake|Scripted)[A-Za-z0-9_]+|ConformanceHarness' crates/agent-sdk-core/src --glob '*.rs'`
  - `rg -n '\\btrait\\b|\\bAdapter\\b|\\bResolver\\b|\\bFake\\b|\\bScripted\\b|ConformanceHarness' crates/agent-sdk-core/src/records --glob '*.rs'`
  - `wc -l crates/agent-sdk-*/src/lib.rs`
- Public API/SemVer review note for any new crate-root exports or builder
  methods.
- `cargo test -p agent-sdk-core`

Reviewer checklist:

- Hook specs remain package-owned and fingerprint-linked.
- Runtime hook registry is a port, not an ambient active-run callback map.
- Every accepted P0 behavior-changing response is journaled before apply.
- `BeforeRunComplete` retry is bounded and does not bypass provider
  intent/result records.
- Context injection proves admission through context items and journal evidence,
  not direct provider request mutation.
- Existing no-hook P0 fixtures remain stable.
