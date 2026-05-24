# Run Handle

## Phase

[Phase 03: Run Control](README.md)

## Parallelism

Parallel-safe with [Agent Runtime](03a-agent-runtime.md) and [Loop State](03b-loop-state.md). Use fake event/journal stores from earlier phases.

## Contract Inputs

- [run-handle-reconnect-contract.md](../../contracts/run-handle-reconnect-contract.md)
- [event-schema.md](../../contracts/event-schema.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)

## Implementation Objective

Implement `RunHandle`, event stream reconnect, `wait()`, `status()`, cancellation, and terminal result consistency.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/run_handle.rs`
- `crates/agent-sdk-core/src/subscription.rs`
- `crates/agent-sdk-core/tests/run_handle_contract.rs`

## Must Deliver

- `RunHandle::stream_from(cursor)`, runtime-wide subscription helpers, `wait()`, `status()`, and idempotent cancel APIs.
- Cursor compatibility checks for all/run/agent/filter scopes.
- Terminal result consistency between handle status, journal terminal record, and event stream catch-up.

## Validation

- `cargo test -p agent-sdk-core --test run_handle_contract`
- reconnect catch-up tests
- duplicate subscriber and `wait()` idempotency tests
- no duplicate side effects on reconnect

## Must Not

- Treat `RunHandle` as the only event API.
- Resolve `wait()` when visible text is final but required journal/output/child bookkeeping is not terminal.
