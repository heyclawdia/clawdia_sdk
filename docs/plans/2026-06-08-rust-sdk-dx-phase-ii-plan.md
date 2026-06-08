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
- Checkpoints remain accelerators; journals remain durable truth.
- Event frames returned from live subscriptions remain live/buffered
  observation; archived frames are read through the archive reader and still do
  not replace journal truth.
- Live credentials, store provisioning, approval UI, retention policy, backup
  policy, and product routing remain host-owned.

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
3. Example expansion:
   - add `examples/06_typed_output_and_events` for `run_typed`, event frames,
     and report evidence;
   - add `examples/07_approval_denial` for fail-closed approval behavior and
     journal/event evidence;
  - add `examples/08_checkpoint_replay` for checkpoint accelerator plus
    replay/resume-readiness projection using durable journal sequence evidence
    and checkpoint record validation;
   - include per-example README files with command, expected output, failure
     modes, SDK-owned boundaries, host-owned boundaries, and "under the hood".
4. Feature-selection and install docs:
   - add a first-30-minute section with two explicit paths:
     checkout facade path and published split-crate path;
   - add a facade feature matrix covering `default = []`,
     `no-default-features`, `providers`, `test-support`, `evals`/`reports`,
     `workspace-tools` plus `macros`, `file-store`, `supabase-store`,
     `stores`, `all-stable`, and the exact feature sets used by every example;
   - add validation for recommended feature combinations through existing
     `cargo test -p clawdia-sdk ...` gates and example `Cargo.toml` manifests.
5. Onboarding docs:
   - update root README, `docs/start-here.md`, `crates/clawdia-sdk/README.md`,
     and `docs/examples/**` index docs so the first-developer path is one
     sequence rather than a list of disconnected examples.
6. Narrow write scope:
   - default implementation scope is `crates/clawdia-sdk`, `examples`,
     `README.md`, `docs/start-here.md`, `docs/examples/**`,
     `docs/reference/dx-upgrade-risk-watchpoints.md`, and Phase 16 docs;
   - touch core, toolkit, eval, macros, or store crates only for a named
     diagnostic or contract gap with a targeted test.
7. Risk/watchpoints and phase evidence:
   - update `docs/reference/dx-upgrade-risk-watchpoints.md` with Phase 16
     evidence-helper and example risks;
   - create the Phase 16 exit report after implementation and validation.
8. Independent review and developer simulation:
   - plan must receive explicit PASS from architecture, testability/
     observability, and developer-perspective planning agents before code
     starts;
   - implementation must receive independent review PASS and developer
     simulation PASS before commit.

## Non-Goals

- Do not reference outside SDKs or use comparison tables.
- Do not add product-specific host adapters, UI behavior, store provisioning,
  credential management, or live infrastructure ownership.
- Do not add new adapter families unless a later launch doc owns the crate,
  feature, tests, and risk notes explicitly.
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
- `cargo test -p agent-sdk-store-supabase --all-features`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo doc --workspace --all-features --no-deps`
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
