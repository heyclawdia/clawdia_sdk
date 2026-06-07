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
- `clawdia_sdk::testing` is available with the `test-support` feature.

It does not own provider credentials, runtime policy, package resolution,
journals, event streams, tool execution, approval UI, telemetry storage,
workflow orchestration, or persistence backends.

## Install Shape

From the repository root:

```toml
[dependencies]
clawdia-sdk = { path = "crates/clawdia-sdk", default-features = false }
```

Optional groups map only to crates that exist today:

```toml
clawdia-sdk = {
  path = "crates/clawdia-sdk",
  default-features = false,
  features = ["providers", "workspace-tools", "evals"]
}
```

## Canonical Lowering

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
