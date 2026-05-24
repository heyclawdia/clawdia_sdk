# Provider Port

## Phase

[Phase 02: Core Records](README.md)

## Parallelism

Parallel-safe with the other Phase 02 core-record launch targets. Keep the provider fake deterministic and transport-free.

## Contract Inputs

- [api-contracts.md](../../contracts/api-contracts.md)
- [loop-state-machine.md](../../contracts/loop-state-machine.md)
- [context-memory-contract.md](../../contracts/context-memory-contract.md)
- [telemetry-privacy-contract.md](../../contracts/telemetry-privacy-contract.md)

## Implementation Objective

Implement the provider adapter port and fake provider used by P0/P1 tests.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/provider.rs`
- `crates/agent-sdk-core/src/projection.rs`
- `crates/agent-sdk-core/tests/provider_projection_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/provider/`

## Must Deliver

- `ProviderAdapter` trait, projected request/response DTOs, stream chunk shell, provider capabilities, usage extraction shell, and deterministic fake provider.
- Projection from `ContextProjection` only, never from raw internal messages.
- Tests that private metadata is stripped unless policy explicitly allows projection.

## Validation

- `cargo test -p agent-sdk-core --test provider_projection_contract`
- projection golden fixtures
- fake-provider text response test
- no live provider/network dependency audit

## Must Not

- Make provider-native schema, streaming, or tool behavior SDK authority.
- Add live provider clients.
