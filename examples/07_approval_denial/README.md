# Example 07: Approval Denial

## Command

```sh
cargo run -p clawdia-sdk-example-07-approval-denial
```

## Expected Output

```text
outcome=closed:PolicyDenial; message=tool call tool.call.example.denied_write did not complete before provider continuation; approval_denials=1; tool_records=0; events=8; report_records=12
```

## What It Proves

This example uses a typed write-like tool that requires approval, then injects
a deterministic host denial. The denied tool handler panics if it ever runs, so
the successful example proves that denial closes before executor release.

## Under The Hood

The provider requests a `write_note` tool call by content ref. `AgentApp`
resolves the typed tool route, dispatches approval through the host-owned
approval dispatcher, records the denial in the journal, publishes live event
frames, and returns a closed policy denial instead of executing the tool. The
example then reads `run_evidence` so approval records, live frames, and report
projection remain observable without merging their source boundaries.

## SDK-Owned Boundary

The SDK owns tool route projection, approval-gated executor release, denial
records, event publication, journal evidence, and report projection.

## Host-Owned Boundary

The host owns approval UI, actor identity, denial copy, provider credentials,
workspace authorization, and any retry or escalation behavior after a denial.

## Failure Modes

- Missing approval dispatch fails closed before tool execution.
- A denied approval does not synthesize a successful tool result.
- Provider continuation after an incomplete denied tool call remains a policy
  denial, which is why this example prints `closed:PolicyDenial`.
