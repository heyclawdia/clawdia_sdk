# Full DX Completion

## Phase

[Phase 15: DX Completion](README.md)

## Parallelism

Only launch target in this phase. This target is intentionally broad because the
user made the previously deferred DX gaps part of the same completion goal.

## Contract Inputs

- [Phase 14 exit report](../14-evaluation-metrics/_phase/phase-exit-report.md)
- [Rust SDK Full DX Completion Plan](../../plans/2026-06-07-rust-sdk-full-dx-completion-plan.md)
- [dx-upgrade-risk-watchpoints.md](../../reference/dx-upgrade-risk-watchpoints.md)
- [facade-crate-proposal.md](../../reference/facade-crate-proposal.md)
- [dx-gap-report-agents-sdk.md](../../reference/dx-gap-report-agents-sdk.md)
- [persistence-ownership-map.md](../../reference/persistence-ownership-map.md)
- [sdk-review-checklist.md](../../reference/sdk-review-checklist.md)
- [simplicity-audit.md](../../reference/simplicity-audit.md)
- [primitive-map.md](../../architecture/primitive-map.md)
- [observability-and-lineage.md](../../architecture/observability-and-lineage.md)
- [tool-pack-contract.md](../../contracts/tool-pack-contract.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)
- [event-schema.md](../../contracts/event-schema.md)
- [agent-pool-contract.md](../../contracts/agent-pool-contract.md)
- [telemetry-privacy-contract.md](../../contracts/telemetry-privacy-contract.md)

## Implementation Objective

Deliver the full first-user SDK experience through tested Rust code and runnable
examples:

- facade `AgentApp`;
- provider-visible tool projection;
- typed tool helpers and optional macros;
- durable file and Supabase store adapters;
- deterministic usage, cost, and run reports;
- facade-first examples that run with fake infrastructure and document
  host-owned live variants.

## Owned Implementation Surface

- `Cargo.toml`
- `Cargo.lock`
- `README.md`
- `docs/start-here.md`
- `docs/examples/**`
- `docs/reference/dx-gap-report-agents-sdk.md`
- `docs/reference/facade-crate-proposal.md`
- `docs/reference/dx-upgrade-risk-watchpoints.md`
- `docs/implementation-workstreams/README.md`
- `docs/implementation-workstreams/15-dx-completion/**`
- `crates/agent-sdk-core/src/application/**`
- `crates/agent-sdk-core/src/ports/**`
- `crates/agent-sdk-core/src/testing/**`
- `crates/agent-sdk-core/tests/**`
- `crates/agent-sdk-toolkit/src/**`
- `crates/agent-sdk-toolkit/tests/**`
- `crates/agent-sdk-eval/src/**`
- `crates/agent-sdk-eval/tests/**`
- `crates/agent-sdk-provider/src/**`
- `crates/agent-sdk-provider/tests/**`
- `crates/clawdia-sdk/**`
- `crates/agent-sdk-macros/**`
- `crates/agent-sdk-store-file/**`
- `crates/agent-sdk-store-supabase/**`
- `examples/**`

## Must Deliver

- `AgentApp` lowers into `Agent`, `RunRequest`, `RuntimePackage`,
  `ProviderRegistry`, `AgentRuntime`, `RunJournal`, `AgentEventBus`,
  `ContentResolver`, and tool policy/execution ports.
- `ProviderRequest` carries package-derived tool specs, and provider adapters
  render those specs in OpenAI, Anthropic, Gemini, and OpenAI-compatible wire
  bodies with deterministic tests.
- `AgentApp` supports approval dispatch for high-risk typed tools, fails closed
  when approval dispatch is required but absent, and journals approval
  intent/result before tool executor release.
- `RunJournalReader`, `EventArchiveReader`, `ContentStore`,
  `ProviderArgumentStore`, and `RuntimeStoreBundle` / `AgentAppStores` are
  defined before file and Supabase adapters depend on them.
- Typed tool helpers provide typed arguments/output, schema snapshots, stable
  identities, policy metadata, sync handlers, async adapter handlers, executor
  adapters, package bundle lowering, and structured tool errors.
- Optional macros generate typed tool helper calls without owning behavior, with
  `schema-generation` backed by deterministic `schemars` output and golden
  fixtures.
- File-backed stores implement journal, checkpoint, content, event archive, and
  provider-argument persistence with redaction and recovery tests.
- Supabase-backed stores implement the same SDK port families plus agent-pool
  persistence with redacted secrets, migration SQL, injected sync transport,
  scripted tests, and facade feature exports.
- Report helpers compute deterministic usage, cost, and run reports from
  supplied traces/journals and injected rate tables.
- Runnable examples cover the facade-first happy path with typed tools,
  approval, events, durable file stores, and run reports; typed tool macros;
  file provider-argument persistence; scripted Supabase store persistence; and
  reporting/eval helpers. Future richer examples can extend into typed output,
  checkpoint/resume, subagents, and live-provider variants only when those
  commands and validation gates exist.
- Onboarding and risk docs reflect the implemented surfaces and alpha breaking
  changes.

## Must Not

- Add provider, store, macro, HTTP, async-runtime, UI, workflow, browser, OTel,
  or product-adapter dependencies to `agent-sdk-core`.
- Create a second runtime, run loop, journal, event stream, package registry,
  policy path, tool executor, telemetry truth store, or global storage context.
- Make Supabase credentials, project provisioning, RLS policy, or live hosted
  infrastructure SDK-owned.
- Store raw provider tool arguments in journals, events, debug output, reports,
  or summaries.
- Claim async core execution or provider-native tool schema projection unless
  those contracts are implemented and tested.

## Required Validation

- Phase 14 exit report exists and records reviewer PASS before Phase 15 code
  implementation starts.
- `cargo fmt --check`
- `cargo test -p clawdia-sdk --no-default-features`
- `cargo test -p clawdia-sdk --all-features`
- `cargo test -p agent-sdk-toolkit --all-features`
- `cargo test -p agent-sdk-macros`
- `cargo test -p agent-sdk-eval`
- `cargo test -p agent-sdk-store-file`
- `cargo test -p agent-sdk-store-supabase --all-features`
- `cargo test -p agent-sdk-provider`
- `cargo test -p agent-sdk-provider --test provider_tool_projection`
- `cargo test -p agent-sdk-core --test tool_execution_contract`
- `cargo test -p agent-sdk-core --test agent_pool_contract`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo doc --workspace --all-features --no-deps`
- `cargo tree -p agent-sdk-core`
- `cargo tree -p clawdia-sdk --no-default-features`
- `cargo tree -p clawdia-sdk --all-features`
- source-layout audit commands from `docs/workstreams/validation-gates.md`
- Rust API Guidelines public-API review notes for every new public facade,
  macro, store, provider projection, and report type
- `cargo run -p clawdia-sdk-example-01-facade-complex-agent`
- `cargo run -p clawdia-sdk-example-02-typed-tool-macro`
- `cargo run -p clawdia-sdk-example-03-file-store`
- `cargo run -p clawdia-sdk-example-04-supabase-scripted-store`
- `cargo run -p clawdia-sdk-example-05-reporting-and-eval`
- `git diff --check`
- `scripts/public-release-audit.sh`

## Handoff Requirements

The final handoff must include:

- changed files by workstream;
- primitive-lowering evidence for facade and typed tools;
- host-owned boundary evidence for providers, live credentials, Supabase, RLS,
  reports, and examples;
- store recovery/redaction evidence;
- example commands and outputs;
- independent implementation-review result;
- independent developer-experience simulation result;
- unresolved risks, if any.
