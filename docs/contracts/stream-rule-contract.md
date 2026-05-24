# Stream Rule And Realtime Contract

Stream rules let hosts observe typed stream channels and request bounded interventions while output is still being produced. Realtime sessions use the same stream delta, event, journal, policy, and typed-ref spine instead of a provider-specific callback path.

This is a feature-layer contract. It layers over `RuntimePackage` sidecars, `StreamDelta`, `AgentEvent`, `RunJournal`, `PolicyRef`, typed `SourceRef` / `DestinationRef` / `EntityRef` values, `EffectIntent` / `EffectResult`, and provider/realtime ports. It must not create a second run loop, package registry, event stream, journal, policy path, telemetry truth store, or output delivery path.

## External Lessons

- oh-my-pi shows the ergonomic value of stopping or redirecting a model while text is streaming. The SDK should provide this as a small primitive, not a full product rulebook UI.
- Strands bidirectional events show that realtime streams need typed channels and interruption events. The SDK should match on channels, not raw logs.
- Hosts need this for assistant output, provider-exposed reasoning summaries, tool argument streams, tool results, realtime transcripts, and realtime media lifecycle.

## Primitive Boundary

| Primitive | SDK owns | Must not own |
| --- | --- | --- |
| `StreamRuleSidecar` | Declarative rule snapshots, matcher limits, policy refs, source refs, redaction refs, repeat policy, and fingerprint fields inside the effective `RuntimePackage`. | Rule authoring UI, product rulebooks, provider transport internals, or live package mutation. |
| `RealtimeSessionSidecar` | Realtime provider capability refs, media/channel policy refs, restart/backpressure policy refs, close policy, and fingerprint fields inside the effective `RuntimePackage`. | Microphone permission, wake/listening UX, audio rendering, provider wire protocol, or product session caches. |
| `StreamDelta` | One typed provider/tool/realtime increment with channel, direction, cursor, attempt/session refs, privacy, policy refs, and optional content refs. | Full transcript storage, raw media buffering, sink delivery, or durable truth. |
| `StreamRuleEngine` | Compiled rule state, bounded rolling windows, repeat-state restoration, and proposed `StreamIntervention` values. | Provider calls, output sink calls, approval transport, or direct message/context mutation. |
| `StreamIntervention` | Policy-reviewed control proposal with rule refs, action, redacted match metadata, partial-output policy, and effect refs when a side effect is needed. | Silent mutation of messages, tools, provider state, or host displays. |
| `RealtimeProviderAdapter` | Replaceable port for connect, send, receive, restart, interrupt/cancel current response, and close. | Tool execution policy, approval UI, transcript truth, or media rendering. |
| `AgentEvent` / `RunJournal` | Live observation and durable records for stream/realtime state, with journal-backed events after append succeeds. | Slow subscriber delivery, global analytics storage, host app-event retention, or provider-native logs as truth. |

`CapabilityKind::StreamControl` and `CapabilityKind::RealtimeAction` are used only when a rule or realtime action is callable or discoverable. Normal stream/realtime configuration is a typed package sidecar, not a `CapabilitySpec` bag.

## Runtime Package Sidecars

Stream and realtime behavior is resolved before `RunStarted` into the effective `RuntimePackage`. Package validation compiles matchers, validates policy refs and provider capability refs, and includes every execution-affecting field in the runtime-package fingerprint.

```rust
// Non-compiling contract sketch.
pub struct StreamRuleSidecar {
    pub sidecar_id: PackageSidecarId,
    pub source: SourceRef,
    pub rules: Vec<StreamRule>,
    pub default_policy_refs: Vec<PolicyRef>,
    pub redaction_policy_ref: PolicyRef,
    pub content_capture_policy_ref: PolicyRef,
    pub matcher_engine_ref: MatcherEngineRef,
}

pub struct RealtimeSessionSidecar {
    pub sidecar_id: PackageSidecarId,
    pub provider_route_ref: ProviderRouteRef,
    pub realtime_capability_ref: ProviderCapabilityRef,
    pub media_policy_ref: PolicyRef,
    pub send_policy_ref: PolicyRef,
    pub receive_policy_ref: PolicyRef,
    pub restart_policy_ref: PolicyRef,
    pub backpressure_policy_ref: PolicyRef,
    pub interruption_policy_ref: PolicyRef,
    pub close_policy_ref: PolicyRef,
}
```

Fingerprint inputs:

- stream rule ID/version, matcher kind/hash, matcher limits, channel set, action, repeat policy, source refs, privacy policy, redaction/content-capture policy refs, and intervention policy refs;
- realtime provider capability version, realtime action IDs when callable/discoverable, media/channel policy refs, restart/backpressure/interruption/close policy refs, queue/overflow policy, and provider route refs;
- matcher engine refs and host-provided matcher risk policy when a host matcher is active.

Validation failures:

- invalid regex, unknown marker, missing policy ref, missing redaction policy, unsupported provider/realtime capability, unresolved matcher engine, or unsafe host matcher configuration fails package validation before the run starts;
- a host may choose an explicit compatibility mode that disables one invalid rule with `StreamRuleCompileFailed`, but the disabled state, policy ID, and warning event are journaled and fingerprinted;
- runtime package deltas may add or remove rules only for the next turn or next run. An active package snapshot is immutable.

## Stream Delta Contract

All provider, tool, and realtime increments that can be observed by stream rules enter the loop as `StreamDelta`. Provider adapters and realtime adapters map native transport chunks into this canonical shape before the rule engine sees them.

```rust
// Non-compiling contract sketch.
pub struct StreamDelta {
    pub delta_id: StreamDeltaId,
    pub run_id: RunId,
    pub agent_id: AgentId,
    pub turn_id: Option<TurnId>,
    pub attempt_id: Option<AttemptId>,
    pub message_id: Option<MessageId>,
    pub tool_call_id: Option<ToolCallId>,
    pub realtime_session_id: Option<RealtimeSessionId>,
    pub channel: StreamChannel,
    pub direction: Option<StreamDirection>,
    pub cursor: StreamCursor,
    pub source: SourceRef,
    pub destination: Option<DestinationRef>,
    pub policy_refs: Vec<PolicyRef>,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
    pub content_ref: Option<ContentRef>,
    pub redacted_summary: RedactedSummary,
    pub runtime_package_fingerprint: RuntimePackageFingerprint,
}

pub enum StreamChannel {
    AssistantText,
    ReasoningSummary,
    ProviderExposedReasoning,
    ToolCallArguments,
    ToolResultText,
    RealtimeTranscript,
    RealtimeMedia,
}

pub enum StreamDirection {
    InputToProvider,
    OutputFromProvider,
}
```

Hidden chain-of-thought is never observable. Provider-exposed reasoning eligibility is declared by provider adapter capability and content policy. If a provider does not expose reasoning as a typed channel, the rule engine has no reasoning data to inspect.

Raw audio, image, video, file bytes, tool arguments, model text, or transcript bodies are not copied through `StreamDelta` by default. They remain behind `ContentRef` values plus bounded summaries, hashes, sizes, MIME hints, and policy refs.

## Rule Schema

```rust
// Non-compiling contract sketch.
pub struct StreamRule {
    pub id: StreamRuleId,
    pub version: RuleVersion,
    pub source: SourceRef,
    pub matcher: StreamMatcher,
    pub channels: Vec<StreamChannelSelector>,
    pub scope: StreamRuleScope,
    pub action: StreamAction,
    pub repeat: RepeatPolicy,
    pub privacy: MatchPrivacyPolicy,
    pub policy_refs: Vec<PolicyRef>,
}

pub enum StreamMatcher {
    Literal {
        text_hash: ContentHash,
        case_sensitive: bool,
        window_bytes: u64,
    },
    Regex {
        pattern: RegexPattern,
        dialect: RegexDialect,
        window_bytes: u64,
        timeout_ms: u64,
    },
    Marker {
        marker_id: MarkerId,
        marker_version: MarkerVersion,
    },
    HostMatcher {
        matcher_ref: MatcherEngineRef,
        risk_policy_ref: PolicyRef,
    },
}
```

Default regex dialect is Rust `regex`: no lookaround or backrefs. A host can provide another matcher only by declaring the matcher engine ref, risk policy, timeout budget, memory budget, and redaction policy in the package.

## Channels And Cursor Semantics

| Channel | Cursor |
| --- | --- |
| `AssistantText` / `ToolResultText` | UTF-8 byte offset plus chunk sequence. |
| `ReasoningSummary` / `ProviderExposedReasoning` | Provider token offset when stable, otherwise chunk sequence plus byte offset in the exposed summary. |
| `ToolCallArguments` | JSON pointer plus byte offset when streaming JSON is available, otherwise chunk sequence plus redacted argument ref. |
| `RealtimeTranscript` | Realtime segment ID, direction, response ID when available, byte offset, and chunk sequence. |
| `RealtimeMedia` | Media frame/segment ID, direction, content ref version, byte/frame count, and chunk sequence. |
| Unknown provider stream | Chunk sequence only, marked with limited cursor precision. |

Chunk-boundary matches use bounded rolling windows and must detect patterns split across adjacent chunks. Cursor precision is part of the event payload so replay and diagnostics do not pretend byte-accurate recovery when only chunk sequence is available.

## Intervention Actions

```rust
// Non-compiling contract sketch.
pub enum StreamAction {
    StopRun {
        reason: StopReason,
        partial_output: PartialOutputPolicy,
    },
    AbortAndRetry {
        injection: ContextInjectionSpec,
        retry_policy_ref: PolicyRef,
        partial_output: PartialOutputPolicy,
    },
    PauseForApproval {
        approval: ApprovalRequestSpec,
        resume_policy_ref: PolicyRef,
    },
    MaskAndContinue {
        replacement: RedactionReplacement,
    },
    EmitOnly {
        notice_kind: StreamNoticeKind,
    },
}

pub struct StreamIntervention {
    pub intervention_id: StreamInterventionId,
    pub rule_ref: EntityRef,
    pub requested_action: StreamAction,
    pub applied_action: Option<StreamAction>,
    pub match_ref: StreamMatchRef,
    pub redacted_match: RedactedMatch,
    pub partial_output_policy: PartialOutputPolicy,
    pub policy_refs: Vec<PolicyRef>,
    pub effect_intent_ref: Option<EffectId>,
    pub effect_result_ref: Option<EffectId>,
}
```

`StreamIntervention` is a proposal until `PolicyStage::Stream` returns a finite decision. Missing policy refs, missing stream policy evaluator, denied policy, missing approval broker, missing realtime/provider adapter for a required control action, or journal append failure fails closed before the control action takes effect.

Intervention effect mapping:

| Action | Effect mapping |
| --- | --- |
| `StopRun` | Local run-control action with `StreamRuleRecord { intervention_intent }`; provider/realtime cancellation uses the existing provider/realtime cancel record before adapter access. |
| `AbortAndRetry` | `StreamRuleRecord { intervention_intent }` plus model/realtime cancel records, then a new provider request intent using `EffectIntent { kind: ProviderRequest }`; terminal records map to `EffectResult`. |
| `PauseForApproval` | Approval broker path with `ApprovalRecord { dispatch_intent }` / `ApprovalRecord { dispatch_result }` wrapping `EffectIntent { kind: ApprovalDispatch }` and `EffectResult`. |
| `MaskAndContinue` | No external side effect by itself; the masked delta and match record are journaled, and any chunk sink delivery later uses output-delivery `EffectIntent` / `EffectResult`. |
| `EmitOnly` | No external side effect; emits and journals a redacted notice only. |

## Intervention Flow

```mermaid
sequenceDiagram
  participant Port as "Provider/Realtime/Tool port"
  participant Loop
  participant Rules as "StreamRuleEngine"
  participant Policy as "PolicyStage::Stream"
  participant Approval as "ApprovalBroker"
  participant Journal
  participant Sink as "OutputSink / subscriber"

  Port-->>Loop: "StreamDelta"
  Loop->>Journal: "ModelStreamDelta / RealtimeInputSent / RealtimeOutputReceived when durable"
  Loop->>Rules: "observe typed delta"
  alt no match
    Loop-->>Sink: "redacted delta candidate after delivery policy"
  else match
    Rules-->>Loop: "StreamIntervention proposal"
    Loop->>Journal: "StreamRuleMatched"
    Loop->>Policy: "classify requested action"
    alt denied or invalid
      Loop->>Journal: "StreamInterventionFailed"
    else ask
      Loop->>Journal: "StreamInterventionRequested"
      Loop->>Approval: "approval request through broker"
      Approval-->>Loop: "approved / denied / timeout / cancelled"
    else allowed or modified
      Loop->>Journal: "StreamInterventionRequested"
      Loop->>Journal: "StreamInterventionApplied"
      alt StopRun
        Loop->>Port: "cancel current attempt/session through port"
      else AbortAndRetry
        Loop->>Port: "cancel current attempt"
        Loop->>Journal: "ContextContribution from stream_rule"
        Loop->>Port: "new provider/realtime attempt through normal port"
      else MaskAndContinue
        Loop-->>Sink: "masked delta candidate after delivery policy"
      else EmitOnly
        Loop-->>Sink: "redacted notice event"
      end
    end
  end
```

Provider, realtime, and tool adapters do not decide stream policy. They supply typed deltas and execute adapter calls only after the loop has appended the required journal record.

## Context Injection

`AbortAndRetry` injection creates a `ContextContribution` candidate that `ContextAssembler` may admit into a `ContextItem` with:

- source kind `stream_rule`;
- injector kind `policy`;
- role no stronger than the policy allows;
- rule ID/version and package sidecar ref;
- redacted match metadata;
- retention, sensitivity, and destination provider projection policy.

Stream rules must not inject a `ContextItem` directly into provider context. The contribution follows the normal `ContextContribution -> ContextItem -> ContextProjection` path and records any omitted, redacted, or admitted decision. Extensions and user-authored rules cannot create system/developer-equivalent context unless the host grants that authority explicitly. Injection text/templates are policy data, not model output.

## Realtime Lifecycle

Realtime sessions are child artifacts owned by the run until they are closed, cancelled, failed, or explicitly detached under child lifecycle policy. Realtime send and receive halves share one `RealtimeSessionId` but have separate cursors and backpressure state.

```rust
// Non-compiling contract sketch.
pub struct RealtimeSessionState {
    pub session_id: RealtimeSessionId,
    pub connection_id: RealtimeConnectionId,
    pub provider_route_ref: ProviderRouteRef,
    pub send_cursor: StreamCursor,
    pub receive_cursor: StreamCursor,
    pub restart_count: u32,
    pub backpressure_state: BackpressureState,
    pub lifecycle_status: RealtimeLifecycleStatus,
    pub policy_refs: Vec<PolicyRef>,
}
```

Lifecycle rules:

- `RealtimeConnected` is emitted only after the session record is appended and the adapter reports a connection ID.
- `RealtimeInputSent` records content refs, media kind, send cursor, policy refs, and backpressure state. It does not copy raw media by default.
- `RealtimeOutputReceived` records response ID when available, receive cursor, content refs or redacted summaries, and media/transcript channel.
- `RealtimeInterrupted` records the response/interruption ref before the adapter cancels or interrupts current output.
- `RealtimeClosed` records which half closed, terminal status, error ref, and whether close was normal, cancel, provider failure, policy denial, or host-owned detach.
- Manual run cancellation and terminal run cleanup close or cancel active realtime sessions through child lifecycle records before `RunCancelled` or successful `RunCompleted` seals.

Host wake phrase detection, microphone permission, push-to-talk affordances, visual voice activity, transcript rendering, and interruption UX are host-owned. The SDK records typed lifecycle facts and calls the realtime port when policy permits.

## Restart And Backpressure

Restart is a journaled state machine:

1. Policy classifies restart under `restart_policy_ref`.
2. The loop appends `RealtimeRestartRequested`.
3. If allowed, the loop appends `RealtimeRestartStarted` before calling `RealtimeProviderAdapter::restart`.
4. During restart, outbound frames are gated or buffered by the declared backpressure policy. No new outbound media is sent through the old connection.
5. Success appends `RealtimeRestartCompleted` with the new connection/session refs and resumed cursors.
6. Failure appends `RealtimeRestartFailed` with typed error and retry classification.

`RealtimeConnectionRestarted` is a compatibility alias only. New adapters emit the requested/started/completed/failed sequence so observers can tell whether a restart was planned, in progress, successful, or failed.

Backpressure rules:

- Realtime send/receive channels are bounded by package policy. Unbounded buffers are test-only and cannot be the default runtime profile.
- `RealtimeBackpressureApplied` is emitted and journaled when the SDK gates, summarizes, drops a noncritical frame, fails a send, or pauses receive processing due to declared policy.
- Dropping host display frames is host-owned and is not a durable SDK fact unless it affects SDK send/receive state.
- Backpressure never blocks the live `AgentEventBus` hot path. Slow event subscribers use subscription overflow policy from the event schema.
- Media backpressure records content refs, byte/frame counts, hashes, and summaries; raw media is captured only through explicit content-capture policy.

## Completion Semantics

Final visible text is not terminal run completion.

Final visible text means the provider/realtime path has produced a final user-visible candidate such as `ModelMessageCompleted`, a final transcript segment, or a final audio/text content ref. That candidate may be renderable by a host before the run is terminal.

Terminal run completion means `RunHandle::wait()` can resolve and the run journal can seal `RunCompleted`, `RunFailed`, or `RunCancelled`. A streaming or realtime run reaches terminal completion only after:

- the run-scoped event iterator has delivered or made replayable the terminal frame;
- provider/model/realtime attempt state is terminal or safely detached under child lifecycle policy;
- pending stream interventions, approval requests, restart decisions, backpressure gates, and realtime close/cancel records are terminal;
- output publication has passed `PolicyStage::Output`, and required output delivery has completed, deduped, failed according to policy, or entered typed recovery;
- context compaction/projection/session persistence has reached terminal state or typed failure;
- pending approval dispatches and approval broker receivers are resolved, denied, cancelled, timed out, or recorded as recovery-needed;
- `RunResult`, terminal journal records, and the terminal checkpoint are durable.

`RunHandle::wait()` waits for terminal run state, not merely the last user-visible chunk. Reconnect from a stream cursor resumes from run state and journal cursors; it must not create a fresh user turn.

## Policy, Approval, And Hooks

Stream/realtime policy uses the existing finite `PolicyDecision` model:

- `PolicyStage::Stream` gates rule matches, interventions, realtime interruptions, restart, backpressure actions that affect SDK send/receive state, and raw match capture.
- `PolicyStage::Output` gates final visible output publication.
- `PolicyStage::Delivery` gates output sink dispatch for stream chunks and finals.
- Approval pauses use `ApprovalBroker` and host `ApprovalDispatcher` through the approval contract. The SDK supplies structured request data and finite decisions; UI copy and transport are host-owned.
- Hook observers such as `OnModelDelta` and `AfterModelCall` may observe content-ref/redacted-summary views only. Hook responses that mutate behavior must lower into an existing domain operation, package delta, policy decision, or stream intervention record; hooks cannot bypass `RuntimePackage`, policy, journal, events, or approval broker.

Missing required policy, dispatcher, adapter, or journal append fails closed before a stream/realtime side effect. Provider-native guardrails can be useful signals, but SDK/host policy records remain authoritative.

## Safety And Privacy Rules

- Compile rules during package assembly.
- Invalid regex fails package validation or becomes disabled with explicit compatibility warning event.
- Literal, regex, marker, and host matchers have byte, time, and memory budgets.
- Regex matchers use safe dialects, compile-time validation, and timeout/backtracking protection.
- Rolling windows have channel-specific byte budgets and never buffer complete transcripts by default.
- Overlapping matches are deduped by rule ID, rule version, channel, direction, attempt/session ID, cursor span, and match hash.
- Repeat state is persisted in journal/checkpoint and restored on resume.
- `MaskAndContinue` applies before live sinks, output delivery, telemetry export, and journal/export payloads that would otherwise contain the matched text.
- Raw matched text is redacted by default; store hash, length, rule ID/version, channel, cursor, and bounded redacted summary.
- Raw match capture requires explicit content-capture policy, retention policy, sink permission, and event privacy of raw-content-allowed.
- Partial-output handling is explicit: keep, discard, mask, or content-ref.
- Hidden chain-of-thought, provider transport diagnostics, credentials, auth headers, and raw media are never captured by default telemetry.

## Events, Journal, And Telemetry

Every stream-rule and realtime event uses the standard event envelope. Feature IDs stay in `EntityRef` values and payloads unless the events/journal owner promotes them to universal envelope fields.

`stream_rule` emitted kinds:

- `StreamRuleRegistered`
- `StreamRuleCompileFailed`
- `StreamRuleMatched`
- `StreamInterventionRequested`
- `StreamInterventionApplied`
- `StreamInterventionFailed`
- `StreamRuleInjectionAppended`

`realtime` emitted kinds:

- `RealtimeConnected`
- `RealtimeInputSent`
- `RealtimeOutputReceived`
- `RealtimeInterrupted`
- `RealtimeRestartRequested`
- `RealtimeRestartStarted`
- `RealtimeRestartCompleted`
- `RealtimeRestartFailed`
- `RealtimeConnectionRestarted` compatibility alias only
- `RealtimeClosed`
- `RealtimeBackpressureApplied`

Journal records:

- `StreamRuleRecord { registered | compile_failed | matched | intervention_intent | intervention_result | repeat_state }`
- `ContextRecord { stream_rule_context_contribution | stream_rule_context_selected | stream_rule_context_omitted }`
- `ApprovalRecord` for approval pauses
- `ModelAttemptRecord` for provider cancel/retry and provider request intent/result
- `RealtimeSessionRecord { connected | input_sent | output_received | interrupted | restart_requested | restart_started | restart_completed | restart_failed | closed | backpressure_applied }`
- `OutputDeliveryIntentRecord` / `OutputDeliveryResultRecord` when a stream chunk or final output is sent to an output sink
- `RecoveryRecord` when terminal append or adapter reconciliation is unsafe or unknown

Telemetry is derived from journal-backed events and usage records. OTel exporters must not become durable truth and must not receive raw match, transcript, tool, media, or provider content unless content-capture policy allows it.

Recommended SDK-specific telemetry attributes for stitching:

- `agent_sdk.stream.rule.id`
- `agent_sdk.stream.rule.version`
- `agent_sdk.stream.channel`
- `agent_sdk.stream.direction`
- `agent_sdk.stream.action`
- `agent_sdk.stream.match.redaction`
- `agent_sdk.realtime.session.id`
- `agent_sdk.realtime.connection.id`
- `agent_sdk.realtime.media.kind`
- `agent_sdk.realtime.restart.count`
- `agent_sdk.realtime.backpressure.policy`

## Phase 05 Fixture Names

Future implementation must add fixtures before emitting these reserved kinds. This documentation-only phase names the fixtures only; it does not create them.

Stream-rule event fixtures:

- `events/stream_rule_registered_v1.json`
- `events/stream_rule_compile_failed_v1.json`
- `events/stream_rule_matched_v1.json`
- `events/stream_intervention_requested_v1.json`
- `events/stream_intervention_applied_v1.json`
- `events/stream_intervention_failed_v1.json`
- `events/stream_rule_injection_appended_v1.json`

Stream-rule journal fixtures:

- `journal/stream_rule_registered_v1.json`
- `journal/stream_rule_compile_failed_v1.json`
- `journal/stream_rule_match_v1.json`
- `journal/stream_intervention_intent_v1.json`
- `journal/stream_intervention_result_v1.json`
- `journal/stream_rule_repeat_state_v1.json`
- `journal/stream_rule_injection_context_v1.json`

Stream-rule redaction fixtures:

- `redaction/stream_rule_match_default_hash_summary_v1.json`
- `redaction/stream_rule_match_raw_capture_policy_allowed_v1.json`
- `redaction/stream_rule_mask_before_sink_telemetry_journal_v1.json`
- `redaction/provider_hidden_reasoning_not_matchable_v1.json`

Realtime event fixtures:

- `events/realtime_connected_v1.json`
- `events/realtime_input_sent_v1.json`
- `events/realtime_output_received_v1.json`
- `events/realtime_interrupted_v1.json`
- `events/realtime_restart_requested_v1.json`
- `events/realtime_restart_started_v1.json`
- `events/realtime_restart_completed_v1.json`
- `events/realtime_restart_failed_v1.json`
- `events/realtime_connection_restarted_compat_v1.json`
- `events/realtime_closed_v1.json`
- `events/realtime_backpressure_applied_v1.json`

Realtime journal fixtures:

- `journal/realtime_session_connected_v1.json`
- `journal/realtime_input_sent_v1.json`
- `journal/realtime_output_received_v1.json`
- `journal/realtime_interrupted_v1.json`
- `journal/realtime_restart_requested_v1.json`
- `journal/realtime_restart_started_v1.json`
- `journal/realtime_restart_completed_v1.json`
- `journal/realtime_restart_failed_v1.json`
- `journal/realtime_closed_v1.json`
- `journal/realtime_backpressure_applied_v1.json`

Realtime redaction fixtures:

- `redaction/realtime_audio_content_ref_only_v1.json`
- `redaction/realtime_transcript_default_summary_v1.json`
- `redaction/realtime_raw_transcript_policy_allowed_v1.json`

OTel projection fixture names for stitching:

- `otel/stream_rule_intervention_span_v1.json`
- `otel/stream_rule_redaction_attributes_v1.json`
- `otel/realtime_restart_span_v1.json`
- `otel/realtime_backpressure_event_v1.json`
- `otel/realtime_media_content_ref_attributes_v1.json`

## Acceptance Tests

Matcher tests:

- `invalid_regex_fails_package_validation`
- `literal_match_within_bounded_window_is_detected`
- `split_chunk_regex_match_is_detected`
- `marker_match_uses_typed_marker_ref`
- `host_matcher_requires_declared_risk_policy`
- `regex_timeout_or_backtracking_is_protected`
- `overlapping_match_is_deduped`
- `channel_privacy_blocks_raw_match_capture`
- `provider_hidden_reasoning_is_not_matchable`

Intervention tests:

- `stop_on_regex_returns_typed_stopped_result`
- `abort_and_retry_uses_new_attempt_id`
- `abort_and_retry_records_intervention_intent_and_result`
- `stream_rule_injection_is_policy_owned_context_contribution`
- `pause_for_approval_uses_broker_and_headless_denial_rules`
- `approval_dispatch_records_effect_intent_and_result`
- `mask_and_continue_redacts_before_sink_delivery`
- `emit_only_records_redacted_notice_without_control_effect`
- `stream_intervention_policy_denial_fails_closed`
- `intervention_append_failure_prevents_control_action`

Resume and completion tests:

- `resume_restores_repeat_state`
- `resume_restores_realtime_restart_and_backpressure_state`
- `final_visible_text_does_not_resolve_wait_before_terminal_bookkeeping`
- `wait_does_not_resolve_until_stream_intervention_terminal`
- `wait_does_not_resolve_until_realtime_session_closed_or_detached`
- `wait_does_not_resolve_until_event_iterator_terminal_frame_replayable`
- `stream_cursor_reconnect_uses_journal_cursor_without_new_turn`

Realtime tests:

- `realtime_connect_records_session_before_connected_event`
- `realtime_send_receive_halves_have_distinct_cursors`
- `realtime_input_sent_uses_content_refs_for_media`
- `interruption_records_response_id_before_cancelling_output`
- `realtime_restart_gates_outbound_audio_frames`
- `realtime_restart_records_requested_started_completed_in_order`
- `realtime_restart_failure_is_observable_before_retry_policy`
- `realtime_backpressure_applied_is_journal_backed_when_it_affects_sdk_state`
- `realtime_close_records_terminal_status`
- `voice_tool_approval_cannot_use_source_extension_as_authority`

Golden and audit tests:

- `event_golden_payload_exists_for_each_stream_rule_kind`
- `event_golden_payload_exists_for_each_realtime_kind`
- `adapter_emitted_kind_matrix_has_no_stream_realtime_fixture_gaps`
- `stream_rule_redaction_fixtures_exclude_raw_content_by_default`
- `realtime_redaction_fixtures_exclude_raw_media_by_default`
- `stream_rule_helper_lowers_to_explicit_rule`
- `stream_rule_helper_and_explicit_rule_emit_equivalent_events`
- `realtime_helper_lowers_to_realtime_sidecar`
- `stream_rule_change_changes_runtime_package_fingerprint`
- `realtime_policy_change_changes_runtime_package_fingerprint`

## Ergonomics

Simple API:

```rust
// Non-compiling contract sketch.
let rule = StreamRule::mask_regex("stop_on_secret", r"sk-[A-Za-z0-9]{20,}")
    .on(StreamChannel::AssistantText)
    .on(StreamChannel::ToolResultText)
    .policy(PolicyRef::new("policy.stream.mask_secret"))
    .build()?;
```

Advanced API:

```rust
// Non-compiling contract sketch.
let rule = StreamRuleBuilder::new(StreamRuleId::new("stop_on_secret"))
    .source(SourceRef::host_policy("host.stream_rules"))
    .matcher(StreamMatcher::regex_with_limits(r"sk-[A-Za-z0-9]{20,}", 4096, 25))
    .channels(vec![StreamChannelSelector::channel(StreamChannel::AssistantText)])
    .action(StreamAction::MaskAndContinue { replacement: "[redacted]".into() })
    .repeat(RepeatPolicy::OncePerAttemptAndSpan)
    .privacy(MatchPrivacyPolicy::HashLengthAndSummary)
    .policy_refs(vec![PolicyRef::new("policy.stream.mask_secret")])
    .build()?;
```

Canonical lowering:

- Helper constructors create `StreamRuleBuilder`.
- Builder emits the same `StreamRule` struct shown above.
- `RuntimePackageBuilder::stream_rule(...)` installs a `StreamRuleSidecar`, compiles the matcher, validates policy refs, and stores repeat/privacy/action fields in the package fingerprint.
- Realtime helpers lower into `RealtimeSessionSidecar` fields, provider capability refs, and policy refs before `AgentRuntime::start_run`.

Equivalence:

- Helper and advanced rule paths emit the same stream-rule events and journal records.
- Helper and advanced realtime paths emit the same realtime lifecycle events and journal records.
- Both paths use the same channel/cursor semantics, redaction defaults, policy checks, event fixtures, journal fixtures, and telemetry projections.

SDK owns / Host owns:

- SDK owns helper lowering, built-in matcher limits, event/journal equivalence, runtime-package sidecars, typed refs, completion gating, and privacy defaults.
- Host owns custom matcher engines, rule authoring UI, realtime UX, media rendering, provider credentials, and permission to grant stronger interventions.

Tests:

- `stream_rule_helper_lowers_to_explicit_rule`
- `stream_rule_helper_and_explicit_rule_emit_equivalent_events`
- `realtime_helper_lowers_to_realtime_sidecar`
- `mask_and_continue_redacts_before_sink_delivery`

## Complete Example

Typed shape:

```rust
// Non-compiling contract sketch.
let rule = StreamRule {
    id: StreamRuleId::new("stop_on_secret"),
    version: RuleVersion::new(1),
    source: SourceRef::host_policy("host.stream_rules"),
    matcher: StreamMatcher::Regex {
        pattern: RegexPattern::new("sk-[A-Za-z0-9]{20,}")?,
        dialect: RegexDialect::RustRegex,
        window_bytes: 4096,
        timeout_ms: 25,
    },
    channels: vec![StreamChannelSelector::channel(StreamChannel::AssistantText)],
    scope: StreamRuleScope::Run,
    action: StreamAction::MaskAndContinue { replacement: "[redacted]".into() },
    repeat: RepeatPolicy::OncePerAttemptAndSpan,
    privacy: MatchPrivacyPolicy::HashLengthAndSummary,
    policy_refs: vec![PolicyRef::new("policy.stream.mask_secret")],
};

let realtime = RealtimeSessionSidecar {
    sidecar_id: PackageSidecarId::new("realtime.voice.default"),
    provider_route_ref,
    realtime_capability_ref,
    media_policy_ref: PolicyRef::new("policy.realtime.media_refs_only"),
    send_policy_ref: PolicyRef::new("policy.realtime.send"),
    receive_policy_ref: PolicyRef::new("policy.realtime.receive"),
    restart_policy_ref: PolicyRef::new("policy.realtime.restart_once"),
    backpressure_policy_ref: PolicyRef::new("policy.realtime.bounded"),
    interruption_policy_ref: PolicyRef::new("policy.realtime.interrupt"),
    close_policy_ref: PolicyRef::new("policy.realtime.close"),
};
```

Replaceable ports:

- `StreamMatcherEngine` can be the built-in literal/regex/marker engine or a host-registered matcher with declared risk.
- `StreamInterventionPolicy` decides whether requested actions are allowed, modified, denied, deferred, or approval-gated.
- `ProviderAdapter` and `RealtimeProviderAdapter` map native chunks to `StreamDelta` and execute connect/send/receive/restart/close only after journal and policy gates.
- `OutputSink` delivery remains the output-delivery contract. Stream rules may mask or suppress candidates before they become delivery requests, but they do not own sink transport.

Wiring:

1. Host resolves rules, realtime policy, provider route, output delivery, and approval policy into one effective `RuntimePackage`.
2. Package validation compiles rules, validates realtime capability refs, and fingerprints sidecars.
3. Provider/tool/realtime stream deltas enter `StreamRuleEngine` with typed channel, direction, cursor, content refs, and package fingerprint.
4. Rule match appends `StreamRuleMatched` with redacted metadata.
5. `PolicyStage::Stream` applies mask/stop/retry/approval/emit-only or realtime interruption/restart decisions.
6. Side-effecting actions append intent records before provider/realtime/approval/output-sink access and terminal result records after.
7. Repeat state, realtime cursors, restart state, backpressure state, pending approvals, and output delivery state are checkpointed for resume.
8. `RunCompleted` seals only after final output state, stream/realtime bookkeeping, session/output/approval bookkeeping, event iterator terminal frame, and terminal journal records are complete.

Events:

- stream rules: `StreamRuleRegistered`, `StreamRuleCompileFailed`, `StreamRuleMatched`, `StreamInterventionRequested`, `StreamInterventionApplied`, `StreamInterventionFailed`, `StreamRuleInjectionAppended`
- realtime: `RealtimeConnected`, `RealtimeInputSent`, `RealtimeOutputReceived`, `RealtimeInterrupted`, `RealtimeRestartRequested`, `RealtimeRestartStarted`, `RealtimeRestartCompleted`, `RealtimeRestartFailed`, `RealtimeClosed`, `RealtimeBackpressureApplied`
- related shared events: `ApprovalRequested`, `ApprovalResponded`, `ApprovalTimedOut`, `ApprovalDenied`, `ModelAttemptCancelled`, `ModelAttemptRetried`, `OutputDispatchRequested`, `OutputDispatchCompleted`, terminal `RunCompleted` / `RunFailed` / `RunCancelled`

Journal:

- `RunRecord { runtime_package_fingerprint }`
- `StreamRuleRecord { registered | compile_failed | matched | intervention_intent | intervention_result | repeat_state }`
- `RealtimeSessionRecord { connected | input_sent | output_received | interrupted | restart_* | closed | backpressure_applied }`
- `ContextRecord { stream_rule_context_contribution }` for abort-and-retry injection
- `ApprovalRecord` for approval pause
- `ModelAttemptRecord` for cancel/retry/provider request intent
- `OutputDeliveryIntentRecord` / `OutputDeliveryResultRecord` for required chunk or final delivery
- `RecoveryRecord` for unsafe pending adapter or append state

Policies and failures:

- Invalid regex fails package validation or is disabled with explicit compatibility warning.
- Hidden reasoning is not a matchable channel.
- `MaskAndContinue` applies before UI, telemetry, journal export, and output sink delivery.
- Extension-authored stop/retry/injection/mask/approval pause requires host policy.
- Missing policy, broker, provider/realtime adapter, sink, or required journal append fails closed before side effects.
- Restart failure is observable before retry policy decides whether to continue, fail, or ask for host action.

SDK owns / Host owns:

- SDK owns channel/cursor semantics, matcher contract, package sidecars, intervention events, repeat state, realtime lifecycle records, restart/backpressure records, typed refs, completion gating, and privacy defaults.
- Host owns provider credentials, provider transport internals, user rule UI, realtime interruption UX, microphone/wake/rendering surfaces, custom matcher sandbox, source-scoped approval transport, output sink implementation, and any durable stores outside the run journal.

Tests:

- `split_chunk_regex_match_is_detected`
- `mask_and_continue_redacts_before_sink_delivery`
- `extension_stream_rule_injection_requires_host_policy`
- `realtime_restart_records_requested_started_completed_in_order`
- `realtime_backpressure_applied_is_journal_backed_when_it_affects_sdk_state`
- `final_visible_text_does_not_resolve_wait_before_terminal_bookkeeping`
