# Telemetry Core

## Phase

[Phase 09: P2 Side Effects](README.md)

## Parallelism

Parallel-safe with the other Phase 09 side-effect launch targets. OTel exporter crate work waits for Phase 10 or later.

## Contract Inputs

- [telemetry-privacy-contract.md](../../contracts/telemetry-privacy-contract.md)
- [otel-mapping-contract.md](../../contracts/otel-mapping-contract.md)
- [event-schema.md](../../contracts/event-schema.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)

## Implementation Objective

Implement bounded telemetry fanout and usage/cost extraction as projections from events, journal, and policy decisions.

## Owned Implementation Surface

- telemetry domain/policy additions in existing `crates/agent-sdk-core/src/domain/` modules where needed
- telemetry durable records in `crates/agent-sdk-core/src/records/telemetry.rs`
- telemetry sink/usage ports in `crates/agent-sdk-core/src/ports/telemetry.rs`
- telemetry fanout and usage extraction coordination in `crates/agent-sdk-core/src/application/telemetry.rs`
- root Cargo test shim `crates/agent-sdk-core/tests/telemetry_core_contract.rs`
- test body `crates/agent-sdk-core/tests/feature_layers/telemetry_core_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/telemetry/`

Do not add flat implementation files directly under `src/`; exports from `lib.rs`
are integration/stitching glue.

## Must Deliver

- `TelemetrySink` core trait or sink spec, bounded fanout queue, overflow policy, sink failure/recovery records, usage extraction shell, and content-capture policy checks.
- Terminal-preserving behavior under slow sink pressure.
- Tests that telemetry cannot decide run state, policy outcome, output delivery success, or side-effect status.

## Validation

- `cargo test -p agent-sdk-core --test telemetry_core_contract`
- terminal preservation tests
- sink failure and repair cursor tests
- redaction/content-capture matrix tests

## Must Not

- Make telemetry a durable truth store or second event stream.
- Export raw content by default.
