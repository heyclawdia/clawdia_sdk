# Subagents

## Phase

[Phase 10: Feature Ports](README.md)

## Parallelism

Parallel-safe with the other Phase 10 feature-port launch targets. Depends on
Phase 05 AgentPool coordination, not on sibling feature ports.

## Contract Inputs

- [subagent-contract.md](../../contracts/subagent-contract.md)
- [agent-pool-contract.md](../../contracts/agent-pool-contract.md)
- [runtime-package-schema.md](../../contracts/runtime-package-schema.md)
- [run-handle-reconnect-contract.md](../../contracts/run-handle-reconnect-contract.md)
- [event-schema.md](../../contracts/event-schema.md)

## Implementation Objective

Implement parent-owned child-run helpers over the existing `AgentPool`,
runtime, package, event, journal, and policy primitives.

## Owned Implementation Surface

- subagent package policy/sidecar snapshots in `crates/agent-sdk-core/src/package/subagent.rs`
- subagent durable records in `crates/agent-sdk-core/src/records/subagent.rs`
- subagent supervisor coordination in `crates/agent-sdk-core/src/application/subagent.rs`
- root Cargo test shim `crates/agent-sdk-core/tests/subagent_contract.rs`
- test body `crates/agent-sdk-core/tests/feature_layers/subagent_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/subagents/`

Do not add flat implementation files directly under `src/`; exports from `lib.rs`
are integration/stitching glue.

## Must Deliver

- `SubagentRequest`, `SubagentSupervisor`, `ContextHandoffPolicy`, child package
  stripping, generic `RunMessage`/`WakeCondition` lowering, wrapped events, child
  journal refs, and usage rollup.
- Depth and recursion limits with no recursive subagent tools by default.
- Parent-owned cancellation and explicit detach behavior through child lifecycle records.

## Validation

- `cargo test -p agent-sdk-core --test subagent_contract`
- child package stripping tests
- `ContextHandoffPolicy::None` default tests
- run-message clarification and wake-condition tests using AgentPool fixtures
- wrapped event and usage rollup fixtures
- parent cancellation/detach replay tests

## Must Not

- Promote child runs into user chats.
- Create a second run loop or child-specific event stream.
- Reintroduce subagent-specific mailbox or clarification DTOs instead of using
  `RunMessage` and `WakeCondition`.
