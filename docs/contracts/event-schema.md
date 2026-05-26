# Event Schema Contract

`AgentEvent` is the canonical live event vocabulary for the SDK. It feeds UIs, CLIs, journals, telemetry sinks, and host adapters, but it is not itself the durable source of truth. The durable source is the run journal.

## Envelope

Every event uses the same envelope shape.

```rust
// Non-compiling contract sketch.
pub struct EventEnvelope<T> {
    pub schema_version: u16,
    pub event_id: EventId,
    pub event_seq: u64,
    pub event_family: EventFamily,
    pub event_kind: EventKind,
    pub payload_schema_version: u16,
    pub timestamp: Timestamp,
    pub recorded_at: Timestamp,
    pub run_id: RunId,
    pub session_id: Option<SessionId>,
    pub agent_id: AgentId,
    pub turn_id: Option<TurnId>,
    pub attempt_id: Option<AttemptId>,
    pub message_id: Option<MessageId>,
    pub context_item_id: Option<ContextItemId>,
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub parent_event_id: Option<EventId>,
    pub caused_by: Option<CausalRef>,
    pub subject_ref: EntityRef,
    pub related_refs: Vec<EntityRef>,
    pub causal_refs: Vec<CausalRef>,
    pub correlation: EventCorrelation,
    pub tags: Vec<EventTag>,
    pub source: SourceRef,
    pub destination: Option<DestinationRef>,
    pub policy_refs: Vec<PolicyDecisionRef>,
    pub journal_cursor: Option<JournalCursor>,
    pub state_before: Option<LoopStateName>,
    pub state_after: Option<LoopStateName>,
    pub delivery_semantics: EventDeliverySemantics,
    pub privacy: EventPrivacy,
    pub content_capture: ContentCaptureMode,
    pub redaction_policy_id: RedactionPolicyId,
    pub runtime_package_fingerprint: RuntimePackageFingerprint,
    pub payload: T,
}
```

Envelope fields are enough to route, redact, correlate, replay, filter, and export the event without parsing the payload. When a `RunRequest` carries a `SessionId`, every event causally produced for that run carries the same `session_id`; every question-scoped event also carries the effective `turn_id`.

`subject_ref`, `related_refs`, and `causal_refs` are the generic entity-linking mechanism. The hot-path IDs above them are limited to run-loop primitives that nearly every event stream needs to route: run, agent, turn, attempt, message, and context item. Feature-specific IDs such as tool call, approval request, stream rule, hook, child artifact, execution environment, isolated process, subagent run, extension action, output delivery, or effect IDs belong in `EntityRef` values and event payloads. New feature layers should not add a new optional envelope ID by default. They should attach `EntityRef` values and promote a field to the envelope only when the integration and events/journal roles agree it is a stable universal hot-path index.

Root events use `caused_by = None`, `causal_refs = []`, and `parent_event_id = None`. Derived events must use at least one causal ref. Consumers must not infer causality from timestamp ordering.

`event_seq` is monotonic per live event stream. It is not a durability cursor. The durability cursor is `journal_cursor`, which is present only after the corresponding journal append succeeds or when the event is derived from a durable replay record.

`delivery_semantics` is finite:

- `best_effort_live`: slow subscribers may miss the event.
- `journal_backed`: the event corresponds to a durable journal record.
- `derived_replay`: emitted while replaying durable records.
- `diagnostic_only`: local diagnostic when durable append/export failed.

Event streams yield frames so subscribers can persist position and detect gaps without inspecting payloads:

```rust
// Non-compiling contract sketch.
pub struct EventFrame {
    pub event: AgentEvent,
    pub cursor: EventCursor,
    pub archive_cursor: Option<ArchiveCursor>,
    pub overflow: Option<EventOverflowNotice>,
}

pub struct EventCursor {
    pub scope: EventStreamScope,
    pub event_seq: u64,
    pub event_id: EventId,
    pub journal_cursor: Option<JournalCursor>,
}

pub struct ArchiveCursor {
    pub archive_id: EventArchiveId,
    pub position: EventArchivePosition,
    pub event_id: Option<EventId>,
    pub watermark: Option<Timestamp>,
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

pub struct EventOverflowNotice {
    pub policy: SubscriberOverflowPolicy,
    pub dropped_count: u64,
    pub gap_start: Option<EventCursor>,
    pub gap_end: EventCursor,
    pub repair_from: Option<JournalCursor>,
    pub terminal_preserved: bool,
    pub reason: EventOverflowReason,
}

pub enum EventOverflowReason {
    SubscriberQueueFull,
    SubscriberLagged,
    LiveBufferExpired,
    PolicyDroppedProgress,
    PolicyDroppedNonTerminal,
}
```

Entity refs are typed and source-qualified:

```rust
// Non-compiling contract sketch.
pub struct EntityRef {
    pub kind: EntityKind,
    pub id: EntityId,
    pub source: Option<SourceRef>,
    pub privacy: EventPrivacy,
    pub redacted_summary: Option<RedactedSummary>,
}

pub enum EntityKind {
    Run,
    Turn,
    Attempt,
    Agent,
    Message,
    ContextContribution,
    ContextItem,
    ContextProjection,
    Content,
    Artifact,
    Capability,
    PackageSidecar,
    PolicyDecision,
    Effect,
    EffectIntent,
    EffectResult,
    ToolCall,
    ApprovalRequest,
    StreamRule,
    Hook,
    ExecutionEnvironment,
    ChildArtifact,
    SubagentRun,
    AgentPool,
    AgentPoolTopic,
    RunMessage,
    WakeCondition,
    ExtensionAction,
    OutputDelivery,
}
```

`EventCursor` is a live or recently buffered stream cursor. `JournalCursor` is the durable replay cursor. Run-scoped durable catch-up can be guaranteed from the run journal. Cross-run, all-event, or arbitrary filtered durable replay requires a host-provided archive or indexed journal view; core must not claim global durable event queries unless the configured storage supports them.

`EventFrame.cursor` always identifies the delivered live/replay frame in SDK event-stream terms. `EventFrame.archive_cursor` is present only for frames produced by an `EventArchive` / `IndexedJournalView` and is the cursor callers persist for the next archive replay. If `overflow` is present, it describes events dropped or skipped before the delivered frame. `overflow.repair_from` is the durable repair point when run-journal catch-up is available.

## Core Event Subscription API

Listening to the agent lifecycle event stream is a core SDK primitive. It is not a product-specific app event bus and it is not only a telemetry export path.

The SDK owns:

- stable `AgentEvent` types and lightweight envelopes;
- `EventCursor`, `JournalCursor`, and replay-from-cursor semantics;
- typed event filters and precompiled filter plans;
- live subscription APIs for all events, one run, one agent, and arbitrary typed filters;
- run-scoped durable replay from a run journal, plus an optional indexed archive port for cross-run durable filtered replay;
- non-blocking emission and bounded subscriber channels;
- envelope-only and redacted-summary default payload access.

Higher-level orchestration can be built on top of these streams, such as "when two agents finish, spawn another agent." Workflow, DAG, barrier, queue, or scheduler engines remain outside core unless they are later added as an optional crate over the same event stream contracts.

```rust
// Non-compiling contract sketch.
pub trait AgentEventBus {
    fn subscribe_all(&self, cursor: Option<EventCursor>) -> Result<AgentEventStream, AgentError>;
    fn subscribe_all_with_options(
        &self,
        cursor: Option<EventCursor>,
        options: SubscriptionOptions,
    ) -> Result<AgentEventStream, AgentError>;
    fn subscribe_run(&self, run_id: RunId, cursor: Option<EventCursor>) -> Result<AgentEventStream, AgentError>;
    fn subscribe_run_with_options(
        &self,
        run_id: RunId,
        cursor: Option<EventCursor>,
        options: SubscriptionOptions,
    ) -> Result<AgentEventStream, AgentError>;
    fn subscribe_agent(&self, agent_id: AgentId, cursor: Option<EventCursor>) -> Result<AgentEventStream, AgentError>;
    fn subscribe_agent_with_options(
        &self,
        agent_id: AgentId,
        cursor: Option<EventCursor>,
        options: SubscriptionOptions,
    ) -> Result<AgentEventStream, AgentError>;
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

pub struct EventFilter {
    pub run_ids: EventFilterSet<RunId>,
    pub session_ids: EventFilterSet<SessionId>,
    pub agent_ids: EventFilterSet<AgentId>,
    pub turn_ids: EventFilterSet<TurnId>,
    pub families: EventFilterSet<EventFamily>,
    pub kinds: EventFilterSet<EventKind>,
    pub sources: EventFilterSet<SourceMatcher>,
    pub destinations: EventFilterSet<DestinationMatcher>,
    pub subject_kinds: EventFilterSet<EntityKind>,
    pub related_entities: EventFilterSet<EntityMatcher>,
    pub correlation_keys: EventFilterSet<CorrelationKey>,
    pub tags: EventFilterSet<EventTag>,
    pub privacy_classes: EventFilterSet<EventPrivacy>,
    pub delivery_semantics: EventFilterSet<EventDeliverySemantics>,
    pub terminal_only: bool,
    pub payload_access: PayloadAccessMode,
    pub queue: SubscriberQueueConfig,
}

pub struct CompiledEventFilter {
    pub filter_id: EventFilterId,
    pub filter_fingerprint: EventFilterFingerprint,
    pub indexed_fields: Vec<EventIndexField>,
    pub payload_access: PayloadAccessMode,
    pub queue: SubscriberQueueConfig,
}

pub enum PayloadAccessMode {
    EnvelopeOnly,
    RedactedSummary,
    PayloadRefs,
    FullPayloadIfPolicyAllows,
}

pub enum SubscriberOverflowPolicy {
    DropNonTerminal,
    DropProgress,
    SummarizeAndContinue,
    BackpressureCaller,
    FailSubscriber,
}

pub struct SubscriberQueueConfig {
    pub capacity: NonZeroUsize,
    pub terminal_reserve: NonZeroUsize,
    pub overflow: SubscriberOverflowPolicy,
}

pub struct SubscriptionOptions {
    pub queue: SubscriberQueueConfig,
    pub payload_access: PayloadAccessMode,
}
```

`EventArchive` is a replaceable host or adapter port. It uses `ArchiveCursor` because archive ordering may span many runs, while `JournalCursor` is scoped to one run journal. Without an archive port, all-run, agent-scoped, and arbitrary filtered durable replay must return `UnsupportedReplayScope` or `HostArchiveRequired`; core `AgentEventBus` replay remains run-scoped.

Filterable fields must be envelope/index fields: `run_id`, `agent_id`, `turn_id`, `event_family`, `event_kind`, `source`, `destination`, `subject_ref.kind`, `related_refs.kind`, `correlation_keys`, `tags`, `privacy_classes`, and `delivery_semantics`. A filter must not require deserializing raw payload content on the hot path.

`subscribe_run(run_id, cursor)` and `RunHandle::stream_from(cursor)` share semantics. `subscribe_agent(agent_id, cursor)` emits all live runs for that agent from the live event bus. `subscribe_filtered(filter, cursor)` is the general live primitive that powers all other subscription helpers. Durable catch-up beyond one run requires `EventArchive` / `IndexedJournalView`; without it, agent-scoped or arbitrary filtered replay returns `UnsupportedReplayScope` or `HostArchiveRequired`.

`AgentRuntime` exposes the same primitives as ergonomic pass-throughs: `subscribe_all(cursor)`, `subscribe_run(run_id, cursor)`, `subscribe_agent(agent_id, cursor)`, and `subscribe_events(filter, cursor)`. These use conservative default `SubscriptionOptions`; advanced callers can use the `*_with_options` forms or put queue settings on a compiled filter.

### Cursor Compatibility

Live `EventCursor`s are scope-specific. A cursor can resume only the same logical stream:

| Requested API | Compatible cursor scope | Incompatible cursor behavior |
| --- | --- | --- |
| `subscribe_all(cursor)` | `None` or `EventStreamScope::All` | Return `CursorScopeMismatch`; caller may start from `None` or use durable replay if an indexed archive exists. |
| `subscribe_run(run_id, cursor)` | `None` or `EventStreamScope::Run(run_id)` | Return `CursorScopeMismatch`; a different run cursor must not be widened or narrowed silently. |
| `subscribe_agent(agent_id, cursor)` | `None` or `EventStreamScope::Agent(agent_id)` | Return `CursorScopeMismatch`; run/all cursors are not valid agent cursors. |
| `subscribe_events(filter, cursor)` | `None` or `EventStreamScope::Filter` with the same `filter_fingerprint` | Return `CursorScopeMismatch` when the compiled filter changed. |

`EventStreamScope::All` cannot be narrowed to a run or agent on the live path, and run/agent/filter cursors cannot be widened to all events. To change scope, callers start a new live subscription or request durable replay through a compatible `JournalCursor` and storage/index port.

### Performance Guarantees

- Event emission is envelope-first and non-blocking on slow subscribers.
- `journal_backed` frames append to `RunJournal` first and fan out only after a `JournalCursor` exists. Append failure emits `diagnostic_only` without a journal cursor and blocks side effects that require durable audit.
- `best_effort_live` frames may fan out without a `JournalCursor`.
- Subscriber matching uses indexed envelope fields and precompiled filters where useful. Entity refs may be indexed by kind and ID; raw payload content must not be inspected to match a subscription.
- Live fanout is separate from durable journal persistence. Journal append cannot depend on live subscriber delivery, and live subscribers cannot introduce durable facts.
- The live filter path must not parse JSON payloads, copy raw content, query the content store, or scan the journal.
- Payload access is opt-in. Default subscribers receive envelopes plus bounded redacted summaries or content refs, not copied raw prompt/model/tool/file content.
- Subscriber channels are bounded and declare overflow behavior at subscription time.
- Terminal lifecycle events are never silently dropped by default overflow policies; if a channel cannot deliver them, the subscriber receives a gap diagnostic or must reconnect through journal replay.
- The SDK may maintain per-run, per-agent, per-family/kind, privacy-class, and tag indexes for filtering. Indexes are derived from envelopes and can be rebuilt from journals.

### Queue And Overflow Semantics

Subscriber queues are bounded by `SubscriberQueueConfig`. `capacity` covers normal frames and `terminal_reserve` is reserved for terminal lifecycle frames or gap diagnostics. Live `AgentLoop` emission must never block on subscriber queues.

| Policy | May drop | Terminal behavior | Notice behavior | Producer blocking |
| --- | --- | --- | --- | --- |
| `DropNonTerminal` | non-terminal `best_effort_live` frames | terminal frames use reserve; if reserve is exhausted, emit a gap diagnostic with `repair_from` when available | next delivered frame includes `EventOverflowNotice` | never blocks live loop |
| `DropProgress` | progress/delta frames such as model deltas or tool progress | terminal frames use reserve | summary/gap notice reports dropped range | never blocks live loop |
| `SummarizeAndContinue` | detailed progress frames after creating a redacted summary frame | terminal frames use reserve | summary frame carries overflow notice | never blocks live loop |
| `BackpressureCaller` | none while producer can await | rejected for live `AgentLoop` hot-path subscriptions; allowed only for replay/archive producers or explicit host-owned streams | if rejected, subscription creation returns `InvalidOverflowPolicy` | never blocks live loop |
| `FailSubscriber` | none after overflow threshold | subscriber receives terminal stream error frame or closes with cursor/gap info | final frame carries overflow notice when possible | never blocks live loop |

### Lifecycle Listener Examples

Listen to one run:

```rust
// Non-compiling contract sketch.
let mut stream = runtime.subscribe_run(run_id, None)?;

while let Some(frame) = stream.next().await {
    match frame.event.envelope().event_kind {
        EventKind::RunCompleted | EventKind::RunFailed | EventKind::RunCancelled
            if frame.event.envelope().delivery_semantics == EventDeliverySemantics::JournalBacked
                && frame.event.envelope().journal_cursor.is_some() => break,
        _ => render_redacted(frame.event.summary()),
    }
}
```

Listen to all runs for one agent:

```rust
// Non-compiling contract sketch.
let cursor = stored_cursor_for(agent_id);
let mut stream = runtime.subscribe_agent(agent_id, cursor)?;

while let Some(frame) = stream.next().await {
    update_agent_activity(frame.event.envelope().run_id, frame.event.envelope().event_kind);
    store_cursor(frame.cursor);
}
```

Listen only for terminal run events, then orchestrate outside core:

```rust
// Non-compiling contract sketch.
let filter = EventFilter::new()
    .families([EventFamily::RunLifecycle])
    .kinds([
        EventKind::RunCompleted,
        EventKind::RunFailed,
        EventKind::RunCancelled,
    ])
    .tags([EventTag::new("batch:daily-research")])
    .payload_access(PayloadAccessMode::EnvelopeOnly)
    .compile()?;

let mut terminals = runtime.subscribe_events(filter, None)?;

while let Some(frame) = terminals.next().await {
    barrier.mark_terminal(frame.event.envelope().run_id, frame.event.envelope().event_kind);
    if barrier.two_successes_ready() {
        // The SDK exposes events. The workflow engine decides what to spawn.
        orchestrator.spawn_next_agent().await?;
    }
}
```

## Event Families

The full taxonomy below is reserved SDK vocabulary, not a requirement that the MVP slice emit every kind. The MVP fake-provider text or typed run should fixture only the emitted kinds it uses:

- `RunStarted`
- `ContextAssembled`
- `ProviderRequestProjected`
- `ModelAttemptStarted`
- `ModelMessageCompleted`
- `StructuredOutputRequested` and `StructuredOutputValidated` only for typed runs
- `OutputDispatchRequested` and `OutputDispatchCompleted` only when a fake required sink is configured
- `RunCompleted` or `RunFailed`

Feature goals must add fixtures before emitting additional reserved kinds. Family-level coverage is useful for review, but per-kind emitted fixtures are the implementation gate.

| Family | Kinds |
| --- | --- |
| `run_lifecycle` | `RunStarted`, `RunCheckpointed`, `RunCompleted`, `RunFailed`, `RunCancelled`, `RunCancelRequested`, `RunResumeRequested`, `RunResumeFailed` |
| `turn_lifecycle` | `TurnStarted`, `ContextAssembled`, `ProviderRequestProjected`, `TurnCompleted`, `TurnFailed` |
| `message` | `MessageAccepted`, `MessagePartAdded`, `MessageCommitted`, `MessageRedacted`, `MessageProjected`, `MessageDropped` |
| `model` | `ModelAttemptStarted`, `ModelStreamDelta`, `ModelMessageCompleted`, `ModelUsageRecorded`, `ModelAttemptRetried`, `ModelAttemptFailed`, `ModelAttemptCancelled` |
| `stream_rule` | `StreamRuleRegistered`, `StreamRuleCompileFailed`, `StreamRuleMatched`, `StreamInterventionRequested`, `StreamInterventionApplied`, `StreamInterventionFailed`, `StreamRuleInjectionAppended` |
| `structured_output` | `StructuredOutputRequested`, `StructuredOutputValidationStarted`, `StructuredOutputValidationFailed`, `StructuredOutputRepairRequested`, `StructuredOutputValidated`, `StructuredOutputFailed` |
| `tool` | `ToolRequested`, `ToolApprovalRequired`, `ToolStarted`, `ToolProgress`, `ToolCompleted`, `ToolFailed`, `ToolRetried`, `ToolCancelled`, `ToolInterrupted` |
| `isolation` | `IsolationRequested`, `IsolationAdapterHealthChecked`, `IsolationCapabilityMatched`, `IsolationDowngradeDenied`, `IsolationDowngradeApproved`, `IsolationImageResolved`, `IsolationRootfsPrepared`, `IsolationSessionPrepared`, `IsolationMountsResolved`, `IsolationNetworkPrepared`, `IsolationSecretsPrepared`, `IsolationEnvironmentPrepared`, `IsolationProcessStarted`, `IsolationProcessIoCaptured`, `IsolationProcessStatsRecorded`, `IsolationProcessSignalled`, `IsolationProcessExited`, `IsolationCleanupStarted`, `IsolationCleanupCompleted`, `IsolationCleanupFailed`, `IsolationFailed` |
| `approval` | `ApprovalRequested`, `ApprovalDispatched`, `ApprovalDispatchUnavailable`, `ApprovalResponded`, `ApprovalTimedOut`, `ApprovalDenied`, `ApprovalCancelled` |
| `hook` | `HookRegistered`, `HookInvoked`, `HookCompleted`, `HookFailed`, `HookTimedOut`, `HookCancelled`, `HookResponseApplied`, `HookResponseRejected` |
| `child_lifecycle` | `ChildLifecycleShutdownRequested`, `ChildLifecycleShutdownCompleted`, `ChildLifecycleShutdownFailed`, `ChildLifecycleDetachRequested`, `ChildLifecycleDetachAcknowledged`, `ChildLifecycleDetached`, `ChildLifecycleDetachDenied`, `ChildLifecycleReclaimRequested`, `ChildLifecycleReclaimed`, `ChildLifecycleReclaimFailed` |
| `memory_context` | `MemoryRetrieved`, `ContextContributionReceived`, `ContextContributionSelected`, `ContextContributionOmitted`, `MemoryStored`, `ContextItemInjected`, `ContextCompactionStarted`, `ContextCompactionCompleted`, `ContextProjectionAudited` |
| `realtime` | `RealtimeConnected`, `RealtimeInputSent`, `RealtimeOutputReceived`, `RealtimeInterrupted`, `RealtimeRestartRequested`, `RealtimeRestartStarted`, `RealtimeRestartCompleted`, `RealtimeRestartFailed`, `RealtimeConnectionRestarted`, `RealtimeClosed`, `RealtimeBackpressureApplied` |
| `agent_pool` | `AgentPoolCreated`, `AgentPoolRunJoined`, `AgentPoolRunLeft`, `RunMessageAccepted`, `RunMessageDelivered`, `RunMessageResponded`, `RunMessageFailed`, `RunMessageTimedOut`, `RunMessageExpired`, `RunMessageCancelled`, `WakeConditionRegistered`, `WakeConditionTriggered`, `WakeConditionTimedOut`, `WakeConditionCancelled` |
| `subagent` | `SubagentStarted`, `SubagentHandoff`, `SubagentEvent`, `SubagentCompleted`, `SubagentFailed`, `SubagentCancelled`, `SubagentUsageRolledUp` |
| `extension` | `ExtensionCapabilityLoaded`, `ExtensionHookInvoked`, `ExtensionToolRequested`, `ExtensionEventObserved`, `ExtensionActionSubmitted`, `ExtensionActionStarted`, `ExtensionActionCompleted`, `ExtensionActionFailed`, `ExtensionActionDenied` |
| `output_delivery` | `OutputDispatchRequested`, `OutputDispatchCompleted`, `OutputDispatchFailed`, `OutputDispatchDeduped` |
| `telemetry_cost` | `TelemetrySinkFailed`, `TelemetrySinkRecovered`, `UsageRecorded`, `CostEstimated`, `CostCorrected` |
| `recovery` | `InvariantFailed`, `JournalAppendFailed`, `RecoveryPlanned`, `ReplayStarted`, `ReplayCompleted`, `ReplayFailed`, `AntiEntropyRepairSuggested`, `AntiEntropyRepairApplied` |

Adding a family or kind requires a contract update and golden fixture. Renaming a family or kind requires a compatibility note. `RealtimeConnectionRestarted` is a compatibility alias only; new adapters should emit the requested/started/completed/failed sequence so observers can tell whether a restart was merely planned, in progress, successful, or failed.

The completed contract-packet Phase 05 feature-layer pass canonicalizes earlier
isolation and child-lifecycle draft names before code exists: process I/O, stats,
and signal events use `IsolationProcessIoCaptured`,
`IsolationProcessStatsRecorded`, and `IsolationProcessSignalled`;
detach/reclaim ownership uses the `child_lifecycle` family rather than
`IsolationProcessDetached`; child-lifecycle event kinds carry the
`ChildLifecycle*` prefix. Implementations should not emit the shorter Phase 04
draft aliases.

`HookRegistered` is run-effective, not a pre-run package construction event. It is emitted only after a hook spec is part of a specific run's immutable runtime package and a `run_id` exists. Package construction and validation use runtime-package records/fixtures rather than run-scoped `AgentEvent`s.

## Core Recovery Vs Host Compensation

Recovery events and anti-entropy events are not workflow compensation and not product self-improvement.

Core recovery may:

- rebuild derived internal event indexes, cursors, projections, checkpoints, and telemetry summaries from journal records;
- validate invariants and report repair plans;
- apply repairs that do not execute tools, mutate external stores, send messages, call extensions, or change user-visible product state.

Hosts or optional workflow crates own external side-effect compensation, user-visible repair UI, durable trigger state, workflow rollback, Evolution-style scoring, and automatic improvement policy. If a repair would touch an external effect ref, core emits `host_action_required` and stops.

## Minimal Payload Contracts

Payloads are versioned. Implementation must define one serde DTO per event kind before adapter work starts.

Every payload must include:

- `status` when the event can succeed, fail, timeout, or cancel.
- `summary` as a bounded, redacted human-readable description.
- IDs for the domain entity being updated when not already in the envelope.
- `error` as a typed, redacted error ref when failed.

Payloads must not duplicate routing, cursor, or filter index facts unless a compatibility payload requires a mirror. If mirrored fields such as `agent_id`, `journal_cursor`, tags, or privacy appear in a payload, they must match the envelope exactly and must never be used for filtering or replay.

Payloads must not include raw content by default. Raw content can appear only when:

- `privacy == RawContentAllowed`
- `content_capture` names the explicit policy
- retention is declared
- the sink is allowed to receive it

## Per-Family Payload Minimums

These are minimum fields. Event-specific DTOs may add optional fields, but they cannot remove these without a compatibility note.

| Family | Required payload fields |
| --- | --- |
| `run_lifecycle` | `source_surface`, `terminal_status`, `stop_reason`, `started_at`, `ended_at`, `duration_ms`, `resume_policy`, `error_ref`. |
| `turn_lifecycle` | `turn_index`, `input_message_ids`, `context_projection_id`, `budget_summary`, `turn_outcome`, `error_ref`. |
| `message` | `role`, `part_kinds`, `lineage_ref`, `content_refs`, `commit_status`, `redaction_status`, `projection_status`. |
| `model` | `provider_id`, `model_id`, `attempt_index`, `stream_cursor`, `delta_kind`, `stop_reason`, `usage_ref`, `retry_classification`, `error_ref`. |
| `stream_rule` | `rule_id`, `rule_version`, `channel`, `cursor`, `matcher_kind`, `action`, `redacted_match`, `repeat_state`, `partial_output_policy`. |
| `structured_output` | `schema_id`, `schema_version`, `validation_attempt`, `repair_attempt`, `validation_status`, `redacted_errors`, `validated_output_ref`. |
| `tool` | `canonical_tool_name`, `tool_source`, `effect_class`, `attempt_index`, `approval_ref`, `idempotency_key`, `status`, `result_ref`, `effect_ref`. |
| `isolation` | `environment_id`, `adapter_kind`, `capability_report_ref`, `image_ref`, `mount_policy_hash`, `network_policy_hash`, `process_ref`, `exit_status`, `cleanup_status`. |
| `approval` | `approval_request_id`, `dispatcher_kind`, `decision`, `actor_ref`, `timeout_ms`, `source_scope`, `policy_refs`, `denial_reason`, `effect_ref`. |
| `hook` | `hook_id`, `hook_point`, `source`, `execution_mode`, `queue_policy`, `timeout_ms`, `failure_policy`, `mutation_rights`, `response_class`, `status`, `error_ref`. |
| `child_lifecycle` | `child_artifact_id`, `child_artifact_kind`, `owner_run_id`, `shutdown_behavior`, `detach_status`, `reclaim_policy_ref`, `host_ack_ref`, `terminal_status`, `error_ref`. |
| `memory_context` | `context_contribution_id`, `context_item_id`, `context_kind`, `producer_ref`, `source_ref`, `content_ref`, `selection_reason`, `policy_refs`, `projection_id`, `included_counts`, `omitted_counts`, `redaction_policy_id`. |
| `realtime` | `connection_id`, `media_kind`, `stream_cursor`, `backpressure_policy`, `restart_count`, `interruption_ref`, `status`. |
| `agent_pool` | `pool_id`, `message_id`, `wake_condition_id`, `source_run_id`, `target_ref`, `topic_id`, `delivery_status`, `timeout_ms`, `policy_refs`, `content_refs`. |
| `subagent` | `parent_run_id`, `child_run_id`, `child_agent_id`, `handoff_context_refs`, `child_policy_summary`, `usage_rollup_ref`, `terminal_status`. |
| `extension` | `extension_id`, `capability_kind`, `protocol_version`, `hook_kind`, `action_kind`, `policy_decision_ref`, `status`. |
| `output_delivery` | `destination`, `dedupe_key`, `source_message_id`, `dispatch_status`, `ack_ref`, `reconciliation_status`. |
| `telemetry_cost` | `sink_id`, `export_cursor`, `usage_units`, `cost_units`, `currency`, `rate_table_version`, `estimate_status`, `correction_ref`. |
| `recovery` | `invariant_id`, `replay_mode`, `repair_plan_id`, `unsafe_pending_reason`, `repair_status`, `host_action_required`. |

## Payload Freeze Gate

Before coding any adapter, implementation must create a golden payload fixture for every event kind that adapter can emit. A fake provider/tool/approval/isolation/subagent/realtime adapter can emit only event kinds that already have fixtures.

Each workstream must maintain an emitted-kind matrix:

| Adapter/workstream | Event kinds it can emit | Required fixtures |
| --- | --- | --- |
| fake provider | all model and structured-output kinds used by tests | one JSON payload per emitted kind |
| fake tool executor | all tool and approval kinds used by tests | one JSON payload per emitted kind |
| fake realtime adapter | all realtime lifecycle/interruption/restart kinds used by tests | one JSON payload per emitted kind |
| fake isolation adapter | all isolation capability, downgrade, lifecycle, process I/O, stats, cleanup, and failure kinds used by tests | one JSON payload per emitted kind plus no-raw-process-data redaction cases |
| fake agent-pool coordinator | all agent-pool run-message and wake-condition kinds used by tests | one JSON payload per emitted kind plus no-raw-message-content redaction cases |
| fake subagent runner | all subagent handoff, wrapped event, terminal, and rollup kinds used by tests | one JSON payload per emitted kind |
| fake hook executor | all hook invocation, timeout, response, cancel, and failure kinds used by tests | one JSON payload per emitted kind plus no-raw-content redaction cases |
| fake child lifecycle reconciler | all child shutdown, detach, reclaim, process signal, and cleanup kinds used by tests | one JSON payload per emitted kind plus replay fixtures |
| fake extension bridge | all extension capability, hook/tool/action, app-event observation, and action terminal kinds used by tests | one JSON payload per emitted kind plus host-manifest-exclusion redaction cases |

Family-level fixture coverage is still required, but it is not enough for implementation. Per-kind emitted fixtures are the coding gate.

## Source And Destination

`SourceRef.kind` has a stable core set plus namespaced extension kinds. Core kinds are finite for indexing, but hosts/adapters may use `other:<namespace>/<kind>` only when they also provide source policy, privacy, and redaction metadata:

- `user`
- `system`
- `developer`
- `memory`
- `tool`
- `model`
- `hook`
- `extension`
- `remote_channel`
- `scheduled_task`
- `cli`
- `desktop`
- `external_runtime`
- `agent_run`
- `agent_pool`
- `subagent`
- `replay`
- `compaction`
- `policy`
- `stream_rule`
- `isolation_runtime`
- `other:<namespace>/<kind>`

`DestinationRef.kind` has the same stable-core-plus-namespaced-extension shape:

- `provider`
- `tool`
- `memory`
- `session`
- `user`
- `remote_channel`
- `child_agent`
- `parent_agent`
- `telemetry`
- `journal`
- `ui`
- `external_runtime`
- `agent_pool`
- `agent_run`
- `isolation_runtime`
- `hook`
- `output_sink`
- `other:<namespace>/<kind>`

Namespaced `other` kinds are not a shortcut around typed refs. Durable SDK-owned behaviors still need normal `EntityRef`, policy refs, events, journal records, and owner review before becoming core kinds.

## Redaction

Default event capture includes:

- IDs, roles, part kinds, MIME types, byte/token counts, hashes, status, latency, stop reasons, policy decisions, and bounded summaries.

Default event capture excludes:

- raw user text, system prompt, model response, hidden reasoning, tool input/output, memory bodies, remote message content, file bytes, environment values, credentials, and auth headers.

## Acceptance Tests / Golden Tests

MVP required golden event tests:

- one JSON fixture for every event kind emitted by implemented adapters
- `subscribe_all_receives_envelope_only_by_default`
- `journal_backed_event_not_fanned_out_until_append_cursor_exists`
- `append_failure_emits_diagnostic_only_without_journal_cursor`
- `subscribe_run_matches_run_handle_stream_semantics`
- `subscribe_agent_filters_by_agent_id_without_payload_deserialization`
- `subscribe_filtered_uses_compiled_envelope_filter`
- `event_stream_yields_frame_with_cursor_and_overflow_notice`
- `archive_replay_frame_exposes_archive_cursor`
- `overflow_notice_reports_gap_cursors_and_repair_cursor`
- `cursor_scope_mismatch_is_typed_error`
- `terminal_event_filter_can_drive_external_barrier_orchestration`
- `slow_subscriber_overflow_does_not_block_event_emission`
- `drop_non_terminal_overflow_preserves_terminal_or_gap_notice`
- `drop_progress_overflow_summarizes_dropped_deltas`
- `summarize_and_continue_emits_redacted_summary_frame`
- `backpressure_caller_rejected_on_agent_loop_hot_path`
- `fail_subscriber_reports_final_cursor_or_gap`
- `terminal_event_gets_gap_or_journal_repair_under_overflow`
- `payload_access_full_content_requires_policy`
- `payload_mirrored_index_fields_must_match_envelope`
- `run_replay_from_journal_cursor_catches_up`
- `cross_run_replay_requires_indexed_journal_view`
- `filtered_replay_without_host_archive_is_rejected`
- `core_event_bus_replay_is_run_scoped_only`
- `repair_replay_never_executes_tool_output_memory_or_extension_side_effects`
- `anti_entropy_repair_event_rejects_external_effect_refs`
- `event_golden_payload_exists_for_each_emitted_kind`
- `adapter_emitted_kind_matrix_has_no_fixture_gaps`
- one failed event with typed error and causal IDs
- one redacted event with content refs only
- `event_seq` monotonicity and `journal_cursor` presence only for journal-backed events
- `state_before/state_after` on loop transition events
- root event `caused_by = None`

Feature-layer golden tests required when those kinds are emitted:

- family-level coverage fixture for each event family emitted by the implemented slice
- one parent/child `SubagentEvent`
- one agent-pool run-message request/response pair
- one wake-condition register/trigger or timeout pair
- one `ContextProjectionAudited`
- one `StreamRuleMatched`
- one realtime restart requested/started/completed sequence
- one realtime restart failed event
- one `IsolationProcessExited`
- one `HookInvoked` / `HookResponseApplied` pair with content refs only
- one `HookTimedOut` nonblocking event
- one `ChildLifecycleShutdownRequested` / `ChildLifecycleShutdownCompleted` pair
- one `ChildLifecycleDetachRequested` / `ChildLifecycleDetached` pair with host ack ref
- one `IsolationProcessSignalled` event with signal intent/result refs
- one `CostCorrected`

Reserved feature families do not block the MVP slice. They become required only for the workstream that emits them.

Every golden fixture must prove:

- stable family/kind strings
- schema version fields
- required envelope IDs
- no raw content in default mode
- stable deserialization when optional payload fields are added

## Complete Example

Typed shape:

```rust
// Non-compiling contract sketch.
pub struct ToolCompletedPayload {
    pub canonical_tool_name: CanonicalToolName,
    pub tool_source: ToolSourceRef,
    pub effect_class: EffectClass,
    pub attempt_index: u16,
    pub approval_ref: Option<ApprovalRequestId>,
    pub idempotency_key: Option<IdempotencyKey>,
    pub status: ToolTerminalStatus,
    pub result_ref: ContentRef,
    pub effect_ref: Option<EffectRef>,
    pub error: Option<ErrorRef>,
}

let event = EventEnvelope {
    schema_version: 1,
    event_id: EventId::new(),
    event_seq: 42,
    event_family: EventFamily::Tool,
    event_kind: EventKind::ToolCompleted,
    payload_schema_version: 1,
    run_id,
    session_id: Some(session_id),
    agent_id,
    turn_id: Some(turn_id),
    attempt_id: Some(attempt_id),
    subject_ref: EntityRef::tool_call(tool_call_id),
    related_refs: vec![EntityRef::effect(effect_id)],
    causal_refs: vec![CausalRef::effect(effect_id)],
    correlation: EventCorrelation::from_run(run_id),
    tags: vec![EventTag::new("tool:workspace")],
    source: SourceRef::tool("workspace_read"),
    destination: Some(DestinationRef::agent_loop()),
    delivery_semantics: EventDeliverySemantics::JournalBacked,
    privacy: EventPrivacy::DefaultRedacted,
    content_capture: ContentCaptureMode::Off,
    journal_cursor: Some(journal_cursor),
    payload: ToolCompletedPayload { /* fields above */ },
    /* remaining envelope fields */
};
```

Replaceable ports:

- `EventSink` implementations can be UI, CLI, journal fanout, telemetry, or host adapters.
- Payload DTOs are versioned per event kind, so a sink can deserialize only the families it understands.
- Unknown optional payload fields are ignored by older readers; unknown required schema versions fail closed.

Wiring:

1. Loop appends the matching journal record.
2. Loop emits a journal-backed `AgentEvent`.
3. Live sinks receive bounded event envelopes.
4. Telemetry and host projections derive from the envelope and payload without raw content.

Events:

- This example emits `ToolCompleted`.
- The tool attempt normally also emits `ToolRequested`, `ToolApprovalRequired`, `ToolStarted`, and optionally `ToolFailed` or `ToolRetried`.

Journal:

- `ToolRecord { intent, approval_ref, attempt, result, effect_metadata }`
- `TelemetryRecord` if usage/cost is derived from the tool.

Policies and failures:

- If journal append fails before the event, emit only `diagnostic_only` and do not claim durable completion.
- If a sink drops the event, reconnect uses `journal_cursor`.
- If payload schema version is unsupported, consumers fail that event only, not the run.

SDK owns / Host owns:

- SDK owns event family/kind strings, envelope IDs, payload DTO versions, redaction defaults, and delivery semantics.
- SDK owns event subscription/filter/replay primitives and fast envelope-field matching.
- Host owns sink retention, UI rendering, trace-store schema, product-specific event grouping, and workflow orchestration that reacts to events.

Tests:

- `event_golden_payload_exists_for_each_emitted_kind`
- `adapter_emitted_kind_matrix_has_no_fixture_gaps`
- `subscribe_filtered_uses_compiled_envelope_filter`
- golden fixture: `events/tool_completed_v1.json`
