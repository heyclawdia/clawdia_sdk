# Validation Repair

## Phase

[Phase 05: P1 Typed Output](README.md)

## Parallelism

Parallel-safe with [Output Contract](05a-output-contract.md) and [Typed Result](05c-typed-result.md).

## Contract Inputs

- [structured-output-contract.md](../../contracts/structured-output-contract.md)
- [loop-state-machine.md](../../contracts/loop-state-machine.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)

## Implementation Objective

Implement local parse/schema/semantic validation and bounded repair accounting over the existing P0 loop.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/validation.rs`
- `crates/agent-sdk-core/src/repair.rs`
- `crates/agent-sdk-core/tests/validation_repair.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/validation/`

## Must Deliver

- `StructuredOutputValidator`, validation errors with redacted summaries, repair prompt accounting, attempt limits, hostile schema limits, and terminal validation failure behavior.
- Journal/event records for validation attempts and repair retries.
- Tests for valid output, invalid output, repair success, repair exhaustion, hostile schema rejection, and content redaction.

## Validation

- `cargo test -p agent-sdk-core --test validation_repair`
- table tests for repair policies
- golden fixtures for validation and repair records

## Must Not

- Commit unvalidated typed output.
- Hide repair attempts from events, journal, or usage accounting.
