# Runtime Package

## Phase

[Phase 02: Core Records](README.md)

## Parallelism

Parallel-safe with the other Phase 02 core-record launch targets. Do not implement run-loop behavior here.

## Contract Inputs

- [runtime-package-schema.md](../../contracts/runtime-package-schema.md)
- [api-contracts.md](../../contracts/api-contracts.md)
- [feature-to-primitive-matrix.md](../../reference/feature-to-primitive-matrix.md)

## Implementation Objective

Implement the resolved per-run `RuntimePackage` as the execution authority and fingerprint source.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/package.rs`
- `crates/agent-sdk-core/src/capability.rs`
- `crates/agent-sdk-core/tests/runtime_package_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/package/`

## Must Deliver

- `RuntimePackage`, first-slice `CapabilitySpec`, `CapabilityKind`, `CapabilitySource`, catalog snapshots, package deltas, and typed sidecar refs.
- Deterministic fingerprint generation for implemented P0/P1/P2 fields.
- Reserved-variant readiness enforcement: inactive variants cannot execute or project.
- Golden fixtures for package snapshots and fingerprints.

## Validation

- `cargo test -p agent-sdk-core --test runtime_package_contract`
- fingerprint golden fixtures
- audit that provider route, output contract normalization, policy refs, executable refs, and sidecar refs use one package authority

## Must Not

- Create ambient registries outside `RuntimePackage`.
- Put host manifest, marketplace, install, runtime, or UI state into core package authority.
