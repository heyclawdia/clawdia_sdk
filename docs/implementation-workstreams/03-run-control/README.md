# Phase 03: Run Control

Implement runtime control surfaces after core records exist. All launch targets in this folder may run in parallel after Phase 02 exits, then Phase 04 integrates them into the first text run.

## Launch Targets

| Title | Run in parallel? | Purpose |
| --- | --- | --- |
| [Agent Runtime](03a-agent-runtime.md) | yes | Runtime ownership, port registry, run registry, cancellation, and package resolution shell. |
| [Loop State](03b-loop-state.md) | yes | Explicit loop state machine, transitions, stop reasons, and recovery classifications. |
| [Run Handle](03c-run-handle.md) | yes | `RunHandle`, reconnectable streams, `wait()`, `status()`, and cursor catch-up semantics. |

## Exit Gate

- [x] Runtime, state-machine, and handle tests pass independently.
- [x] Cancellation/status/reconnect surfaces compile without a provider run loop.
- [x] No duplicate event, journal, package, or policy path appears.
- [x] Phase exit report records reviewer PASS.
