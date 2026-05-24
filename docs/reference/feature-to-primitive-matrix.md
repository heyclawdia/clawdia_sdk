# Feature To Primitive Matrix

This matrix is the Phase 00 artifact that prevents each workstream from building a private SDK. Every feature must map to shared kernel primitives first, then to a feature-layer primitive, optional adapter, or host-owned boundary.

## Primitive Categories

| Category | Meaning | Examples |
| --- | --- | --- |
| Kernel primitive | Required for P0 fake-provider text run and P1 typed-output run. | `Agent`, `RunRequest`, `RuntimePackage`, `AgentEvent`, `RunJournal`, `ContextProjection`, `OutputContract`, `PolicyRef`, typed IDs. |
| Feature-layer primitive | Adds behavior over the kernel without creating a second run loop, package registry, event stream, journal, policy path, context path, or side-effect path. | `ToolPack`, `StreamRule`, `ExecutionEnvironment`, `SubagentRequest`, `CoreExtensionCapabilities`, `TelemetrySinkSpec`. |
| Optional adapter | Replaceable implementation behind a typed port. | Provider client, memory backend, isolation runtime, extension bridge, OTel exporter, output sink. |
| Host-owned behavior | Product workflow outside the SDK. | UI, credentials, marketplace, approval copy, dashboards, trace store, conversation store, channel UX. |

## Matrix

| Feature | Kernel primitives reused | Feature-layer primitives | Optional adapter / host boundary | Owner | Validation focus |
| --- | --- | --- | --- | --- | --- |
| Basic run | `Agent`, `AgentRuntime`, `RunRequest`, `RunHandle`, `RunResult`, typed IDs | none | provider adapter is replaceable; host owns UI | [01 Core API](../workstreams/_roles/01-core-api-runtime.md) | fake-provider run, cancel, status, final result, no product imports |
| Runtime package | `RuntimePackage`, `PolicyRef`, `SourceRef`, `DestinationRef`, fingerprint | `CapabilitySpec`, typed package sidecars, catalog snapshot | host owns installed capabilities and credentials | [00 Integration](../workstreams/_roles/00-integration-stitching.md) | deterministic fingerprint, no parallel registries |
| Provider projection | `AgentMessage`, `ContextProjection`, `RuntimePackage`, `ProviderAdapter` | provider projection sidecar | provider transport is adapter-owned | [01 Core API](../workstreams/_roles/01-core-api-runtime.md), [03 Context](../workstreams/_roles/03-context-structured-output.md) | projected request derives from canonical messages/context |
| Context and memory | `AgentMessage`, `ContextContribution`, `ContextItem`, `ContextProjection`, `ContentRef`, `PolicyRef` | `MemoryPort`, `CompactionPolicy`, `ContextSelectionDecision` | host owns memory backend and browsing UX | [03 Context](../workstreams/_roles/03-context-structured-output.md) | candidate admission, no raw content by default, projection audit |
| Artifacts and content refs | `ArtifactRef`, `ContentRef`, `SourceRef`, `PrivacyClass`, `RetentionClass` | resource/artifact store refs | host owns bytes, storage, retention implementation | [00 Integration](../workstreams/_roles/00-integration-stitching.md), [03 Context](../workstreams/_roles/03-context-structured-output.md) | refs/summaries over raw blobs, lineage and policy present |
| Structured output | `OutputContract`, `ValidatedOutput`, `RunJournal`, `AgentEvent` | schema registry refs, repair policy | host owns product rendering/business scoring | [03 Context](../workstreams/_roles/03-context-structured-output.md) | local validation, bounded repair, typed errors |
| Events | `AgentEvent`, `EventEnvelope`, `EventFrame`, `EventCursor`, `EntityRef` | optional event archive port | host owns display event store and global archive implementation | [02 Events](../workstreams/_roles/02-events-journal-replay.md) | envelope filters, golden emitted kinds, no payload hot-path parsing |
| Journal and replay | `RunJournal`, `JournalRecord`, `JournalCursor`, `EffectIntent`, `EffectResult` | checkpoint, replay reducer, anti-entropy job | host owns storage backend | [02 Events](../workstreams/_roles/02-events-journal-replay.md) | intent-before-effect, replay modes, side-effect reconciliation |
| Tools and approval | `RuntimePackage`, `CapabilitySpec`, `PolicyRef`, `EffectIntent`, `EffectResult`, `RunJournal`, `AgentEvent` | `ToolRegistry`, `ToolRouter`, `ToolExecutor`, `ApprovalBroker`, `ToolPack` | host owns approval transport and installed tools | [04 Tools](../workstreams/_roles/04-tools-approval-toolpacks.md) | fail-closed policy, tool-pack fingerprint, execution intent/result for every tool, mutation metadata where needed |
| Streaming and realtime | `AgentEvent`, `RunJournal`, `RunHandle`, `ProviderAdapter`, `PolicyRef` | `StreamDelta`, `StreamRule`, `StreamIntervention`, `RealtimeProviderAdapter` | provider realtime transport and UI rendering are adapters/host | [05 Streaming](../workstreams/_roles/05-streaming-realtime-rules.md) | bounded matcher, intervention journal, terminal completion after drain |
| Isolation and execution | `RuntimePackage`, `PolicyRef`, `EffectIntent`, `EffectResult`, `RunJournal`, `AgentEvent` | `ExecutionEnvironment`, `IsolationRuntime`, `ProcessSpec` | concrete container/VM/sandbox is adapter-owned | [06 Isolation](../workstreams/_roles/06-isolation-execution.md) | no silent class/capability/trust downgrade, lifecycle journal, cleanup/reclaim |
| Subagents | `RunRequest`, `RuntimePackage`, `AgentEvent`, `RunJournal`, `PolicyRef`, `ContentRef` | `SubagentRequest`, `ContextHandoffPolicy`, `SubagentSupervisor` | host owns promotion to user conversation and inspector UI | [07 Subagents](../workstreams/_roles/07-subagents-coordination.md) | child package stripping, explicit handoff, parent lifecycle |
| Extensions | `RuntimePackage`, `CapabilitySpec`, `PolicyRef`, `EffectIntent`, `EffectResult`, `AgentEvent` | `CoreExtensionCapabilities`, extension action refs | host/optional crate owns `HostExtensionManifest`, subprocess runtime, install, marketplace, and UI | [08 Extension SDK](../workstreams/_roles/08-extension-sdk-packaging.md) | core-capability lowering, browser-safe smoke, no self-approval |
| Telemetry and privacy | `AgentEvent`, `RunJournal`, `PolicyRef`, `PrivacyClass`, usage refs | `TelemetrySinkSpec`, OTel mapping projection | host owns telemetry sink, dashboards, retention | [09 Telemetry](../workstreams/_roles/09-telemetry-privacy-cost.md) | redaction/content-capture matrix, bounded fanout, sink failure isolation |
| Output delivery | `DestinationRef`, `OutputSink`, `EffectIntent`, `EffectResult`, `RunJournal`, `AgentEvent` | `OutputDeliveryPolicy`, dedupe refs | host owns channel transport and UX | [11 Output Delivery](../workstreams/_roles/11-output-delivery-channels.md) | intent/result records, dedupe, privacy policy |
| Generic scenarios | all relevant kernel primitives | scenario-specific combinations only | host owns product workflow and surface | [10 Scenarios](../workstreams/_roles/10-generic-scenario-coverage.md) | scenario -> primitive -> host-boundary mapping |

## New Primitive Decision Ladder

Before accepting a new primitive or capability variant, answer in order:

1. Can this be a typed field or sidecar on an existing kernel primitive?
2. Can this be a `CapabilitySpec` entry that points to a typed sidecar contract?
3. Can this be an optional adapter behind an existing port?
4. Can this be host-owned behavior with only refs/events/journal records in the SDK?
5. If all answers are no, add a new primitive proposal with owner, fingerprint impact, events, journal records, policy/ref fields, validation, and compatibility risk.

If the proposal skips these questions, it is not ready for a coding goal.
