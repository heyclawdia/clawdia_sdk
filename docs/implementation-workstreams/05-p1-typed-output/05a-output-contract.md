# Output Contract

## Phase

[Phase 05: P1 Typed Output](README.md)

## Parallelism

Parallel-safe with [Validation Repair](05b-validation-repair.md) and [Typed Result](05c-typed-result.md). Coordinate shared DTO names through the phase exit report.

## Contract Inputs

- [structured-output-contract.md](../../contracts/structured-output-contract.md)
- [runtime-package-schema.md](../../contracts/runtime-package-schema.md)
- [api-contracts.md](../../contracts/api-contracts.md)

## Implementation Objective

Implement output schema requests and helper lowering without creating a second run path.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/output_contract.rs`
- `crates/agent-sdk-core/tests/output_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/output_contract/`

## Must Deliver

- `OutputContract`, schema refs, inline schema support, typed-mode refs, validation policy, repair policy, and projection hints.
- `agent.run_typed::<T>` helper shape if a derive or schema provider is available in this phase; otherwise a documented staged placeholder.
- Normalization of `RunRequest.output_contract` into the effective runtime package sidecar/fingerprint.

## Validation

- `cargo test -p agent-sdk-core --test output_contract`
- schema ref serde fixtures
- helper lowering test proving canonical `RunRequest` path

## Must Not

- Trust provider-native schema enforcement as authoritative.
- Add business scoring, product form rendering, or custom host postprocessors.
