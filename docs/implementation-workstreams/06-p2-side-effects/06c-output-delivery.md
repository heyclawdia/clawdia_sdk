# Output Delivery

## Phase

[Phase 06: P2 Side Effects](README.md)

## Parallelism

Parallel-safe with the other Phase 06 side-effect launch targets.

## Contract Inputs

- [output-delivery-contract.md](../../contracts/output-delivery-contract.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)
- [event-schema.md](../../contracts/event-schema.md)

## Implementation Objective

Implement output delivery as a host sink port with dedupe and journaled intent/result records.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/output_delivery.rs`
- `crates/agent-sdk-core/tests/output_delivery_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/output_delivery/`

## Must Deliver

- `DestinationRef`, `OutputSink`, output delivery policy, sink capability checks, chunk/final dispatch records, dedupe refs, and reconciliation records.
- Required-sink missing behavior as `HostConfigurationNeeded`.
- Replay repair that does not resend without dedupe proof.

## Validation

- `cargo test -p agent-sdk-core --test output_delivery_contract`
- delivery intent/result golden fixtures
- dedupe replay tests
- raw content policy tests

## Must Not

- Own channel UX, credentials, notification copy, ack stores, or retry schedulers.
