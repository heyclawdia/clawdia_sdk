# Typed Result

## Phase

[Phase 07: P1 Validation Result](README.md)

## Parallelism

Parallel-safe with [Validation Repair](07a-validation-repair.md) after Phase 06 exits. This target may use fake validation reports; Phase 08 owns the end-to-end P1 loop integration.

## Contract Inputs

- [structured-output-contract.md](../../contracts/structured-output-contract.md)
- [output-delivery-contract.md](../../contracts/output-delivery-contract.md)
- [event-schema.md](../../contracts/event-schema.md)

## Implementation Objective

Implement `ValidatedOutput` records and typed result DTOs after validation succeeds.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/records/validated_output.rs`
- `crates/agent-sdk-core/tests/feature_layers/validated_output_contract.rs`
- root Cargo test-target shim `crates/agent-sdk-core/tests/validated_output_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/validated_output/`

The phase integration owner will reconcile shared facade exports and shared
event/journal indices after parallel targets return.

## Must Deliver

- `ValidatedOutput` with schema version, validation report, source attempt IDs, content refs, lineage, and redacted summary.
- Typed extraction DTOs and error surfaces that Phase 08 can wire into `RunResult`.
- Publication ordering checks using fake validation reports: validation record before typed result publication.

## Validation

- `cargo test -p agent-sdk-core --test validated_output_contract`
- validated-output golden record fixtures
- fake-validation publication ordering tests
- SDK package architecture audit for root facades and feature-layer test shim

## Must Not

- Treat output delivery as a prerequisite for P1.
- Publish raw content by default.
- Own full P1 fake-provider loop integration.
