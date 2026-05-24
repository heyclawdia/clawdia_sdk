# Run Handle And Reconnect Contract

The SDK should be simple to use like Cursor's agent/run model while keeping richer journals and host adapters behind ports.

## External Lessons

Cursor's SDK exposes a clear split between agent and per-prompt run. Runs can stream events, be cancelled, and reconnect through run-scoped APIs. The SDK should copy that simplicity without copying Cursor product assumptions.

## Public Shape

```rust
// Non-compiling contract sketch.
pub trait RunRegistry {
    async fn get_run(&self, run_id: RunId) -> Result<RunSnapshot, AgentError>;
    async fn get_handle(&self, run_id: RunId) -> Result<RunHandle, AgentError>;
    async fn subscribe_from(&self, run_id: RunId, cursor: Option<EventCursor>) -> Result<AgentEventStream, AgentError>;
    async fn catch_up_from_journal(&self, run_id: RunId, cursor: JournalCursor) -> Result<Vec<EventFrame>, AgentError>;
    async fn list_runs(&self, query: RunQuery) -> Result<Vec<RunSummary>, AgentError>;
}

pub trait AgentEventBus {
    fn subscribe_all(&self, cursor: Option<EventCursor>) -> Result<AgentEventStream, AgentError>;
    fn subscribe_all_with_options(&self, cursor: Option<EventCursor>, options: SubscriptionOptions) -> Result<AgentEventStream, AgentError>;
    fn subscribe_run(&self, run_id: RunId, cursor: Option<EventCursor>) -> Result<AgentEventStream, AgentError>;
    fn subscribe_run_with_options(&self, run_id: RunId, cursor: Option<EventCursor>, options: SubscriptionOptions) -> Result<AgentEventStream, AgentError>;
    fn subscribe_agent(&self, agent_id: AgentId, cursor: Option<EventCursor>) -> Result<AgentEventStream, AgentError>;
    fn subscribe_agent_with_options(&self, agent_id: AgentId, cursor: Option<EventCursor>, options: SubscriptionOptions) -> Result<AgentEventStream, AgentError>;
    fn subscribe_filtered(
        &self,
        filter: CompiledEventFilter,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError>;
    fn replay_run_from_cursor(
        &self,
        run_id: RunId,
        cursor: JournalCursor,
    ) -> Result<AgentEventStream, AgentError>;
}

pub trait EventArchive {
    fn replay_filtered_from_cursor(
        &self,
        filter: CompiledEventFilter,
        cursor: ArchiveCursor,
    ) -> Result<AgentEventStream, AgentError>;
}

pub struct RunHandle {
    pub run_id: RunId,
    pub status: RunStatusHandle,
    pub events: AgentEventStreamHandle,
    pub final_result: RunResultHandle,
    pub cancellation: CancellationHandle,
}

impl RunHandle {
    pub async fn wait(&self) -> Result<RunResult, AgentError>;
    pub async fn wait_with_timeout(&self, timeout: Duration) -> Result<Option<RunResult>, AgentError>;
    pub async fn status(&self) -> Result<RunStatus, AgentError>;
    pub fn stream_from(&self, cursor: Option<EventCursor>) -> Result<AgentEventStream, AgentError>;
    pub fn stream_from_journal(&self, cursor: JournalCursor) -> Result<AgentEventStream, AgentError>;
    pub async fn cancel(&self) -> Result<(), AgentError>;
}
```

## Cursor Semantics

`EventCursor` is a live or recently buffered stream cursor. It is not the same as `JournalCursor`, which is the durable replay cursor.

```rust
// Non-compiling contract sketch.
pub struct EventCursor {
    pub scope: EventStreamScope,
    pub event_seq: u64,
    pub event_id: EventId,
    pub journal_cursor: Option<JournalCursor>,
}

pub enum EventStreamScope {
    All,
    Run(RunId),
    Agent(AgentId),
    Filter {
        filter_id: EventFilterId,
        filter_fingerprint: EventFilterFingerprint,
    },
}
```

The HTTP/SSE host adapter maps `EventCursor.event_id` to `Last-Event-ID`. SDK internals must not depend on HTTP, but they must preserve enough cursor data for an SSE adapter to reconnect exactly like a run-scoped event stream.

`ArchiveCursor` belongs to optional `EventArchive` / `IndexedJournalView` implementations and is not interchangeable with per-run `JournalCursor`.

Rules:

- `RunHandle::stream_from(cursor)` is shorthand for `AgentEventBus::subscribe_run(run_id, cursor)`.
- `AgentRuntime::subscribe_all`, `subscribe_run`, `subscribe_agent`, and `subscribe_events` use the same cursor semantics.
- Resubscribe from `EventCursor` returns live/replayed events after that cursor when available and when the cursor scope is compatible with the requested subscription.
- Cursor compatibility is exact by logical stream: `All` resumes only `subscribe_all`, `Run(run_id)` resumes only that run, `Agent(agent_id)` resumes only that agent, and `Filter { filter_fingerprint, .. }` resumes only the same compiled filter fingerprint. Incompatible cursors return `CursorScopeMismatch` instead of silently widening, narrowing, or changing filters.
- If live cursor data has expired and `EventCursor.journal_cursor` is present, registry replays summarized journal-backed events from that journal cursor before tailing live events.
- If live cursor data has expired and no journal cursor is available, the stream starts with `RunResumeFailed` or `ReplayFailed` explaining the gap and returns the latest `RunSnapshot`.
- `RunRegistry::subscribe_from` and `RunHandle::stream_from` share cursor semantics.
- `RunHandle::stream_from_journal` always emits `derived_replay` events until it catches up to the live tail.
- Run-scoped durable catch-up is guaranteed from the run journal. Agent-scoped, all-run, or arbitrary filtered durable catch-up is available only when the configured journal/archive port exposes the needed indexed view.
- `wait()` is idempotent. Calling it twice returns the same terminal result.
- `wait_with_timeout()` never cancels the run; it returns `Ok(None)` when the wait expires.
- `status()` is idempotent and reflects the latest sealed journal state plus current volatile execution state when available.
- Terminal result is consistent with the sealed journal terminal state.
- Cancel is idempotent.
- Duplicate subscribers do not duplicate model calls, tool execution, output dispatch, or journal records.
- Terminal events are emitted once per run in the durable journal. Live streams may replay terminal events with delivery semantics `derived_replay`.
- All-run, agent-scoped, and filtered subscriptions must match from envelope fields and must not deserialize raw payload content on the hot path.
- Filtered subscriptions can drive external orchestration, but they never execute workflow/DAG/barrier logic inside core.

## Run Child Lifecycle And Cancellation

Manual stop/cancel is a user or host intent to stop the run and agent-owned work. The default SDK behavior is to cascade cancellation to every child artifact the agent started or owns.

```rust
// Non-compiling contract sketch.
pub struct RunChildLifecyclePolicy {
    pub policy_id: PolicyId,
    pub on_manual_cancel: ChildShutdownBehavior,
    pub on_run_completed: ChildShutdownBehavior,
    pub on_run_failed: ChildShutdownBehavior,
    pub detach_policy: DetachPolicy,
    pub grace_period_ms: u64,
    pub kill_after_grace: bool,
    pub require_detach_intent: bool,
}

pub enum ChildArtifactKind {
    SubagentRun,
    ToolProcess,
    IsolatedProcess,
    RealtimeSession,
    ExternalRuntimeSession,
    ApprovalWait,
    HookInvocation,
}

pub enum ChildShutdownBehavior {
    CascadeCancel,
    InterruptThenTerminate,
    Terminate,
    DetachIfAllowed,
    PreserveIfCompleted,
    HostManaged,
}

pub struct DetachPolicy {
    pub allowed: bool,
    pub requires_explicit_user_intent: bool,
    pub requires_host_ack: bool,
    pub max_detached_children: u32,
    pub reclaim: ReclaimPolicy,
    pub health_check: DetachedHealthCheckPolicy,
}
```

Policy placement:

- `RuntimePackage` owns the default child lifecycle policy and the allowed policy refs. Package defaults and allowed refs are fingerprint inputs.
- `RunRequest` may select or tighten a policy before start. It cannot loosen package or host policy.
- The effective `RunChildLifecyclePolicy` is immutable once `RunStarted` is journaled.
- `RunRecord` stores the effective policy hash/ref. Agent events include policy refs so observers can explain why a child was cancelled, preserved, or detached.

Defaults:

- `on_manual_cancel = CascadeCancel`.
- `on_run_failed = CascadeCancel`.
- `on_run_completed = InterruptThenTerminate` for non-detached agent-owned processes and sessions.
- `detach_policy.allowed = false` unless package and host policy explicitly enable it.
- `require_detach_intent = true`.

Rules:

- `RunHandle::cancel()` and `AgentRuntime::cancel_run(run_id)` append `RunCancelRequested`, then apply the effective child lifecycle policy before sealing `RunCancelled`.
- `wait_with_timeout()` still does not cancel the run or any child artifact.
- A normal `RunCompleted` does not silently orphan work. Agent-owned child artifacts must be terminal, cleaned up, or explicitly detached before the run seals.
- A "start this script and leave it running" workflow is represented as explicit detach: journal `ChildLifecycleRecord::DetachIntent`, obtain required policy/user/host acknowledgement, append `ChildLifecycleRecord::Detached`, then complete the run with a detached-work summary.
- Detached work remains observable after the parent run seals through durable journal records and host-owned process/reclaim tracking. Live events are not sufficient ownership proof.
- Missing or failed detach acknowledgement blocks completion with `RepairNeeded` or cancels/terminates according to policy.
- Cleanup, process signaling, child cancellation, hook cancellation, detach transfer, and reclaim follow intent-before-effect ordering: append intent, perform bounded effect, append terminal result or recovery record.
- Hooks cannot veto cancellation. `OnRunCancelRequested` hooks may propose cleanup repair or existing child/process lifecycle operations inside the same policy deadline; accepted proposals lower into those operations' normal intent/result records.

## Reconnect Flow

```mermaid
sequenceDiagram
  participant Client
  participant Registry as "RunRegistry"
  participant Handle as "RunHandle"
  participant Journal as "RunJournal"
  participant Loop as "AgentLoop"

  Client->>Registry: "get_handle(run_id)"
  Registry-->>Client: "RunHandle"
  Client->>Handle: "stream_from(EventCursor / Last-Event-ID)"
  alt live buffer has cursor
    Handle-->>Client: "events after cursor"
  else live cursor expired and journal cursor exists
    Handle->>Journal: "catch_up_from_journal(journal_cursor)"
    Journal-->>Handle: "derived replay events"
    Handle-->>Client: "catch-up then live tail"
  else live cursor expired with no journal cursor
    Handle-->>Client: "gap diagnostic + latest RunSnapshot"
  end
  Client->>Handle: "wait()"
  Loop-->>Journal: "Terminal records: approvals/compaction/output delivery/session drain"
  Loop-->>Journal: "RunCompleted"
  Handle-->>Client: "same RunResult on every wait"
```

## Transport Adapters

The core API is transport-neutral. Host adapters map it to UI, CLI, or HTTP:

| Transport | Cursor field | Required behavior |
| --- | --- | --- |
| in-process UI | `EventCursor` object | reconnect from stored cursor and catch up from journal if live buffer expired |
| CLI JSONL | last event envelope | read `event_id`, `event_seq`, and `journal_cursor` from the last line |
| HTTP/SSE | `Last-Event-ID` | map to `event_id`, resolve stored cursor, then apply normal catch-up rules |
| remote channel | host ack cursor | host maps ack to `EventCursor` and records output delivery dedupe |

Transport adapters cannot synthesize terminal success. Terminal state comes from the sealed journal.

`RunCompleted` means the run journal is sealed, not merely that final visible text exists. `RunHandle::wait()` resolves only after provider/model streams drain and all required terminal bookkeeping has reached a durable state: pending approvals are resolved or cancelled, compaction/projection records are terminal, required output delivery has completed/deduped/failed according to policy, agent-owned child artifacts are terminal/detached, and the final journal record is sealed. Feature contracts that add completion work must name the terminal records that gate `wait()`.

## Acceptance Tests

- `run_handle_wait_is_idempotent`
- `run_handle_cancel_is_idempotent`
- `subscriber_drop_and_resubscribe_from_cursor_catches_up`
- `cursor_scope_mismatch_is_rejected_without_widening_or_narrowing`
- `filter_cursor_requires_same_filter_fingerprint`
- `expired_event_cursor_replays_from_journal_summary`
- `last_event_id_maps_to_event_cursor_without_http_leaking_into_core`
- `expired_event_cursor_without_journal_cursor_returns_gap_diagnostic`
- `stream_from_journal_uses_derived_replay_until_live_tail`
- `wait_with_timeout_does_not_cancel_run`
- `status_is_idempotent_across_duplicate_calls`
- `duplicate_subscribers_do_not_duplicate_side_effects`
- `terminal_result_matches_sealed_journal_state`
- `terminal_event_replay_uses_derived_replay_delivery_semantics`
- `subscribe_agent_returns_all_matching_live_runs`
- `subscribe_filtered_terminal_events_can_resume_from_cursor`
- `filtered_subscription_does_not_require_payload_deserialization`
- `slow_filtered_subscriber_uses_declared_overflow_policy`
- `manual_cancel_cascades_to_agent_owned_children_by_default`
- `wait_with_timeout_does_not_cancel_children`
- `run_completion_preserves_explicitly_detached_process_when_policy_allows`
- `run_completion_terminates_non_detached_agent_owned_process_by_default`
- `detached_process_requires_intent_record_before_run_completion`
- `wait_does_not_resolve_until_output_delivery_and_journal_seal`
- `wait_does_not_resolve_until_pending_approval_is_terminal`
- `wait_does_not_resolve_until_compaction_or_projection_repair_is_terminal`
- `child_shutdown_policy_is_configurable_per_run_without_loosening_package_policy`
- `cancel_interrupts_inflight_hooks_and_continues_child_shutdown`

## Complete Example

Typed shape:

```rust
// Non-compiling contract sketch.
let handle = registry.get_handle(run_id).await?;
let stream = handle.stream_from(Some(EventCursor {
    scope: EventStreamScope::Run(run_id),
    event_seq: 128,
    event_id: EventId::from_last_event_id("evt_128"),
    journal_cursor: Some(JournalCursor::at(91)),
}))?;

while let Some(frame) = stream.next().await {
    render_event(frame.event)?;
    persist_cursor(frame.cursor)?;
}

let result = handle.wait_with_timeout(Duration::from_secs(30)).await?;
```

Lifecycle subscription examples:

```rust
// Non-compiling contract sketch.
let one_run = runtime.subscribe_run(run_id, None)?;
let agent_activity = runtime.subscribe_agent(agent_id, stored_cursor)?;
let terminal_filter = EventFilter::terminal_run_events()
    .tag(EventTag::new("fanout:research"))
    .payload_access(PayloadAccessMode::EnvelopeOnly)
    .compile()?;
let terminal_events = runtime.subscribe_events(terminal_filter, None)?;
```

Replaceable ports:

- `RunRegistry` can be in-memory, persisted, remote, or host-backed.
- `AgentEventStream` yields `EventFrame` values and can be projected to in-process channels, CLI JSONL, SSE, or remote-channel transports.
- `RunJournal` provides catch-up when the live buffer expires.
- `AgentEventBus` can be in-memory, persisted, or remote-backed as long as it preserves cursor/filter semantics.

Wiring:

1. UI stores the last `EventCursor`.
2. UI reconnects through `RunRegistry::subscribe_from`.
3. Registry replays from journal when live events expired.
4. UI tails live events after catch-up.
5. `wait()` returns the sealed terminal result without starting another run.

Events:

- `ReplayStarted`
- `ReplayCompleted`
- replayed events with `derived_replay`
- live events after replay
- terminal `RunCompleted`, `RunFailed`, or `RunCancelled`

Journal:

- `RunRecord` for terminal state.
- Any journal-backed records after the supplied cursor.
- `RecoveryRecord` only if cursor gap cannot be repaired.

Policies and failures:

- `Last-Event-ID` belongs to HTTP/SSE adapters; core uses `EventCursor`.
- Expired live cursor without journal cursor returns a gap diagnostic plus latest `RunSnapshot`.
- Duplicate subscribers never duplicate model/tool/output side effects.
- `wait_with_timeout()` returns `Ok(None)` and does not cancel the run.
- `cancel()` cascades to agent-owned child artifacts by default and records each shutdown or detach decision.
- Explicit detach requires policy, intent, acknowledgement, and reclaim metadata before a run can complete.

SDK owns / Host owns:

- SDK owns cursor semantics, replay/catch-up, run idempotency, and terminal consistency.
- SDK owns run/agent/filter subscription semantics and fast envelope filtering.
- SDK owns child lifecycle policy semantics, shutdown ordering, detach journal records, and cancellation propagation.
- Host owns transport headers, client storage of cursors, UI retry timing, remote-channel acknowledgements, concrete process control adapters, detached process inspectors/reclaim jobs, and any higher-level orchestration engine that reacts to events.

Tests:

- `subscriber_drop_and_resubscribe_from_cursor_catches_up`
- `last_event_id_maps_to_event_cursor_without_http_leaking_into_core`
- `duplicate_subscribers_do_not_duplicate_side_effects`
- `subscribe_filtered_terminal_events_can_drive_external_orchestration_without_core_workflow_engine`
- `manual_cancel_cascades_to_agent_owned_children_by_default`
- `run_completion_preserves_explicitly_detached_process_when_policy_allows`
