# Event Frames

## Phase

[Phase 02: Core Records](README.md)

## Parallelism

Parallel-safe with the other Phase 02 core-record launch targets. Use shared IDs/refs from Phase 01 only.

## Contract Inputs

- [event-schema.md](../../contracts/event-schema.md)
- [run-handle-reconnect-contract.md](../../contracts/run-handle-reconnect-contract.md)
- [otel-mapping-contract.md](../../contracts/otel-mapping-contract.md)

## Implementation Objective

Implement canonical live event types, event frames, filters, cursors, overflow notices, and redaction defaults.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/event.rs`
- `crates/agent-sdk-core/src/event_bus.rs`
- `crates/agent-sdk-core/tests/event_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/events/`

## Must Deliver

- `AgentEvent`, `EventEnvelope`, `EventFrame`, `EventFilter`, `CompiledEventFilter`, `EventCursor`, optional `ArchiveCursor`, and overflow notice records.
- Payload-free hot-path filter matching over envelope/index fields.
- Redacted-summary and envelope-only defaults.
- Golden fixtures for every implemented event family/kind.

## Validation

- `cargo test -p agent-sdk-core --test event_contract`
- per-kind golden fixture checks
- property/table tests for filter compatibility and payload-free matching
- overflow terminal-preservation tests

## Must Not

- Parse payload JSON, query content stores, or scan journals on the live fanout path.
- Treat live events as durable truth.
