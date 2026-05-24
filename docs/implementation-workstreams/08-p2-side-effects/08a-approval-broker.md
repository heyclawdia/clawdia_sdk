# Approval Broker

## Phase

[Phase 08: P2 Side Effects](README.md)

## Parallelism

Parallel-safe with the other Phase 08 side-effect launch targets. Coordinate shared policy enum changes through the phase exit report.

## Contract Inputs

- [tool-approval-contract.md](../../contracts/tool-approval-contract.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)
- [event-schema.md](../../contracts/event-schema.md)

## Implementation Objective

Implement approval policy and broker behavior as SDK policy decisions, not UI events.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/approval.rs`
- `crates/agent-sdk-core/tests/approval_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/approval/`

## Must Deliver

- Approval request lifecycle, finite decisions, timeout/cancel behavior, source-scoped approvals, and dispatcher port.
- `EffectKind::ApprovalDispatch` intent/result records before any host dispatcher can release a side effect.
- Fail-closed missing dispatcher and timeout tests.

## Validation

- `cargo test -p agent-sdk-core --test approval_contract`
- approval policy matrix tests
- golden fixtures for approval dispatch intent/result records

## Must Not

- Implement UI copy, out-of-band messages, or extension self-approval in core.
