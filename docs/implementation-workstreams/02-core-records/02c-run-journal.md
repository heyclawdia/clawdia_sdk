# Run Journal

## Phase

[Phase 02: Core Records](README.md)

## Parallelism

Parallel-safe with the other Phase 02 core-record launch targets. Keep reducer behavior local until Phase 08 hardening.

## Contract Inputs

- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)
- [event-schema.md](../../contracts/event-schema.md)
- [tool-approval-contract.md](../../contracts/tool-approval-contract.md)

## Implementation Objective

Implement append-only run journal records, cursors, checkpoints, and the shared effect intent/result spine.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/journal.rs`
- `crates/agent-sdk-core/src/effect.rs`
- `crates/agent-sdk-core/tests/journal_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/journal/`

## Must Deliver

- `RunJournal`, `JournalRecord`, `JournalRecordKind`, `JournalCursor`, checkpoint records, recovery records, `EffectIntent`, `EffectResult`, `IdempotencyKey`, and `DedupeKey`.
- Append-before-effect guard utilities.
- Terminal-result and unsafe-pending recovery markers.
- Golden fixtures for implemented journal record kinds.

## Validation

- `cargo test -p agent-sdk-core --test journal_contract`
- golden fixture tests for record schema versions
- intent append failure prevents execution test
- result append failure enters recovery test

## Must Not

- Add feature-specific side-effect ledgers.
- Let telemetry, live events, or trace sinks become durable run truth.
