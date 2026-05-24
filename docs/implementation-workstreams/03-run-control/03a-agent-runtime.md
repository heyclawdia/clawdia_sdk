# Agent Runtime

## Phase

[Phase 03: Run Control](README.md)

## Parallelism

Parallel-safe with [Loop State](03b-loop-state.md) and [Run Handle](03c-run-handle.md). Do not wire a complete P0 run here.

## Contract Inputs

- [api-contracts.md](../../contracts/api-contracts.md)
- [runtime-package-schema.md](../../contracts/runtime-package-schema.md)
- [run-handle-reconnect-contract.md](../../contracts/run-handle-reconnect-contract.md)

## Implementation Objective

Implement `Agent`, `AgentBuilder`, `AgentRuntime`, run registry shell, port registry, cancellation handles, and effective package resolution.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/application/agent.rs`
- `crates/agent-sdk-core/src/application/runtime.rs`
- `crates/agent-sdk-core/src/ports/mod.rs`
- `crates/agent-sdk-core/tests/runtime/runtime_contract.rs`
- root Cargo test-target shim `crates/agent-sdk-core/tests/runtime_contract.rs`

## Must Deliver

- Public `Agent` and `AgentRuntime` constructors.
- Runtime-owned provider, journal, event, content, policy, and optional sink ports.
- Package resolution before run start with deterministic fingerprint capture.
- Cancellation token creation and run registry entries without executing a full loop.

## Validation

- `cargo test -p agent-sdk-core --test runtime_contract`
- compile test for core without optional crates
- SDK package architecture audit for root facades and runtime test shims
- test that missing required ports return typed `AgentError` and fail closed

## Must Not

- Hide global mutable package or provider registries outside `AgentRuntime`.
- Implement product routing, UI dispatch, external runtime caches, or live providers.
