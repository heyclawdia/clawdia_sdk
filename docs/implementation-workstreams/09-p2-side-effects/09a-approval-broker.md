# Approval Broker

## Phase

[Phase 09: P2 Side Effects](README.md)

## Parallelism

Parallel-safe with the other Phase 09 side-effect launch targets. Coordinate shared policy enum changes through the phase exit report.

## Contract Inputs

- [tool-approval-contract.md](../../contracts/tool-approval-contract.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)
- [event-schema.md](../../contracts/event-schema.md)

## Implementation Objective

Implement approval policy and broker behavior as SDK policy decisions, not UI events.

## Owned Implementation Surface

- approval domain additions in existing `crates/agent-sdk-core/src/domain/` modules where the finite policy vocabulary belongs
- approval durable records in `crates/agent-sdk-core/src/records/approval.rs`
- approval dispatcher/port traits in `crates/agent-sdk-core/src/ports/approval.rs`
- approval broker coordination in `crates/agent-sdk-core/src/application/approval.rs`
- root Cargo test shim `crates/agent-sdk-core/tests/approval_contract.rs`
- test body `crates/agent-sdk-core/tests/feature_layers/approval_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/approval/`

Do not add flat implementation files directly under `src/`; exports from `lib.rs`
are integration/stitching glue.

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
