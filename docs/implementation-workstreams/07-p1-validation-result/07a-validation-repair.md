# Validation Repair

## Phase

[Phase 07: P1 Validation Result](README.md)

## Parallelism

Parallel-safe with [Typed Result](07b-typed-result.md) after Phase 06 exits. This target owns validator and repair behavior only; Phase 08 owns the end-to-end P1 loop integration.

## Contract Inputs

- [structured-output-contract.md](../../contracts/structured-output-contract.md)
- [loop-state-machine.md](../../contracts/loop-state-machine.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)

## Implementation Objective

Implement local parse/schema/semantic validation and bounded repair accounting over the existing P0 loop.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/application/validation.rs`
- `crates/agent-sdk-core/src/application/repair.rs`
- `crates/agent-sdk-core/tests/feature_layers/validation_repair.rs`
- root Cargo test-target shim `crates/agent-sdk-core/tests/validation_repair.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/validation/`

The phase integration owner will reconcile shared facade exports and shared
event/journal indices after parallel targets return.

## Must Deliver

- `StructuredOutputValidator`, validation errors with redacted summaries, repair prompt accounting, attempt limits, hostile schema limits, and terminal validation failure behavior.
- Journal/event records for validation attempts and repair retries.
- Tests for valid output, invalid output, repair success, repair exhaustion, hostile schema rejection, and content redaction.

## Validation

- `cargo test -p agent-sdk-core --test validation_repair`
- table tests for repair policies
- golden fixtures for validation and repair records
- SDK package architecture audit for root facades and feature-layer test shim

## Must Not

- Commit unvalidated typed output.
- Hide repair attempts from events, journal, or usage accounting.
- Own `RunResult` typed extraction or P1 end-to-end integration tests.
