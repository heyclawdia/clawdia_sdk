# Typed Result

## Phase

[Phase 05: P1 Typed Output](README.md)

## Parallelism

Parallel-safe with [Output Contract](05a-output-contract.md) and [Validation Repair](05b-validation-repair.md).

## Contract Inputs

- [structured-output-contract.md](../../contracts/structured-output-contract.md)
- [output-delivery-contract.md](../../contracts/output-delivery-contract.md)
- [event-schema.md](../../contracts/event-schema.md)

## Implementation Objective

Implement `ValidatedOutput` publication and typed result extraction after validation succeeds.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/validated_output.rs`
- `crates/agent-sdk-core/tests/p1_typed_output.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/p1/`

## Must Deliver

- `ValidatedOutput` with schema version, validation report, source attempt IDs, content refs, lineage, and redacted summary.
- `RunResult` typed extraction over validated output.
- Publication ordering: validation record before typed result publication.
- P1 integration test over the same P0 loop.

## Validation

- `cargo test -p agent-sdk-core --test p1_typed_output`
- P1 golden event and journal fixtures
- test that output delivery sinks are not required for typed result extraction

## Must Not

- Treat output delivery as a prerequisite for P1.
- Publish raw content by default.
