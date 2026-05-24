# Errors Policy

## Phase

[Phase 01: Shared Kernel](README.md)

## Parallelism

Parallel-safe with [Typed IDs](01a-typed-ids.md) and [Fake Fixtures](01c-fake-fixtures.md).

## Contract Inputs

- [api-contracts.md](../../contracts/api-contracts.md)
- [tool-approval-contract.md](../../contracts/tool-approval-contract.md)
- [telemetry-privacy-contract.md](../../contracts/telemetry-privacy-contract.md)
- [sdk-review-checklist.md](../../reference/sdk-review-checklist.md)

## Implementation Objective

Create shared error, policy, permission, sandbox, approval, and failure types that all later phases can use without inventing local enums.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/error.rs`
- `crates/agent-sdk-core/src/policy.rs`
- `crates/agent-sdk-core/tests/policy_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/policy/`

## Must Deliver

- `AgentError` with typed context and causal IDs.
- Finite policy decisions: allow, deny, ask, modify, defer, interrupt.
- Permission, sandbox, approval, escalation, privacy, and content-capture policy structs.
- Fail-closed defaults for missing policy, dispatcher, adapter, sink, store, or journal append.
- Table tests for policy matrices and missing dependency behavior.

## Validation

- `cargo test -p agent-sdk-core --test policy_contract`
- policy matrix table tests
- audit that no missing dispatcher/adapter path defaults to allow

## Must Not

- Encode UI copy, approval transport, provider routing UI, or product autonomy modes as core behavior.
