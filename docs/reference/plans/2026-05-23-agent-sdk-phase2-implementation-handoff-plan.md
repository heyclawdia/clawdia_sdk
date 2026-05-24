# Agent SDK Phase 2 Implementation Handoff Plan

> Historical reference only. Current implementation authority lives in [../../contracts/README.md](../../contracts/README.md) and [../../workstreams/README.md](../../workstreams/README.md). This plan may contain superseded workstream numbering; use it for background only unless the user explicitly reactivates it.

## Objective

Turn the accepted Agent SDK Phase 1 architecture docs into a test-first implementation plan for the future Rust-first Agent SDK package family, starting with the concrete `agent-sdk-core` crate and extension SDK bridge.

This plan is meant for human review and editing before it is handed to a coding agent. It intentionally spells out contracts, invariants, test gates, and host boundaries so the implementer does not have to guess.

## Current Status

Phase 1 is documentation-only. The existing docs define the desired SDK primitives, product-neutral scenario requirements, external SDK lessons, observability model, journal/replay model, structured output, stream rules, built-in tool packs, extension SDK compatibility, and execution isolation concepts.

Phase 2 should not begin until this handoff plan is reviewed and accepted.

## Independent Review Result

An independent review of the Phase 1 docs judged them blocking for direct coding handoff because several architecture sections were still "middle-level" rather than contract-level. This plan treats that as a real blocker and resolves it by making the implementation agent start from explicit contracts and tests.

Reviewer findings that must stay visible during review:

- The loop state machine and Rust sketch must not diverge.
- The event envelope must include event family/kind and payload versioning.
- Runtime package fingerprinting needs deterministic canonicalization.
- Journal/replay needs concrete record identity, ordering, checkpoint, and atomicity rules.
- Approval semantics need a compatibility note because the SDK's fail-closed target may be stricter than legacy host behavior.
- Structured output, stream rules, built-in tool packs, isolation, extension packaging, telemetry/cost, and generic scenario mapping need exact acceptance tests before implementation.
- Tool packs, container runtimes, approval transport, host display-event feeds, trace ingestion, external-runtime session policy, and product recommendation UX remain host-owned or optional adapter/toolkit layers.
- Core recovery repairs only derived internal views. Workflow compensation, external side-effect rollback, user-visible repair UI, durable trigger state, and self-improvement products stay host-owned or optional workflow-layer concerns.

## Contract File Strategy

This document is the review/edit surface. Normative SDK contract docs now live under `docs/contracts/`. Scenario references live under `docs/examples/` and must stay product-neutral; product-specific host adapters are outside the active SDK handoff unless explicitly requested.

If Phase 2 implementation creates or changes contract docs, keep the normative set focused on:

- `api-contracts.md`
- `event-schema.md`
- `run-handle-reconnect-contract.md`
- `runtime-package-schema.md`
- `loop-state-machine.md`
- `journal-replay-schema.md`
- `tool-approval-contract.md`
- `structured-output-contract.md`
- `stream-rule-contract.md`
- `tool-pack-contract.md`
- `isolation-runtime-contract.md`
- `subagent-contract.md`
- `extension-sdk-contract.md`
- `otel-mapping-contract.md`
- `telemetry-privacy-contract.md`

Each contract doc must include public Rust types, serde schema/version, invariants, fail-closed behavior, compatibility notes, event/journal records emitted, and exact tests expected. Do not create a new gap matrix for this; these are implementation contracts.

The current contract packet lives under `docs/contracts/`. The scenario packet lives under `docs/examples/`. Product-specific host reference packets are not part of the active SDK handoff.

## Non-Negotiables

- Do not create a branch unless the user explicitly approves it.
- Do not modify production host runtime behavior until the standalone SDK contracts and tests are accepted.
- Do not add a third production runtime path during early SDK prototyping.
- Do not clone Strands, Cursor, Claude Agent SDK, Pi, oh-my-pi, or Apple Containerization.
- Keep the SDK product-neutral. Desktop apps, CLIs, voice UI, remote messaging, coding harnesses, recommendation products, trace UI, and extension marketplace behavior stay host-owned.
- Use Rust-first domain contracts. JavaScript/TypeScript extension packages may bridge to those contracts, but they must not become the source of truth for the core runtime.
- Tests come before behavior. Every implementation slice starts with contract tests, golden records, fake adapters, or smoke tests that define the expected behavior.
- Raw content capture is opt-in. Default events, telemetry, and journals use IDs, hashes, sizes, redacted summaries, and content references.

## Relevant Existing Context

The coding agent must read these before editing:

- `/Users/clawdia/goals/agent_sdk_phase1.md`: durable source of truth for Phase 1 scope and constraints.
- `coding-standards.md`: repo-wide DDD, source-of-truth, typed-contract, testing, extensibility, approval, and no-heuristic rules.
- `docs/start-here.md`: navigation, design posture, core thesis, and non-goals.
- `docs/architecture/coding-standards.md`: future SDK standards.
- `docs/examples/README.md`: product-neutral scenarios the SDK must support.
- `docs/architecture/external-sdk-lessons.md`: Strands, Cursor, Claude Agent SDK, Pi, oh-my-pi, Apple Containerization, and OTel lessons.
- `docs/architecture/primitive-map.md`: primitive ownership, methods, and must-not-own boundaries.
- `docs/architecture/observability-and-lineage.md`: event envelope, event taxonomy, journal/replay, privacy, telemetry, cost, and recovery guarantees.
- `docs/examples/*.md`: desktop chat, voice/realtime, CLI/headless, structured output, isolation, stream rule, and recovery examples.
- `docs/architecture/architecture-proposal.md`: module layout, state machine, conceptual Rust skeletons, and migration path.
- `docs/reference/risks/agent-sdk-phase1-2026-05-21.md`: risks and watchpoints that must carry into Phase 2.

Primary external references reviewed for this packet:

- [Strands hooks](https://strandsagents.com/latest/documentation/docs/user-guide/concepts/agents/hooks/), [Strands streaming](https://strandsagents.com/latest/documentation/docs/user-guide/concepts/streaming/), and [Strands bidirectional hooks](https://strandsagents.com/latest/documentation/docs/user-guide/concepts/bidirectional-streaming/hooks/).
- [Cursor SDK release notes](https://cursor.com/changelog/sdk-release), especially durable run streaming and `Last-Event-ID` reconnect.
- [Pi package docs](https://pi.dev/docs/latest/packages) and [pi-multiagent package](https://pi.dev/packages/pi-multiagent).
- [oh-my-pi](https://github.com/can1357/oh-my-pi).
- [Apple Containerization](https://github.com/apple/containerization).
- [OpenTelemetry GenAI](https://opentelemetry.io/docs/specs/semconv/gen-ai/), [GenAI agent spans](https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-agent-spans/), [GenAI model/tool spans](https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-spans/), and [MCP semconv](https://opentelemetry.io/docs/specs/semconv/gen-ai/mcp/).

Useful prior plans:

- `docs/reference/plans/2026-05-21-agent-sdk-phase1-docs-plan.md`
- `docs/reference/plans/2026-05-23-agent-sdk-phase1-completion-plan.md`
- `docs/reference/plans/2026-05-23-agent-sdk-containerization-primitive-plan.md`

## Root Problem Shape

Modern host products commonly have multiple agent execution surfaces: desktop or web chat, external runtimes, provider-native runtimes, CLI, headless jobs, scheduled tasks, remote channels, memory ingestion, extensions, MCP tools, app tools, voice/realtime, and subagents. They need one future SDK vocabulary for runs, messages, context, tools, policy, events, journals, telemetry, and recovery.

The risk is not that the SDK lacks features. The risk is that contracts stay too conceptual and a coding agent fills in behavior ad hoc. Phase 2 must therefore freeze small implementation contracts in tests before building out adapters.

## Source Of Truth Rules

- `RuntimePackage` is the source of truth for provider-visible tools, executable registries, hooks, stream rules, policies, subagents, extension capabilities, and isolation requirements.
- `AgentEvent` and `AgentEventBus` are the source of truth for live observability vocabulary and lifecycle subscriptions.
- `RunJournal` is the durable source of truth for audit, replay, resume, and recovery.
- Host settings and policies remain the source of truth for approval, permission, sandbox, autonomy, escalation, retention, and content capture.
- Provider adapters receive `ContextProjection`, not lossless internal messages.
- Extension SDK manifests declare capabilities; extension processes may observe or request, but they do not own approval, memory, provider routing, or telemetry.

## Contract 1: Crate Boundary And Module Layout

Phase 2 should prototype a standalone SDK crate before wiring production host runtime paths.

Expected first crate shape:

```text
crates/agent-sdk-core/
  Cargo.toml
  src/
    lib.rs
    domain/
    package/
    providers/
    stream/
    tools/
    isolation/
    policy/
    context/
    hooks/
    session/
    telemetry/
    subagents/
    channels/
    recovery/
```

Initial modules may be smaller than the full Phase 1 layout, but the public module names should not conflict with the proposed layout unless the docs are updated.

Required public exports in the first usable slice:

- `Agent`
- `AgentBuilder`
- `AgentRuntime`
- `RunHandle`
- `RunRequest`
- `RunResult`
- `AgentEvent`
- `EventEnvelope`
- `EventFrame`
- `EventCursor`
- `JournalCursor`
- `ArchiveCursor`
- `AgentEventBus`
- `EventFilter`
- `CompiledEventFilter`
- `EventOverflowNotice`
- `EventFilterFingerprint`
- `SubscriptionOptions`
- `SubscriberQueueConfig`
- `AgentEventStream`
- `EventArchive`
- `RuntimePackage`
- `RuntimePackageBuilder`
- `AgentMessage`
- `ContextItem`
- `ContextProjection`
- `ProviderAdapter`
- `ToolRegistry`
- `ToolExecutor`
- `ApprovalPolicy`
- `ApprovalBroker`
- `RunJournal`
- `TelemetrySink`

Public support types used by these signatures, such as `RunRegistry`, `RunSnapshot`, `RunStatus`, `RunQuery`, `EventStreamScope`, handle types, replay results, and advanced config types, must either be exported or reachable from stable modules in the first slice.

Must not own:

- Product UI routing.
- External-runtime session cache.
- Trace ingestion policy.
- Extension installation UX.
- Coding-agent prompt recipes.
- Evolution proposal generation or scoring.

Acceptance gates:

- Crate compiles independently.
- Public exports are documented enough for rustdoc to explain ownership.
- No production host runtime route imports the crate until an explicit integration plan is accepted.

## Contract 2: Identity, Versioning, And Errors

All durable and cross-boundary IDs must be typed newtypes, not raw strings in domain logic.

Required ID types:

- `RunId`
- `TurnId`
- `AttemptId`
- `MessageId`
- `ContextItemId`
- `ContextProjectionId`
- `ToolCallId`
- `ApprovalRequestId`
- `AgentId`
- `SubagentRunId`
- `RuntimePackageId`
- `RuntimePackageFingerprint`
- `EventId`
- `TraceId`
- `SpanId`
- `ExecutionEnvironmentId`
- `StreamRuleId`

Versioned contracts:

- Event envelope schema version.
- Event payload schema version.
- Runtime package schema version.
- Journal record schema version.
- Extension protocol version.
- Structured output schema ID/version.
- Stream rule version.

Error contract:

- `AgentError` must distinguish invalid package, invalid transition, provider, projection, tool, approval, policy, journal, telemetry, isolation, structured output, stream rule, subagent, extension, cancellation, timeout, and recovery errors.
- Errors carry causal IDs when available.
- Retriability is typed: retryable, not retryable, repair needed, user action needed, host configuration needed.

Acceptance tests:

- `ids_do_not_serialize_as_ambiguous_untyped_records`
- `errors_preserve_run_and_causal_ids`
- `retry_classification_is_explicit_for_public_errors`

## Contract 3: Event Envelope And Taxonomy

The event family and event kind names from `observability-and-lineage.md` are stable Phase 1 vocabulary. Phase 2 may add optional payload fields, but renaming a family or kind requires a migration note.

Required envelope fields:

- `schema_version`
- `event_id`
- `event_seq`
- `event_family`
- `event_kind`
- `payload_schema_version`
- `timestamp`
- `recorded_at`
- `run_id`
- `agent_id`
- optional `turn_id`
- optional `attempt_id`
- optional `message_id`
- optional `context_item_id`
- optional `tool_call_id`
- optional `approval_request_id`
- optional `stream_rule_id`
- optional `execution_environment_id`
- optional `isolated_process_id`
- optional `subagent_run_id`
- `trace_id`
- `span_id`
- optional `parent_event_id`
- `caused_by`
- `source`
- optional `destination`
- `correlation`
- `tags`
- `policy_refs`
- optional `journal_cursor`
- optional `state_before`
- optional `state_after`
- `privacy`
- `delivery_semantics`
- `content_capture`
- `redaction_policy_id`
- `runtime_package_fingerprint`

Event invariants:

- Every event has one run ID.
- Every event has one agent ID.
- Turn-scoped events have a turn ID.
- Attempt-scoped events have an attempt ID.
- Tool events have a tool call ID.
- Approval events have an approval request ID.
- Isolation process events have an execution environment ID.
- Child agent events carry parent run ID and child run ID.
- Raw content is absent unless content-capture policy explicitly allows it.
- Sinks can route, redact, replay, filter, and correlate from the envelope alone.
- Live event filtering must use envelope/index fields only: run IDs, agent IDs, turn IDs, families, kinds, source, destination, correlation keys, tags, privacy classes, and delivery semantics.
- The live filter path must not parse JSON payloads, copy raw content, query content stores, or scan the journal.
- Runtime-wide event listening is core: `runtime.subscribe_all(cursor)`, `runtime.subscribe_run(run_id, cursor)`, `runtime.subscribe_agent(agent_id, cursor)`, and `runtime.subscribe_events(filter, cursor)`.
- Event streams yield `EventFrame { event, cursor, archive_cursor, overflow }`.
- `EventCursor` is the live stream cursor. `JournalCursor` is the per-run durable replay cursor. `ArchiveCursor` is the optional indexed archive cursor for cross-run/all-event durable replay. Run-scoped replay is guaranteed from the run journal through `replay_run_from_cursor`; cross-run/all-event durable replay requires an optional host `EventArchive` or indexed journal view.
- `EventCursor` compatibility is exact by logical stream: all resumes all, run resumes the same run, agent resumes the same agent, and filter resumes only the same `EventFilterFingerprint`.
- `EventOverflowNotice` carries overflow policy, dropped count, gap start/end cursors, optional repair journal cursor, terminal-preserved flag, and reason. `EventFrame.cursor` always identifies the delivered SDK event-stream frame; `EventFrame.archive_cursor` carries the next indexed archive replay position when present.
- `SubscriberQueueConfig` carries bounded capacity, terminal reserve, and overflow policy. `BackpressureCaller` is rejected for live `AgentLoop` hot-path subscriptions; no overflow policy may block the live loop or journal append.
- Higher-level orchestration may consume terminal filtered events and call `start_run`, but workflow/DAG/barrier engines remain outside `agent-sdk-core`.

Acceptance tests:

- Golden JSON for every event kind emitted by the first fake adapters.
- A family coverage matrix proving at least one fixture exists for every event family in the taxonomy.
- A workstream emitted-kind matrix that fails review when an adapter emits a kind without a fixture.
- Envelope redaction removes raw content while preserving routeable IDs.
- Parent/child event wrapping preserves both run identities.
- Unknown optional payload fields do not break deserialization.
- Compiled event filters match run, agent, turn, kind/family, source/destination, correlation, tags, privacy, and delivery semantics without payload parsing.
- Slow subscriber overflow does not block event emission and reports gaps through `EventOverflowNotice`.
- Saturated-queue tests cover every overflow policy and prove terminal events get either delivery, a gap diagnostic, or journal repair cursor.
- Cross-run replay without an indexed archive returns `UnsupportedReplayScope` or `HostArchiveRequired`.
- Core event bus replay is run-scoped unless an `EventArchive` / `IndexedJournalView` port is configured.
- Anti-entropy repair cannot execute tools, mutate memory/output stores, call extensions, or apply workflow compensation.

## Contract 4: Run Journal And Replay

The journal is an append-only ledger, not a UI stream and not a transcript blob.

Required journal traits:

```rust
// Non-compiling target shape.
trait RunJournal {
    async fn append(&self, record: JournalRecord) -> Result<JournalCursor, JournalError>;
    async fn replay(&self, run_id: RunId, mode: ReplayMode) -> Result<ReplayResult, JournalError>;
    async fn seal(&self, run_id: RunId, terminal: TerminalRunState) -> Result<(), JournalError>;
}
```

Required record families:

- `RunRecord`
- `TurnRecord`
- `ContextRecord`
- `MessageRecord`
- `ModelAttemptRecord`
- `StructuredOutputRecord`
- `StreamRuleRecord`
- `ApprovalRecord`
- `ToolRecord`
- `IsolationRecord`
- `OutputDispatchRecord`
- `SubagentRecord`
- `TelemetryRecord`
- `RecoveryRecord`

Every journal record must carry:

- `journal_schema_version`
- `journal_seq`
- `record_id`
- `record_kind`
- `run_id`
- `agent_id`
- optional `turn_id`
- optional `attempt_id`
- optional causal IDs for message, tool, approval, stream rule, isolation environment, subagent, output dispatch, or telemetry export
- `source`
- optional `destination`
- `correlation_keys`
- `tags`
- `delivery_semantics`
- `event_index: EventIndexProjection`
- `timestamp`
- `runtime_package_fingerprint`
- `privacy`
- `content_refs`
- `redaction_policy_id`
- optional `idempotency_key`
- optional `dedupe_key`
- optional `checkpoint_ref`

Ordering and concurrency rules:

- `journal_seq` is monotonic per run.
- Concurrent tool and stream records may interleave globally, but each attempt has per-attempt order.
- A record's `caused_by` edge is authoritative for causality; readers must not infer causality from wall-clock timestamp alone.
- Every journal record that can produce an event frame carries or deterministically derives `EventIndexProjection` with run, agent, turn, family, kind, source, destination, correlation keys, tags, privacy class, and delivery semantics. Durable replay filters use that projection, never payload parsing.
- Checkpoints accelerate replay, but journal records remain the durable truth. A corrupt checkpoint falls back to replay from the last valid journal cursor.
- Content store references include content hash, media type, byte length, retention class, and availability state. Missing required content refs fail resume instead of substituting empty content.
- Deterministic test IDs are allowed in tests, but production IDs must be collision-resistant and should not encode private content.

Append order around side effects:

1. Classify policy.
2. Append side-effect intent with idempotency or dedupe key.
3. Execute external side effect only if append succeeds.
4. Append terminal status.
5. Emit derived telemetry.

Replay modes:

- `AuditReplay`: reconstruct metadata, trace, decisions, terminal state. Never re-executes side effects.
- `ResumeReplay`: reconstruct to last safe checkpoint and continue. Re-executes only idempotent or explicitly approved retry steps.
- `RepairReplay`: rebuild derived indexes, projections, or telemetry. Never emits user-visible sends or external side effects.

Resume rules:

- Resume starts with `RunResumeRequested`.
- Runtime package fingerprint is validated before continuation.
- If required content references are missing, emit `RunResumeFailed`.
- Pending provider streams resume from provider cursor only when the adapter declares cursor support. Otherwise retry creates a new attempt.
- Pending tool calls resume only with idempotency keys or explicit host retry approval.
- Pending remote output sends use output dedupe keys and channel reconciliation.

Cancel rules:

- Cancel emits `RunCancelRequested`.
- Provider streams, tools, realtime sessions, approvals, isolated processes, and child agents receive cancellation.
- Cleanup has bounded timeout.
- Terminal state is `RunCancelled` only after cleanup or timeout is recorded.

Failure rules:

- The most specific failure event is appended first.
- Partial deltas remain partial until `ModelMessageCompleted`.
- Irrecoverable failure appends `RunFailed` and seals the journal.
- Journal append failure before a required non-idempotent side effect fails closed.

Acceptance tests:

- Intent is recorded before a fake mutating tool executes.
- Journal append failure prevents non-idempotent tool execution.
- Audit replay never calls provider/tool/output fakes.
- Resume replay refuses a non-idempotent pending tool.
- Cancel closes pending approval without executing the tool.
- Provider partial stream failure does not commit an assistant message.

## Contract 5: Runtime Package

`RuntimePackage` is immutable once a run starts.

Required package contents:

- Agent identity and defaults.
- Provider/model route.
- Provider-visible tool specs.
- Executable tool registry fingerprint.
- Tool packs and their policies.
- MCP snapshot.
- Hook snapshot.
- Stream rule snapshot.
- Isolation requirements and accepted adapter kinds.
- Subagent snapshot.
- Approval, permission, sandbox, autonomy, escalation, retention, and content-capture policies.
- Extension capability snapshot.

Fingerprint contract:

- Fingerprint includes all provider-visible schemas and executable routing metadata.
- Fingerprint excludes volatile values such as timestamps and process IDs.
- Provider projection and tool execution use the same package snapshot.
- Runtime package deltas are next-turn or next-run snapshots, not hidden ambient mutation.

Canonicalization algorithm:

- Build a `RuntimePackageCanonicalV1` serde DTO before hashing.
- Sort maps and lists by stable keys: provider route ID, canonical tool name, MCP server ID, hook ID, stream rule ID, subagent ID, extension ID, and policy ID.
- Hash canonical UTF-8 JSON bytes or another explicitly versioned canonical binary encoding. The algorithm name and schema version are part of the fingerprint preimage.
- Include provider route, model ID, provider capability version, tool specs, tool handler route IDs, tool pack IDs/versions/policies, MCP exposure snapshot, hook capabilities, compiled stream rule IDs/versions/matchers/actions, isolation requirement policy, accepted adapter capability class/version, subagent package summaries, extension capabilities, and policy snapshots.
- Exclude runtime-only values: timestamps, run IDs, process IDs, adapter health results, cache hit state, temporary paths, telemetry sink health, and live approval request IDs.
- Local workspace and mount access use host-provided stable workspace identity and mount policy hash. Raw absolute paths can be journaled with privacy policy, but they should not make package fingerprints machine-unique unless the host explicitly chooses path-bound packages.
- Any field that can change provider-visible behavior, executable routing, approval, privacy, or replay safety must change the fingerprint.

Acceptance tests:

- Same logical package produces deterministic fingerprint.
- Changing provider-visible tool schema changes fingerprint.
- Changing executable handler route changes fingerprint.
- Provider projection and executable registry fingerprints match.
- Tool discovery activation cannot mutate an active package without an explicit package delta record.

## Contract 6: Agent Loop State Machine

The loop is explicit state, not hidden recursion.

Required states:

- `Starting`
- `ContextAssembly`
- `ProviderProjection`
- `ModelStreaming`
- `StreamIntervention`
- `ToolPlanning`
- `Approval`
- `ToolDenied`
- `ToolExecution`
- `Interrupted`
- `WaitingForResume`
- `Compaction`
- `Continue`
- `Recovery`
- `Completed`
- `Failed`

Required invariants:

- `RunStarted` is emitted before first turn state.
- `ContextProjectionAudited` is emitted before provider request.
- Model attempts produce attempt IDs and terminal attempt events.
- Tool execution cannot start before policy classification and required approval.
- Denied tool calls become typed denied results only when policy allows continuation.
- Max-iteration stop is a typed terminal result, not a generic failure.
- Cancellation can interrupt provider streams, tools, approval waits, realtime loops, isolation runtimes, and child agents.
- Recovery cannot silently hide failed attempts.

Acceptance tests:

- Legal transition table covers all states.
- Illegal transition fails with `InvalidTransition`.
- Tool request path emits approval events before tool start when policy asks.
- Denied approval returns either typed denied tool result or terminal run failure by policy.
- Max iterations stops deterministically.
- Cancellation while streaming and cancellation while waiting for approval produce distinct event sequences.

## Contract 7: Provider Adapter And Projection

Provider adapters must not receive lossless internal messages by default.

Projection contract:

- `ContextAssembler` produces `ContextProjection`.
- Projection strips internal metadata unless allowlisted.
- Projection emits `ContextProjectionAudited`.
- Projection records included and omitted counts by kind and reason.
- Tool specs come from `RuntimePackage`, not live discovery.
- Provider response maps back into internal `AgentMessage` with lineage.

Provider stream contract:

- Provider chunks map to typed `StreamDelta`.
- Deltas carry channel, cursor, attempt ID, privacy, and optional content reference.
- Stop reasons are canonicalized.
- Usage is recorded even on failed attempts when provider reports it.
- Provider-native structured output is an optimization only; local validation still runs.

Acceptance tests:

- Message metadata is stripped before provider call.
- Unsupported modality omission is recorded in projection audit.
- Tool spec hash in projection matches package fingerprint.
- Provider text deltas commit only after final message event.
- Usage from failed provider attempt is preserved.

## Contract 8: Message, Context, Privacy, And Metadata

Required message fields:

- ID.
- Role.
- Parts.
- Lineage.
- Sensitivity.
- Retention.
- Projection status.
- Usage and metrics refs.
- Bounded custom metadata.

Required context item fields:

- ID.
- Kind.
- Content or content ref.
- Source and injection path.
- Destination projection.
- Sensitivity.
- Retention.
- Budget class.
- Projection policy.

Metadata rules:

- Keys are namespaced.
- Values are small scalar/map/list data, IDs, hashes, sizes, timestamps, MIME types, or redacted summaries.
- Secrets, credentials, raw file bytes, raw remote messages, raw memory bodies, and auth headers are invalid metadata.
- Unknown metadata is stripped before provider calls unless allowlisted.

Acceptance tests:

- Oversized metadata is rejected or converted to content refs by policy.
- Unknown custom metadata is stripped from provider projection.
- Context item lineage survives compaction.
- Raw content remains absent under default event and telemetry policy.

## Contract 9: Structured Output

When a host provides an `OutputContract`, the SDK owns parsing, validation, repair attempts, and typed result delivery.

Required output contract fields:

- Schema ID/version.
- Schema definition.
- Output mode.
- Local validation policy.
- Repair policy.
- Retry budget.
- Content-capture and redaction policy.
- Schema dialect. Phase 2 starts with JSON Schema 2020-12 subset plus SDK semantic validators; adding another dialect requires a versioned adapter.
- Failure result shape.

Validation rules:

- Provider-native schema support is optional.
- Local validator is authoritative.
- Validation failures emit events and journal records.
- Repair prompt contains schema and redacted validation errors only.
- Repair retry is a new model attempt.
- Final success returns `ValidatedOutput`.
- Exhausted retries return a typed validation error with redacted raw output or content reference.
- Streaming partials are candidates only. The SDK validates after a complete candidate boundary unless the output mode explicitly supports incremental validation.
- Tool-call interaction is explicit: a model tool request pauses structured-output validation until tool results are appended and a final output candidate is produced.
- Semantic validators may reject syntactically valid output, but they must return bounded, redacted error summaries for repair prompts.
- Default retry budget is host-configured and finite; tests should use deterministic budgets such as 0, 1, and 2.

Acceptance tests:

- Valid JSON object returns typed validated result.
- Missing required field triggers repair attempt.
- Invalid enum value triggers repair attempt with redacted error summary.
- Exhausted repair budget returns structured error.
- Provider-native schema response is still locally validated.
- Usage and attempt lineage include original and repair attempts.

## Contract 10: Stream Rules

`StreamRuleEngine` observes typed streaming channels and can request policy-gated interventions.

Supported matchers:

- Literal.
- Regex.
- Semantic marker placeholder for future host-provided strategies.

Supported channels:

- Assistant text.
- Reasoning summary.
- Provider-exposed reasoning.
- Tool-call arguments.
- Tool result text.
- Realtime transcript.

Supported actions:

- Stop run.
- Abort and retry with injected context.
- Pause for approval.
- Emit only.
- Mask and continue.

Safety rules:

- Rules compile during package assembly.
- Invalid regexes fail package validation or become disabled with explicit warning events.
- Matching uses bounded rolling windows.
- Regex timeouts and backtracking protections are required.
- Hidden chain-of-thought is never exposed. Only provider-exposed typed reasoning channels can be observed.
- Raw matched content is redacted by default.
- Repeat state is persisted so resume does not re-trigger incorrectly.
- Regex dialect must be Rust `regex`-compatible by default: no lookaround/backrefs unless a host installs a separate matcher with explicit risk labeling.
- Cursor units are channel-specific and explicit: byte offsets for UTF-8 text buffers, token offsets when provider supplies stable token cursors, frame/sample offsets for realtime media transcripts, and sequence numbers for unknown providers.
- Chunk-boundary matches use the bounded rolling window and must detect patterns split across adjacent chunks.
- Overlapping matches are deduped by rule ID, channel, attempt ID, and match span.
- `MaskAndContinue` applies before live sinks and telemetry export. The journal records redacted match metadata and replacement policy, not raw matched text by default.
- Partial-output retention is an action field: keep partial, discard partial, mask partial, or content-ref partial. Replay uses the recorded choice.
- Provider-exposed reasoning channel eligibility is declared by the provider adapter and content policy; the stream rule engine cannot ask a provider for hidden reasoning.

Acceptance tests:

- Invalid regex fails at package assembly.
- Stop-on-regex over assistant text emits match and stopped result.
- Abort-and-retry cancels current attempt and creates a new attempt.
- Pause-for-approval follows headless dispatcher absence denial rules.
- Mask-and-continue prevents raw matched text from reaching sinks.
- Resume restores once-only repeat state.

## Contract 11: Tools, Tool Packs, And Effects

Tool registry contracts:

- Tool source is typed: SDK built-in, host app, MCP, extension, subagent, external runtime, or test.
- Tool names are canonical and namespace-safe.
- Tool specs include risk, permissions, input schema, output schema, idempotency hints, timeout, cancellation support, and effect class.
- Tool execution produces a canonical `ToolResultEnvelope`.

Tool execution rules:

- Policy classification happens before execution.
- Approval happens through broker when required.
- Tool attempts are evented and journaled.
- Concurrent tool execution has explicit ordering policy.
- Mutating tools append intent and effect metadata before apply when possible.
- Tool result content is redacted by default.

Built-in tool packs:

- `workspace_readonly`: read, list, grep, glob, AST grep, resource read.
- `workspace_edit`: anchored edit, apply patch, structural edit preview/apply.
- `workspace_write`: create/overwrite/archive/structured write.
- `shell`: command, PTY, long-running job, cancellation, stdout/stderr stream.
- `resource_readers`: document, image, archive, SQLite, URL, notebook, artifact readers.
- `tool_discovery`: search and activate hidden tools through package deltas.

Placement rule:

- The core crate owns shared tool contracts, result envelopes, approval/effect semantics, and tiny fake tools for tests.
- Broad read/search/edit/write/shell/resource implementations should live in optional SDK toolkit crates or feature-gated modules. A host chooses which packs enter a runtime package.

Minimum tool-pack semantics:

- Paths are resolved through a workspace policy, not raw model text.
- Symlinks, parent-directory traversal, hidden files, ignored files, and external mounts have explicit policy decisions.
- Search tools define regex dialect, glob rules, gitignore behavior, max matches, max bytes, context-line bounds, and pagination cursor shape.
- Read outputs include stable anchors, source identity, content hash, byte ranges, truncation metadata, MIME type, and sensitivity.
- Edit planning and applying are separate phases unless the host explicitly grants one-step apply.
- Apply records before hash, after hash, diff summary, created/deleted paths, formatter/diagnostic result, and inverse patch candidate when available.
- Write/overwrite has a stricter policy than anchored edit.
- Shell uses structured argv by default. Raw shell-string execution is a distinct high-risk mode with separate approval.

Reversibility boundary:

- SDK records before/after hashes, diffs, idempotency keys, created/deleted paths, and inverse patch candidates when available.
- SDK does not promise every effect can be reversed.
- Evolution, benchmark scoring, proposal ranking, and product undo UX stay host-owned.

Acceptance tests:

- Read/search output includes source path, anchor, hash, truncation, and limits.
- Edit fails on stale anchor unless bounded recovery has a known before-state.
- Write requires explicit policy scope.
- Shell cannot run without sandbox/timeout policy.
- Mutating tool emits before/after effect metadata.
- Tool discovery activation creates a package delta or next-turn snapshot, not ambient mutation.

## Contract 12: Approval, Permission, Sandbox, Escalation, And Autonomy

Policies are separate layers:

- `PermissionPolicy`: capability checks.
- `SandboxPolicy`: environment, cwd, network, filesystem, command limits.
- `ApprovalPolicy`: allow, deny, ask, modify, defer, interrupt.
- `EscalationPolicy`: out-of-band source-scoped dispatch.
- `AutonomyPolicy`: explicit preapproval modes without hiding decisions.

Approval broker lifecycle:

1. Request created.
2. Optional dispatcher selected.
3. Prompt dispatched or unavailable.
4. Decision received, timeout, denied, or cancelled.
5. Loop continues, fails, or records denied tool result by policy.

Edge semantics:

- Headless with no dispatcher denies.
- Source-scoped remote run uses only source-approved channel or configured escalation.
- Dispatcher timeout denies.
- UI dispatcher missing or unhealthy denies unless action is preapproved by policy.
- Extensions cannot approve their own actions.
- Exact finite decision tokens are required for voice/out-of-band approval.
- Autonomy mode still emits policy and approval decisions.

Acceptance tests:

- Headless no dispatcher emits unavailable and denied.
- Timeout emits timeout before denied.
- Extension-submitted action cannot self-approve.
- Voice/out-of-band approval rejects non-canonical decision text.
- Autonomy mode still records risk and approval decision refs.
- Cancellation while pending approval closes request and prevents tool execution.

## Contract 13: Execution Isolation

Isolation is a portable SDK contract with host-provided adapters.

Required concepts:

- `ExecutionEnvironment`
- `EnvironmentSpec`
- `IsolationRuntime`
- `ContainerRuntimeAdapter`
- `ProcessSpec`
- `FilesystemIsolationPolicy`
- `NetworkIsolationPolicy`
- `SecretExposurePolicy`
- `IsolationCapabilityReport`
- `PreparedEnvironment`
- `IsolatedProcess`
- `CleanupMode`

Adapter rules:

- Adapter health and capabilities are checked before use.
- Unsupported host/platform fails with typed diagnostics.
- Image/rootfs/kernel/init readiness is observable.
- Mount expansion is audited.
- Network policy is explicit.
- Secret mounts and environment variables are redacted by default.
- Process I/O captures sizes, hashes, truncation, and redacted summaries by default.
- Cleanup is always attempted and journaled.
- Fallback is policy-driven. If a requested adapter is unavailable, the SDK may choose another declared acceptable adapter only when package policy lists it; otherwise it denies before execution.
- Capability downgrade must be explicit. For example, losing network isolation, mount read-only enforcement, stats collection, or cleanup guarantees changes the environment plan and requires policy approval or denial.
- Registry credentials and image pulls are adapter readiness concerns. They must not be embedded into model-visible context or event metadata.
- Workspace mounts define snapshot vs live mount, read-only vs writable, path expansion, host path privacy, and cleanup expectations.

Recovery rules:

- Prepared environment without process can be cleaned or reused only if adapter declares safety.
- Started process without terminal status requires process reconciliation before retry.
- Missing cleanup result becomes repair-needed or host-action-needed, not silent success.

Acceptance tests:

- Fake adapter capability negotiation denies unsupported image kind.
- Unsupported Apple Containerization host falls back only when policy allows another adapter.
- Single-file mount expansion audit is recorded.
- Network denied policy blocks outbound process request in fake adapter.
- Process timeout sends signal and records terminal status.
- Cleanup failure records repair-needed recovery.

## Contract 14: Subagents

Subagents are parent-owned child runs.

Rules:

- Child agents are not directly user-chatable.
- Child runtime packages do not receive subagent-creation tools by default.
- Parent owns routing, depth budget, context policy, tool policy, approval inheritance, cancellation, timeout, and usage rollup.
- Child events are wrapped into parent event stream with parent and child IDs.
- Child journals are trace/session artifacts, not top-level conversations unless host policy promotes them.

Acceptance tests:

- Depth limit prevents recursive child creation.
- Child package strips subagent tools by default.
- Parent cancellation cancels child run.
- Child usage rolls up to parent while preserving child run ID.
- User cannot address child agent as normal chat without host promotion.

## Contract 15: Extension SDK Bridge

The extension SDK is a layer over core contracts, not the core owner. `agent-sdk-core` sees typed capability refs and hook/tool/provider/subagent ports; the optional `agent-sdk-extension` crate or host adapter owns JSON-RPC process management, app-event fanout, UI surfaces, and host action transport.

Manifest capabilities:

- Tools.
- Hooks.
- Providers.
- Subagents.
- App-event subscriptions.
- Commands.
- UI surfaces.
- Action permissions.

Protocol:

- JSON-RPC 2.0 over NDJSON for extension subprocesses managed outside `agent-sdk-core`.
- Initialize handshake declares protocol version and capabilities.
- Request/response IDs must match.
- Known finite protocol values use typed enums.
- Extension stderr is drained.
- Hook timeouts fail open unless the capability is explicitly blocking.
- Tool/action timeouts fail according to policy.

Packaging compatibility:

- Bun packaged fallback must smoke root, browser-safe helper, and process-only media subpaths from outside the repo before fallback support is documented.
- Node ESM through normal `node_modules` resolution must smoke root, browser-safe helper, and process-only media subpaths.
- Node ESM `NODE_PATH` packaged fallback remains unsupported until a loader/import-map/install strategy is added and verified.
- CommonJS `require` is not a Phase 2 contract unless explicitly added.
- Browser-safe helper subpaths must be explicitly declared. The active generic helper subpath is `@agent-sdk/extension-sdk/browser-safe`.
- Extension-local dependencies win over host fallback.

Acceptance tests:

- Temp-directory Bun fallback import smoke.
- Temp-directory Node ESM normal `node_modules` import smoke.
- Node `NODE_PATH` fallback unsupported smoke remains documented until changed.
- Browser bundle check for the browser-safe helper excludes Node/process/native deps.
- Extension action crosses host approval and cannot self-approve.
- `agent-sdk-core` has no extension runtime, JSON-RPC subprocess, UI surface, or app-event transport imports.

## Contract 16: Telemetry And Cost Accounting

Minimum host-reliable telemetry:

- Every run emits root trace/span, runtime package fingerprint, source surface, terminal status, start/end time, and duration.
- Every model attempt emits provider, model, attempt ID, latency, stop reason, retry classification, token usage, and estimate marker when needed.
- Every tool attempt emits source, canonical name, approval ref, strategy, latency, status, retry count, cancellation, and timeout.
- Every isolated workload emits environment ID, adapter, capability status, image/rootfs refs, resource policy, mount/network policy hash, process latency, exit status, cleanup, and stats when available.
- Usage records are monotonic; corrections append adjustment records.
- Cost records include source, identifier, units, currency, rate table version, estimated vs reported marker, and child-agent rollup.
- Telemetry sink failures do not fail the run.

Acceptance tests:

- Model usage rollup is monotonic across retries.
- Cost correction appends adjustment instead of mutating previous record.
- Child cost rolls up with child run ID retained.
- Telemetry sink failure emits health event and run continues.
- Raw content is not required for cost accounting.

## Contract 17: Security, Privacy, And Redaction

Default captured data:

- IDs.
- Roles.
- Part kinds.
- Sizes.
- MIME types.
- Hashes.
- Token usage.
- Stop reasons.
- Tool names.
- Status.
- Latency.
- Policy decisions.

Default omitted data:

- Raw user text.
- System prompt.
- Model response.
- Hidden reasoning.
- Tool input/output.
- Memory content.
- Remote message content.
- File bytes.
- Environment variable values.
- Credentials and auth headers.

Default metadata limits for Phase 2 tests:

- Custom metadata keys per message/event/context item: 32.
- Key length: 128 UTF-8 bytes.
- Scalar string value length: 2048 UTF-8 bytes.
- Total custom metadata size per item: 16 KiB.
- Event payload summary fields: 8 KiB unless the event kind defines a smaller limit.
- Stream delta live payload budget: host-configured, but tests must prove truncation and redaction at 64 KiB.

Redaction rules:

- Secrets use full-value elision plus stable hash when policy allows correlation.
- File and remote-message content use content refs plus summaries generated by host-approved summarizers.
- Safe summaries must be marked as derived content and carry the policy/summarizer ID.
- Per-sink policy can further redact, but a sink cannot request more raw content than the event/journal policy captured.

Projection audit must include:

- Projection ID.
- Run and turn IDs.
- Provider route and model ID.
- Runtime package fingerprint.
- Tool spec hash.
- Included counts by kind/source.
- Omitted counts by reason.
- Redaction policy ID.
- Content-capture mode.
- Budget limits.
- Media counts.
- Policy refs.
- Provider-facing tool names and executable registry fingerprint.

Acceptance tests:

- Default telemetry has no raw content fields.
- Projection audit includes omitted sensitive item count.
- Opt-in content capture requires scope, retention, sink, redaction policy, and approval marker when required.
- Secret metadata is rejected at boundary validation.

## Scenario Reference 18: Generic Host Integration Examples

The scenario plan should prove the SDK can model demanding host flows without copying product behavior. This section is coverage guidance, not an SDK-core contract.

Required examples:

- Desktop chat run with context, stream, tool approval, journal, and UI event fanout.
- Voice/realtime run with media refs, interruption, tool request, approval, and connection restart event.
- CLI/headless run with source metadata, stdout sink, explicit approval dispatcher, and headless denial behavior.
- Remote channel run with source/destination metadata, output dedupe key, and source-scoped approval.
- App-event capture map showing bounded host display-event semantics as host-owned live events, with SDK event IDs linked but not replacing host UI state.
- Trace mapping showing run, turn, model, tool, approval, subagent, output dispatch, usage, and cost IDs flowing into a durable trace store.
- External-runtime session map showing restore key, prewarm, retirement, compacted replay, and runtime session fingerprint as host adapter state.
- Structured output workflow returning `ValidatedOutput`.
- Shell/code tool routed through fake isolation adapter.
- Stream rule stop/retry/mask/approval example.
- Parent-owned subagent with wrapped child events and usage rollup.
- External runtime adapter smoke with mapped events, but no production runtime replacement.

Acceptance gates:

- Examples are doc or test fixtures with fake adapters first.
- Production host integration is a separate reviewed plan.
- Host dispatch boundaries remain product-owned until replacement is explicitly approved.

## Historical Implementation Workstreams

The current ownership map is [../../workstreams/README.md](../../workstreams/README.md), with workstreams `00` through `11`. The older workstream list below is retained only as background sequencing context and must not override the current workstream files, primitive kernel, or emitted-kind fixture rules.

### Historical Workstream 0: Contract Freeze

Owner: main implementer.

Deliverables:

- Confirm this handoff plan against the latest Phase 1 docs.
- Convert event families, journal records, runtime package fields, and loop states into test names.
- Add a small "Phase 2 contract index" doc if implementation creates more than one contract file.

Exit gate:

- Human-reviewed contract list.
- No code behavior merged yet.

### Historical Workstream 1: Domain IDs, Errors, Events

Deliver:

- Typed IDs.
- `AgentError`.
- `EventEnvelope`.
- `EventFrame`.
- `EventCursor` and `JournalCursor` distinction.
- `AgentEventBus` and runtime subscription helpers.
- `EventFilter` and `CompiledEventFilter`.
- `AgentEvent` enum with Phase 1 event families.
- Redaction/privacy enums.
- Golden event fixtures.

Exit gate:

- Event golden tests pass.
- Redaction tests pass.
- Event subscription/filter tests pass without payload parsing.

### Historical Workstream 2: Journal And Replay Skeleton

Deliver:

- `RunJournal` trait.
- In-memory fake journal.
- Journal record enums.
- Replay modes.
- Seal/terminal-state behavior.

Exit gate:

- Append-order, audit replay, resume replay, and cancel replay tests pass.

### Historical Workstream 3: Runtime Package

Deliver:

- `RuntimePackage` and builder.
- Deterministic fingerprint.
- Tool/provider projection fingerprint invariants.
- Package validation errors.

Exit gate:

- Deterministic fingerprint and projection/execution alignment tests pass.

### Historical Workstream 4: Loop State Machine With Mock Provider

Deliver:

- `Agent`, `AgentBuilder`, `AgentRuntime`, `RunHandle`, `RunRequest`, `RunResult`.
- Explicit state transition table.
- Fake provider adapter with scripted streams.
- Cancellation and max-iteration behavior.

Exit gate:

- Legal/illegal transition tests pass.
- Basic run, retry, cancel, and max-iteration tests pass.

### Historical Workstream 5: Projection, Messages, And Context

Deliver:

- `AgentMessage`, message parts, `MediaRef`.
- `ContextItem`, `ContextAssembler`, `ContextProjection`.
- Projection audit event.
- Metadata limits and stripping.

Exit gate:

- Provider projection privacy tests pass.
- Context lineage and compaction-safe metadata tests pass.

### Historical Workstream 6: Tools And Approval

Deliver:

- `ToolRegistry`, `ToolRouter`, `ToolExecutor`.
- `ToolResultEnvelope`.
- `ApprovalPolicy`, `ApprovalBroker`, dispatcher abstraction.
- Tool attempt journal integration.

Exit gate:

- Approval, timeout, dispatcher absence, cancellation, and extension-submitted denial tests pass with fakes.

### Historical Workstream 7: Structured Output And Stream Rules

Deliver:

- `OutputContract`.
- `StructuredOutputValidator`.
- Validation/repair event path.
- `StreamRule`, `StreamMatcher`, `StreamRuleEngine`, `StreamIntervention`.

Exit gate:

- Structured-output repair tests pass.
- Stream rule stop/retry/mask/approval/resume tests pass.

### Historical Workstream 8: Built-In Tool Packs

Deliver:

- Read/search pack.
- Edit/write pack.
- Shell pack behind fake sandbox/isolation policy.
- Resource URI router.
- Tool discovery index.

Exit gate:

- Tool-pack contract tests pass using mock workspace and fake process runner.

### Historical Workstream 9: Isolation Runtime

Deliver:

- Portable isolation traits and specs.
- Fake isolation adapter.
- Capability negotiation.
- Environment/process lifecycle events.
- Cleanup and recovery records.

Exit gate:

- Fake adapter tests pass for unsupported host, mount expansion, network denial, timeout, cleanup failure, and partial lifecycle recovery.

### Historical Workstream 10: Subagents

Deliver:

- `SubagentSupervisor`.
- Depth budget.
- Child package policy.
- Event wrapping.
- Usage rollup.

Exit gate:

- No recursive tools, parent cancellation, child event wrapping, and usage rollup tests pass.

### Historical Workstream 11: Extension SDK Bridge

Deliver:

- Rust-side extension capability DTOs if needed.
- TypeScript package compatibility smokes.
- Browser-safe helper verification.
- Extension action approval bridge tests.

Exit gate:

- Bun/Node subpath smoke tests pass from temp directories.
- Browser-safe helper smoke passes.
- Extension self-approval remains impossible.

### Historical Workstream 12: Generic Scenario Prototypes

Deliver only after prior gates:

- Host adapter concept prototype with fakes first.
- External runtime adapter prototype with fake runtime events first.
- Generic examples mapping desktop, voice, CLI/headless, remote, and subagent events.

Exit gate:

- No production behavior is changed.
- Adapter prototypes prove event/journal mapping without replacing any product dispatch boundary.

## Validation Plan

Baseline validation for documentation-only changes to this plan:

- `git diff --check`
- Markdown content audit for required contract names.

Baseline validation once implementation starts:

- `cargo test -p agent-sdk-core`
- `cargo check -p agent-sdk-core`
- Package smoke commands for extension SDK subpaths.
- Targeted host integration tests only after a product-specific integration plan is accepted.
- Product-specific verification suites run before any commit that touches production host code.

## Review Gates

Before implementation:

- Independent plan review must pass with no blocking findings.
- Human user review must approve this plan or an edited version.

During implementation:

- Each workstream PR or local slice includes a compact review packet:
  - Objective.
  - Non-goals.
  - Contracts implemented.
  - Invariants.
  - Tests added.
  - Risk notes updated.

After implementation:

- Independent implementation review must validate the diff against this plan, the Phase 1 docs, and `coding-standards.md`.
- Any new future gotcha updates `docs/reference/risks/agent-sdk-phase1-2026-05-21.md` or a Phase 2 risk note.

## Risk And Gotcha Carry-Forward

- Do not let event payload detail explode before the envelope and family/kind names are stable.
- Do not let runtime package assembly reach into live discovery after a run starts.
- Do not let stream rules inspect hidden reasoning or record raw matches by default.
- Do not let built-in tool packs become a coding-agent product.
- Do not overpromise reversibility. Record lineage and inverse candidates, but leave product undo/Evolution outside the SDK.
- Do not make Apple Containerization a hard dependency. It is an adapter candidate behind portable isolation traits.
- Do not let extensions approve themselves or become memory/provider/telemetry owners.
- Do not collapse host conversation IDs, trace IDs, app-event IDs, runtime IDs, and journal IDs into one field.
- Do not route headless/source-scoped approvals to desktop UI as a silent fallback.
- Do not require raw content for replay, telemetry, or cost accounting.

## Done Criteria For Handoff

This plan is ready to hand to a coding agent when:

- The independent review findings have been resolved or explicitly accepted by the user.
- Contract sections are specific enough to create test names without guessing.
- Non-goals and host-owned boundaries are clear.
- Workstreams have exit gates.
- The user has reviewed or edited the plan and explicitly approves implementation.
