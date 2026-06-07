# Facade Complex Agent Example

## Command

```sh
cargo run -p clawdia-sdk-example-01-facade-complex-agent
```

## Expected Output

```text
facade example completed; records=18; events=13; usage_total_tokens=7
```

## What It Proves

This example runs a credential-free fake provider through `clawdia_sdk::AgentApp`
with file-backed stores, typed provider-argument refs, a typed tool,
approval dispatch before tool executor release, event subscription, durable
journal readback, and `RunReport` generation.

## SDK-Owned Boundary

The SDK owns the facade assembly, runtime request lowering, tool routing,
approval records, journal/event evidence, typed tool execution adapter, file
store ports, and report projection.

## Host-Owned Boundary

The host owns provider credentials, live provider routing, approval UI,
store root selection, and any live persistence provisioning. This example uses
deterministic local fakes instead of live services.

## Failure Modes

- Missing approval dispatcher for the approval-gated typed tool fails closed
  before executor release.
- Invalid provider argument JSON fails in the typed argument readback path.
- The temporary file-store directory is local example state and can be deleted
  after the run.
