# Isolation Port

## Phase

[Phase 10: Feature Ports](README.md)

## Parallelism

Parallel-safe with the other Phase 10 feature-port launch targets. Concrete adapters stay optional or host-owned.

## Contract Inputs

- [isolation-runtime-contract.md](../../contracts/isolation-runtime-contract.md)
- [runtime-package-schema.md](../../contracts/runtime-package-schema.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)

## Implementation Objective

Implement portable isolated execution contracts without requiring a concrete container/VM runtime.

## Owned Implementation Surface

- isolation package sidecars/snapshots in `crates/agent-sdk-core/src/package/isolation.rs`
- isolation durable records in `crates/agent-sdk-core/src/records/isolation.rs`
- isolation runtime and adapter ports in `crates/agent-sdk-core/src/ports/isolation.rs`
- isolation matching/lifecycle coordination in `crates/agent-sdk-core/src/application/isolation.rs`
- deterministic isolation fakes or conformance helpers in `crates/agent-sdk-core/src/testing/` only when they are public SDK test-kit helpers
- root Cargo test shim `crates/agent-sdk-core/tests/isolation_contract.rs`
- test body `crates/agent-sdk-core/tests/feature_layers/isolation_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/isolation/`
- optional `crates/agent-sdk-isolation/` for fake/portable adapter helpers only if the phase exit plan chooses a crate split

Do not add flat implementation files directly under `src/`; exports from `lib.rs`
are integration/stitching glue.

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
