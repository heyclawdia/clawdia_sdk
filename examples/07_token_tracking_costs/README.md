# Token Tracking Costs

Usage and cost projection from durable journal evidence.

Run:

```sh
cargo run -p clawdia-sdk-example-07-token-tracking-costs
```

Expected output shape:

```text
output=cost evidence ready; records=10; provider_tokens=6; input_tokens=3; output_tokens=3; cost_micros=9
```

No credentials are required. The fake provider emits deterministic usage counts.

Common failures:

- Missing provider usage in real adapters will produce report limitations rather
  than invented token totals.
- Cost numbers depend on the host-provided `StaticRateTable`; the SDK does not
  own pricing.

## Under The Hood

SDK-owned boundaries:

- `RunTrace`, `UsageReport`, `RunReport`, and cost projection over supplied
  journal evidence.

Host-owned boundaries:

- Rate tables, billing interpretation, dashboards, budgets, and long-term
  telemetry storage.

The example derives `UsageReport` and `RunReport` from `RunTrace` built from
journal records. It does not create a second telemetry source of truth.
