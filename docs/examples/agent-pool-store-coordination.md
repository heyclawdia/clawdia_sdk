# Agent Pool Store Coordination

Two independent SDK processes can join the same logical pool by opening the same
`AgentPoolId` through a shared `AgentPoolStore`. Core owns the portable
coordination port. Toolkit can provide a local SQLite adapter; a host may provide
another adapter with the same semantics.

```rust
use agent_sdk_core::{
    AgentId, AgentPool, AgentPoolId, AgentPoolMember, IdempotencyKey, MessageId,
    RunAddress, RunId, RunMessage, WakeCondition, WakeConditionId,
    domain::ContentRef,
    event::{EventFamily, EventFilter, EventFilterSet, EventKind},
};
use agent_sdk_toolkit::SqliteAgentPoolStore;

fn terminal_a(runtime: agent_sdk_core::AgentRuntime) -> Result<(), agent_sdk_core::AgentError> {
    let store = SqliteAgentPoolStore::open("./agent-pool.sqlite")?;
    let pool = AgentPool::builder(AgentPoolId::new("pool.shared.review"))
        .runtime(runtime)
        .store(store)
        .build()?;

    pool.join_run(AgentPoolMember::new(
        RunId::new("run.review.a"),
        AgentId::new("agent.reviewer"),
    ))?;

    let receipt = pool.send(RunMessage::new(
        MessageId::new("message.review.request.1"),
        RunId::new("run.review.a"),
        RunAddress::run(RunId::new("run.review.b")),
        ContentRef::new("content.review.request.1"),
        IdempotencyKey::new("idem.review.request.1"),
    ))?;

    assert!(receipt.delivered_to.contains(&RunId::new("run.review.b")));
    Ok(())
}

fn terminal_b(runtime: agent_sdk_core::AgentRuntime) -> Result<(), agent_sdk_core::AgentError> {
    let store = SqliteAgentPoolStore::open("./agent-pool.sqlite")?;
    let pool = AgentPool::builder(AgentPoolId::new("pool.shared.review"))
        .runtime(runtime)
        .store(store)
        .build()?;

    pool.join_run(AgentPoolMember::new(
        RunId::new("run.review.b"),
        AgentId::new("agent.implementer"),
    ))?;

    let wake = WakeCondition::new(
        WakeConditionId::new("wake.review.message.1"),
        RunId::new("run.review.b"),
        EventFilter {
            run_ids: EventFilterSet::Include(vec![RunId::new("run.review.a")]),
            families: EventFilterSet::Include(vec![EventFamily::AgentPool]),
            kinds: EventFilterSet::Include(vec![EventKind::RunMessageDelivered]),
            ..EventFilter::default()
        },
        IdempotencyKey::new("idem.wake.review.message.1"),
    );
    pool.suspend_until(RunId::new("run.review.b"), wake)?;

    let cursor = pool.snapshot()?.cursor;
    for record in pool.watch_pool(cursor)? {
        let _record = record;
        // Host code decides how to react to pool-scoped records.
    }

    Ok(())
}
```

Semantics:

- Both handles coordinate through `AgentPool`, not through a daemon-specific API.
- `RunMessage` content remains content-ref-backed and journaled by the sending
  runtime before the store records the message status.
- `WakeCondition` remains an envelope-filter registration. The store persists the
  condition and latest wake status so another handle can trigger or rehydrate it.
- `watch_pool` returns pool-scoped store records only. It is not a global event
  subscription and does not grant access to other pools or journals.
- MCP, RPC, sockets, or network databases can be implemented as adapters behind
  `AgentPoolStore`; they should not change the SDK primitives agents use.
