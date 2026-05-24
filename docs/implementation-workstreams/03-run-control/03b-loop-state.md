# Loop State

## Phase

[Phase 03: Run Control](README.md)

## Parallelism

Parallel-safe with [Agent Runtime](03a-agent-runtime.md) and [Run Handle](03c-run-handle.md). Keep this as pure state-machine logic until Phase 04.

## Contract Inputs

- [loop-state-machine.md](../../contracts/loop-state-machine.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)
- [tool-approval-contract.md](../../contracts/tool-approval-contract.md)

## Implementation Objective

Implement the explicit agent loop state machine and transition validation.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/application/loop_state.rs`
- `crates/agent-sdk-core/src/application/recovery.rs`
- `crates/agent-sdk-core/tests/runtime/loop_state_contract.rs`
- root Cargo test-target shim `crates/agent-sdk-core/tests/loop_state_contract.rs`

## Must Deliver

- Finite states for input accepted, context assembled, provider requested, model output received, tool requested, approval required, recovery, completed, cancelled, and failed.
- Stop reasons, max-iteration outcomes, cancellation transitions, denial transitions, and retry/recovery classifications.
- Transition table tests and invalid-transition tests.

## Validation

- `cargo test -p agent-sdk-core --test loop_state_contract`
- table tests for every legal transition
- property/table tests for invalid transition rejection
- SDK package architecture audit for root facades and runtime test shims

## Must Not

- Perform provider, tool, journal, event, or UI side effects inside transition validation.
- Encode host product callbacks as loop states.
