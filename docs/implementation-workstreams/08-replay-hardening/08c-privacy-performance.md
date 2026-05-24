# Privacy Performance

## Phase

[Phase 08: Replay Hardening](README.md)

## Parallelism

Parallel-safe with [Golden Coverage](08a-golden-coverage.md) and [Replay Recovery](08b-replay-recovery.md).

## Contract Inputs

- [telemetry-privacy-contract.md](../../contracts/telemetry-privacy-contract.md)
- [event-schema.md](../../contracts/event-schema.md)
- [context-memory-contract.md](../../contracts/context-memory-contract.md)
- [stream-rule-contract.md](../../contracts/stream-rule-contract.md)

## Implementation Objective

Harden default privacy, redaction, queue bounds, and hot-path performance before scenario release.

## Owned Implementation Surface

- `crates/agent-sdk-core/tests/privacy_performance.rs`
- benchmark or allocation-test surfaces only if the repo accepts them in Phase 00
- redaction fixtures under `crates/agent-sdk-core/tests/fixtures/privacy/`

## Must Deliver

- Raw-content opt-in tests across event, journal, telemetry, context, stream-rule, tool, and output paths.
- Bounded queue and slow-subscriber behavior tests.
- Hot-path event filter tests proving no payload parsing, content-store lookup, or journal scan.
- Backpressure/overflow behavior tests with terminal preservation.

## Validation

- `cargo test -p agent-sdk-core --test privacy_performance`
- redaction matrix audit
- event hot-path audit
- bounded channel and slow sink tests

## Must Not

- Add unbounded queues by default.
- Emit raw prompt/model/tool/file content to telemetry or events without policy opt-in.
