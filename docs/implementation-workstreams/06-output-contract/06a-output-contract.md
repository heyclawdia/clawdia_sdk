# Output Contract

## Phase

[Phase 06: Output Contract](README.md)

## Parallelism

Only launch target in this phase. It must finish before validation/repair and typed-result records start in Phase 07.

## Contract Inputs

- [structured-output-contract.md](../../contracts/structured-output-contract.md)
- [runtime-package-schema.md](../../contracts/runtime-package-schema.md)
- [api-contracts.md](../../contracts/api-contracts.md)

## Implementation Objective

Implement output schema requests and helper lowering without creating a second run path.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/records/output.rs`
- `crates/agent-sdk-core/src/domain/ids.rs` only for output-contract typed IDs.
- `crates/agent-sdk-core/src/domain/mod.rs` only for facade re-exports.
- `crates/agent-sdk-core/src/application/run.rs`
- `crates/agent-sdk-core/src/application/agent.rs`
- `crates/agent-sdk-core/src/application/runtime.rs`
- `crates/agent-sdk-core/src/package/mod.rs`
- `crates/agent-sdk-core/src/lib.rs` only for public facade wiring and re-exports.
- `crates/agent-sdk-core/tests/feature_layers/output_contract.rs`
- root Cargo test-target shim `crates/agent-sdk-core/tests/output_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/output_contract/`

## Must Deliver

- `OutputContract`, schema refs, inline schema support, typed-mode refs, validation policy, repair policy, and projection hints.
- `agent.run_typed::<T>` helper shape if a derive or schema provider is available in this phase; otherwise a documented staged placeholder that still lowers into canonical `RunRequest`.
- Normalization of `RunRequest.output_contract` into the effective runtime package sidecar/fingerprint.

## Validation

- `cargo test -p agent-sdk-core --test output_contract`
- schema ref serde fixtures
- helper lowering test proving canonical `RunRequest` path
- runtime package fingerprint test proving output contracts are normalized into
  the effective package snapshot without adding a second run path
- SDK package architecture audit for root facades and feature-layer test shim

## Must Not

- Trust provider-native schema enforcement as authoritative.
- Add business scoring, product form rendering, or custom host postprocessors.
- Implement validation, repair, or typed-result publication before Phase 07/08.
