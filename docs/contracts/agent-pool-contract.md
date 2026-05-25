# Agent Pool Contract

`AgentPool` is the feature-layer coordination scope for agent runs that need to
communicate, observe each other, and suspend/resume on shared events. It layers on
the existing `AgentRuntime`, `RunRequest`, `RunHandle`, `AgentEventBus`,
`RunJournal`, `RuntimePackage`, `PolicyRef`, `ContentRef`, and child lifecycle
contracts. It is not a workflow engine and it is not a second event bus.

Subagents, swarms, reviewers, researchers, and other multi-agent patterns are
higher-order helpers over `AgentPool`. The pool contract owns communication,
addressing, delivery status, and wake registration. Optional workflow crates or
hosts own DAGs, barriers, schedules, compensation, "wait for N", dashboards, and
product-specific orchestration.

## Public Shape

```rust
// Non-compiling contract sketch.
pub struct AgentPool {
    pub pool_id: AgentPoolId,
    pub runtime: AgentRuntimeRef,
    pub members: Vec<RunId>,
    pub topics: Vec<TopicId>,
    pub message_policy: AgentPoolMessagePolicy,
    pub wake_policy: AgentPoolWakePolicy,
    pub policy_refs: Vec<PolicyRef>,
}

impl AgentPool {
    pub async fn start_run(&self, request: RunRequest) -> Result<RunHandle, AgentError>;
    pub fn join_run(&self, member: AgentPoolMember) -> Result<(), AgentError>;
    pub fn leave_run(&self, run_id: &RunId) -> Result<AgentPoolMember, AgentError>;
    pub fn members(&self) -> Result<Vec<AgentPoolMember>, AgentError>;
    pub fn snapshot(&self) -> Result<AgentPoolSnapshot, AgentError>;
    pub async fn send(&self, message: RunMessage) -> Result<MessageReceipt, AgentError>;
    pub fn subscribe(&self, filter: EventFilter, cursor: Option<EventCursor>) -> Result<AgentEventStream, AgentError>;
    pub fn watch_pool(&self, cursor: Option<AgentPoolStoreCursor>) -> Result<AgentPoolStoreStream, AgentError>;
    pub async fn suspend_until(&self, run_id: RunId, condition: WakeCondition) -> Result<WakeRegistration, AgentError>;
}

impl AgentPoolBuilder {
    pub fn store<S: AgentPoolStore + 'static>(self, store: S) -> Self;
    pub fn shared_store(self, store: Arc<dyn AgentPoolStore>) -> Self;
}

pub struct RunAddress {
    pub target: RunAddressTarget,
    pub destination_ref: DestinationRef,
    pub related_refs: Vec<EntityRef>,
}

pub enum RunAddressTarget {
    Run(RunId),
    Agent(AgentId),
    Topic(TopicId),
    Pool(AgentPoolId),
}

pub struct RunMessage {
    pub message_id: MessageId,
    pub from: RunId,
    pub to: RunAddress,
    pub content_ref: ContentRef,
    pub correlation: EventCorrelation,
    pub reply_to: Option<MessageId>,
    pub response_contract: Option<MessageResponseContract>,
    pub expires_at: Option<Timestamp>,
    pub idempotency_key: IdempotencyKey,
    pub policy_refs: Vec<PolicyRef>,
}

pub struct MessageReceipt {
    pub message_id: MessageId,
    pub status: MessageStatus,
    pub delivered_to: Vec<RunId>,
    pub journal_cursor: Option<JournalCursor>,
}

pub enum MessageStatus {
    Accepted,
    Delivered,
    Responded,
    Failed,
    TimedOut,
    Expired,
    Cancelled,
}

pub struct WakeCondition {
    pub condition_id: WakeConditionId,
    pub run_id: RunId,
    pub filter: EventFilter,
    pub timeout: Option<Duration>,
    pub resume_with: ResumeInputPolicy,
    pub idempotency_key: IdempotencyKey,
    pub policy_refs: Vec<PolicyRef>,
}

pub struct WakeRegistration {
    pub condition_id: WakeConditionId,
    pub run_id: RunId,
    pub status: WakeRegistrationStatus,
    pub journal_cursor: Option<JournalCursor>,
}

pub enum WakeRegistrationStatus {
    Registered,
    Triggered,
    TimedOut,
    Cancelled,
    Failed,
}

pub trait AgentPoolStore {
    fn open_pool(&self, pool_id: AgentPoolId, config: AgentPoolStoreConfig) -> Result<AgentPoolSnapshot, AgentError>;
    fn snapshot(&self, pool_id: &AgentPoolId) -> Result<AgentPoolSnapshot, AgentError>;
    fn record_pool_created(&self, pool_id: &AgentPoolId) -> Result<AgentPoolStoreCursor, AgentError>;
    fn join_member(&self, pool_id: &AgentPoolId, member: AgentPoolMember) -> Result<AgentPoolStoreCursor, AgentError>;
    fn leave_member(&self, pool_id: &AgentPoolId, run_id: &RunId) -> Result<(AgentPoolMember, AgentPoolStoreCursor), AgentError>;
    fn message_receipt(&self, pool_id: &AgentPoolId, idempotency_key: &IdempotencyKey) -> Result<Option<MessageReceipt>, AgentError>;
    fn record_message(&self, pool_id: &AgentPoolId, message: RunMessage, receipt: MessageReceipt) -> Result<AgentPoolStoreCursor, AgentError>;
    fn wake_registration(&self, pool_id: &AgentPoolId, idempotency_key: &IdempotencyKey) -> Result<Option<WakeRegistration>, AgentError>;
    fn wake(&self, pool_id: &AgentPoolId, condition_id: &WakeConditionId) -> Result<Option<AgentPoolStoredWake>, AgentError>;
    fn record_wake(&self, pool_id: &AgentPoolId, condition: WakeCondition, filter: CompiledEventFilter, registration: WakeRegistration) -> Result<AgentPoolStoreCursor, AgentError>;
    fn watch(&self, pool_id: &AgentPoolId, cursor: Option<AgentPoolStoreCursor>) -> Result<AgentPoolStoreStream, AgentError>;
    fn next_event_sequence(&self, pool_id: &AgentPoolId) -> Result<u64, AgentError>;
}
```

`RunAddress` is an ergonomic wrapper over existing `RunId`, `AgentId`,
`DestinationRef`, and `EntityRef` values. It must not become a parallel identity
system or bypass typed refs. Every addressable target must still be represented
in event envelopes and journal records through existing source, destination,
subject, related, and correlation fields.

Address resolution happens before delivery and the resolved recipient set is
journaled in `RunMessageRecord`. Resolution rules are finite:

- `Run(RunId)` targets exactly one existing run that is a current pool member and
  visible under policy.
- `Agent(AgentId)` targets policy-selected existing pool members for that agent.
  It does not start a run, consult host routing, or pick an arbitrary active run.
  If policy requires a single recipient and the match is empty or ambiguous,
  delivery fails with a typed status.
- `Topic(TopicId)` targets the current policy-selected topic members at delivery
  time; future subscribers do not receive old messages unless replay policy
  explicitly replays matching journal records.
- `Pool(AgentPoolId)` is policy-gated broadcast to current pool members, excluding
  the sender unless policy explicitly includes it. It does not cross pool
  boundaries.

## Rules

- `AgentPool` uses `AgentEventBus` for observation and wake matching; it does not
  introduce a second bus.
- `AgentPool::subscribe` and `WakeCondition` matching intersect caller-provided
  `EventFilter` values with pool membership, topic refs, privacy policy, and
  caller authority. A pool-scoped subscription cannot observe events the same run
  could not observe through the runtime event API.
- `AgentPool` uses `RunJournal` records for durable message delivery, wake
  registration, wake trigger, timeout, and replay; live delivery is not durable
  proof.
- `AgentPool::start_run` lowers to `AgentRuntime::start_run` with a normal
  `RunRequest`.
- `AgentPool::send` records a run-message intent before delivery when the
  message mutates another run, wakes a parked run, crosses process/runtime
  boundaries, or otherwise has externally visible effects.
- Message bodies use `ContentRef`. Raw bodies are omitted from default events and
  telemetry unless content-capture policy explicitly allows them.
- `WakeCondition` is `EventFilter` plus timeout and resume policy. It does not
  own scheduling, barriers, retry graphs, or workflow state.
- Duplicate messages and wake registrations with the same idempotency key are
  deduped across replay.
- If a target run is cancelled, failed, expired, or unavailable, delivery returns
  a typed message status and records the outcome rather than prompt-injecting a
  narrative failure.
- A pool may expose topics for fan-out, but topic membership and topic delivery
  are policy-gated and journaled.

## Durable Store And Watch

`AgentPoolStore` is the SDK-owned port for shared pool coordination. It is a
pool-scoped record and snapshot surface for membership, message status, wake
registrations, dedupe indexes, lifecycle, topics, and watch cursors. It is not a
daemon, scheduler, broker, workflow engine, or second event system.

The default core store is in-memory and process-local. Cloned
`InMemoryAgentPoolStore` values share one backing map for fake/conformance tests.
Concrete durable stores, such as a SQLite file opened by two terminal processes,
belong in toolkit or host adapters and implement the same port.

Rules:

- `AgentPool::builder(...).build()` creates or opens the named pool through the
  configured store. Reopening the same `AgentPoolId` must rehydrate from durable
  records only.
- Store records are pool-scoped coordination projections linked to journal-backed
  operations. They do not replace `RunJournal`, `AgentEventBus`, `EventArchive`,
  or telemetry.
- `watch_pool(cursor)` returns durable pool-store records after a pool cursor. It
  must not widen access to global event streams, journals, or other pools.
- `RunMessage` delivery still appends run-message journal records and emits
  `RunMessage*` events. The store records message status and idempotency so a
  second handle can dedupe and rehydrate.
- `WakeCondition` remains an event-filter registration. The store records the
  condition, compiled envelope-only filter, latest status, and idempotency so a
  different handle can trigger or poll it from matching pool events.
- Store conflicts, config mismatches, corrupt records, or concurrent append
  failures fail closed with typed errors or retry classifications. An adapter must
  not silently fork logical pool state.
- A store may expose RPC, MCP, socket, file, SQL, or network transport behind the
  port, but core callers still coordinate through `AgentPool`, `RunMessage`,
  `WakeCondition`, snapshots, and pool-scoped watches.

## Subagent Lowering

A subagent is a supervised child-run preset over `AgentPool`:

1. The supervisor validates child depth, package, route, lifecycle, and handoff
   policy.
2. It creates or joins an `AgentPool` scoped to the parent run and child run.
3. It starts the child through a normal child `RunRequest` with a stripped
   `RuntimePackage`.
4. It sends parent-to-child instructions through `RunMessage`.
5. It models child-to-parent clarification as `RunMessage { reply_to, response_contract }`.
6. It waits through `WakeCondition` values over terminal child events, message
   responses, failure, cancellation, or timeout.
7. It preserves subagent-specific safety records: child package stripping,
   no user-chat promotion, parent-owned lifecycle, event wrapping, and usage
   rollup.

## Event And Journal Rules

Agent-pool event kinds are feature-layer events and require golden fixtures
before emission:

- `AgentPoolCreated`
- `AgentPoolRunJoined`
- `AgentPoolRunLeft`
- `RunMessageAccepted`
- `RunMessageDelivered`
- `RunMessageResponded`
- `RunMessageFailed`
- `RunMessageTimedOut`
- `RunMessageExpired`
- `RunMessageCancelled`
- `WakeConditionRegistered`
- `WakeConditionTriggered`
- `WakeConditionTimedOut`
- `WakeConditionCancelled`

Journal records:

| Record | Required fields |
| --- | --- |
| `AgentPoolRecord` | pool ID, member run IDs, topics, policy refs, lifecycle status |
| `RunMessageRecord` | message ID, source run ID, address target, content ref, correlation, reply-to ID, delivery status, policy refs, idempotency key |
| `WakeRecord` | condition ID, run ID, event filter fingerprint, timeout, resume policy, trigger status, policy refs, idempotency key |

`RunMessageRecord` maps to `EffectIntent` / `EffectResult` when delivery crosses
a side-effect boundary. Internal delivery inside the same runtime may still use
the same fields without claiming an external operation ID.

## Compatibility Boundary

The core SDK does not own:

- DAG or barrier engines.
- Durable trigger state beyond message and wake records.
- Product swarm dashboards.
- Prompt-based role relationships such as reviewer, delegate, or peer.
- Direct user-chat promotion for child runs.
- Detached-child schedulers or external compensation.

## Complete Example

Typed shape:

```rust
// Non-compiling contract sketch.
let pool = AgentPool::builder(AgentPoolId::new("pool.research"))
    .message_policy(AgentPoolMessagePolicy::bounded_defaults())
    .wake_policy(AgentPoolWakePolicy::safe_defaults())
    .build(runtime)?;

let researcher = pool.start_run(
    researcher_agent
        .request("Look up the migration risk")
        .source(SourceRef::agent_run(parent_run_id))
        .destination(DestinationRef::agent_run())
        .build(runtime)?
).await?;

pool.send(RunMessage {
    message_id: MessageId::new(),
    from: parent_run_id,
    to: RunAddress::run(researcher.run_id),
    content_ref: ContentRef::new("content/research_request_1"),
    correlation: EventCorrelation::new("research.1"),
    reply_to: None,
    response_contract: Some(MessageResponseContract::one_response(Duration::from_secs(600))),
    expires_at: Some(Timestamp::now_plus_secs(600)),
    idempotency_key: IdempotencyKey::new("research-request-1"),
    policy_refs: vec![PolicyRef::new("policy.agent_pool.message")],
}).await?;

pool.suspend_until(parent_run_id, WakeCondition {
    condition_id: WakeConditionId::new(),
    run_id: parent_run_id,
    filter: EventFilter::message_response("research.1").compile()?,
    timeout: Some(Duration::from_secs(600)),
    resume_with: ResumeInputPolicy::MatchingEventRefs,
    idempotency_key: IdempotencyKey::new("wake-research-1"),
    policy_refs: vec![PolicyRef::new("policy.agent_pool.wake")],
}).await?;
```

Replaceable ports:

- The pool uses the configured `AgentRuntime`, `AgentEventBus`, `RunJournal`, and
  optional `EventArchive`; core owns the `AgentPoolStore` port but not a concrete
  durable storage or transport choice.
- Hosts and toolkit adapters may provide durable store/archive/index
  implementations for cross-run and cross-process replay.
- Optional workflow crates may build barriers and schedules over pool events.

Wiring:

1. A pool-scoped run starts through `AgentRuntime::start_run`.
2. A `RunMessageRecord` is appended before delivery when delivery mutates or wakes
   another run.
3. `RunMessage*` events are emitted after journal append.
4. A parked run resumes when its `WakeCondition` matches a journal-backed event or
   times out.
5. Reconnect and replay use normal `EventCursor`, `JournalCursor`, and optional
   `ArchiveCursor` semantics plus pool-scoped `AgentPoolStoreCursor` semantics.

Events:

- `AgentPoolCreated`
- `AgentPoolRunJoined`
- `RunMessageAccepted`
- `RunMessageDelivered`
- `RunMessageResponded`, `RunMessageFailed`, `RunMessageTimedOut`, `RunMessageExpired`, or `RunMessageCancelled`
- `WakeConditionRegistered`
- `WakeConditionTriggered`, `WakeConditionTimedOut`, or `WakeConditionCancelled`

Journal:

- `AgentPoolRecord`
- `RunMessageRecord`
- `WakeRecord`
- `EffectIntent` / `EffectResult` when delivery crosses a side-effect boundary

Policies and failures:

- Message send, topic fan-out, wake registration, timeout, and resume input are
  policy-scoped.
- Missing target, denied target, expired message, duplicate idempotency key, and
  failed delivery return typed statuses.
- A pool timeout wakes the waiting run with a timeout status; it does not
  automatically cancel the target run.

SDK owns / Host owns:

- SDK owns pool membership records, message/wake DTOs, event/journal semantics,
  policy refs, redaction defaults, pool-store records, pool-scoped watch
  semantics, and replay/idempotency behavior.
- Host owns UI, durable global archive implementation, concrete pool store
  deployment choices, product workflow logic, dashboards, schedules, and external
  compensation.

Tests:

- `agent_pool_message_uses_content_ref_by_default`
- `run_address_lowers_to_existing_refs`
- `run_message_delivery_is_journaled_before_wake`
- `duplicate_run_message_is_deduped_by_idempotency_key`
- `agent_address_requires_policy_selected_existing_members`
- `pool_address_broadcasts_only_current_policy_selected_members`
- `pool_subscription_intersects_filter_with_membership_and_policy`
- `wake_condition_matches_event_filter_without_payload_parsing`
- `wake_timeout_does_not_cancel_target_run`
- `shared_store_handles_see_members_messages_and_rehydrate_state`
- `shared_store_wake_registered_by_one_handle_triggers_from_other_handle_message`
- `shared_store_missing_target_fails_closed_and_dedupes_across_handles`
- `shared_store_subscriptions_remain_scoped_to_pool_members`
- `agent_pool_does_not_own_barrier_or_dag_logic`
