# Agent Pool Coordination

## Phase

[Phase 05: Agent Pool Coordination](README.md)

## Parallelism

Only launch target in this phase. Run after Phase 04 P0 text run exits and
before Phase 06 output-contract work starts.

## Contract Inputs

- [agent-pool-contract.md](../../contracts/agent-pool-contract.md)
- [api-contracts.md](../../contracts/api-contracts.md)
- [event-schema.md](../../contracts/event-schema.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)
- [run-handle-reconnect-contract.md](../../contracts/run-handle-reconnect-contract.md)
- [runtime-package-schema.md](../../contracts/runtime-package-schema.md)
- [subagent-contract.md](../../contracts/subagent-contract.md)
- [primitive-map.md](../../architecture/primitive-map.md)
- [feature-to-primitive-matrix.md](../../reference/feature-to-primitive-matrix.md)

## Implementation Objective

Implement a minimal feature-layer coordination scope that lets agent runs send
content-ref-backed messages, subscribe to pool-scoped events, and suspend/resume
through event-filter wake conditions. This is the reusable base for later
subagent helpers and optional workflow crates.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/application/agent_pool.rs`
- `crates/agent-sdk-core/src/domain/ids.rs` only for adding `AgentPoolId`,
  `TopicId`, and `WakeConditionId` through the shared typed-ID pattern.
- `crates/agent-sdk-core/src/domain/refs.rs` only for adding agent-pool, topic,
  and wake-condition ref/destination vocabulary required by the new IDs.
- `crates/agent-sdk-core/src/domain/mod.rs` only for re-exporting the new domain
  IDs/refs through the established facade.
- `crates/agent-sdk-core/src/lib.rs` only for public facade wiring and re-exports.
- `crates/agent-sdk-core/src/records/event.rs`
- `crates/agent-sdk-core/src/records/journal.rs`
- `crates/agent-sdk-core/tests/feature_layers/agent_pool_contract.rs`
- root Cargo test-target shim `crates/agent-sdk-core/tests/agent_pool_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/agent_pool/`

## Must Deliver

- `AgentPool`, `RunAddress`, `RunMessage`, `MessageReceipt`, `MessageStatus`,
  and `WakeCondition` DTOs.
- Pool membership and topic records using existing typed IDs, `SourceRef`,
  `DestinationRef`, `EntityRef`, policy refs, and content refs.
- Shared typed IDs for `AgentPoolId`, `TopicId`, and `WakeConditionId`; these
  are added by this phase after Phase 01 establishes the ID/ref pattern, so
  earlier Phase 01/02 workers do not need to own agent-pool-specific IDs.
- Run-message event and journal records for accepted, delivered, responded,
  failed, timed out, expired, and cancelled delivery states.
- Address resolution rules proving `RunAddressTarget::Agent` selects only
  existing pool members under policy and does not implicitly start a run.
- Address resolution rules proving `RunAddressTarget::Pool` broadcasts only to
  current policy-selected members and never crosses pool boundaries.
- Wake-condition event and journal records for registered, triggered, timed out,
  and cancelled wake states.
- Pool subscriptions and wake filters that intersect caller filters with pool
  membership/topic refs and policy before reaching the runtime event bus.
- Idempotency/dedupe behavior for duplicate messages and wake registrations.
- Tests proving `WakeCondition` uses `EventFilter`/envelope fields and does not
  parse payload content on the hot path.

## Validation

- `cargo test -p agent-sdk-core --test agent_pool_contract`
- golden event fixtures for every emitted `agent_pool` event kind
- golden journal fixtures for `AgentPoolRecord`, `RunMessageRecord`, and
  `WakeRecord`
- ID/ref serde fixtures for `AgentPoolId`, `TopicId`, and `WakeConditionId`
- redaction fixture proving message bodies stay behind `ContentRef` by default
- idempotency fixture proving duplicate run messages do not duplicate delivery
  or wake effects
- visibility fixture proving pool-scoped subscriptions cannot broaden event-bus
  access beyond membership and policy
- timeout fixture proving wake timeout does not cancel the target run
- SDK package architecture audit for root facades and feature-layer test shim

## Must Not

- Move workflow/DAG/barrier/schedule/compensation logic into `agent-sdk-core`.
- Create a second event bus, journal, package registry, run loop, context path,
  or identity system.
- Let `RunAddress` bypass `SourceRef`, `DestinationRef`, `EntityRef`, `RunId`, or
  `AgentId`.
- Add subagent-specific mailbox or clarification DTOs here; those lower through
  generic `RunMessage` and `WakeCondition`.
