# Phase 15 Exit Report

## Phase Objective

Phase 15 turned the previously deferred first-user DX surfaces into
repo-grounded Rust implementation:

- `clawdia-sdk::AgentApp` facade assembly over canonical runtime ports;
- typed tool helpers, optional macros, provider-visible schema projection, and
  approval-gated execution;
- durable file and Supabase store adapters, including provider-argument
  readback and Supabase agent-pool state;
- deterministic usage, cost, and run reports;
- five credential-free runnable examples with per-example README evidence.

## Dependency Status

- Phase 14 exit report exists at
  `docs/implementation-workstreams/14-evaluation-metrics/_phase/phase-exit-report.md`.
- Phase 15 implementation followed
  `docs/plans/2026-06-07-rust-sdk-full-dx-completion-plan.md` and
  `docs/implementation-workstreams/15-dx-completion/15a-full-dx-completion.md`.

## Goal Status

PASS.

- Facade: `AgentApp`, `AgentAppStores`, approval dispatcher wiring, typed-tool
  registration, and store-backed journal reader access are implemented.
- Tools: typed tool schemas are sidecar payloads, provider specs project inline
  redacted schemas when available, and `require_approval()` lowers into
  `requires_approval` routes.
- Approval: core journals approval intent/result before executor release for
  explicit approval-gated routes and fails closed when the dispatcher is absent.
- Stores: file and Supabase stores implement journal, checkpoint, content,
  event archive, provider-argument, and agent-pool surfaces where in scope.
- Reports: usage, cost, and run reports are deterministic projections over
  supplied durable evidence and host-provided rate policy.
- Examples: five checkout examples run without live credentials and document
  command, expected output, failure modes, and SDK-owned/host-owned boundaries.

## Validation Evidence

All commands passed:

```text
cargo fmt --check
cargo test -p clawdia-sdk --no-default-features
cargo test -p clawdia-sdk --all-features
cargo test -p agent-sdk-toolkit --all-features
cargo test -p agent-sdk-macros
cargo test -p agent-sdk-eval
cargo test -p agent-sdk-store-file
cargo test -p agent-sdk-store-supabase --all-features
cargo test -p agent-sdk-provider
cargo test -p agent-sdk-provider --test provider_tool_projection
cargo test -p agent-sdk-core --test tool_execution_contract
cargo test -p agent-sdk-core --test agent_pool_contract
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
```

## Source Audit

Mandatory layout audit passed.

- `crates/agent-sdk-core/src` has no extra top-level implementation files
  beyond `lib.rs` and the existing responsibility folders.
- Root `crates/agent-sdk-core/tests/*.rs` files remain two-line Cargo shims;
  full test bodies live under responsibility folders.
- Optional store/provider/toolkit/eval crates use narrow `src/lib.rs` facades
  and responsibility modules.
- Public fakes and scripted helpers remain in `agent_sdk_core::testing`.
- `crates/agent-sdk-core/src/records` contains no adapter, resolver, fake, or
  scripted behavior.
- `cargo tree -p agent-sdk-core` shows core remains limited to serde,
  serde_json, sha2, and thiserror families. `clawdia-sdk --no-default-features`
  pulls only core plus dev-only serde.

## Review Results

- Implementation review agent `019ea396-c438-7a50-ae34-5bab5781f8be`: PASS.
- Developer-experience simulation agent `019ea397-8911-7e13-8e4c-c99ae02e91e4`:
  initial BLOCK for missing per-example READMEs; fixed by adding README files
  under all five example directories; focused re-review PASS.

No blocking findings remain.

## Accepted Proposals

- Add an unpublished `clawdia-sdk` facade while keeping split crates
  authoritative.
- Add typed tool helpers and optional macros that lower into package/tool
  contracts and core execution.
- Add file and Supabase store crates with explicit per-port ownership instead
  of a global state store.
- Add report helpers as projections over supplied evidence and host-owned rate
  policy.
- Add deterministic checkout examples without live credentials.

## Rejected Proposals

- Publishing or renaming crates in this phase.
- Moving optional provider/toolkit/eval/store dependencies into
  `agent-sdk-core`.
- Adding provider credentials, Supabase provisioning, RLS policy, approval UI,
  product workflow state, or live hosted infrastructure as SDK-owned behavior.
- Preserving legacy implicit approval behavior for high-risk routes that only
  carry approval policy metadata.

## Deferred Work

- Published facade release metadata and crates.io release packaging.
- Provider streaming/model catalog quickstarts.
- Live-provider and live-Supabase examples behind explicit live gates.
- Additional store backends beyond file, Supabase, and the existing toolkit
  SQLite agent-pool support.
- OTel/exporter, MCP/browser/web, and workflow adapter crates.

## Shared Contract Changes

- `ToolRoute` and `ToolPackToolSnapshot` now carry `requires_approval`; host
  approval dispatch is explicit and no longer inferred from risk plus policy
  metadata alone.
- `PackageSidecarSnapshot` can carry redacted package payloads for provider
  schema projection.
- `ProviderArgumentStore` now includes JSON readback by content ref for typed
  tool execution.
- `ProviderRequest` carries provider-visible tool specs with optional inline
  redacted schemas.
- `AgentAppStores` carries both journal write and journal reader ports.

These alpha breaking changes are documented in
`docs/reference/dx-upgrade-risk-watchpoints.md`.

## Unresolved Risks

No unresolved Phase 15 blockers.

Operational risks that remain host-owned are documented in the DX risk
watchpoints: live provider credentials, Supabase provisioning/RLS, approval UI,
rate policy selection, retention/backups, and live migration rollout.

## Next-Phase Readiness

Phase 15 is ready to exit. Later phases can start from the implemented facade,
tool, store, report, and example surfaces without treating those Phase 15 DX
items as deferred roadmap work.
