# Isolation Port

## Phase

[Phase 07: Feature Ports](README.md)

## Parallelism

Parallel-safe with the other Phase 07 feature-port launch targets. Concrete adapters stay optional or host-owned.

## Contract Inputs

- [isolation-runtime-contract.md](../../contracts/isolation-runtime-contract.md)
- [runtime-package-schema.md](../../contracts/runtime-package-schema.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)

## Implementation Objective

Implement portable isolated execution contracts without requiring a concrete container/VM runtime.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/isolation.rs`
- optional `crates/agent-sdk-isolation/` for fake/portable adapter helpers
- `crates/agent-sdk-core/tests/isolation_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/isolation/`

## Must Deliver

- `ExecutionEnvironment`, `IsolationRequirementSnapshot`, `IsolationRuntime` trait, capability report, process spec, filesystem/network/secret policies, and isolation journal/event records.
- Class plus capability/trust-vector matching.
- Explicit downgrade approval/denial and fail-closed unsupported-host behavior.
- Fake isolation adapter for contract tests.

## Validation

- `cargo test -p agent-sdk-core --test isolation_contract`
- downgrade denial tests
- lifecycle intent/result fixtures
- process I/O redaction tests
- cleanup/reclaim recovery tests

## Must Not

- Silently downgrade to host execution.
- Vendor-lock core semantics to Docker, Apple Containerization, Firecracker, or remote sandboxes.
