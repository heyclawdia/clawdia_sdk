# Facade Quickstart

Deterministic first-run example for the `clawdia-sdk` facade.

Run:

```sh
cargo run -p clawdia-sdk-example-10-facade-quickstart
```

Expected output shape:

```text
output=quickstart complete; status=Completed; records=10; events=8; provider_calls=1
```

The exact counts may change when core event or journal vocabulary changes, but
the run should complete without credentials.

Common failures:

- Missing local checkout dependency paths mean the command is not running from
  the repository root.
- A public API or journal vocabulary change should update this example and its
  expected output together.

## Under The Hood

SDK-owned boundaries:

- `Agent`, `RuntimePackage`, `AgentRuntime`, `RunRequest`, journal records,
  live event frames, and report projection.

Host-owned boundaries:

- Provider choice, credentials, storage location, retention policy, prompt copy,
  and any app UI.

This uses `AgentApp` with the file store and fake provider. The facade wires
canonical runtime ports; durable evidence still comes from the journal and
optional store readers.
