# Agent SDK Examples

These examples show how the contract docs handle complex host workflows without making any product host part of SDK core. They are conceptual and documentation-only.

Each example names:

- SDK IDs.
- host adapter IDs where relevant.
- event families.
- hook points where relevant.
- journal records.
- output sinks.
- host-owned boundaries.
- acceptance tests.

## Examples

| Example | Purpose |
| --- | --- |
| [live-vs-durable-event-flow.md](live-vs-durable-event-flow.md) | Separates SDK live events, host display events, run journal, telemetry, and durable trace stores. |
| [desktop-chat-tool-approval.md](desktop-chat-tool-approval.md) | Desktop chat with context projection, model streaming, tool approval, journal, and UI fanout. |
| [realtime-voice-workflow.md](realtime-voice-workflow.md) | Wake phrase, realtime send/receive halves, interruptions, voice approval, restart gating, and UI effects. |
| [remote-headless-approval.md](remote-headless-approval.md) | Remote-channel run with source-scoped output and explicit headless approval behavior. |
| [external-runtime-session-lifecycle.md](external-runtime-session-lifecycle.md) | External runtime prewarm, restore keys, runtime session IDs, fingerprint invalidation, compaction retirement, surface close, and host shutdown. |
| [subagent-supervision-workflow.md](subagent-supervision-workflow.md) | Parent-owned subagents, package stripping, route validation, event wrapping, telemetry rollup, and read-only host display. |
| [tool-pack-isolation-anti-entropy.md](tool-pack-isolation-anti-entropy.md) | Read/search/edit/write/shell tool packs, hooks, isolation, approvals, side-effect intent, long-running process detach, telemetry, and anti-entropy repair. |
| [memory-context-compaction.md](memory-context-compaction.md) | Memory/context injection, projection audits, compaction, protected context, and resume after context pressure. |
| [structured-output-stream-rules.md](structured-output-stream-rules.md) | Typed output validation, repair, output delivery, and stream-rule intervention over the same run loop. |
| [extension-action-boundary.md](extension-action-boundary.md) | Extension capability resolution, action approval, app-event observation, and host-manifest/runtime boundaries. |

## Scenario Coverage Matrix

| Scenario | Example | SDK primitives | Host-owned boundaries | Events, journals, telemetry, recovery | Validation |
| --- | --- | --- | --- | --- | --- |
| Desktop or web chat with approval | [desktop-chat-tool-approval.md](desktop-chat-tool-approval.md) | `AgentRuntime`, `RunRequest`, `ContextAssembler`, `ProviderAdapter`, `ApprovalBroker`, `ToolExecutor`, `OutputSink`, `RuntimePackage` | chat UI, prompt copy, conversation store, approval transport | run/turn/message/model/tool/approval/output events; `RunRecord`, `ContextRecord`, `ModelAttemptRecord`, `ToolRecord`, `ApprovalRecord`, `OutputDispatchRecord`, `TelemetryRecord`; approval/redaction policy; journal replay and output dedupe | approval-before-tool, projection-before-provider, output dispatch dedupe |
| CLI and headless approval | [remote-headless-approval.md](remote-headless-approval.md) | `SourceRef`, `DestinationRef`, `EscalationPolicy`, `ApprovalBroker`, `ToolExecutor`, `OutputSink` | terminal prompt, scheduler, escalation manager, remote credentials | approval unavailable/denied events; tool denial result; `ApprovalRecord`, `ToolRecord`, `OutputDispatchRecord`; policy denies by default; recovery uses dedupe and journal replay | no dispatcher denies, source-scoped approval, headless compatibility explicit |
| Remote channel input/output | [remote-headless-approval.md](remote-headless-approval.md) | `RemoteChannelAdapter`, `RunRequest`, `DestinationRef`, `OutputDeliveryPolicy`, `DedupeKey` | channel transport, message DB, ack lookup, retry scheduler | `OutputDispatchRequested`, `OutputDispatchCompleted`, `OutputDispatchFailed`; `OutputDispatchRecord`; delivery policy; repair replay never resends without dedupe | remote output uses dedupe before resend |
| Realtime voice or streaming input | [realtime-voice-workflow.md](realtime-voice-workflow.md) | `RealtimeSessionSidecar`, `RealtimeProviderAdapter`, `StreamDelta`, `RealtimeSessionRecord`, `ApprovalBroker`, `ContentRef` | microphone permission, wake/listening UX, audio rendering, exact approval token transport | realtime lifecycle/interruption/restart/backpressure events; `RealtimeSessionRecord`, tool/approval records, telemetry latency/usage/cost; restart and close recovery | restart gates outbound frames, interruption records response ID |
| External runtime sessions | [external-runtime-session-lifecycle.md](external-runtime-session-lifecycle.md) | `ExternalAgentAdapter`, `RuntimePackageFingerprint`, `AgentEvent`, `RunJournal`, `TelemetrySink` | process/session cache, restore keys, install/bootstrap, runtime cleanup, surface retry UX | mapped run/model/recovery events; `RunRecord`, `ModelAttemptRecord`, `RecoveryRecord`; fingerprint policy; trace export and retirement recovery | fingerprint mismatch retires runtime, prewarm failure nonfatal |
| Live events, durable journal, and trace export | [live-vs-durable-event-flow.md](live-vs-durable-event-flow.md) | `AgentEventBus`, `EventFrame`, `EventCursor`, `RunJournal`, `TelemetrySink`, optional `EventArchive` | display event bridge/store, trace store, UI selectors, archive implementation | any emitted family; journal cursor as durable truth; `TelemetryRecord`; replay/repair events; overflow policy preserves terminal facts | display loss does not affect journal or trace |
| Structured output and output delivery | [structured-output-stream-rules.md](structured-output-stream-rules.md) | `OutputContract`, `StructuredOutputValidator`, `ValidatedOutput`, `OutputSink`, `EffectIntent`, `EffectResult` | form rendering, business scoring, channel copy, sink credentials | structured-output and output-delivery events; `StructuredOutputRecord`, `OutputDispatchRecord`, `TelemetryRecord`; repair policy; recovery blocks publication until validation and delivery records are durable | validation before publication, repair bounded, delivery intent/result |
| Memory, context, and compaction | [memory-context-compaction.md](memory-context-compaction.md) | `ContextContribution`, `ContextAssembler`, `ContextProjection`, `ContextProjectionAudit`, `MemoryPort`, `CheckpointStore` | memory backend, browsing UI, ingestion product, extension proposal source | memory/context and compaction events; `ContextRecord`, checkpoint records, telemetry/cost where exported; projection/redaction policy; replay uses content-ref manifest | protected context preserved, extension proposals cannot bypass admission |
| Tool packs, hooks, isolation, and repair | [tool-pack-isolation-anti-entropy.md](tool-pack-isolation-anti-entropy.md) | `ToolPack`, `CapabilitySpec`, `HookSpec`, `IsolationRuntime`, `EffectIntent`, `EffectResult`, `AntiEntropyJob` | installed tool packs, workspace policy, approval UI, concrete runtime, repair scheduler | tool/hook/isolation/recovery/telemetry events; `ToolRecord`, `HookRecord`, `IsolationRecord`, `RecoveryRecord`, `TelemetryRecord`; side-effect and isolation policy; anti-entropy repair | stale anchor denied, no host-process downgrade, terminal append recovery |
| Long-running detach and child artifacts | [tool-pack-isolation-anti-entropy.md](tool-pack-isolation-anti-entropy.md) | `RunChildLifecyclePolicy`, `ChildLifecycleRecord`, `ProcessSpec`, `IsolationRuntime` | process owner, reclaim job, process inspector UI | `ChildLifecycleDetachRequested`, `ChildLifecycleDetachAcknowledged`, `ChildLifecycleDetached`, reclaim/failure events; detach/reclaim records; policy requires acknowledgement; recovery preserves reclaim ticket | implicit orphan denied, explicit detach journaled |
| Stream rules and realtime safeguards | [structured-output-stream-rules.md](structured-output-stream-rules.md), [realtime-voice-workflow.md](realtime-voice-workflow.md) | `StreamRuleSidecar`, `StreamRuleEngine`, `StreamDelta`, `StreamIntervention`, `PolicyStage::Stream` | rule authoring UI, custom matcher sandbox, provider transport internals | stream-rule match/intervention/injection events; `StreamRuleRecord`, approval/provider/output records when actions side-effect; OTel projection redacted by default; repeat-state replay | split-chunk match, mask before sink, no hidden reasoning match |
| Subagent supervision | [subagent-supervision-workflow.md](subagent-supervision-workflow.md) | `AgentPool`, `RunMessage`, `WakeCondition`, `SubagentSupervisor`, `SubagentRequest`, stripped child `RuntimePackage`, child `RunJournal`, `ContextHandoffPolicy` | inspector UI, promotion to conversation, detached-child dashboard, rate tables | agent-pool, subagent, and child-lifecycle events; pool/message/wake records, subagent records, child journal refs, usage/cost rollup; handoff/message/wake policy; cancellation/detach recovery | child package strips recursive tools, usage rolls up once |
| Extension capability and action boundary | [extension-action-boundary.md](extension-action-boundary.md) | `CoreExtensionCapabilities`, `CapabilityCatalogSnapshot`, extension action sidecars, `ApprovalBroker`, `EffectIntent`, `EffectResult` | host manifest, runtime/install/marketplace, trust/action permission state, app-event transport, browser-safe packaging | extension capability/action/app-event events; catalog sidecar records, approval records, extension action effect records, recovery records; no self-approval; OTel redacts host manifest fields | host manifest never becomes core authority, action records intent/result |

## Example Rules

- Examples are not implementation promises by themselves. Contracts own the rules.
- If an example contradicts a contract, fix the contract or the example before coding.
- Host-owned surfaces are drawn explicitly so the SDK stays smaller than any product built on top.
- Every scenario must name SDK primitives, host-owned boundaries, event/journal records, policy decisions, telemetry/cost behavior, recovery behavior, and future validation proof.
