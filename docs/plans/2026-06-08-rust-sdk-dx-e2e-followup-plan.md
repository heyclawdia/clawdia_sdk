# Rust SDK DX E2E Follow-Up Plan

Status: Implemented, validated, independently reviewed, and PASS.

## Objective

Run a second developer-experience pass over Phase 16 and remove the remaining repeated ceremony from observable, testable SDK examples without adding a second runtime, trace store, telemetry layer, or product-specific behavior.

## Current Finding

Phase 16 delivered `AgentApp` read helpers, deterministic examples, and validation evidence. The remaining developer friction is that examples and e2e tests still need to manually assemble the same post-run evidence sequence:

- collect live run frames from the runtime event bus
- read durable journal records from `RunJournalReader`
- optionally read archived event frames through `EventArchiveReader`
- optionally load a checkpoint accelerator
- derive eval reports from journal records

That sequence is correct, but repeated. New SDK consumers should have a compact, clearly named facade helper for "show me the evidence for this run" while still seeing that live events, archives, journals, checkpoints, and eval reports are separate projections.

## Proposed SDK Shape

Add a small read-side facade DTO and helpers in `crates/clawdia-sdk/src/app.rs`:

- `AgentAppRunEvidence`
  - `run_id`
  - `live_event_frames`
  - `archived_event_frames`
  - `journal_records`
  - `latest_checkpoint`
- `AgentApp::run_evidence(&RunId) -> Result<AgentAppRunEvidence, AgentError>`
  - requires configured `AgentAppStores` because journal truth is part of the evidence snapshot
  - collects live frames through the canonical runtime event bus
  - reads journal records through `RunJournalReader`
  - reads archive frames only if an archive reader is configured, filtering the global archive by `event.envelope.run_id`
  - loads the latest checkpoint only if a checkpoint store is configured
  - never publishes events, appends journals, writes checkpoints, or mutates stores
- `AgentApp::run_report_from_evidence(&AgentAppRunEvidence, Option<&dyn CostPolicy>)`
  - behind the existing `evals` feature
  - derives reports only from `evidence.journal_records`

Keep existing granular helpers. They remain useful when a developer needs a single source or custom cursor.

## Boundary Decisions

- Core remains unchanged; this is a facade lowering helper over existing ports.
- Live events remain observation, not durable truth.
- Archived events remain an optional projection, not a journal replacement.
- Checkpoints remain resume accelerators, not append-only history.
- Eval reports remain derived projections over journal records.
- Missing optional archive/checkpoint ports should produce empty archive frames or `None` checkpoint in `run_evidence`; missing `AgentAppStores` remains a host-configuration error.

## Writable Scope

This follow-up stays inside the Phase 16 default writable scope:

- `crates/clawdia-sdk/**`
- `examples/**`
- this follow-up plan:
  `docs/plans/2026-06-08-rust-sdk-dx-e2e-followup-plan.md`
- `docs/reference/dx-upgrade-risk-watchpoints.md`
- `docs/implementation-workstreams/16-dx-phase-ii/_phase/phase-exit-report.md`

No core crate changes are planned.

## Test Plan

- Add public API tests proving:
  - `run_evidence` requires configured stores
  - configured stores with no archive reader return empty archived frames
  - configured stores with no checkpoint store return `None` for the latest
    checkpoint
  - the helper keeps live, archive, journal, report, and checkpoint evidence separate
  - archive reads are run-filtered even when the archive contains frames for another run
  - report derivation uses the evidence journal records
  - checkpoint evidence is absent until the checkpoint store is explicitly written
- Update examples 06 and 07 to use `run_evidence` and `run_report_from_evidence`.
- Keep example 08 focused on checkpoint replay, optionally using `run_evidence` for the initial journal read if it improves clarity.

## Validation Plan

Run the same Phase 16 validation loop after implementation:

- `cargo fmt --check`
- `cargo test -p clawdia-sdk --no-default-features`
- `cargo test -p clawdia-sdk --all-features`
- `cargo test -p agent-sdk-core`
- `cargo test -p agent-sdk-toolkit --all-features`
- `cargo test -p agent-sdk-eval`
- `cargo test -p agent-sdk-store-file`
- `cargo test -p agent-sdk-store-supabase --all-features`
- `cargo test --workspace --all-features`
- `cargo run -p clawdia-sdk-example-06-typed-output-and-events`
- `cargo run -p clawdia-sdk-example-07-approval-denial`
- `cargo run -p clawdia-sdk-example-08-checkpoint-replay`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo doc --workspace --all-features --no-deps`
- `cargo tree -p agent-sdk-core`
- `cargo tree -p clawdia-sdk --no-default-features`
- `cargo tree -p clawdia-sdk --all-features`
- source-layout audit commands from `docs/workstreams/validation-gates.md`
- `git diff --check`
- `scripts/public-release-audit.sh`

## Review Gate

Implementation may start only after independent reviewers explicitly PASS this plan:

- developer-experience reviewer: confirms the helper improves the first e2e testing path and does not hide required concepts
- architecture/testability reviewer: confirms the helper preserves SDK primitive boundaries and has enough tests

After implementation, run the same two-review loop against the diff and fix any blocking findings before commit.

## Risks And Mitigations

- Risk: a compact helper could imply one canonical "truth bundle."
  - Mitigation: name fields by source and document that journals are durable truth while other fields are projections or accelerators.
- Risk: archive helpers could accidentally report frames for other runs because archive reads are global.
  - Mitigation: filter archived frames by `frame.event.envelope.run_id` and test mixed-run archives.
- Risk: eval reports could be built from live events.
  - Mitigation: `run_report_from_evidence` reads only `evidence.journal_records`.
