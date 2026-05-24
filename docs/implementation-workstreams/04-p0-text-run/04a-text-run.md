# Text Run

## Phase

[Phase 04: P0 Text Run](README.md)

## Parallelism

Only launch target in this phase. It integrates Phase 02 core records and Phase 03 run control.

## Contract Inputs

- [api-contracts.md](../../contracts/api-contracts.md)
- [loop-state-machine.md](../../contracts/loop-state-machine.md)
- [runtime-package-schema.md](../../contracts/runtime-package-schema.md)
- [event-schema.md](../../contracts/event-schema.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)
- [context-memory-contract.md](../../contracts/context-memory-contract.md)

## Implementation Objective

Prove the smallest complete SDK run: one fake provider text response through package resolution, context projection, provider call, events, journal records, and final `RunResult`.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/application/loop_driver.rs`
- `crates/agent-sdk-core/src/application/agent.rs`
- `crates/agent-sdk-core/src/application/run.rs`
- `crates/agent-sdk-core/src/application/runtime.rs`
- `crates/agent-sdk-core/src/records/event.rs`
- `crates/agent-sdk-core/src/records/journal.rs`
- `crates/agent-sdk-core/src/ports/event_bus.rs`
- `crates/agent-sdk-core/tests/p0/p0_text_run.rs`
- root Cargo test-target shim `crates/agent-sdk-core/tests/p0_text_run.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/p0/`

## Must Deliver

- `Agent::run_text(...)` and explicit `AgentRuntime::start_run(...)` path lowering into the same `RunRequest`.
- Context projection before provider request.
- Provider attempt event and journal records.
- Terminal run result with replayable journal facts.
- No optional feature requirement for P0.

## Validation

- `cargo test -p agent-sdk-core --test p0_text_run`
- `cargo test -p agent-sdk-core`
- P0 golden event and journal fixtures
- audit that P0 core builds with optional features disabled
- SDK package architecture audit for root facades and P0 test shim

## Must Not

- Add tools, approvals, typed output, stream rules, isolation, extensions, subagents, telemetry exporters, or host output delivery as prerequisites for P0.
