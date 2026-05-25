# Agent Pool Store Implementation Plan

## Objective

Add durable/shared `AgentPool` coordination so independent SDK processes can open
the same logical pool, rehydrate membership/messages/wakes, dedupe by
idempotency key, and watch pool-scoped changes without introducing a daemon,
workflow engine, second event system, or product-specific host behavior.

## Current Problem Shape

`AgentPool` is already the correct coordination primitive over `RunMessage`,
`WakeCondition`, `RunJournal`, and `AgentEventBus`, but its mutable membership,
topic, dedupe, wake, and event-counter state is private process-local
`Arc<Mutex<AgentPoolState>>` in `crates/agent-sdk-core/src/application/agent_pool.rs`.
Two independent handles using the same `AgentPoolId` therefore do not share pool
facts.

The source of truth must stay SDK records plus journal-backed event semantics.
The new storage surface is a replaceable coordination port. A database, RPC
service, MCP server, or test fake can implement that port, but core must not
hard-code any concrete transport or storage engine.

## Primitive Decision

- Kernel primitives reused: `AgentRuntime`, `RunRequest`, `RunHandle`,
  `AgentEventBus`, `EventFilter`, `EventFrame`, `RunJournal`, `JournalCursor`,
  `PolicyRef`, `ContentRef`, `SourceRef`, `DestinationRef`, `EntityRef`,
  `EffectIntent`, `EffectResult`, and typed IDs.
- Feature-layer primitive extended: `AgentPool`.
- New port: `AgentPoolStore`, a pool-scoped coordination store/watch port.
- Optional adapter: toolkit `SqliteAgentPoolStore`.
- Host-owned behavior kept out: workflow DAGs, barriers, schedulers, dashboards,
  user chat UX, remote identity trust, RPC/MCP service lifecycle, credentials,
  and network deployment.

Decision ladder:

1. A plain field on `AgentPool` is insufficient because two process-local values
   need shared state.
2. `CapabilitySpec` is wrong because pool coordination is not a model-visible
   callable capability.
3. An optional adapter behind a typed port is the right fit.
4. Hosts may own concrete deployments, but the SDK must own the portable record
   shape and semantics because `RunMessage`/`WakeCondition` correctness depends
   on it.
5. No new workflow primitive is needed.

## Behavior Contract

New behavior:

- `AgentPool::builder(...).store(...)` lets callers choose an in-memory or shared
  durable store.
- Default builder behavior remains in-memory and test-friendly.
- A pool opened with the same `AgentPoolId` against the same store rehydrates
  durable members, topics, messages, dedupe receipts, wake registrations, and
  wake statuses.
- `AgentPool::watch_pool(...)` exposes pool-scoped durable changes from a cursor.
- Duplicate message and wake idempotency keys dedupe across separate
  `AgentPool` handles.
- Registered wakes can be triggered by a message/event emitted from another
  handle using the same store.
- `AgentPool::leave_run` records and persists membership removal.
- Toolkit exposes a SQLite-backed store that implements the same core port.
- Docs include a short Rust example showing two agents/processes communicating
  through shared `AgentPool` primitives.

Preserved behavior:

- `AgentPool` remains a coordination scope, not a workflow engine.
- Run messages stay content-ref-backed and journaled.
- Wake conditions stay event-filter based and envelope-only by default.
- Pool subscriptions still intersect event filters with current pool membership
  and policy.
- Missing targets fail closed with typed statuses.
- The current in-memory path remains available for tests/fake mode.

Removed behavior:

- None. Process-local behavior becomes the default `InMemoryAgentPoolStore`.

## Workstreams

1. Core store port and DTOs:
   - Add `AgentPoolStore`, `AgentPoolStoreCursor`, `AgentPoolStoreRecord`,
     `AgentPoolSnapshot`, and stored message/wake DTOs.
   - Implement `InMemoryAgentPoolStore` as the default and SDK test fake.

2. AgentPool integration:
   - Replace direct private-state ownership with store-backed snapshots and
     mutations.
   - Add builder `.store(...)`.
   - Add `leave_run` and `watch_pool`.
   - Persist pool lifecycle, membership, run-message statuses, wake conditions,
     dedupe, and wake triggers.
   - Trigger registered wakes from journal-backed message events without adding
     a second event bus.

3. Toolkit SQLite adapter:
   - Add `agent_pool` toolkit module with `SqliteAgentPoolStore`.
   - Persist append-only pool records as JSON plus indexed pool/sequence fields.
   - Use SQLite uniqueness/transaction behavior to fail closed on pool config or
     duplicate/conflicting records.

4. Tests and docs:
   - Extend core `agent_pool_contract` tests for two handles sharing one backing
     store, cross-handle membership visibility, message delivery, wake trigger,
     idempotency, missing target, scoped visibility, leave, watch, and
     rehydration.
   - Add toolkit tests proving two SQLite store instances opened on the same file
     share membership/messages/wakes.
   - Update `docs/contracts/agent-pool-contract.md`,
     `docs/contracts/subagent-contract.md`, crate READMEs, and examples.

## Validation Plan

- `cargo fmt --check`
- `cargo test -p agent-sdk-core --test agent_pool_contract`
- `cargo test -p agent-sdk-toolkit`
- `cargo test --workspace`

If full workspace tests expose pre-existing unrelated failures, record the
focused passing evidence and the exact unrelated failure.

## Risks And Watchpoints

- Do not let the store become a second event system. Store watch returns
  pool-scoped durable coordination records linked to journal/event evidence; live
  event subscription remains `AgentEventBus`.
- Do not make SQLite a core dependency. Core owns the trait and fake; toolkit owns
  concrete SQLite.
- Do not infer members/messages/wakes that are absent from durable store records.
- Do not let two handles with conflicting pool policies silently share a pool.
  Conflict must be a typed fail-closed error.
- Do not use same `RunId` for two process owners without a future explicit lease
  model. This plan shares pool state between distinct runs.
- Do not add MCP/RPC server lifecycle in this slice. A later MCP adapter can
  expose the same `AgentPoolStore` port.

## Relevant Existing Context

- `AGENTS.md`: keep SDK product-neutral, do not branch, do not add concrete
  storage to core when it belongs in adapters/toolkit.
- `coding_standards.md` and `docs/architecture/coding-standards.md`: public
  ports need deterministic fakes; preserve journal durability, lineage, privacy,
  policy, and recovery.
- `docs/contracts/agent-pool-contract.md`: `AgentPool` is not a workflow engine
  or second event bus; messages are content-ref-backed and wake filters are
  envelope based.
- `docs/contracts/subagent-contract.md`: subagents lower onto generic
  `RunMessage` and `WakeCondition`.
- `docs/architecture/primitive-map.md` and
  `docs/reference/feature-to-primitive-matrix.md`: concrete storage belongs
  behind ports/adapters; optional workflow behavior stays out of core.

## Review Packet

Primitive decision:

- Reused kernel primitives: event bus, run journal, effect spine, typed refs,
  content refs, policy refs, event filters.
- New feature-layer primitive: none; this extends `AgentPool`.
- New port: `AgentPoolStore`.
- New capability variants: none.
- Host-owned behavior kept out: workflow, UI, MCP/RPC service lifecycle,
  credentials, remote deployment.

Validation evidence to collect:

- Contract/unit tests: core two-handle shared fake tests and toolkit SQLite tests.
- Golden fixtures: existing agent-pool journal/event fixtures remain authoritative
  for emitted events and records.
- Smoke/scenario tests: toolkit SQLite store with two independent store handles.
- Docs audits: contracts/readmes/examples mention multi-process rehydration
  without downstream product names.
