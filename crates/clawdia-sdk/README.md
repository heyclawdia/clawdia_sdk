# clawdia-sdk

`clawdia-sdk` is an unpublished convenience facade over the split Agent SDK
crates. It exists to make first imports and examples easier without changing
runtime ownership.

The facade is intentionally thin:

- `clawdia_sdk::prelude::*` re-exports the common core app-building surface.
- `clawdia_sdk::core` re-exports `agent-sdk-core` for explicit advanced imports.
- `clawdia_sdk::providers` is available with the `providers` feature.
- `clawdia_sdk::tools` is available with the `workspace-tools` feature.
- `clawdia_sdk::eval` is available with the `evals` feature.
- `clawdia_sdk::stores` exposes optional file, SQLite, Postgres-style, and
  Supabase store adapters behind their feature flags.
- `clawdia_sdk::testing` is available with the `test-support` feature.

It does not own provider credentials, runtime policy, package resolution,
journals, event streams, tool execution, approval UI, telemetry storage,
workflow orchestration, or persistence backends.

## Install Shape

Use this unpublished facade from a repository checkout:

```toml
[dependencies]
clawdia-sdk = { path = "crates/clawdia-sdk", default-features = false }
```

Published-alpha consumers should use the split crates directly. The facade is
for checkout-based onboarding and examples until a release decision changes
`publish = false`.

The `macros` feature is also checkout-only in this repository because the
`agent-sdk-macros` crates.io package name is occupied by an unrelated project.

Optional groups map only to crates that exist today:

```toml
clawdia-sdk = {
  path = "crates/clawdia-sdk",
  default-features = false,
  features = ["providers", "workspace-tools", "evals"]
}
```

## Feature Matrix

| Feature | Use When | Pulls In |
| --- | --- | --- |
| `default = []` | You want only the core facade imports and `AgentApp`. | `agent-sdk-core` |
| `providers` | You need live provider adapter types. | `agent-sdk-provider` |
| `workspace-tools` | You need typed tools or workspace/toolkit helpers. | `agent-sdk-toolkit` |
| `macros` | You want checkout-only derive/attribute helpers for typed tools. | `workspace-tools`, `agent-sdk-macros` |
| `evals` | You need post-hoc trace, usage, cost, or run reports. | `agent-sdk-eval` |
| `reports` | Alias for report-focused users. | `evals` |
| `file-store` | You need local file-backed journal/content/provider-argument/checkpoint/event/agent-pool/tool-execution adapters. | `agent-sdk-store-file` |
| `sqlite-store` | You need SQLite-backed durable store adapters. | `agent-sdk-store-sqlite` |
| `postgres-store` | You need Postgres-style durable store adapters over host-owned SQL transport. | `agent-sdk-store-postgres` |
| `supabase-store` | You need the Supabase store adapter types. | `agent-sdk-store-supabase` |
| `stores` | You need all current store adapter families. | `file-store`, `sqlite-store`, `postgres-store`, `supabase-store` |
| `test-support` | You need deterministic fakes or scripted dispatchers in examples/tests. | `agent-sdk-core/test-support` |
| `all-stable` | You are testing every current facade surface. | Current provider, toolkit, eval, macro, and store features |

Example feature sets:

```toml
# Typed output, events, reports, and local durable evidence.
clawdia-sdk = { path = "crates/clawdia-sdk", features = ["evals", "file-store", "test-support"] }

# Approval-gated typed tools with deterministic dispatch.
clawdia-sdk = { path = "crates/clawdia-sdk", features = ["evals", "file-store", "test-support", "workspace-tools"] }

# Macro-authored typed tool schemas.
clawdia-sdk = { path = "crates/clawdia-sdk", features = ["macros"] }
```

## Runnable Checkout Path

From the repository root:

```sh
cargo run -p clawdia-sdk-example-10-facade-quickstart
cargo run -p clawdia-sdk-example-02-typed-tool-builder
cargo run -p clawdia-sdk-example-01-live-provider-text-run
cargo run -p clawdia-sdk-example-06-checkpoint-resume
cargo run -p clawdia-sdk-example-07-token-tracking-costs
cargo run -p clawdia-sdk-example-06-typed-output-and-events
cargo run -p clawdia-sdk-example-07-approval-denial
cargo run -p clawdia-sdk-example-08-checkpoint-replay
cargo run -p clawdia-sdk-example-01-facade-complex-agent
```

These commands require no live credentials. They use fake providers, local file
stores, and scripted approval dispatchers so tests and examples stay
deterministic.

## Under The Hood

### Canonical Lowering

The facade adds import convenience only. Calls still go through the same
canonical core types:

```rust
use clawdia_sdk::prelude::*;

fn main() -> Result<(), AgentError> {
    let agent = Agent::builder()
        .id(AgentId::new("agent.docs.facade"))
        .name("facade docs")
        .build()?;

    let request = RunRequest::text(
        RunId::new("run.docs.facade"),
        agent.id().clone(),
        SourceRef::with_kind(SourceKind::Host, "source.docs.facade"),
        "hello",
    );

    assert_eq!(request.agent_id, agent.id().clone());
    Ok(())
}
```

`AgentApp` evidence helpers also remain projections over canonical ports:

- `run_evidence` collects the common per-run evidence snapshot while keeping
  live events, archived events, journal records, and checkpoints in separate
  fields.
- `event_frames_for_run` reads buffered live frames from the runtime event bus.
- `journal_records_for_run` reads durable evidence through `RunJournalReader`.
- `archived_event_frames` reads configured event archives without replacing
  journal truth.
- `latest_checkpoint` reads checkpoint accelerators without creating resume
  execution.
- `run_report_from_stores` derives reports from journal records when `evals`
  is enabled.
- `run_report_from_evidence` derives reports from the snapshot's journal
  records when `evals` is enabled.
- `AgentAppStores` carries `tool_execution` as an optional rebuildable
  projection port. It does not replace the run journal as durable truth.
