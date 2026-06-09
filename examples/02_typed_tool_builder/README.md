# Typed Tool Builder

Builder-first typed tool example.

Run:

```sh
cargo run -p clawdia-sdk-example-02-typed-tool-builder
```

Expected output shape:

```text
output=typed tool builder complete; records=16; tool_calls=1; provider_calls=2
```

No credentials are required. The provider call sequence is deterministic: first
the fake provider requests `workspace_read`, then it returns final text after
the tool result is journaled.

Common failures:

- A schema or tool identity change should update both the builder and tests.
- Missing `AgentAppStores` provider-argument support will fail typed tool
  execution because raw provider arguments are intentionally stored by ref.

## Under The Hood

SDK-owned boundaries:

- Tool schema snapshot, package sidecar, `ToolRoute`, `ToolExecutionCoordinator`,
  policy check, journal intent/result records, event evidence, and output
  content refs.

Host-owned boundaries:

- Workspace authorization, real file I/O, approval UI, provider credentials,
  and retention policy.

This intentionally uses `FunctionTool::builder(...)` before any macro. The
built tool lowers to the same `TypedTool`, tool route, executor, journal, and
content-ref contracts used by the core runtime.
