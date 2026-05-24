# Subagents

## Phase

[Phase 09: Feature Ports](README.md)

## Parallelism

Parallel-safe with the other Phase 09 feature-port launch targets.

## Contract Inputs

- [subagent-contract.md](../../contracts/subagent-contract.md)
- [runtime-package-schema.md](../../contracts/runtime-package-schema.md)
- [run-handle-reconnect-contract.md](../../contracts/run-handle-reconnect-contract.md)
- [event-schema.md](../../contracts/event-schema.md)

## Implementation Objective

Implement parent-owned child runs over the existing runtime, package, event, journal, and policy primitives.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/subagent.rs`
- `crates/agent-sdk-core/tests/subagent_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/subagents/`

## Must Deliver

- `SubagentRequest`, `SubagentSupervisor`, `ContextHandoffPolicy`, child package stripping, parent mailbox/clarification DTOs, wrapped events, child journal refs, and usage rollup.
- Depth and recursion limits with no recursive subagent tools by default.
- Parent-owned cancellation and explicit detach behavior through child lifecycle records.

## Validation

- `cargo test -p agent-sdk-core --test subagent_contract`
- child package stripping tests
- `ContextHandoffPolicy::None` default tests
- wrapped event and usage rollup fixtures
- parent cancellation/detach replay tests

## Must Not

- Promote child runs into user chats.
- Create a second run loop or child-specific event stream.
