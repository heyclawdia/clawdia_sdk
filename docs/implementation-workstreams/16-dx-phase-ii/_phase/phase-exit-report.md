# Phase 16 Exit Report

## Phase Objective

Phase 16 turned the Phase 15 DX surfaces into a clearer first-developer path
without adding a second runtime or moving optional dependencies into core:

- read-side `AgentApp` helpers for live events, durable journal records,
  archived events, checkpoints, and report projection;
- deterministic examples for typed output/events, approval denial, and
  checkpoint/replay resume-readiness;
- README, Start Here, facade README, and example index updates for local
  checkout facade usage versus published split-crate usage;
- risk/watchpoint updates for evidence-helper boundaries and checkpoint wording;
- review and validation evidence for developer-friendliness, testability, and
  observability.

## Dependency Status

- Phase 15 exit report exists at
  `docs/implementation-workstreams/15-dx-completion/_phase/phase-exit-report.md`
  and records PASS.
- Phase 16 implementation followed
  `docs/plans/2026-06-08-rust-sdk-dx-phase-ii-plan.md` and
  `docs/implementation-workstreams/16-dx-phase-ii/16a-dx-phase-ii.md`.

## Goal Status

PASS.

- Facade evidence helpers are read-only projections over configured ports.
- `AgentAppRunEvidence` now gives developers one compact run evidence snapshot
  while keeping live events, archived events, journal records, and checkpoints
  in separate fields.
- Missing durable-evidence stores return typed
  `HostConfigurationNeeded` diagnostics instead of panics or silent success.
- Missing optional archive/checkpoint ports return empty archive frames or no
  checkpoint in `run_evidence`; missing `AgentAppStores` still fails with a
  typed host-configuration diagnostic.
- New examples run without live credentials and use fake providers, file
  stores, scripted approval dispatch, and replay reducer validation.
- Docs now present one first-developer path and an explicit facade feature
  matrix.
- Checkpoint language is limited to resume-readiness evidence; no run
  continuation API is claimed.

## Validation Evidence

All commands passed:

```text
cargo fmt --check
cargo test -p clawdia-sdk --no-default-features
cargo test -p clawdia-sdk --all-features
cargo test -p agent-sdk-core
cargo test -p agent-sdk-toolkit --all-features
cargo test -p agent-sdk-eval
cargo test -p agent-sdk-store-file
cargo test -p agent-sdk-store-supabase --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo doc --workspace --all-features --no-deps
cargo tree -p agent-sdk-core
cargo tree -p clawdia-sdk --no-default-features
cargo tree -p clawdia-sdk --all-features
git diff --check
scripts/public-release-audit.sh
```

Example outputs:

```text
cargo run -p clawdia-sdk-example-01-facade-complex-agent
facade example completed; records=18; events=13; usage_total_tokens=7

cargo run -p clawdia-sdk-example-02-typed-tool-macro
lookup_docs:1

cargo run -p clawdia-sdk-example-03-file-store
content.provider_arguments.7d6441497d2a000b8143602a "README.md"

cargo run -p clawdia-sdk-example-04-supabase-scripted-store
content.provider_arguments.7d6441497d2a000b8143602a "README.md" https://example.supabase.co/rest/v1/agent_sdk_provider_arguments

cargo run -p clawdia-sdk-example-05-reporting-and-eval
run.example.report cost report has no provider or tool usage; scope elapsed time is unavailable from durable timestamps; usage report has no journal records

cargo run -p clawdia-sdk-example-06-typed-output-and-events
typed_title=Review Phase 16; priority=high; validation_reports=1; events=11; records=16; report_records=16

cargo run -p clawdia-sdk-example-07-approval-denial
outcome=closed:PolicyDenial; message=tool call tool.call.example.denied_write did not complete before provider continuation; approval_denials=1; tool_records=0; events=8; report_records=12

cargo run -p clawdia-sdk-example-08-checkpoint-replay
output=checkpoint evidence ready; records=11; resume_allowed=true; replay_seq=11; next_loop_state=terminal:completed; checkpoint=checkpoint.example.ready
```

## Source Audit

Mandatory layout audit passed.

- `crates/agent-sdk-core/src` has no stray top-level implementation files
  beyond `lib.rs` and existing responsibility folders.
- Root `crates/agent-sdk-core/tests/*.rs` files remain two-line Cargo shims;
  full test bodies live under responsibility folders.
- Optional crate `src/lib.rs` files remain narrow facades relative to their
  responsibility modules.
- Public fakes and scripted helpers remain under `agent_sdk_core::testing`.
- `crates/agent-sdk-core/src/records` contains no adapter, resolver, fake, or
  scripted behavior.
- `cargo tree -p agent-sdk-core` remains limited to serde, serde_json, sha2,
  and thiserror families. `clawdia-sdk --no-default-features` pulls only core
  plus dev-only serde.

## Primitive-Lowering Evidence

- `AgentApp::run_text` and `run_typed` still lower into `RunRequest` and the
  canonical `AgentRuntime` path.
- `event_frames_for_run` collects the runtime event bus stream through
  `subscribe_run`; it does not read or mutate durable stores.
- `journal_records_for_run` reads through `RunJournalReader`.
- `archived_event_frames` reads through `EventArchiveReader` when configured.
- `latest_checkpoint` reads through `CheckpointStore`.
- `run_report_from_stores` derives `RunReport` from durable journal records.
- `run_evidence` collects live frames through `subscribe_run`, durable records
  through `RunJournalReader`, run-filtered archive frames through the optional
  `EventArchiveReader`, and optional checkpoints through `CheckpointStore`.
- `run_report_from_evidence` derives `RunReport` from the evidence snapshot's
  durable journal records only.
- Approval denial in example 07 goes through the toolkit typed tool route,
  approval dispatcher, core approval records, journal/event evidence, and
  fail-closed policy behavior before executor release.
- Checkpoint example 08 uses the run journal sequence as evidence, writes the
  checkpoint accelerator through `CheckpointStore`, appends a checkpoint
  journal record, reads that record back through `RunJournalReader`, and
  validates the durable checkpoint record through `ReplayReducer`.

## Host-Owned Boundaries

- Provider credentials, live provider routing, schema authoring policy, prompt
  copy, approval UI, actor identity, workspace authorization, store
  provisioning, retention/backups, and real resume execution remain host-owned.
- Examples use deterministic fake providers, file stores, and scripted
  approval dispatch; no live credentials or hosted provisioning are required.
- Reports remain post-hoc projections over supplied evidence and optional
  host-owned cost policy.

## Review Results

- Planning review before implementation: architecture PASS, testability/
  observability PASS, and developer-perspective PASS.
- Implementation review agent `019ea586-09f8-71a0-9dd2-b0ac2f29fe01`:
  initial BLOCK on checkpoint durable replay evidence and pending report
  status; fixed by appending, rereading, and replaying the durable checkpoint
  record plus updating this report; focused re-review PASS.
- First-developer simulation agent `019ea586-504d-7872-b3e8-7d12e2f975e7`:
  initial BLOCK on the same checkpoint durable replay evidence; fixed and
  focused re-review PASS.
- Second-pass DX E2E follow-up plan review:
  developer-experience initial BLOCK on a validation-command typo, fixed and
  re-reviewed PASS; architecture/testability initial BLOCK on shortened
  validation scope, missing optional-port tests, and broad docs scope, fixed and
  re-reviewed PASS.
- Second-pass implementation review:
  developer-experience reviewer `019ea667-db9d-7a61-b851-7164ce89d619`
  returned PASS with no blocking findings; architecture/testability reviewer
  `019ea667-bd32-7a90-9915-3e0c02eef5d2` returned PASS with no blocking
  findings. The non-blocking public API note was addressed by marking
  `AgentAppRunEvidence` `#[non_exhaustive]`.

## Accepted Proposals

- Add read-side facade helpers over existing evidence ports.
- Add `AgentAppRunEvidence` and `run_report_from_evidence` so examples and e2e
  tests can collect common evidence without hand-stitching every read helper.
- Add credential-free examples 06-08 with per-example README evidence.
- Document local checkout facade usage separately from published split-crate
  usage.
- Add a feature matrix for the current real facade features.
- Tighten risk docs around live events, event archives, journals, checkpoints,
  and reports as separate evidence surfaces.

## Rejected Proposals

- Adding a facade-owned runtime, package registry, event stream, journal,
  policy path, tool executor, telemetry truth store, session store, or approval
  UI.
- Moving provider, toolkit, macro, store, report, UI, live infrastructure, or
  product-adapter dependencies into `agent-sdk-core`.
- Claiming checkpoint/replay examples provide run continuation.
- Adding external SDK comparisons or unrelated package-family guidance.

## Deferred Work

- Actual run-continuation resume API and tests.
- Published facade release decision.
- Live-provider or live-store examples behind explicit live gates.
- Additional adapter families beyond the current implemented crates.

## Shared Contract Changes

- `AgentApp` now stores its optional `AgentAppStores` bundle so read-side
  helpers can access configured evidence ports and return host-configuration
  diagnostics when missing.
- `AgentAppRunEvidence` is now re-exported from `clawdia_sdk` and the prelude
  as a `#[non_exhaustive]` facade DTO. It is a read-side convenience type, not
  a new primitive or durable trace store.
- The first-developer docs now treat `clawdia-sdk` as a local checkout facade
  while split crates remain the published-alpha install path.

These alpha changes are documented in
`docs/reference/dx-upgrade-risk-watchpoints.md`.

## Unresolved Risks

No unresolved Phase 16 implementation blockers.
