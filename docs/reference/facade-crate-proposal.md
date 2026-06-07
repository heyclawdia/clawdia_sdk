# Facade Crate Proposal

## Recommendation

Add a convenience facade crate named `clawdia-sdk`, initially with
`publish = false`. Keep the existing split crates as the authoritative
implementation and dependency path. Do not create or publish a crate named
`agent-sdk` unless the repository policy is explicitly changed.

The facade is a dependency, re-export, and assembly convenience only. It must
not own runtime behavior, provider selection policy, credentials, journals,
events, tool execution, persistence truth, UI approval transport, workflow
orchestration, or telemetry storage.

Phase 15 adds `AgentApp` as the first app-building facade. It wires
caller-supplied canonical runtime ports and lowers helper calls into
`AgentRuntime::run_text` / `run_typed`. Its store helper accepts typed
journal-reader, event-archive, content, provider-argument, checkpoint, and
agent-pool ports from the file and Supabase adapter crates.

## Split Crates Only Or Convenience Facade?

Keep split crates and add a convenience facade.

Why keep split crates:

- `agent-sdk-core` stays lightweight and product-neutral.
- Optional crates can carry provider, toolkit, eval, protocol, persistence, and
  telemetry dependencies without forcing them on core users.
- Release cadence and SemVer pressure stay isolated by responsibility.

Why add a facade:

- First-time users need one obvious import path for common SDK assembly.
- The facade can group features and quickstarts without moving implementation
  into core.
- Documentation can show a simple app-builder path while advanced users keep
  direct access to the canonical crates.

## Naming

| Candidate | Decision | Reason |
| --- | --- | --- |
| `clawdia-sdk` | Recommended | Clear repository/product family identity, avoids the intentionally unused generic crate name, and can start unpublished. |
| `agent-sdk` | Rejected for now | Root docs already state this repository does not publish that crate name. Using it now would reverse policy and confuse existing split-crate guidance. |
| `agent-sdk-facade` | Deferred | Accurate but awkward for users and less useful as the main onboarding path. |
| Split crates only | Rejected as the final DX state | Architecturally clean but still too much setup friction for new users. |

## Re-Export Scope

The facade should re-export stable, user-facing surfaces from existing crates:

| Namespace | Re-exports |
| --- | --- |
| `prelude` | `agent_sdk_core::prelude::*` for the common core app-building surface. |
| `core` | Stable core crate-root exports for users who want explicit advanced imports through the facade. |
| `providers` | All current provider adapter exports when the `providers` feature is enabled. |
| `tools` | Toolkit crate-root exports when the `workspace-tools` feature is enabled. |
| `eval` | Evaluation crate-root exports when the `evals` feature is enabled. |
| `stores` | File and Supabase store adapter exports when their store features are enabled. |
| `testing` | Deterministic test helpers behind an explicit `test-support` feature. |

The facade should not re-export deep implementation modules as stable import
paths. It should prefer a prelude plus explicit namespaces such as
`clawdia_sdk::core`, `clawdia_sdk::providers`, `clawdia_sdk::tools`,
`clawdia_sdk::eval`, and `clawdia_sdk::testing`.

## Feature Design

Current first manifest posture:

```toml
[dependencies]
agent-sdk-core = { path = "../agent-sdk-core", version = "=0.1.0-alpha.3", default-features = false }
agent-sdk-provider = { path = "../agent-sdk-provider", version = "=0.1.0-alpha.3", optional = true, default-features = false }
agent-sdk-toolkit = { path = "../agent-sdk-toolkit", version = "=0.1.0-alpha.3", optional = true, default-features = false }
agent-sdk-eval = { path = "../agent-sdk-eval", version = "=0.1.0-alpha.3", optional = true, default-features = false }

[features]
default = []
providers = ["dep:agent-sdk-provider"]
workspace-tools = ["dep:agent-sdk-toolkit"]
evals = ["dep:agent-sdk-eval"]
reports = ["evals"]
macros = ["dep:agent-sdk-macros", "workspace-tools"]
file-store = ["dep:agent-sdk-store-file"]
supabase-store = ["dep:agent-sdk-store-supabase"]
stores = ["file-store", "supabase-store"]
test-support = ["agent-sdk-core/test-support"]
all-stable = ["providers", "workspace-tools", "evals", "macros", "file-store", "supabase-store"]
```

Future features should only be added when their crates exist and have tests:

- provider-specific splits if dependencies require them;
- MCP/ACP adapters;
- additional store adapters by persistence surface;
- OTel/exporter adapters;
- workflow helpers;
- isolation runtime adapters.

Default features should remain conservative until there is a deliberate release
decision to trade dependency weight for onboarding convenience.

## Dependencies That Remain Optional

- Provider adapters and HTTP/transport stacks.
- Toolkit dependencies such as regex, workspace readers, shell helpers,
  protocol helpers, and SQLite agent-pool support.
- Macro/proc-macro dependencies.
- Future store backends for journals, checkpoints, content blobs, event
  archives, agent-pool state, tool execution state, and provider argument
  capture.
- OTel/exporter dependencies.
- MCP/ACP/browser/web protocol clients.
- Workflow/orchestration helpers.
- Async runtime dependencies unless a feature explicitly documents them.

## Canonical Lowering

The facade may expose a simple builder such as `AgentApp::builder`, but the
builder must lower into the existing runtime path:

```text
AgentApp::builder(...)
  -> Agent::builder(...)
  -> RuntimePackage::builder(...)
  -> ProviderRegistry / ProviderAdapter
  -> RunRequest::text or RunRequest::typed_text
  -> AgentRuntime::run_text / run_typed
  -> policy checks
  -> journal records
  -> event frames
  -> telemetry/redaction projections
  -> output validation and delivery records
```

The facade builder should not own provider credentials, approval UI, file
policy, shell policy, workflow state, trace stores, or session storage. It can
accept host-provided adapters/stores or optional crate helpers and wire them
into `AgentRuntimeBuilder`.

## Migration Path

Existing users:

- No migration required. Direct dependencies on `agent-sdk-core`,
  `agent-sdk-provider`, `agent-sdk-toolkit`, and `agent-sdk-eval` remain
  supported.
- Existing crate-root imports and `agent_sdk_core::prelude::*` keep working.

New users:

- Start with `clawdia-sdk` once the facade exists.
- Use explicit current features for providers, workspace tools, evals, and
  deterministic test support.
- Use `workspace-tools`/`macros` for typed tool helpers, `stores` for file and
  Supabase adapters, and `reports`/`evals` for usage, cost, and run reports.
- Use `AgentAppStores::file` or `AgentAppStores::supabase` when a facade app
  needs typed tools to read provider argument refs or reports to read durable
  journal evidence.
- Treat live credentials, Supabase project provisioning, RLS policy, approval
  UI, and product routing as host-owned setup.
- Drop to split crates when they need tighter dependency control, advanced
  imports, or direct conformance testing.

Docs migration:

- Keep root README split-crate install examples.
- Add a separate facade quickstart only after the facade compiles.
- Each facade quickstart must include an "under the hood" section naming the
  canonical core contracts it lowers into.

## Implementation Phases

1. Documentation approval: accept or revise this proposal.
2. Create `crates/clawdia-sdk` with `publish = false`, empty default features,
   narrow facade `src/lib.rs`, and rustdoc examples that compile.
3. Add feature-gated re-exports for current provider/toolkit/eval crates.
4. `AgentApp` builder implemented with tests proving canonical lowering.
5. Add deterministic numbered examples for facade, macros, file store,
   Supabase scripted store, and reporting/eval.
6. Run full workspace validation and public-release audit.
7. Decide whether and when to publish the facade in release notes.

## Required Tests For The Implementation Phase

- `cargo test -p clawdia-sdk`
- `cargo test -p clawdia-sdk --no-default-features`
- `cargo test -p clawdia-sdk --all-features`
- `cargo tree -p clawdia-sdk --no-default-features`
- `cargo tree -p clawdia-sdk --features providers`
- `cargo doc -p clawdia-sdk --no-deps`
- Public API tests proving the facade imports core types and does not define a
  second runtime.
- Product-neutrality audit over facade docs and examples.
- `scripts/public-release-audit.sh`

## Proposal Blocks

Accepted:

- Add a convenience facade.
- Name it `clawdia-sdk` unless the user explicitly changes the naming policy.
- Keep default features minimal.
- Keep split crates as the authoritative implementation path.

Rejected:

- Publishing or renaming crates in this phase.
- Making `agent-sdk-core` depend on optional provider/toolkit/eval/store crates.
- Hiding policy, journal, event, telemetry, redaction, or tool execution behind
  facade-only behavior.
- Claiming a facade quickstart is runnable before it exists in CI.

Deferred:

- Exact default feature policy for a published facade.
- Store backend crate names beyond the implemented file and Supabase adapters.
- Whether an all-inclusive feature should include experimental features before a
  stable release.
