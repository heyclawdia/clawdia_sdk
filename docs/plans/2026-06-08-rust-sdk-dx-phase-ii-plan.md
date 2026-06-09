# Rust SDK DX Phase II Implementation Plan

## Objective

Implement Phase 16 from
`docs/implementation-workstreams/16-dx-phase-ii/16a-dx-phase-ii.md` as a
repo-grounded DX improvement, not as a copy of any external suggestion. A new
SDK user should be able to move from install, to a deterministic first run, to
typed output, typed tools, approval, events, reports, and resume evidence
without learning the whole primitive kernel first.

This is not a comparison packet. Do not cite outside SDKs, outside package
names, hosted product examples, or unrelated backend families as justification.
The plan is grounded in this repository's current crates, examples, contracts,
and validation gates.

## Root Cause / Problem Shape

Phase 15 implemented the required building blocks, but the first-developer path
still asks users to discover too many seams on their own:

- `AgentApp` can run, subscribe, and report, but it does not yet expose a
  compact evidence-gathering helper for tests and examples.
- Existing examples prove the Phase 15 smoke paths, but they do not separately
  teach typed output, event observation, approval denial, checkpoint/replay
  evidence, and feature selection as small reusable developer scenarios.
- The first-30-minute path is split across published split-crate usage and
  checkout-only facade usage; Phase II must make both explicit so new users do
  not confuse the unpublished facade with the current public install path.
- Docs explain the architecture, but the first-user path should foreground
  testable mock-based runs and observability from the first example.

The structural fix is to add a thin facade evidence layer over current ports
and to add deterministic examples/tests around that layer. The implementation
must keep `RunJournal` as durable truth, `AgentEventBus` as live observation,
`CheckpointStore` as an accelerator, and `agent-sdk-eval` reports as derived
projections.

## Current Baseline

Phase 15 already delivered the first DX implementation slice:

- `clawdia-sdk::AgentApp` facade assembly over canonical runtime ports;
- typed tool helpers and optional macros;
- provider-visible typed tool projection;
- file-backed and optional durable store adapters over existing store ports;
- deterministic usage, cost, and run reports;
- five credential-free checkout examples.

Phase II starts from those implemented surfaces. It must not re-plan them as
future work.

## Current User-Requested Addendum

This addendum scopes the next implementation pass over the Phase 16 launch
target. The user explicitly asked for the layered SDK shape to stay intact:

- `agent-sdk-core` remains the primitive kernel for runtime packages, runs,
  journals, events, policy, checkpoints, provider ports, and output contracts.
- Optional crates own provider, toolkit, eval, store, protocol, telemetry, and
  workflow helpers.
- `clawdia-sdk` remains a facade with one import path and feature-gated
  re-exports; it must not own behavior.
- `AgentApp` remains a first-user builder only if tests prove it lowers into
  `Agent`, `RuntimePackage`, `AgentRuntime`, `RunRequest`, provider registry,
  journal, event bus, policy, and output contracts.
- Examples must be copy-paste runnable with deterministic fake paths first and
  explicit live-provider gates where live providers are shown.

The concrete requested roadmap for this pass is:

- `examples/01_live_provider_text_run`
- `examples/02_typed_tool_builder`
- `examples/06_checkpoint_resume`
- `examples/07_token_tracking_costs`
- `examples/10_facade_quickstart`

The pass also adds a builder-first tool authoring path before macro ergonomics:

```rust
let read_file = FunctionTool::builder("workspace_read")
    .description("Read a file from the workspace")
    .input_schema(ReadFileInput::schema())
    .executor(read_file_executor)
    .build()?;
```

That builder must lower into the existing toolkit typed-tool path, package
sidecars, core tool routes, `ToolExecutionCoordinator`, policy, approval,
journal records, and event frames. The existing `#[agent_tool]` macro remains a
later convenience and must not bypass the builder-proven execution path.

Exact builder contract:

- `FunctionTool<A, R>` lives in `agent-sdk-toolkit::typed_tool` and is
  re-exported through `clawdia-sdk::tools`.
- `A` must implement `ToolArgs`; `R` must implement `ToolOutput`.
- `.input_schema(A::schema())` accepts a provider-safe schema value and
  normalizes it before hashing; if omitted, the builder uses `A::schema()`.
- `.description(...)` stores provider-visible description metadata on the
  tool-pack snapshot, tool route, and provider spec. Descriptions are metadata
  only and do not alter executor identity, policy refs, approval behavior, or
  package ownership.
- `.executor(...)` accepts a typed sync executor
  `Fn(A, TypedToolContext) -> ToolResult<R>` and lowers to the same
  `TypedToolExecutor` used by `TypedTool::builder`.
- `#[agent_tool]` macro tests must prove macro output lowers through
  `FunctionTool` or through the same `TypedTool` execution path with identical
  package, route, schema, approval, journal, and output behavior.

The persistence request adds explicit backend mapping for file, SQLite, and
Postgres-style adapters. The current repo has `agent-sdk-store-file` and
`agent-sdk-store-supabase`; this pass should add concrete SQLite and Postgres
adapter crates only with deterministic tests and without live provisioning.
Each backend bundle must expose separate surfaces for:

- `RunJournal`
- `CheckpointStore`
- `ContentStore`
- `EventArchive`
- `AgentPoolStore`
- `ToolExecutionStore`
- `ProviderArgumentStore`

`ToolExecutionStore` is not a runtime truth source. It is a new optional
projection/cache port over journaled tool records and effect lineage. The run
journal remains durable truth for tool side effects.

`ToolExecutionStore` conformance semantics:

- It stores and reads `ToolCallRecord` values plus their intent/result cursors
  for fast lookup, idempotency-key lookup, and result-cache diagnostics.
- It is rebuildable from `RunJournalReader` records. Dropping the store cannot
  change replay, policy, executor release, recovery, or report results.
- It must reject attempts to store raw provider arguments or raw tool output;
  those stay behind `ProviderArgumentStore` and `ContentStore` refs.
- It must preserve `run_id`, `tool_call_id`, `effect_intent.effect_id`,
  `idempotency_key`, `dedupe_key`, `policy_refs`, `requested_args_refs`,
  `result_content_refs`, redacted summaries, terminal status, journal cursor,
  and package/source/destination lineage.
- It must expose stale-cache detection by comparing stored journal cursor or
  journal sequence against the caller-supplied durable journal evidence.
- It must not approve tools, release executors, synthesize effect results,
  decide replay safety, or replace recovery markers.

Backend conformance matrix:

| Backend | Required proof | Explicit limitation |
| --- | --- | --- |
| File | Restart rehydrates every surface; tool execution cache can be rebuilt from journal records; missing content/provider args stay typed errors. | Local filesystem durability only; no backup/retention policy. |
| SQLite | Local in-process migration fixtures for all seven surfaces; transaction ordering, cursor reads, idempotency lookup, and stale-cache detection. | SQLite is an optional local adapter, not a product session store. |
| Postgres-style | Scripted SQL/transport tests for generated statements, bound parameters, row decoding, cursor reads, idempotency lookup, and stale-cache detection. | Scripted tests prove adapter contract and SQL shape only; live database provisioning, migrations, RLS, backups, and durability remain host-owned. |
| Supabase | Existing scripted PostgREST/RPC adapter keeps coverage for the currently implemented hosted-store crate; update docs to map it as a hosted Postgres-family adapter, not as the generic `agent-sdk-store-postgres` crate. | Supabase project provisioning and RLS remain host-owned. |

Requested first-developer sequence:

1. `examples/10_facade_quickstart`: first documented command for the local
   checkout path; deterministic fake provider and facade `AgentApp`.
2. `examples/01_live_provider_text_run`: deterministic provider-injected text
   run by default, with an explicitly gated live-provider path.
3. `examples/02_typed_tool_builder`: builder-first typed tool authoring with
   `FunctionTool::builder`.
4. `examples/02_typed_tool_macro`: existing macro example, documented after
   the builder to show macro sugar only after the execution path is proven.
5. `examples/06_checkpoint_resume`: checkpoint/replay resume-readiness
   evidence, not continuation.
6. `examples/07_token_tracking_costs`: usage/cost/run report projections from
   journal evidence and host-supplied rate policy.

Existing Phase 15/16 examples with overlapping numeric prefixes remain
regression smoke examples unless this implementation later renames them in a
single documented pass. They must not be presented as the first-developer
ordering when they conflict with the requested roadmap above.

## Relevant Existing Context

- `AGENTS.md`: keep the SDK packet product-neutral, choose the Phase 16 launch
  file as the single target, do not create a branch without approval, and do
  not add parallel packets outside this workspace.
- `coding_standards.md` and `docs/architecture/coding-standards.md`:
  testability and observability are core requirements; every port and scenario
  surface must be mockable with deterministic fakes, and public APIs must be
  reviewed against Rust API Guidelines.
- `docs/workstreams/validation-gates.md`: implementation goals need tests,
  commands, primitive-lowering evidence, event/journal/telemetry boundary
  evidence, host-owned boundary evidence, and source-layout audits.
- `docs/reference/sdk-review-checklist.md`: helpers must lower into
  `Agent`, `RunRequest`, `RuntimePackage`, `AgentEvent`, `RunJournal`, policy
  refs, source/destination refs, and typed ports rather than creating a parallel
  concept.
- `docs/reference/simplicity-audit.md`: keep one canonical lowering path; make
  common event, typed output, and report usage easy without hiding the explicit
  contracts underneath.
- `docs/architecture/primitive-map.md`: simple/builder APIs must compile down
  to existing kernel primitives and use the same validation, policy, event,
  journal, and recovery paths as advanced APIs.
- `docs/architecture/observability-and-lineage.md`,
  `docs/contracts/event-schema.md`, and
  `docs/contracts/journal-replay-schema.md`: live event streams are not durable
  truth; replay and resume-readiness must derive from journals and checkpoint
  evidence, not from a facade-only session store.
- `docs/contracts/tool-pack-contract.md`: tool helpers and approvals must keep
  intent-before-effect records, policy refs, and approval dispatch explicit.
- `docs/contracts/telemetry-privacy-contract.md`: reports and telemetry are
  derived projections over events, journals, usage, cost, and policy records;
  they do not decide run state.
- `docs/reference/dx-upgrade-risk-watchpoints.md`: current high-risk areas are
  facade dependency creep, typed-tool approval flags, raw provider-argument
  leakage, global state-store drift, and examples claiming to be runnable
  without proof.
- `docs/implementation-workstreams/15-dx-completion/_phase/phase-exit-report.md`:
  Phase 15 is PASS; `AgentApp`, typed tools/macros, durable stores,
  reports, provider tool projection, and five credential-free examples are
  implemented and should be treated as the baseline.
- `crates/clawdia-sdk/src/app.rs`: `AgentApp` already lowers `run_text` and
  `run_typed` through canonical `AgentRuntime`, has `subscribe_run`, and
  builds reports from caller-supplied journal records. `AgentAppStores` already
  carries journal writer, journal reader, content resolver, provider-argument
  store, checkpoint store, event archive, and agent-pool ports.
- `crates/agent-sdk-core/src/application/replay.rs` and
  `crates/agent-sdk-core/src/application/checkpoint.rs`: checkpoint and replay
  are existing projection/accelerator contracts; Phase II should expose them
  through examples and thin helpers, not implement a new resume engine.
- Current runtime APIs do not expose a full run-continuation resume method.
  Phase II examples must describe checkpoint/replay readiness and durable
  evidence inspection unless a later phase adds and tests actual continuation.
- `crates/agent-sdk-core/src/testing/**`: deterministic fake providers,
  journals, content resolvers, event helpers, and scripted approval dispatchers
  are available and should be the first proof path.

## Behavior Contract

New behavior:

- `AgentApp` keeps its existing execution path and adds small read-side helpers
  for collecting run evidence from canonical ports: journal records through
  `RunJournalReader`, live buffered frames through `subscribe_run`, archived
  frames through `EventArchiveReader` when configured, optional latest
  checkpoints through `CheckpointStore`, and optional run reports through
  `agent-sdk-eval`.
- The facade exposes missing-store diagnostics through typed
  `AgentError::host_configuration_needed` errors instead of panics or silent
  empty evidence.
- New deterministic examples cover typed output, event observation, approval
  denial, report projection, and checkpoint/replay evidence using fake
  providers, file stores, and scripted approval dispatchers.
- README, Start Here, facade README, and example READMEs point to one coherent
  first-developer sequence with two explicit install paths:
  checkout-only facade usage and published split-crate usage. Each path names
  when to switch, copy-paste `Cargo.toml`, first command, expected output, and
  the canonical contracts underneath each simple example.
- Facade feature-selection guidance names real current features and validates
  the recommended combinations: `no-default-features`, `providers`,
  `test-support`, `evals`/`reports`, `workspace-tools` plus `macros`,
  `file-store`, `supabase-store`, `all-stable`, and the exact feature sets
  used by Phase II examples.
- Phase 16 risk/watchpoints document the new evidence-helper and example
  boundaries, including what must stay true when future examples or store
  helpers are added.
- `FunctionTool` becomes the builder-first public typed-tool authoring path in
  `agent-sdk-toolkit` and through the `clawdia-sdk::tools` facade. It accepts
  an explicit name, description, input schema, and typed executor, then lowers
  into `TypedTool`, `ToolPackBundle`, `ToolRoute`, provider-visible tool specs,
  `ToolExecutionCoordinator`, policy, journal records, and output content refs.
- Provider-visible tool descriptions become package/provider metadata, not
  executor behavior. Descriptions travel through tool-pack snapshots, routes,
  provider specs, and provider adapter request projection where the provider
  request shape supports descriptions.
- Store backend bundles explicitly expose the seven persistence surfaces named
  in the addendum. File-store coverage is updated for the new
  `ToolExecutionStore`. SQLite and Postgres-style adapter crates are added only
  with deterministic local or scripted transport tests.
- New roadmap examples with the requested names become runnable workspace
  packages. They may share implementation patterns with existing examples, but
  their package names, READMEs, expected outputs, SDK-owned boundaries,
  host-owned boundaries, and under-the-hood sections must match the requested
  roadmap.

Preserved behavior:

- `agent-sdk-core` remains dependency-light and receives no provider, toolkit,
  macro, store, report, UI, live infrastructure, or product-adapter dependency.
- `AgentApp` remains a facade over `AgentRuntime`; it does not own a runtime,
  package registry, event stream, journal, policy path, tool executor,
  telemetry truth store, session store, or approval UI.
- Tool execution still goes through `ToolExecutionCoordinator`, tool policy,
  approval dispatch when required, effect intent/result, journals, and events.
- Run reports remain post-hoc projections over durable records and host-owned
  cost policy.
- Tool execution remains journal-first. Any new `ToolExecutionStore` adapter is
  a subordinate projection/cache and cannot release executors, approve tools,
  replace effect intent/result records, or decide replay safety.
- Checkpoints remain accelerators; journals remain durable truth.
- Event frames returned from live subscriptions remain live/buffered
  observation; archived frames are read through the archive reader and still do
  not replace journal truth.
- Live credentials, store provisioning, approval UI, retention policy, backup
  policy, and product routing remain host-owned.
- Live provider examples remain deterministic by default. Real credentials,
  model selection, rate tables, and provider enablement remain host-owned and
  must be gated by environment variables or explicit Cargo features.

Removed behavior:

- None. Phase II is additive and documentation-tightening only.

Tests proving behavior:

- Facade public API tests for evidence helpers with deterministic fake/file
  stores, including missing-store diagnostics.
- Facade all-feature tests proving:
  `event_frames_for_run` reads live buffered frames only from the event bus;
  `archived_event_frames` reads only the archive reader when configured;
  `journal_records_for_run` reads only journal records; reports derive only
  from journal records; checkpoints remain accelerators and do not create
  journal truth.
- Deterministic example `cargo run` commands for every new example.
- Existing core replay, event, approval, typed-output, and report tests remain
  the lower-level contract proof.
- Toolkit tests proving `FunctionTool::builder` builds deterministic schemas,
  carries description metadata, lowers to package sidecars and provider specs,
  executes through `ToolExecutionCoordinator`, and stores output by content ref.
- Core/package/provider tests proving provider-visible tool descriptions flow
  from tool-pack snapshots to provider request bodies without changing executor
  identity, policy refs, or runtime-package truth.
- Store tests proving file, SQLite, and Postgres-style bundles map to
  `RunJournal`, `CheckpointStore`, `ContentStore`, `EventArchive`,
  `AgentPoolStore`, `ToolExecutionStore`, and `ProviderArgumentStore` without a
  global state-store umbrella.

## Phase II Workstreams

1. Facade evidence helpers:
   - add `AgentApp::stores`, `journal_records_for_run`,
     `event_frames_for_run`, `archived_event_frames`, `latest_checkpoint`, and,
     behind `evals`, `run_report_from_stores`;
   - optionally add a compact `AgentAppRunEvidence` DTO when it improves
     examples and tests without becoming a second trace store;
   - keep all helpers read-only projections over existing ports.
2. Facade diagnostics and tests:
   - add public API tests for missing stores, no second runtime path, canonical
     event subscription, report derivation, and checkpoint read behavior;
   - use deterministic fake providers, file stores, and scripted approval
     dispatchers.
3. Builder-first tool authoring:
   - add `FunctionTool` as a typed builder-first helper in
     `agent-sdk-toolkit` and expose it through `clawdia-sdk::tools`;
   - add description metadata to the package/provider projection path with
     targeted core, provider, and toolkit tests;
   - keep `#[agent_tool]` macro output lowering through the same typed-tool
     execution path.
4. Store backend mapping:
   - add `ToolExecutionStore` as an optional subordinate port over tool records
     and effect lineage, not as a runtime truth source;
   - update `agent-sdk-store-file` to expose the seven requested surfaces;
   - add `agent-sdk-store-sqlite` with deterministic local SQLite tests for
     all seven surfaces;
   - add `agent-sdk-store-postgres` as a scripted SQL/transport adapter with
     deterministic request/response tests for all seven surfaces and no live
     database requirement.
5. Requested example roadmap:
   - add `examples/10_facade_quickstart` as the first documented deterministic
     local checkout command;
   - add `examples/01_live_provider_text_run` for a deterministic text run
     with an explicitly gated live-provider path;
   - add `examples/02_typed_tool_builder` for `FunctionTool::builder` before
     the macro;
   - add `examples/06_checkpoint_resume` for checkpoint/replay
     resume-readiness evidence without claiming continuation;
   - add `examples/07_token_tracking_costs` for
     `UsageReport`/`StaticRateTable`/`RunReport` projections from journal
     evidence;
   - include per-example README files with command, expected output, failure
     modes, SDK-owned boundaries, host-owned boundaries, and "under the hood".
6. Existing Phase 16 example expansion:
   - add `examples/06_typed_output_and_events` for `run_typed`, event frames,
     and report evidence;
   - add `examples/07_approval_denial` for fail-closed approval behavior and
     journal/event evidence;
  - add `examples/08_checkpoint_replay` for checkpoint accelerator plus
    replay/resume-readiness projection using durable journal sequence evidence
    and checkpoint record validation;
   - include per-example README files with command, expected output, failure
     modes, SDK-owned boundaries, host-owned boundaries, and "under the hood".
7. Feature-selection and install docs:
   - add a first-30-minute section with two explicit paths:
     checkout facade path and published split-crate path;
   - add a facade feature matrix covering `default = []`,
     `no-default-features`, `providers`, `test-support`, `evals`/`reports`,
     `workspace-tools` plus `macros`, `file-store`, `supabase-store`,
     `stores`, `all-stable`, and the exact feature sets used by every example;
   - add validation for recommended feature combinations through existing
     `cargo test -p clawdia-sdk ...` gates and example `Cargo.toml` manifests.
8. Onboarding docs:
   - update root README, `docs/start-here.md`, `crates/clawdia-sdk/README.md`,
     and `docs/examples/**` index docs so the first-developer path is one
     sequence rather than a list of disconnected examples.
9. Narrow write scope:
   - default implementation scope is `crates/clawdia-sdk`, `examples`,
     `README.md`, `docs/start-here.md`, `docs/examples/**`,
     `docs/reference/dx-upgrade-risk-watchpoints.md`, and Phase 16 docs;
   - touch core, toolkit, eval, macros, or store crates only for a named
     diagnostic or contract gap with a targeted test;
   - this addendum names `crates/agent-sdk-core/**`,
     `crates/agent-sdk-toolkit/**`, `crates/agent-sdk-store-file/**`,
     `crates/agent-sdk-store-sqlite/**`, `crates/agent-sdk-store-postgres/**`,
     `crates/agent-sdk-provider/**`, and `crates/clawdia-sdk/**` as the
     current escalation/write surfaces.
10. Risk/watchpoints and phase evidence:
   - update `docs/reference/dx-upgrade-risk-watchpoints.md` with Phase 16
     evidence-helper and example risks;
   - create the Phase 16 exit report after implementation and validation.
11. Independent review and developer simulation:
   - plan must receive explicit PASS from architecture, testability/
     observability, and developer-perspective planning agents before code
     starts;
   - implementation must receive independent review PASS and developer
     simulation PASS before commit.

## Non-Goals

- Do not reference outside SDKs or use comparison tables.
- Do not add product-specific host adapters, UI behavior, store provisioning,
  credential management, or live infrastructure ownership.
- Do not add new adapter families beyond the named file, SQLite, and
  Postgres-style persistence scope in this addendum unless a later launch doc
  owns the crate, feature, tests, and risk notes explicitly.
- Do not move optional provider, toolkit, macro, store, report, or test-support
  dependencies into `agent-sdk-core`.
- Do not create a second runtime, package registry, event stream, journal,
  policy path, tool executor, telemetry truth store, or global state store.
- Do not claim live examples are runnable unless deterministic CI-safe paths and
  explicit live gates exist.
- Do not implement every item from the suggestion text. Implement only the
  pieces that fit current SDK contracts and can be made testable in this phase.

## Acceptance Criteria

- A new user can follow the docs from README to a deterministic agent run
  without reading architecture docs first.
- The same path then teaches typed output, typed tools, approval, event
  observation, run reports, and checkpoint/replay resume-readiness evidence
  through current SDK APIs.
- Every new helper or example states the canonical contracts underneath it.
- Every new public affordance has a lowering test, rustdoc or example coverage,
  and clear feature-gate behavior.
- E2E proof uses mocks/fakes or deterministic local stores; no live provider,
  product UI, or network infrastructure is required.
- Observability proof includes event frames, journal records, report evidence,
  and explicit checkpoint/replay limitations where relevant.
- Docs avoid external package references and only mention current local crates,
  current local examples, and SDK-owned contracts.
- Risk docs record any alpha breaking changes before release handoff.

## Validation Plan

- `cargo fmt --check`
- `cargo test -p clawdia-sdk --no-default-features`
- `cargo test -p clawdia-sdk --all-features`
- `cargo test -p agent-sdk-core`
- `cargo test -p agent-sdk-toolkit --all-features`
- `cargo test -p agent-sdk-eval`
- `cargo test -p agent-sdk-store-file`
- `cargo test -p agent-sdk-store-sqlite`
- `cargo test -p agent-sdk-store-postgres`
- `cargo test -p agent-sdk-store-supabase --all-features`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo doc --workspace --all-features --no-deps`
- `cargo run -p clawdia-sdk-example-10-facade-quickstart`
- `cargo run -p clawdia-sdk-example-01-live-provider-text-run`
- `cargo run -p clawdia-sdk-example-02-typed-tool-builder`
- `cargo run -p clawdia-sdk-example-06-checkpoint-resume`
- `cargo run -p clawdia-sdk-example-07-token-tracking-costs`
- `cargo run -p clawdia-sdk-example-01-facade-complex-agent`
- `cargo run -p clawdia-sdk-example-02-typed-tool-macro`
- `cargo run -p clawdia-sdk-example-03-file-store`
- `cargo run -p clawdia-sdk-example-04-supabase-scripted-store`
- `cargo run -p clawdia-sdk-example-05-reporting-and-eval`
- all Phase II example `cargo run` commands added by the launch target
- `cargo tree -p agent-sdk-core`
- `cargo tree -p clawdia-sdk --no-default-features`
- `cargo tree -p clawdia-sdk --all-features`
- source-layout audit commands from `docs/workstreams/validation-gates.md`
- `git diff --check`
- `scripts/public-release-audit.sh`

If time or environment constraints prevent the full workspace sweep, the
handoff must include the exact skipped command, why it was skipped, and the
targeted replacement evidence that was run.

## Review Packet Requirements

The final Phase II handoff must include:

- changed files by workstream;
- example commands and concise outputs;
- primitive-lowering evidence for every new helper and example;
- host-owned boundary evidence for providers, approval UI, live credentials,
  store provisioning, retention, and reports;
- feature-gate and dependency-tree evidence;
- independent implementation-review result;
- first-developer simulation result;
- unresolved risks or explicit statement that no blockers remain.

## Risk / Gotcha Carry-Forward

- If future helpers add more convenience, they must read through existing ports
  and lower into current runtime contracts; do not add an app-local event,
  report, journal, or session store.
- If future examples add live variants, keep the deterministic fake path as the
  default command and gate live credentials explicitly.
- If future checkpoint helpers become more automatic, keep checkpoint data as an
  accelerator and require journal evidence for resume readiness.
- If future report helpers need cost policy, keep rate tables host-owned and
  preserve limitations when evidence is absent.
- If future typed-tool helpers change approval behavior, keep
  `requires_approval` explicit and fail closed when no dispatcher is present.
- If future docs mention optional adapter areas, require concrete crates, tests,
  feature flags, and risk notes before advertising them in the first-developer
  path.
- If future store backends add convenience bundles, keep every persistence
  surface explicit. Do not merge journals, checkpoints, content, events, agent
  pools, tool execution cache records, and provider arguments into one global
  state store.
- If future tool builders add more ergonomic shortcuts, preserve description
  projection, explicit schema metadata, typed executor errors, content-ref
  output storage, and core tool-coordinator execution tests before exposing a
  macro-first path.
