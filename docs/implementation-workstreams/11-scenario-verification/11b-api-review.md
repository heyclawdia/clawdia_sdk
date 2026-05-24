# API Review

## Phase

[Phase 11: Scenario Verification](README.md)

## Parallelism

Parallel-safe with [Scenario Tests](11a-scenario-tests.md). Release readiness is a later Phase 12 stitching target.

## Contract Inputs

- [api-contracts.md](../../contracts/api-contracts.md)
- [sdk-review-checklist.md](../../reference/sdk-review-checklist.md)
- [simplicity-audit.md](../../reference/simplicity-audit.md)
- [architecture/coding-standards.md](../../architecture/coding-standards.md)

## Implementation Objective

Run the final public API and simplicity review before release packaging.

## Owned Implementation Surface

- public exports in `crates/agent-sdk-core/src/lib.rs`
- rustdoc examples across crate modules
- `crates/agent-sdk-core/tests/public_api.rs`
- API docs under `docs/` only where implementation feedback requires contract clarification

## Must Deliver

- Public export audit for `Agent`, `AgentRuntime`, `RunRequest`, `RunHandle`, `RunResult`, runtime package, events, journal, context, output, policy, and ports.
- Rustdoc examples for common helpers and explicit advanced paths.
- Simplicity pass proving one-line helpers lower into canonical DTOs and do not bypass validation, policy, journal, event, telemetry, lineage, or redaction.
- SemVer posture for public enums, DTOs, and non-exhaustive fields.

## Validation

- `cargo test -p agent-sdk-core --test public_api`
- doctest/rustdoc compile checks
- no product-specific public export audit
- canonical helper lowering tests

## Must Not

- Add ergonomic helpers that create a second behavior path.
- Expose host-owned product behavior as core API.
