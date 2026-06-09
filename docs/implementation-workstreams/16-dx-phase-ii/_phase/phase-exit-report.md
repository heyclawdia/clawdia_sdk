# Phase 16 Exit Report

## Phase Objective

Phase 16 keeps the SDK's primitive kernel intact while making the first
developer path substantially shorter:

- `clawdia-sdk` remains a thin facade with feature-gated re-exports and no
  behavior fork.
- `AgentApp` wires canonical `Agent`, `RuntimePackage`, `AgentRuntime`,
  `RunRequest`, provider registry, journal, event bus, policy, output, and
  optional store ports.
- `FunctionTool::builder(...)` proves typed tool execution before any macro
  convenience path.
- File, SQLite, and Postgres-style store crates explicitly map the durable
  truth surfaces: `RunJournal`, `CheckpointStore`, `ContentStore`,
  `EventArchive`, `AgentPoolStore`, `ToolExecutionStore`, and
  `ProviderArgumentStore`.
- Usage and cost examples derive from journal-backed trace evidence instead of
  introducing a second telemetry source of truth.

## Dependency Status

- Phase 15 exit report exists at
  `docs/implementation-workstreams/15-dx-completion/_phase/phase-exit-report.md`
  and records PASS.
- Phase 16 implementation followed
  `docs/plans/2026-06-08-rust-sdk-dx-phase-ii-plan.md` and
  `docs/implementation-workstreams/16-dx-phase-ii/16a-dx-phase-ii.md`.
- The addendum implementation used the approved escalation scope for core
  tool metadata, provider tool projection, toolkit typed-tool authoring, and
  store-file/sqlite/postgres adapter contracts.

## Goal Status

PASS.

- `FunctionTool::builder("workspace_read")` lowers into `TypedTool`, package
  sidecars, `ToolRoute`, provider tool specs, policy, journal, event, and
  content-ref contracts.
- `ToolExecutionStore` is a rebuildable projection over journaled
  `ToolCallRecord` evidence, with run, tool-call, effect-id, idempotency-key,
  journal-sequence, and journal-cursor-range reads.
- `agent-sdk-store-file`, `agent-sdk-store-sqlite`, and
  `agent-sdk-store-postgres` implement every requested durable surface.
- `agent-sdk-store-sqlite` owns its SQLite `AgentPoolStore` adapter directly;
  the `clawdia-sdk/sqlite-store` feature does not pull toolkit/provider/eval/
  macro dependencies.
- The requested examples are workspace packages and run without credentials by
  default:
  `examples/10_facade_quickstart`,
  `examples/01_live_provider_text_run`,
  `examples/02_typed_tool_builder`,
  `examples/06_checkpoint_resume`, and
  `examples/07_token_tracking_costs`.
- Example READMEs include expected output, common failures, and explicit
  "Under The Hood" contract sections.

## Validation Evidence

All commands passed:

```text
cargo fmt --check
cargo test -p agent-sdk-store-file
cargo test -p agent-sdk-store-sqlite
cargo test -p agent-sdk-store-postgres
cargo test -p clawdia-sdk --all-features --test public_api
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo doc --workspace --all-features --no-deps
git diff --check
scripts/public-release-audit.sh
```

Feature-boundary evidence:

```text
sqlite-store boundary OK: no toolkit/provider/eval/macro dependencies
```

Requested example outputs:

```text
cargo run -p clawdia-sdk-example-10-facade-quickstart
output=quickstart complete; status=Completed; records=10; events=8; provider_calls=1

cargo run -p clawdia-sdk-example-01-live-provider-text-run
output=fake provider text run; status=Completed; records=10

cargo run -p clawdia-sdk-example-02-typed-tool-builder
output=typed tool builder complete; records=16; tool_calls=1; provider_calls=2

cargo run -p clawdia-sdk-example-06-checkpoint-resume
output=checkpoint ready; records=11; resume_allowed=true; next_state=terminal:completed; checkpoint=checkpoint.example.resume.ready

cargo run -p clawdia-sdk-example-07-token-tracking-costs
output=cost evidence ready; records=10; provider_tokens=6; input_tokens=3; output_tokens=3; cost_micros=9
```

## Source Audit

Mandatory layout audit passed.

- `agent-sdk-core` stays dependency-light; no provider, toolkit, macro, store,
  report, UI, live infrastructure, or product-adapter dependency moved into
  core.
- Core tool-port changes are DTO/trait contract changes only; they do not add
  built-in tool behavior.
- Optional store crates keep behavior in responsibility modules. `mod.rs` /
  `lib.rs` surfaces remain facades with declarations and re-exports.
- The Postgres-style store crate is a scripted host-owned SQL transport
  adapter, not a live database client.
- Raw provider arguments and raw content remain behind `ProviderArgumentStore`
  and `ContentStore`; `ToolExecutionStore` stores redacted tool evidence only.

## Primitive-Lowering Evidence

- `AgentApp::run_text` and typed-tool examples still call the canonical
  `AgentRuntime` path.
- `AgentApp` stores are optional read/write ports. Missing required evidence
  ports return typed host-configuration diagnostics.
- `FunctionTool` has no private executor path. It builds `TypedTool`, which
  registers through the same toolkit package and core coordinator path used by
  existing typed tools.
- Tool descriptions are projected from toolkit builder metadata through core
  `ToolRoute` and provider `ProviderToolSpec` into OpenAI-compatible,
  Anthropic, and Gemini request tests.
- Checkpoint example 06 writes checkpoint evidence, appends a checkpoint
  journal record, rereads the durable record through `RunJournalReader`, and
  feeds that durable record into `ReplayReducer`.
- Token/cost example 07 derives `UsageReport` and `RunReport` from a
  journal-backed `RunTrace`; the rate table remains host-owned.

## Host-Owned Boundaries

- Provider credentials, live provider routing, endpoint/network access,
  billing, prompt copy, approval UI, actor identity, workspace authorization,
  migrations, hosted database provisioning, retention, backups, dashboards, and
  production retry policy remain host-owned.
- Live provider example 01 has a deterministic fake default path. The OpenAI
  path is explicitly gated by `OPENAI_API_KEY`.
- Postgres store tests use a scripted SQL transport and prove statement/param
  shape without claiming database provisioning.
- SQLite/file stores are local adapter implementations, not product session
  stores or global runtime state.

## Review Results

- Planning review before implementation: Socrates returned PASS after plan
  fixes; Galileo returned PASS after plan fixes.
- First implementation/DX review: Averroes returned BLOCK on tool approval
  docs, checkpoint replay evidence, README under-the-hood guidance, and example
  ordering. The issues were fixed.
- Architecture/testability review: Dewey returned BLOCK on missing
  `ToolExecutionStore` effect-id and cursor-range reads, the SQLite store
  crate's toolkit dependency, and missing explicit README "Under The Hood"
  headings. The issues were fixed.
- Final architecture/testability re-review: Dewey returned BLOCK on invalid
  Postgres `on conflict do update` SQL shape in the new scripted store
  adapter. The upserts were fixed with explicit conflict targets and `SET`
  clauses, regression assertions were added, and Dewey returned PASS.
- Final developer-experience re-review: Averroes returned PASS and
  independently reran all five requested examples with deterministic outputs.

## Accepted Proposals

- Add the local checkout `clawdia-sdk` facade quickstart as the fastest entry
  point while keeping split crates authoritative.
- Add `FunctionTool::builder(...)` before macro ergonomics.
- Add `ToolExecutionStore` as a rebuildable subordinate projection over
  journaled tool records.
- Add SQLite and Postgres-style store crates with the explicit durable-surface
  map requested by the user.
- Add deterministic fake-first examples with optional live-provider gates.

## Rejected Proposals

- Adding a facade-owned runtime, package registry, event stream, journal,
  policy path, tool executor, telemetry truth store, session store, or approval
  UI.
- Moving optional provider, toolkit, macro, store, report, UI, live
  infrastructure, or product-adapter dependencies into `agent-sdk-core`.
- Making `ToolExecutionStore` authoritative for replay, approvals, executor
  release, retries, or recovery completion.
- Claiming hosted Postgres provisioning or live credentials are SDK-owned.

## Deferred Work

- Attribute macro sugar beyond the existing `#[agent_tool]` path.
- Actual run-continuation resume API.
- Published facade release decision.
- Additional live-store examples behind explicit live gates.

## Shared Contract Changes

- `ToolRoute`, `ProviderToolSpec`, provider adapters, and toolkit builders now
  preserve tool descriptions across the projection path.
- `ToolExecutionStore` gained effect-id and journal-cursor-range reads.
- `agent-sdk-store-sqlite` now owns its SQLite `AgentPoolStore` implementation
  instead of re-exporting toolkit.
- `AgentAppStores` carries optional `tool_execution` as a rebuildable
  projection port.

These alpha changes are documented in
`docs/reference/dx-upgrade-risk-watchpoints.md`.

## Unresolved Risks

No unresolved implementation blockers.
