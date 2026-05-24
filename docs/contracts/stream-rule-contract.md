# Stream Rule Contract

Stream rules let hosts observe typed stream channels and request bounded interventions while output is still being produced.

## External Lessons

- oh-my-pi shows the ergonomic value of stopping or redirecting a model while text is streaming. The SDK should provide this as a small primitive, not a full product rulebook UI.
- Strands bidirectional events show that realtime streams need typed channels and interruption events. The SDK should match on channels, not raw logs.
- Hosts need this for assistant output, provider-exposed reasoning summaries, tool argument streams, tool results, and voice transcripts.

## Rule Schema

```rust
// Non-compiling contract sketch.
pub struct StreamRule {
    pub id: StreamRuleId,
    pub version: RuleVersion,
    pub matcher: StreamMatcher,
    pub channels: Vec<StreamChannel>,
    pub scope: StreamRuleScope,
    pub action: StreamAction,
    pub repeat: RepeatPolicy,
    pub privacy: MatchPrivacyPolicy,
}
```

Default regex dialect is Rust `regex`: no lookaround or backrefs. A host can provide another matcher only by declaring the risk and policy.

## Channels

- `AssistantText`
- `ReasoningSummary`
- `ProviderExposedReasoning`
- `ToolCallArguments`
- `ToolResultText`
- `RealtimeTranscript`

Hidden chain-of-thought is never observable. Provider-exposed reasoning eligibility is declared by provider adapter capability and content policy.

## Cursor Semantics

| Channel | Cursor |
| --- | --- |
| Assistant/tool text | UTF-8 byte offset plus chunk sequence |
| Provider token stream | provider token offset when stable, otherwise chunk sequence |
| Tool arguments | JSON pointer plus byte offset when streaming JSON is available |
| Realtime transcript | segment ID plus byte offset |
| Unknown provider stream | chunk sequence only |

Chunk-boundary matches use bounded rolling windows and must detect patterns split across adjacent chunks.

## Completion Semantics

Final visible text is not terminal run completion. A streaming or realtime run completes only after:

- the event iterator has delivered or made replayable the terminal frame;
- provider/model attempt state is terminal;
- pending stream interventions, approvals, output delivery, context compaction, and session/journal persistence have reached terminal state or a typed failure;
- `RunResult` and terminal journal records are durable.

`RunHandle::wait()` waits for terminal run state, not merely the last user-visible chunk. Reconnect from a stream cursor must resume from run state and journal cursors, not create a fresh user turn.

## Intervention Flow

```mermaid
sequenceDiagram
  participant Provider
  participant Loop
  participant Rules as "StreamRuleEngine"
  participant Approval as "ApprovalBroker"
  participant Journal
  participant Sink

  Provider-->>Loop: "StreamDelta"
  Loop->>Rules: "observe typed delta"
  alt no match
    Loop-->>Sink: "delta"
  else match
    Rules-->>Loop: "StreamIntervention"
    Loop->>Journal: "StreamRuleMatched"
    alt StopRun
      Loop->>Journal: "StreamInterventionApplied"
      Loop-->>Sink: "stopped result"
    else AbortAndRetry
      Loop->>Provider: "cancel attempt"
      Loop->>Journal: "ModelAttemptCancelled + injected context"
      Loop->>Provider: "new attempt"
    else PauseForApproval
      Loop->>Approval: "approval request"
    else MaskAndContinue
      Loop-->>Sink: "masked delta"
    else EmitOnly
      Loop-->>Sink: "notice event"
    end
  end
```

## Safety Rules

- Compile rules during package assembly.
- Invalid regex fails package validation or becomes disabled with explicit warning event.
- Rolling windows have byte budgets and timeout budgets.
- Overlapping matches are deduped by rule ID, channel, attempt ID, and match span.
- Repeat state is persisted in journal/checkpoint.
- `MaskAndContinue` applies before live sinks and telemetry export.
- Raw matched text is redacted by default; store hash, length, rule ID, and redacted summary.
- Partial-output handling is explicit: keep, discard, mask, or content-ref.

## Stream Intervention Authority And Context Injection

Stream interventions are policy-owned control actions. A rule can request an action, but policy decides whether it is allowed.

Rule authors:

| Author | Allowed by default | Requires host policy |
| --- | --- | --- |
| SDK built-in | emit-only, mask common secret patterns | stop, retry, approval |
| Host runtime policy | all actions inside declared scope | raw match capture |
| User workflow | stop, emit-only inside that run | retry injection, approval pause |
| Extension | emit-only | stop, retry injection, mask, approval pause |
| Skill/plugin | emit-only | all control actions |

`AbortAndRetry` injection creates a `ContextContribution` candidate that `ContextAssembler` may admit into a `ContextItem` with:

- source kind `stream_rule`
- injector kind `policy`
- role no stronger than the policy allows
- rule ID/version
- redacted match metadata
- retention and sensitivity

Stream rules must not inject a `ContextItem` directly into provider context. The contribution follows the normal `ContextContribution -> ContextItem -> ContextProjection` path and records any omitted, redacted, or admitted decision.
- destination provider projection policy

Extensions and user-authored rules cannot create system/developer-equivalent context unless the host grants that authority explicitly. Injection text/templates are policy data, not model output.

## Acceptance Tests

- `invalid_regex_fails_package_validation`
- `split_chunk_regex_match_is_detected`
- `overlapping_match_is_deduped`
- `stop_on_regex_returns_typed_stopped_result`
- `abort_and_retry_uses_new_attempt_id`
- `pause_for_approval_uses_broker_and_headless_denial_rules`
- `mask_and_continue_redacts_before_sink_delivery`
- `resume_restores_repeat_state`
- `provider_hidden_reasoning_is_not_matchable`
- `extension_stream_rule_injection_requires_host_policy`
- `stream_rule_injection_is_policy_owned_context_item`
- `mask_applies_before_ui_telemetry_journal_export`
- `stream_rule_helper_lowers_to_explicit_rule`
- `stream_rule_helper_and_explicit_rule_emit_equivalent_events`

## Ergonomics

Simple API:

```rust
// Non-compiling contract sketch.
let rule = StreamRule::mask_regex("stop_on_secret", r"sk-[A-Za-z0-9]{20,}")
    .on(StreamChannel::AssistantText)
    .on(StreamChannel::ToolResultText)
    .build()?;
```

Advanced API:

```rust
// Non-compiling contract sketch.
let rule = StreamRuleBuilder::new(StreamRuleId::new("stop_on_secret"))
    .matcher(StreamMatcher::regex_with_limits(r"sk-[A-Za-z0-9]{20,}", 4096, 25))
    .channels(vec![StreamChannel::AssistantText, StreamChannel::ToolResultText])
    .action(StreamAction::MaskAndContinue { replacement: "[redacted]".into() })
    .repeat(RepeatPolicy::OncePerAttemptAndSpan)
    .privacy(MatchPrivacyPolicy::HashLengthAndSummary)
    .build()?;
```

Canonical lowering:

- Helper constructors create `StreamRuleBuilder`.
- Builder emits the same `StreamRule` struct shown below.
- Runtime package validation compiles the matcher and stores repeat/privacy policy in the package fingerprint.

Equivalence:

- Helper and advanced rule paths emit the same stream-rule events and journal records.
- Both paths use the same channel/cursor semantics, redaction defaults, and intervention policy checks.

SDK owns / Host owns:

- SDK owns helper lowering, built-in matcher limits, event/journal equivalence, and privacy defaults.
- Host owns custom matcher engines, rule authoring UI, and permission to grant stronger interventions.

Tests:

- `stream_rule_helper_lowers_to_explicit_rule`
- `stream_rule_helper_and_explicit_rule_emit_equivalent_events`
- `mask_and_continue_redacts_before_sink_delivery`

## Complete Example

Typed shape:

```rust
// Non-compiling contract sketch.
let rule = StreamRule {
    id: StreamRuleId::new("stop_on_secret"),
    version: RuleVersion::new(1),
    matcher: StreamMatcher::Regex {
        pattern: RegexPattern::new("sk-[A-Za-z0-9]{20,}")?,
        dialect: RegexDialect::RustRegex,
        window_bytes: 4096,
        timeout_ms: 25,
    },
    channels: vec![StreamChannel::AssistantText, StreamChannel::ToolResultText],
    scope: StreamRuleScope::Run,
    action: StreamAction::MaskAndContinue { replacement: "[redacted]".into() },
    repeat: RepeatPolicy::OncePerAttemptAndSpan,
    privacy: MatchPrivacyPolicy::HashLengthAndSummary,
};
```

Replaceable ports:

- `StreamMatcherEngine` can be the built-in literal/regex engine or a host-registered matcher with declared risk.
- `StreamInterventionPolicy` decides whether requested actions are allowed.
- `StreamSink` delivery can be UI, CLI, realtime, telemetry, or journal projection.

Wiring:

1. Rule compiles during runtime package build.
2. Provider/tool/realtime stream deltas enter `StreamRuleEngine` with typed channel and cursor.
3. Rule match emits redacted metadata.
4. Policy applies mask/stop/retry/approval/emit-only action.
5. Repeat state is checkpointed for resume.

Events:

- `StreamRuleRegistered`
- `StreamRuleMatched`
- `StreamInterventionRequested`
- `StreamInterventionApplied`
- `StreamRuleInjectionAppended` for retry injection

Journal:

- `StreamRuleRecord { compiled }`
- `StreamRuleRecord { match cursor, redacted match, action }`
- `ContextRecord { injected stream-rule context }` for abort-and-retry

Policies and failures:

- Invalid regex fails package validation or is disabled with explicit warning.
- Hidden reasoning is not a matchable channel.
- `MaskAndContinue` applies before UI, telemetry, and journal export.
- Extension-authored stop/retry/injection requires host policy.

SDK owns / Host owns:

- SDK owns channel/cursor semantics, matcher contract, intervention events, repeat state, and privacy defaults.
- Host owns user rule UI, policy granting stronger intervention rights, and any custom matcher sandbox.

Tests:

- `split_chunk_regex_match_is_detected`
- `mask_and_continue_redacts_before_sink_delivery`
- `extension_stream_rule_injection_requires_host_policy`
