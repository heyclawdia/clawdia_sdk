# Phase 01: Shared Kernel

Implement the shared low-level types every later phase needs. All launch targets in this folder may run in parallel after Phase 00 exits.

## Launch Targets

| Title | Run in parallel? | Purpose |
| --- | --- | --- |
| [Typed IDs](01a-typed-ids.md) | yes | Public ID newtypes, entity refs, source/destination refs, privacy, retention, trust, and correlation keys. |
| [Errors Policy](01b-errors-policy.md) | yes | `AgentError`, policy enums, fail-closed decisions, and shared result types. |
| [Fake Fixtures](01c-fake-fixtures.md) | yes | Deterministic ID/time helpers, fake ports, fixture writers, and golden-test harness utilities. |

## Exit Gate

- [ ] Shared types compile and have serde/golden coverage where durable.
- [ ] Fakes are deterministic and usable by Phase 02 without live providers or hosts.
- [ ] Policy/error paths fail closed by default.
- [ ] Phase exit report records reviewer PASS.
