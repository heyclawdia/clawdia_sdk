# DX Phase II

## Phase

[Phase 16: DX Phase II](README.md)

## Parallelism

Only launch target in this phase. This target is intentionally serialized
because the work stitches docs, examples, facade ergonomics, diagnostics, and
review evidence into one first-developer path.

## Contract Inputs

- [Phase 15 exit report](../15-dx-completion/_phase/phase-exit-report.md)
- [Rust SDK DX Phase II Plan](../../plans/2026-06-08-rust-sdk-dx-phase-ii-plan.md)
- [dx-upgrade-risk-watchpoints.md](../../reference/dx-upgrade-risk-watchpoints.md)
- [facade-crate-proposal.md](../../reference/facade-crate-proposal.md)
- [sdk-review-checklist.md](../../reference/sdk-review-checklist.md)
- [simplicity-audit.md](../../reference/simplicity-audit.md)
- [primitive-map.md](../../architecture/primitive-map.md)
- [observability-and-lineage.md](../../architecture/observability-and-lineage.md)
- [tool-pack-contract.md](../../contracts/tool-pack-contract.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)
- [event-schema.md](../../contracts/event-schema.md)
- [telemetry-privacy-contract.md](../../contracts/telemetry-privacy-contract.md)

## Implementation Objective

Make the current SDK facade genuinely approachable without weakening the
kernel. The completed packet should let a new developer follow public docs and
run deterministic examples for:

- first text run;
- typed output;
- typed tool registration and execution;
- approval success or denial;
- event observation;
- usage, cost, and run reports;
- checkpoint/replay resume-readiness over current journal and checkpoint ports;
- feature-gated import and setup guidance.

## Owned Implementation Surface

Default writable scope:

- `Cargo.toml`
- `Cargo.lock`
- `README.md`
- `docs/start-here.md`
- `docs/examples/**`
- `docs/plans/2026-06-08-rust-sdk-dx-phase-ii-plan.md`
- `docs/reference/dx-upgrade-risk-watchpoints.md`
- `docs/reference/facade-crate-proposal.md`
- `docs/implementation-workstreams/README.md`
- `docs/implementation-workstreams/16-dx-phase-ii/**`
- `crates/clawdia-sdk/**`
- `examples/**`

Escalation-only diagnostic gap scope:

- `docs/reference/sdk-review-checklist.md`
- `crates/agent-sdk-core/src/application/**`
- `crates/agent-sdk-core/src/ports/**`
- `crates/agent-sdk-core/src/testing/**`
- `crates/agent-sdk-core/tests/**`
- `crates/agent-sdk-toolkit/src/**`
- `crates/agent-sdk-toolkit/tests/**`
- `crates/agent-sdk-eval/src/**`
- `crates/agent-sdk-eval/tests/**`
- `crates/agent-sdk-macros/**`
- `crates/agent-sdk-store-file/**`
- `crates/agent-sdk-store-supabase/**`

Do not edit escalation-only paths unless the handoff names the diagnostic or
contract gap, explains why the default facade/examples/docs scope is
insufficient, and adds or updates a targeted test for that gap.

## Must Deliver

- A single documented first-developer path through README, Start Here, facade
  docs, and example READMEs.
- Deterministic examples for event observation, checkpoint/replay
  resume-readiness, typed output, approval behavior, report projection, and
  feature selection where the current APIs support them.
- For each new example, a concise "under the hood" section naming the
  canonical contracts used by the simple path.
- Public diagnostics, rustdoc, or tests for common setup failures:
  missing feature flag, provider route, approval dispatcher, store reader,
  provider-argument readback, content ref, rate policy, or report evidence.
- Facade or toolkit helpers only where they reduce repeated ceremony and prove
  canonical lowering into existing contracts.
- Approval ergonomics that keep UI transport host-owned, journal intent/result
  before executor release, and fail-closed behavior when approval dispatch is
  required but absent.
- Event observation guidance that keeps live buffered events, archived frames,
  and durable journal truth distinct.
- Resume-readiness guidance that uses checkpoint, replay reducer, and journal
  reader ports instead of a facade-only session store.
- Feature-gate and dependency evidence showing `agent-sdk-core` stays
  dependency-light.
- Updated risk/watchpoint docs for any alpha breaking changes.

## Must Not

- Reference outside SDKs, comparison reports, or unrelated package families.
- Add provider, toolkit, macro, store, report, UI, live infrastructure, or
  product-adapter dependencies to `agent-sdk-core`.
- Create a second runtime, run loop, journal, event stream, package registry,
  policy path, context projection path, tool executor, telemetry truth store, or
  global state store.
- Make live credentials, hosted provisioning, approval UI, retention policy,
  backup policy, or product routing SDK-owned.
- Add advertised features for future adapters unless the concrete crate,
  namespace, tests, docs, and risk notes are implemented in this phase.
- Claim live examples are runnable unless they have deterministic fake paths and
  explicit live gates.

## Required Validation

- Phase 15 exit report exists and records reviewer PASS before Phase 16
  implementation starts.
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
- all new Phase II `cargo run` example commands
- `cargo tree -p agent-sdk-core`
- `cargo tree -p clawdia-sdk --no-default-features`
- `cargo tree -p clawdia-sdk --all-features`
- source-layout audit commands from `docs/workstreams/validation-gates.md`
- `git diff --check`
- `scripts/public-release-audit.sh`

## Handoff Requirements

The final handoff must include:

- changed files by workstream;
- tests, fixtures, examples, and commands run;
- skipped tests and why;
- primitive-lowering evidence for every helper and example;
- events, journal, telemetry, policy, content, store, and report boundaries
  touched;
- host-owned boundary evidence for providers, live credentials, approval UI,
  store provisioning, retention, and reports;
- feature-gate and dependency-tree evidence;
- independent implementation-review result;
- first-developer simulation result;
- unresolved risks, if any.
