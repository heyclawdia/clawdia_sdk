# Replay Recovery

## Phase

[Phase 11: Replay Hardening](README.md)

## Parallelism

Parallel-safe with [Golden Coverage](11a-golden-coverage.md) and [Privacy Performance](11c-privacy-performance.md). Own only replay/recovery fixtures.

## Contract Inputs

- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)
- [run-handle-reconnect-contract.md](../../contracts/run-handle-reconnect-contract.md)
- [output-delivery-contract.md](../../contracts/output-delivery-contract.md)

## Implementation Objective

Prove resume, replay, repair, anti-entropy, and cursor compatibility across the implemented profiles and feature ports.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/application/replay.rs`
- `crates/agent-sdk-core/src/application/checkpoint.rs`
- `crates/agent-sdk-core/src/application/anti_entropy.rs`
- public facade exports wired from `crates/agent-sdk-core/src/lib.rs`
- `crates/agent-sdk-core/tests/replay_recovery.rs` as a root shim, with implementation tests under `crates/agent-sdk-core/tests/replay_recovery/`
- fixture files under `crates/agent-sdk-core/tests/fixtures/replay/`

## Must Deliver

- Replay reducer, checkpoint load/save shell, repair-needed outcomes, unsafe-pending side-effect manifest, anti-entropy scan/repair for derived views, and cursor compatibility checks.
- Tests for duplicate subscribers, output dedupe repair, non-idempotent pending tool refusal, missing content refs, sink repair cursors, and terminal checkpoint preservation.

## Validation

- `cargo test -p agent-sdk-core --test replay_recovery`
- recovery golden fixtures
- property/table tests for cursor compatibility

## Must Not

- Pretend core owns global durable all-event replay without an `EventArchive` or indexed journal view port.
- Compensate external side effects inside core.
- Add flat implementation modules directly under `crates/agent-sdk-core/src/`; the replay APIs must preserve the DDD package layout gate.
