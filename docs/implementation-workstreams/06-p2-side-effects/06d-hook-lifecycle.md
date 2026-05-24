# Hook Lifecycle

## Phase

[Phase 06: P2 Side Effects](README.md)

## Parallelism

Parallel-safe with the other Phase 06 side-effect launch targets. Extension-provided hooks wait for Phase 07 extension work.

## Contract Inputs

- [hook-lifecycle-contract.md](../../contracts/hook-lifecycle-contract.md)
- [runtime-package-schema.md](../../contracts/runtime-package-schema.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)

## Implementation Objective

Implement hook specs and lifecycle execution as package sidecars that lower into existing domain operations.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/hooks.rs`
- `crates/agent-sdk-core/tests/hook_lifecycle_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/hooks/`

## Must Deliver

- `HookSpec`, `HookPoint`, hook input/response DTOs, ordering, timeout, cancellation, failure policy, and mutation-right matrix.
- Config/code helper equivalence into package sidecars.
- Journal-before-apply for accepted hook responses.
- Security hooks fail closed.

## Validation

- `cargo test -p agent-sdk-core --test hook_lifecycle_contract`
- hook ordering and timeout tests
- package fingerprint equivalence test
- journal-before-apply tests

## Must Not

- Create a generic event emission hatch, generic side-effect queue, or active-run callback registry.
