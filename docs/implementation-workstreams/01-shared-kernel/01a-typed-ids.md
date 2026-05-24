# Typed IDs

## Phase

[Phase 01: Shared Kernel](README.md)

## Parallelism

Parallel-safe with [Errors Policy](01b-errors-policy.md) and [Fake Fixtures](01c-fake-fixtures.md). Do not depend on their outputs.

## Contract Inputs

- [api-contracts.md](../../contracts/api-contracts.md)
- [event-schema.md](../../contracts/event-schema.md)
- [content-artifact-ref-contract.md](../../contracts/content-artifact-ref-contract.md)
- [open-questions-and-ambiguities.md](../../reference/open-questions-and-ambiguities.md)

## Implementation Objective

Implement public newtypes and durable refs used across run, event, journal, context, package, policy, and side-effect records.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/ids.rs`
- `crates/agent-sdk-core/src/refs.rs`
- `crates/agent-sdk-core/src/privacy.rs`
- `crates/agent-sdk-core/tests/id_ref_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/ids/`

## Must Deliver

- Newtypes for `RunId`, `AgentId`, `TurnId`, `AttemptId`, `EventId`, `MessageId`, `ContextItemId`, `RuntimePackageId`, `EffectId`, and cursor IDs.
- `EntityRef`, `SourceRef`, `DestinationRef`, `PolicyRef`, correlation keys, privacy classes, retention classes, and trust classes.
- Stable serde forms and redacted debug/display behavior.
- Golden fixtures proving durable JSON shape.

## Validation

- `cargo test -p agent-sdk-core --test id_ref_contract`
- golden fixture comparison for ID/ref serde
- no raw string IDs in public durable DTOs introduced by this slice

## Must Not

- Add feature-specific envelope IDs unless they are represented through `EntityRef`.
- Add product-specific source or destination variants.
