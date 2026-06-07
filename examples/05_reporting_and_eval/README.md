# Reporting And Eval Example

## Command

```sh
cargo run -p clawdia-sdk-example-05-reporting-and-eval
```

## Expected Output

```text
run.example.report cost report has no provider or tool usage; scope elapsed time is unavailable from durable timestamps; usage report has no journal records
```

## What It Proves

This example builds a deterministic `RunReport` over an empty local trace and
surfaces limitations explicitly instead of fetching rates, inferring missing
usage, or calling a provider.

## SDK-Owned Boundary

The SDK owns deterministic usage, cost, and run-report projections over
caller-supplied durable evidence.

## Host-Owned Boundary

The host owns which journal records are supplied, whether a rate table is
provided, and whether any provider-backed evaluator runs separately.

## Failure Modes

- Missing records produce explicit limitations.
- Missing rate policies keep unknown cost at zero with a limitation instead of
  using hidden network pricing.
