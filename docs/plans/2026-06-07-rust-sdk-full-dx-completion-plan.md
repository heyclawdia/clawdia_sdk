# Rust SDK Full DX Completion Plan

## Objective

Complete the Rust SDK DX upgrade as a repo-grounded implementation, not a
deferred roadmap. The completed packet must let a new SDK user start from the
`clawdia-sdk` facade and build a realistic agent with typed tools, deterministic
test doubles, durable stores, Supabase-backed persistence, usage/cost reports,
and runnable examples while preserving the primitive kernel as the only
behavior authority.

## Root Cause / Problem Shape

The SDK has a strong product-neutral kernel and several optional crates, but the
first-user path still requires users to understand too many split ownership
surfaces before they can run a complete agent. The prior DX slice intentionally
added only a behavior-free facade and carried forward later gaps. That is no
longer sufficient for this goal.

The structural fix is to add thin convenience layers at the existing ownership
seams, each with lowering tests:

- `clawdia-sdk` owns facade assembly only.
- `agent-sdk-toolkit` owns typed tool authoring helpers.
- `agent-sdk-macros` owns optional compile-time sugar over toolkit helpers.
- dedicated store crates own durable adapters over existing persistence ports.
- `agent-sdk-eval` owns deterministic report projections.
- examples prove the full path with deterministic fakes and optional live
  provider or Supabase configuration.

No layer may introduce a second runtime loop, hidden global store, duplicate
journal, duplicate event stream, private tool execution path, or product-owned
adapter.

## Pre-Implementation Gate

Phase 15 must not start implementation until Phase 14 has a current exit report
at `docs/implementation-workstreams/14-evaluation-metrics/_phase/phase-exit-report.md`
showing its README exit gate and reviewer PASS. If the report is absent, the
first action after plan approval is to run the Phase 14 validation commands,
record the exit evidence, and get review confirmation before touching Phase 15
code.

## Relevant Existing Context

- `AGENTS.md`: no branch creation without approval; keep the SDK packet
  product-neutral; start from the repo reading path; choose one implementation
  launch file; document breaking alpha changes in risk docs.
- `README.md` and `docs/start-here.md`: the source-of-truth onboarding path and
  crate map must keep split crates visible while making the facade path easy.
- `coding_standards.md` and `docs/architecture/coding-standards.md`: public
  APIs must be small, mockable, deterministic, feature-gated, and validated by
  tests rather than examples only.
- `docs/workstreams/validation-gates.md`: implementation packets require
  ownership evidence, primitive-lowering evidence, host-boundary evidence,
  validation commands, and explicit accepted/rejected/deferred proposal blocks.
- `docs/implementation-workstreams/README.md`: active implementation work is
  phase-gated; this goal creates Phase 15 as a single DX-completion launch
  target because the user explicitly made the full DX surface in scope.
- `docs/reference/sdk-review-checklist.md`: review must check product
  neutrality, simplicity, canonical lowering, journal/event durability, privacy,
  package topology, and no mini-SDK drift.
- `docs/architecture/primitive-map.md`: simple, builder, and advanced APIs must
  all lower into the same kernel primitives.
- `docs/reference/persistence-ownership-map.md`: durable storage must stay
  separated by journal, checkpoint, content, event archive, agent pool, and
  provider argument ownership rather than becoming a vague state store.
- `docs/reference/simplicity-audit.md`: helpers are acceptable only when they
  remove repeated ceremony without hiding the canonical advanced path.
- `docs/reference/dx-gap-report-agents-sdk.md`,
  `docs/reference/facade-crate-proposal.md`, and
  `docs/reference/dx-upgrade-risk-watchpoints.md`: the first DX slice documented
  the facade and named the later gaps; this plan converts those gaps into the
  current implementation scope.
- `docs/implementation-workstreams/14-evaluation-metrics/14a-trace-metrics-and-comparison.md`:
  report helpers belong in `agent-sdk-eval` and optional toolkit ergonomics, not
  in core runtime execution.
- `crates/agent-sdk-core/src/application/runtime.rs`,
  `crates/agent-sdk-core/src/application/agent.rs`,
  `crates/agent-sdk-core/src/application/tool.rs`,
  `crates/agent-sdk-core/src/application/approval.rs`,
  `crates/agent-sdk-core/src/application/replay.rs`,
  `crates/agent-sdk-core/src/application/agent_pool.rs`,
  `crates/agent-sdk-core/src/ports/journal.rs`,
  `crates/agent-sdk-core/src/ports/content.rs`,
  `crates/agent-sdk-core/src/ports/event_bus.rs`, and
  `crates/agent-sdk-core/src/ports/tool.rs`: these are the runtime and port
  owners the new helpers must lower into.
- Official Supabase docs verified during planning: Supabase exposes database
  rows through a PostgREST REST API at `/rest/v1`; API keys belong in the
  `apikey` header; user/session identity, when used, belongs in
  `Authorization: Bearer ...`. The SDK adapter must keep credentials and RLS
  policy host-owned.

## Full DX Smoke Contract

The implementation must include one canonical facade-first program that uses
only `clawdia_sdk` public APIs and proves the full new-user path:

```rust
use clawdia_sdk::{eval::*, prelude::*, providers::*, stores::*, testing::*, tools::*};

#[derive(ToolArgs, serde::Deserialize, serde::Serialize)]
struct LookupArgs {
    query: String,
}

#[agent_tool(name = "workspace_lookup", version = "1")]
fn workspace_lookup(args: LookupArgs) -> ToolResult<LookupOutput> {
    Ok(LookupOutput::new(format!("found {}", args.query)))
}

let agent = Agent::builder()
    .id(AgentId::new("agent.dx.smoke"))
    .name("DX smoke agent")
    .build()?;
let stores = AgentAppStores::file(temp_dir.path());
let app = AgentApp::builder(agent)
    .provider("provider.fake", ScriptedProvider::tool_then_text(
        "workspace_lookup",
        "final answer",
    ))?
    .stores(stores.clone())
    .policy(AllowRunPolicy)
    .tool_policy(AllowToolPolicy)
    .approval_dispatcher(ScriptedApprovalDispatcher::new(
        ApprovalDispatchResponse::decision(ApprovalDecision::approved("actor.host.user")),
    ))
    .typed_tool(workspace_lookup_tool()?.require_approval())?
    .build()?;

let run_id = RunId::new("run.dx.smoke");
let run = app.run_text(run_id.clone(), "Use the lookup tool, then answer.")?;
let events = app.subscribe_run(run_id.clone(), None)?;
let records = stores.journal_reader.records_for_run(&run_id)?;
let rate_table = StaticRateTable::usd_per_million_tokens([
    ("provider.fake/model.fake", 1.0, 2.0),
]);
let report = app.run_report(&run_id, records.iter(), Some(&rate_table))?;

assert!(run.status.is_terminal());
assert!(events.count() > 0);
assert_eq!(report.usage.provider_call_count, 2);
```

The exact program may use the final API names created during implementation,
but it must preserve this capability set and live in a checked-in runnable
example plus targeted automated coverage for each boundary it exercises.

Required deterministic commands:

```bash
cargo run -p clawdia-sdk-example-01-facade-complex-agent
cargo run -p clawdia-sdk-example-02-typed-tool-macro
cargo run -p clawdia-sdk-example-03-file-store
cargo run -p clawdia-sdk-example-04-supabase-scripted-store
cargo run -p clawdia-sdk-example-05-reporting-and-eval
```

Live-provider or live-Supabase variants may exist, but the commands above must
run without credentials.

## Behavior Contract

New behavior:

- `clawdia-sdk::AgentApp` provides a facade builder that assembles an
  `Agent`, `RuntimePackage`, `AgentRuntime`, provider routes, tools, policies,
  journals, event bus, content resolver, stores, and report helpers through the
  canonical builders.
- Typed tool helpers let users declare typed arguments, output, schema snapshots,
  policy metadata, sync handlers, async-handler adapters, package bundle
  lowering, and core `ToolExecutor` adapters from `agent-sdk-toolkit`.
- Optional macros provide `ToolArgs` derive support and a typed tool macro that
  generates toolkit builder calls only.
- Provider tool projection carries typed tool declarations into provider
  requests and provider adapter wire bodies. Fake-only tool visibility is not
  enough for this goal.
- Facade approval wiring accepts a host approval dispatcher or broker. High-risk
  typed tools request approval before executor release; missing dispatchers fail
  closed and journal denial evidence.
- Read-side store contracts let resume, reports, examples, file stores, and
  Supabase share the same durable evidence API.
- Durable store adapters implement real SDK ports for file-backed local stores
  and Supabase-backed hosted stores. Store bundles are named adapter factories,
  not global state owners.
- Supabase support includes config/auth, redacted secrets, injected sync HTTP
  transport, curl-backed transport, migration SQL, scripted transport tests,
  journal/checkpoint/content/event archive/agent pool/provider argument adapter
  support, and facade re-exports behind a real feature.
- Report helpers produce deterministic usage, cost, and run reports from
  journals/traces with an injected rate table and explicit limitations.
- Runnable examples under `examples/` compile and run through deterministic
  fakes by default, with host-owned live provider and Supabase variants gated by
  env/config.
- Docs and risk notes state all SDK-owned and host-owned boundaries and any
  alpha breaking changes.

Preserved behavior:

- `agent-sdk-core` remains dependency-light and owns runtime, records, ports,
  policies, journals, events, replay, approval, checkpoints, and run control.
- Advanced split-crate APIs remain visible and supported.
- Provider credentials, project provisioning, RLS policy, UI, workflow engines,
  and product-specific adapters remain host-owned.
- Tool execution continues through `ToolExecutionCoordinator`,
  `ToolPolicyPort`, effect intent/result records, journals, and events.
- Evaluation remains post-hoc and optional; normal runs do not perform extra
  evaluator calls or cost lookups.

Removed behavior:

- The prior DX packet's deferral of `AgentApp`, typed tool helpers/macros,
  durable stores, observability/report helpers, Supabase support, and runnable
  examples is removed for this goal.
- Any docs that imply these surfaces are only future work must be updated once
  the code exists.

Tests proving behavior:

- Facade public API tests prove `AgentApp` lowers into the canonical `Agent`,
  `RunRequest`, `RuntimePackage`, `AgentRuntime`, provider registry, journal,
  content, policy, event bus, and store-reader ports instead of defining a
  second runtime.
- The facade complex-agent example runs a deterministic fake provider through
  file stores, typed tool arguments, approval dispatch, tool execution, events,
  and run reports.
- Provider projection tests prove package tool specs enter `ProviderRequest` and
  OpenAI, Anthropic, Gemini, and OpenAI-compatible adapters render provider
  tool declarations into their wire bodies without leaking raw tool arguments.
- Typed tool tests cover manual schema, generated schema, deterministic hash,
  stable IDs, sync handler success, async adapter success, argument decode
  failure, handler failure, output serialization failure, and coordinator
  journal integration.
- Macro compile tests prove generated code lowers into toolkit helpers and fails
  clearly for unsupported signatures.
- Store tests cover append ordering, idempotency, conflict detection, crash or
  partial-write recovery, checkpoint latest/prune behavior, missing content,
  privacy denial, bounded raw content reads, event cursors, agent-pool
  sequencing/dedupe, provider argument redaction, no secret debug leakage, and
  scripted Supabase request/response shapes.
- Report tests cover token/usage totals, tool counts, cost tables, missing-rate
  limitations, run report source cursors, and no evaluator calls.
- Example tests or CI commands compile and run deterministic fake paths for all
  checked-in runnable examples.

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

Do not edit product-specific host adapters or add non-SDK examples unless the
user explicitly requests a separate external task.

## Workstreams

0. Phase 14 exit preflight:
   - Run the Phase 14 README exit-gate commands if no current exit report exists.
   - Add `docs/implementation-workstreams/14-evaluation-metrics/_phase/phase-exit-report.md`
     with validation output summaries, source-layout/API audit notes, and
     reviewer PASS before Phase 15 implementation begins.

1. Facade application assembly:
   - Add `AgentApp` and `AgentAppBuilder` in `crates/clawdia-sdk`.
   - Keep the facade sync-first because `AgentRuntime` is sync today.
   - Provide methods for `run_text`, `run_typed`, event subscription, runtime
     access, package access, and report projection.
   - Add `AgentAppStores` / `RuntimeStoreBundle` wiring for journal reader,
     journal writer, checkpoint store, content store, event archive, provider
     argument store, and agent-pool store where configured.
   - Add facade approval wiring through a product-neutral approval dispatcher
     bridge that implements the core tool policy seam and fails closed without a
     dispatcher for tools marked `require_approval`.
   - Add direct-vs-facade lowering tests.

2. Provider-visible tool projection:
   - Add a typed provider-tool field to `ProviderRequest` using SDK-owned
     projection DTOs derived from `RuntimePackage::provider_tool_specs()`.
   - Lower package tool specs into every provider request in `loop_driver`
     before the first model call and continuation calls.
   - Update OpenAI, Anthropic, Gemini, and OpenAI-compatible adapters with
     deterministic wire-body tests for provider-native tool declarations.
   - Keep raw provider tool arguments out of provider request debug output,
     journals, events, and reports.

3. Typed tool helpers and macros:
   - Add `ToolArgs`, `ToolOutput`, `ToolIdentity`, `ToolSchemaSnapshot`,
     typed builders, typed executors, JSON argument/content store traits, and
     async-runner adapters to `agent-sdk-toolkit`.
   - Add `agent-sdk-toolkit` feature `schema-generation` with optional
     `schemars` support. Generated schemas must be normalized, SHA-256 hashed,
     and covered by golden fixtures; manual schemas remain supported.
   - Add `agent-sdk-macros` for derive and tool macro support that emits
     toolkit helper calls only.
   - Add macro compile-pass and compile-fail tests using `trybuild`.
   - Re-export from `clawdia-sdk::tools` behind real features.

4. Durable stores:
   - Add `agent-sdk-store-file` with file-backed journal, checkpoint, content,
     event archive, and provider argument support.
   - Add core read-side contracts before adapters:
     `RunJournalReader`, `EventArchiveReader`, `ContentStore`, and
     `ProviderArgumentStore`. `RunJournal` remains append-only; read helpers are
     explicit separate traits.
   - Add a named `RuntimeStoreBundle` / `AgentAppStores` adapter factory that
     groups compatible ports for facade wiring without becoming a global
     storage context.
   - Preserve separate ownership for each store family.

5. Supabase-backed persistence:
   - Add `agent-sdk-store-supabase` with config/auth, redacted secrets, transport
     injection, curl-backed transport, migration SQL, and scripted transport
     tests.
   - Implement journal, checkpoint, content, event archive, agent-pool, and
     provider-argument adapters over existing SDK records.
   - Add facade `supabase-store` feature and docs.

6. Observability and reports:
   - Add `UsageReport`, `CostPolicy`, `StaticRateTable`, `CostReport`,
     `RunReport`, and `RunReportLimitations` in `agent-sdk-eval`.
   - Add `UsageReport::from_run_trace`, `UsageReport::from_journal_records`,
     `CostReport::from_usage_report`, `RunReport::from_run_trace`, and
     `RunReport::from_journal_reader`.
   - Use injected rate tables and deterministic trace/journal inputs.
   - Missing model rates produce explicit limitations and zero unknown-cost
     totals; helpers must not fetch pricing or infer missing usage.
   - Add optional facade and toolkit conveniences without changing normal run
     execution.

7. Runnable examples:
   - Add numbered Cargo examples covering the facade full smoke path, typed
     tool macros, file provider-argument persistence, scripted Supabase stores,
     and reporting/eval helpers.
   - Treat typed output, checkpoint/resume, subagent coordination, live
     providers, and richer hosted Supabase flows as future examples unless
     their commands and validation gates are added with the same rigor.
   - Make fake paths deterministic and runnable without credentials.
   - Document live-provider and Supabase env/config as host-owned variants.
   - Add the five deterministic commands from the Full DX Smoke Contract as
     required validation gates.

8. Docs, risk notes, and navigation:
   - Update onboarding docs to show facade-first and split-crate advanced paths.
   - Update DX reference docs from gap language to implemented language.
   - Update risk watchpoints for new alpha breaking changes and future
     extension constraints.

## Observability And Testability From Day One

- Every new helper must have deterministic unit tests at its ownership seam.
- Every facade helper must prove canonical lowering rather than relying on a
  copy-run example.
- Every store adapter must use fake or scripted I/O in tests; no live Supabase
  project or credentials are required for CI.
- Report helpers must accept records/traces/rate tables as inputs; they must not
  fetch pricing, infer missing usage, or perform provider/evaluator calls.
- Examples must compile in deterministic mode. Live paths must be opt-in and
  skipped unless the host supplies credentials.
- Public docs must describe SDK-owned and host-owned boundaries for providers,
  tools, stores, Supabase, credentials, RLS, reports, and examples.

## Risk / Gotcha Carry-Forward

- Do not turn `AgentApp` into a second runtime. It may assemble and expose the
  canonical runtime but must not own execution semantics.
- Do not hide durable truth behind a session helper. Journals, checkpoints,
  content, event archives, agent-pool records, and provider arguments remain
  separate surfaces.
- Do not put store, HTTP, async-runtime, macro, OTel, browser, workflow, or UI
  dependencies into `agent-sdk-core`.
- Do not make Supabase credentials, project provisioning, migrations, or RLS
  policy SDK-owned. The SDK owns adapter shape and migration SQL only.
- Do not store raw provider tool arguments in journals, events, debug output, or
  summaries. Store raw arguments behind content refs with redacted summaries.
- Do not claim provider-native tool schema support unless the provider request
  contract actually projects tool schemas to providers. For this goal, provider
  request projection and adapter wire-body tests are required.
- Do not make async typed tools pretend the core runtime is async. Use an
  explicit host-provided runner until core has an async execution contract.
- Do not update examples as prose-only docs. Runnable examples need real Cargo
  projects or example targets and validation commands.

## Validation Plan

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

## Review Gates

Before implementation:

- Independent plan review must confirm this plan read the repo standards and
  relevant docs first, preserves primitive ownership, and is implementable
  without hidden deferrals for the requested gaps.

After implementation:

- Independent implementation review must compare changed files to this plan,
  coding standards, and DX risk watchpoints.
- Independent developer-experience simulation must start from the new docs and
  examples and verify a new user can build the full complex agent through the
  facade with deterministic fake infrastructure.
- Blocking findings must be fixed and re-reviewed before the goal can be marked
  complete.
