# Primitive Map

This map describes important future `agent_sdk` primitives at middle-level detail. Names are provisional, but the layering rule is not: first-slice SDK work should reuse the primitive kernel before adding a new concept.

## Ergonomics Principle

Every primitive should expose three layers:

| Layer | Shape | Rule |
| --- | --- | --- |
| Simple | one-line helpers such as `run_text`, `run_typed::<T>`, `tool_pack(WorkspaceReadOnly)`, `StreamRule::mask_regex`, or `IsolationRequirement::at_least(IsolationClass::Sandbox).prefer("adapter.ref")` | lowers into the same stable contract used by advanced callers |
| Builder | defaulted builders with conservative safe defaults and common overrides | keeps normal customization readable without hand-building every DTO |
| Advanced | explicit contract structs, registries, policies, adapters, and refs | remains the canonical source of truth for tests, events, journals, and wire formats |

Simple APIs must never become a second behavior path. They compile down to `RunRequest`, `OutputContract`, `RuntimePackage`, `StreamRule`, `EnvironmentSpec`, `SubagentRequest`, `CoreExtensionCapabilities`, or `TelemetrySinkSpec` and then use the same validation, policy, event, journal, and recovery paths as advanced APIs.

## Primitive Decision Ladder

Before adding a primitive, capability variant, registry, or side-effect path, answer these questions in order:

1. Can this be a typed field or sidecar on an existing kernel primitive?
2. Can this be a `CapabilitySpec` entry that points to a typed sidecar contract?
3. Can this be an optional adapter behind an existing port?
4. Can this stay host-owned with only typed refs, events, journals, and policy decisions in the SDK?
5. If all answers are no, add a primitive proposal with owner role, fingerprint impact, events, journal records, policy/ref fields, validation, and compatibility risk.

The answer must be recorded in the goal review packet. A feature is not ready for implementation if it needs a new primitive but cannot explain why the existing kernel, a typed sidecar, an optional adapter, or a host-owned boundary is insufficient.

## Primitive Kernel And Layers

The SDK should grow from a small kernel, not from one primitive per feature. The MVP Rust slice should prove a fake-provider text or typed run before broad feature work begins:

| Layer | Primitives | Rule |
| --- | --- | --- |
| Kernel run control | `Agent`, `AgentRuntime`, `RunRequest`, `RunHandle`, `RunResult`, `AgentLoop` | Starts, observes, cancels, resumes, and completes runs. |
| Kernel package | `RuntimePackage`, first-slice `CapabilitySpec`, `CapabilityKind`, `CapabilitySource`, `PolicyRef`, typed package sidecars | Freezes provider route, callable/discoverable capabilities, policies, and execution-affecting sidecars for one run. `CapabilitySpec` is not a universal bag. |
| Kernel content/context/output | `AgentMessage`, `AgentInputPart`, `AgentOutputPart`, `ArtifactRef`, `ContentRef`, `ContextContribution`, `ContextItem`, `ContextProjection`, `OutputContract`, `ValidatedOutput` | Keeps content references, context admission/projection, and typed output inside lineage-aware paths. Not all content becomes provider context. |
| Kernel side effects | `EffectIntent`, `EffectResult`, `IdempotencyKey`, `DedupeKey`, `PolicyDecision` | Gives tools, provider calls, output delivery, extension actions, memory writes, process actions, and child starts one journaled side-effect spine. |
| Kernel observability | `AgentEvent`, `EventEnvelope`, `EventFrame`, `EventFilter`, `EventCursor`, `RunJournal`, `JournalRecord`, `JournalCursor`, `TurnTrace`, `RunTrace`, `SessionTimeline` | Separates live observation from durable truth. |
| Kernel ports | `ProviderAdapter`, fake `ToolExecutor`, approval policy/broker port, `OutputSink`, journal/store ports, `AgentPoolStore` | Lets hosts and optional crates supply behavior without owning SDK semantics. |
| Kernel boundaries | `EntityRef`, `SourceRef`, `DestinationRef`, typed IDs, `PrivacyClass`, `RetentionClass`, `TrustClass`, `LineageRef` | Makes origin, destination, policy, privacy, retention, trust, and causality explicit. |

Feature primitives must layer on the kernel:

- Tool packs install callable/discoverable `CapabilitySpec` entries plus policy, executor refs, and typed tool-pack sidecars.
- Hooks are package sidecars plus ordered hook executor refs; they are not `CapabilitySpec` entries unless a hook is itself exposed as a callable/discoverable capability.
- Stream rules are package sidecars plus `StreamRuleEngine` state and journaled interventions; they are not `CapabilitySpec` entries unless exposed as callable/discoverable controls.
- Structured output is `OutputContract` plus local validation and repair over the normal run loop.
- Memory, tool results, skills, host input, subagents, and compaction may create `ContextContribution` candidates. `ContextAssembler` admits only selected items into `ContextItem` and `ContextProjection` under policy and budget.
- Isolation is `ExecutionEnvironment` plus an `IsolationRuntime` port; concrete runtimes stay optional or host-owned.
- Agent pools are feature-layer coordination scopes over existing runs, messages,
  event subscriptions, wake conditions, and optional pool-scoped store/watch
  adapters. They are not workflow engines.
- Subagents are higher-order supervised child-run presets over `AgentPool` with
  stripped `RuntimePackage` snapshots, parent-owned lifecycle, wrapped events,
  and usage rollup.
- Extensions declare `CoreExtensionCapabilities` that a host may resolve into runtime-package capabilities and sidecars after policy checks.
- Telemetry is a projection from events, journals, and usage records; it is not durable run truth.
- Outcome attribution is a derived eval-layer report over journals, traces,
  context projection audits, events, outputs, and `agent-sdk-eval` records.
  Self-cited support IDs are evidence, not proof; measured impact requires a
  recorded baseline, ablation, paired run, repeated experiment, or evaluator
  metric delta.
- Output delivery is a destination/sink port with dedupe and journaled intent; product channel UX stays host-owned.

Reserved feature ports may be sketched in contracts before implementation, but they are not part of the MVP runtime proof: telemetry exporters, isolation adapters, extension bridges, subagents, realtime providers, stream rules, full tool packs, and global event archive replay.

New primitives must pass the decision ladder above before they are added.

## Core Runtime

| Primitive | Owns | Key methods | Must not own |
| --- | --- | --- | --- |
| `Agent` | Immutable/default agent configuration and identity. | `builder`, `run`, `stream`, `as_tool_descriptor`. | Host UI, global provider registry, durable analytics store. |
| `AgentBuilder` | Construction of an `Agent` from identity, default instructions, default model route, and policy/context refs. Hook helpers may attach default `HookSpec` refs that are resolved into the run package before execution. | `id`, `name`, `instructions`, `default_model_route`, `default_context_policy`, `on`, `hooks_from_config`, `build`. | Runtime execution, mutable package construction after build, approval decisions, external process lifecycle. |
| `AgentRuntime` | Active execution service, shared ports, stores, policies, subscriptions, and per-run package resolution. | `start_run`, `resume_run`, `cancel_run`, `subscribe_all`, `subscribe_run`, `subscribe_agent`, `subscribe_events`, `finalize`. | Product routing between external runtimes, desktop, and headless surfaces; UI dispatch; assuming one global package for all runs. |
| `AgentLoop` | The loop state machine for one run. | `step`, `drive`, `transition`, `recover`. | Provider-specific transport, UI events, durable host analytics. |
| `AgentStateMachine` | Legal loop states and transitions. | `next`, `validate_transition`, `invariants`. | Tool execution implementation, provider networking. |
| `RunContext` | Run-level IDs, policies, runtime package, cancellation, session, telemetry, and host metadata. | `turn`, `emit`, `policy`, `checkpoint`, `cancel_token`. | Mutable transcript contents beyond scoped APIs. |
| `RunChildLifecyclePolicy` | Effective run policy for agent-owned children, shutdown, detach, grace periods, and reclaim. | `on_manual_cancel`, `on_run_completed`, `detach_policy`, `reclaim_policy`. | Concrete process/container control or product process inspector UI. |
| `TurnContext` | One loop turn, context projection, attempt counters, provider request, and turn events. | `assemble_context`, `record_model_call`, `record_tool_call`, `complete`. | Cross-run state, global memory. |

## Runtime Package And Adapters

| Primitive | Owns | Key methods | Must not own |
| --- | --- | --- | --- |
| `RuntimePackage` | Typed per-run snapshot of effective provider route, callable/discoverable capabilities, typed package sidecars, policies, lifecycle bounds, and fingerprint. | `fingerprint`, `provider_tool_specs`, `tool_registry`, `sidecars`, `child_lifecycle`, `output_sinks`. | Live discovery side effects after snapshot creation, ambient runtime mutation, or a parallel feature registry. |
| `CapabilitySpec` | Stable identity, source, namespace, visibility, projection mode, executor ref, policy ref, and sidecar refs for callable/discoverable capabilities. Reserved variants are inactive until their owner names sidecar contract, fingerprint fields, events, journal records, and acceptance tests. | `project`, `executor`, `sidecar_ref`, `fingerprint_fields`. | Provider route, output contracts, delivery sinks, telemetry policy, or arbitrary feature maps when those are better modeled as package fields or typed sidecars. |
| `CapabilityCatalogSnapshot` | Source-qualified provenance for tools, skills, packages, MCP resources, extension actions, and subagent/tool candidates. | `source_kind`, `source_ref`, `version`, `hash`, `trust`, `activation_policy`. | Host marketplace UX, installation, or unreviewed runtime activation. |
| `ProviderAdapter` | Text/multimodal model transport. | `project_request`, `stream`, `count_tokens`, `capabilities`. | Internal transcript mutation, approval, memory. |
| `RealtimeProviderAdapter` | Bidirectional realtime transport. | `connect`, `send`, `receive`, `restart`, `close`. | Tool execution policy or UI media rendering. |
| `ExternalAgentAdapter` | Stateful external runtimes. | `launch`, `restore`, `send`, `receive`, `retire`, `fingerprint`. | Host runtime-session cache or host-specific restore policy. |
| `ProviderEventMapper` | Provider stream to canonical events. | `map_chunk`, `final_message`, `usage`, `stop_reason`. | Direct state transitions without loop validation. |

## Execution Isolation

| Primitive | Owns | Key methods | Must not own |
| --- | --- | --- | --- |
| `ExecutionEnvironment` | The typed workload environment requested by an agent run, tool, subagent, or extension action: kind, image, resources, mounts, network, secrets, lifecycle, and cleanup policy. | `spec`, `fingerprint`, `policy_summary`, `requires_adapter`. | Concrete VM/container implementation or host UI. |
| `IsolationRuntime` | Adapter contract for preparing environments, running processes, streaming I/O, sending signals, collecting stats, and cleanup. | `prepare`, `start_process`, `stream_io`, `signal`, `collect_stats`, `cleanup`. | Approval decisions or model/tool routing. |
| `ContainerRuntimeAdapter` | A concrete implementation such as Apple Containerization, Docker, Firecracker, or a remote sandbox. | `capabilities`, `health_check`, `pull_image`, `create_rootfs`, `run_process`. | Portable SDK semantics or assuming all hosts have the same runtime. |
| `ProcessSpec` | Command arguments, env, cwd, user, terminal mode, stdin/stdout/stderr wiring, timeout, and rlimits. | `validate`, `redacted_summary`, `to_adapter_request`. | Shell parsing or approval. |
| `FilesystemIsolationPolicy` | Mounts, read-only roots, writable layers, workspace snapshots, single-file mount expansion, secret mounts, and cleanup mode. | `mounts`, `validate_paths`, `expanded_mount_audit`, `snapshot_plan`. | File mutation itself. |
| `NetworkIsolationPolicy` | Network disabled/enabled/egress-scoped state, DNS, hosts, exposed ports, socket relays, and dedicated IP policy. | `egress_allowed`, `dns_config`, `port_policy`, `socket_policy`. | Runtime-specific networking side effects. |
| `IsolationCapabilityReport` | Adapter health and support matrix: platform, image formats, CPU architecture, Rosetta/emulation, kernel availability, network mode, mount support, stats, and cleanup guarantees. | `supports`, `missing_requirements`, `diagnostics`. | Running workloads. |

## Streaming Control

| Primitive | Owns | Key methods | Must not own |
| --- | --- | --- | --- |
| `StreamDelta` | One typed provider/tool/realtime stream increment with channel, cursor, attempt ID, privacy, and optional content reference. | `channel`, `cursor`, `redacted_summary`, `content_ref`. | Full transcript storage or sink delivery. |
| `StreamRule` | Declarative matching rule: matcher, target channels, scope, repeat policy, privacy, and requested action. | `compile`, `scope_matches`, `describe`. | Product rulebook UI or unbounded content capture. |
| `StreamMatcher` | Literal, regex, marker, or host-provided matching strategy over bounded stream windows. | `push_delta`, `matches`, `reset`, `memory_budget`. | Long-term transcript buffering. |
| `StreamRuleEngine` | Active compiled rules for a run or package and their per-channel buffers/repeat state. | `observe_delta`, `reset_turn`, `restore_state`, `snapshot_state`. | Provider transport or direct side-effect execution. |
| `StreamIntervention` | A matched rule's proposed control action with causality, match metadata, redaction, and policy refs. | `action`, `redacted_match`, `requires_approval`, `journal_payload`. | Silent mutation of messages or tools. |

## Messages, Context, And Memory

| Primitive | Owns | Key methods | Must not own |
| --- | --- | --- | --- |
| `AgentMessage` | Lossless internal message, parts, lineage, metadata, privacy, retention, projection status. | `parts`, `lineage`, `redacted_summary`, `project`. | Provider-specific raw request shape as the only representation. |
| `AgentInputPart` / `AgentOutputPart` | Text, reasoning, image, audio, video, file, tool call, tool result, citation, redacted content. | `media_refs`, `summary`, `token_hint`. | Global media storage. |
| `ArtifactRef` / `ContentRef` | Stable reference to text, media, files, tool outputs, generated artifacts, schemas, or external content, with MIME/type, scope, version, storage service, privacy, and retention. | `resolve`, `mime`, `size`, `version`, `redacted_summary`, `policy_summary`. | Copying bytes through every event, owning the backing content store, or implying provider visibility. |
| `ContextContribution` | Candidate context from memory, tool results, skills, host input, remote channels, subagents, compaction, or files before admission. | `source`, `content_ref`, `producer_ref`, `trust`, `policy_refs`, `budget_hint`, `derived_from`. | Provider projection or durable memory write authority. |
| `ContextItem` | A policy-admitted unit injected into model context with source, destination, policy, lineage, sensitivity, retention, trust, and projection role. | `project`, `summarize`, `requires_policy`, `selection_decision`. | Durable memory write ownership or arbitrary artifact storage. |
| `ContextSelectionDecision` | Why a contribution was included, omitted, compacted, redacted, deduped, pinned, or denied. | `reason`, `policy_refs`, `budget`, `redacted_summary`. | Provider transport or product ranking UX. |
| `OutputContract` | User/host-requested output shape, schema ID, validation policy, repair policy, and typed result mode. | `schema`, `validator`, `repair_policy`, `projection_hint`. | Product-specific form rendering or business scoring. |
| `StructuredOutputValidator` | Local parse, schema validation, semantic validation, and error reporting for model output candidates. | `validate`, `explain_errors`, `redacted_error_summary`. | Provider transport or hidden mutation of model output. |
| `ValidatedOutput` | Typed output value, schema version, validation report, source attempt IDs, and lineage. | `as_json`, `typed_ref`, `lineage`, `redacted_summary`. | Raw provider transcript storage. |
| `MemoryPort` | Retrieval and storage port for memories. | `glance`, `recall`, `detail`, `store`, `ingest`. | UI memory browsing or extension-side shadow memory. |
| `ContextAssembler` | Turns messages, memory, system instructions, tools, hooks, and host context into `ContextProjection`. | `assemble`, `budget`, `explain_projection`. | Provider call transport. |
| `ConversationManager` | Transcript windowing and history policy. | `append`, `project_history`, `reduce_context`, `snapshot`. | Memory store or trace analytics. |
| `CompactionPolicy` | When and how to compact context, plus protected context preservation. | `should_compact`, `compact`, `before_compact`, `after_compact`. | Hidden deletion without journal events. |

## Tools And MCP

| Primitive | Owns | Key methods | Must not own |
| --- | --- | --- | --- |
| `ToolRegistry` | Effective tool definitions and executable handlers by source. | `register`, `lookup`, `specs_for_provider`, `snapshot`. | Approval decisions or provider route selection. |
| `ToolRouter` | Tool name resolution, namespace, MCP/app/extension/subagent selection. | `resolve`, `canonical_name`, `available_tools`. | Executing the tool. |
| `ToolExecutor` | Tool attempts, timeouts, concurrency, ordering, streaming, cancellation. | `execute`, `execute_many`, `stream_attempt`. | Policy decision to allow the tool. |
| `ToolExecutionStrategy` | Sequential, concurrent, bounded, ordered, or custom execution policy. | `plan`, `join`, `overflow_policy`. | Tool schemas or approval. |
| `McpRegistry` | MCP server/resource/tool prompt exposure. | `discover`, `filter`, `call`, `snapshot`. | Global tool policy or model prompts. |
| `ToolPack` | A reusable group of tool specs, handlers, permissions, and default policy hints. | `tool_specs`, `register_handlers`, `required_permissions`, `fingerprint`. | Global installation or host product workflow. |
| `BuiltinToolPack` | SDK-provided packs such as workspace read-only, workspace edit, write, shell, resource readers, and tool discovery. | `workspace_readonly`, `workspace_edit`, `shell`, `resource_readers`. | Making tools available without a host runtime package. |
| `ResourceUriRouter` | Resolution for `memory://`, `artifact://`, `mcp://`, `rule://`, `skill://`, and host-defined internal URLs. | `resolve`, `source_path`, `privacy`, `read_capabilities`. | Owning the backing memory/artifact/MCP stores. |
| `WorkspaceSearch` | Regex/glob/AST search with result limits, anchors, truncation metadata, and cancellation. | `grep`, `glob`, `ast_grep`, `format_anchors`. | Applying edits or bypassing policy. |
| `WorkspaceEditPlanner` | Anchor validation, patch planning, preview diffs, stale-anchor recovery, and formatter/diagnostic hints. | `plan`, `preview`, `validate_preconditions`, `inverse_candidate`. | Writing files without executor/policy approval. |
| `PatchApplier` | Durable application of planned file/archive/structured-data mutations. | `apply`, `rollback_candidate`, `invalidate_caches`, `record_effect`. | Product-specific Evolution or review UI. |
| `ShellTool` | Command/PTY execution with cwd/env/network/sandbox/timeout/cancellation settings. | `spawn`, `stream_output`, `cancel`, `exit_status`. | Ambient shell access outside sandbox and approval. |
| `ToolDiscoveryIndex` | Searchable index of available but hidden tools and activation rules. | `search`, `activate`, `deactivate`, `explain_activation`. | Expanding the runtime package without host policy. |

## Policy And Approval

| Primitive | Owns | Key methods | Must not own |
| --- | --- | --- | --- |
| `ApprovalPolicy` | Preflight rules for whether a tool or action needs approval. | `classify`, `decision_hint`, `explain`. | UI transport or out-of-band messaging. |
| `ApprovalBroker` | Request lifecycle, parked receiver, response, timeout, attribution. | `request`, `respond`, `timeout`, `cancel`. | Deciding UI copy, extension self-approval. |
| `PermissionPolicy` | Capability permissions such as filesystem, network, shell, MCP, media, contacts. | `check`, `scope`, `grant`, `deny`. | Tool execution result. |
| `SandboxPolicy` | Execution sandbox, working directory, environment, network, and command limits. | `prepare`, `validate`, `isolate`. | User approval UX. |
| `EscalationPolicy` | Policy for when approval must be escalated, timed out, denied, or resolved from a finite host decision. | `classify`, `timeout`, `required_dispatcher`, `validate_response`. | Sending messages, collecting replies, or owning approval transport. |
| `ApprovalDispatcher` | Host-provided approval delivery/reply collection port. | `dispatch`, `cancel`, `health`. | Approval policy decisions or bypassing fail-closed behavior. |

## Hooks, Events, And Telemetry

| Primitive | Owns | Key methods | Must not own |
| --- | --- | --- | --- |
| `HookSpec` / `HookPoint` | Core lifecycle hook contract for config and code-first hooks. | `point`, `ordering`, `timeout`, `failure_policy`, `mutation_rights`. | Extension subprocess management or arbitrary mutable callbacks. |
| `AgentHooks` | Hook provider registration. | `register`, `snapshot`, `capabilities`. | Runtime loop state. |
| `HookBus` | Ordered hook invocation and mutation collection. | `emit_before`, `emit_after`, `apply_response`. | Arbitrary mutation outside typed hook response or blocking observe hooks. |
| `AgentEvent` | Canonical observable event enum. | `kind`, `envelope`, `privacy`, `causal_refs`. | Slow sink delivery. |
| `EventEnvelope` | IDs, time, run/session/agent/turn, family/kind, source, destination, trace context, correlation, tags, privacy, payload schema version. | `redact`, `summarize`, `to_otel`, `cursor`. | Payload business meaning. |
| `TurnTrace` / `SessionTimeline` | Derived journal views for one turn or one session. | Group records by `session_id`, `turn_id`, and `run_id`; expose related attempt, message, effect, tool, and context projection ids. | Trace database, host conversation store, dashboard state, or raw payload access. |
| `AgentEventBus` | Direct lifecycle event subscriptions, typed filters, scoped `EventCursor`s, live fanout, and run-scoped replay from the run journal. | `subscribe_all`, `subscribe_run`, `subscribe_agent`, `subscribe_filtered`, `replay_run_from_cursor`. | Workflow/DAG/barrier engines, UI rendering, or global durable event archive ownership. |
| `EventArchive` / `IndexedJournalView` | Optional host or adapter port for cross-run/all-agent/filtered durable event replay with `ArchiveCursor`. | `replay_filtered_from_cursor`, `supports_filter`, `cursor_bounds`. | Core guarantee of global durable event queries. |
| `EventFrame` | Stream item containing `AgentEvent`, current `EventCursor`, optional `ArchiveCursor`, and optional overflow notice. | `event`, `cursor`, `archive_cursor`, `overflow`. | Durable replay guarantee by itself. |
| `EventFilter` / `CompiledEventFilter` | Fast envelope-field filtering by run, agent, turn, family/kind, source/destination, correlation keys, tags, privacy, and delivery semantics. | `compile`, `matches_envelope`, `payload_access`, `overflow_policy`. | Payload parsing or raw content authorization. |
| `TelemetrySink` | Non-blocking delivery to abstract telemetry/export destinations declared by host policy. | `record`, `flush`, `health`, `overflow`. | Core loop control decisions or product-specific storage/query semantics. |
| `UsageExtractor` | Usage/cost extraction from provider/tool events. | `extract`, `merge`, `estimate`. | Billing product UI. |
| `CostEstimator` | Per-provider/tool cost estimates and budgets. | `estimate`, `budget_check`. | Provider credentials. |
| `EvidenceBundle` / `EvaluatorJudgment` | Optional eval-layer support claims and evidence bundles that cite existing run, context, tool, output, capability, effect, or event refs. | `support_refs`, `rejected_support_refs`, `redacted_summary`, `validate_support_refs`, `derived_from`. | Causal proof, raw chain-of-thought capture, or bypassing context projection policy. |
| `EvaluationReport` | Optional eval-layer attribution view connecting cited evidence, expected outcomes, and comparison designs to terminal or intermediate outcomes. | `EvaluationConfidence`, `EvaluationMetricDelta`, `ComparisonDesign`, `EvaluationUsage`, `limitations`. | A second journal, trace database, product scoring rubric, dashboard state, or claim that self-report alone measured impact. |

## Sessions, Recovery, And Robustness

| Primitive | Owns | Key methods | Must not own |
| --- | --- | --- | --- |
| `SessionManager` | Session restoration/sync through lifecycle events. | `initialize`, `append_message`, `sync_state`, `redact_latest`. | Hot loop networking. |
| `CheckpointStore` | Snapshots for resume and repair. | `save`, `load`, `list`, `gc`. | Full append-only history. |
| `RunJournal` | Append-only event/message/action log. | `append`, `replay`, `seal`, `compact`. | UI display decisions. |
| `RecoveryPolicy` | What to do after crash, timeout, invalid state, or provider disconnect. | `classify`, `repair_plan`, `resume_point`. | Hiding irrecoverable errors. |
| `InvariantChecker` | Runtime and replay invariants. | `check_message_pairs`, `check_ids`, `check_policy_edges`. | Mutating state silently. |
| `AntiEntropyJob` | Background validation and repair of derived internal indexes, projections, cursors, and telemetry summaries. | `scan`, `repair_internal_view`, `report`. | External side-effect compensation, product repair UI, or self-improvement workflows. |

## Multi-Agent And Channels

| Primitive | Owns | Key methods | Must not own |
| --- | --- | --- | --- |
| `AgentPool` | Feature-layer coordination scope for run membership, generic run messages, topic fan-out, event subscriptions, wake registration, and optional pool-scoped rehydration/watch. | `start_run`, `join_run`, `send`, `subscribe`, `suspend_until`, `snapshot`, `watch_pool`. | Workflow/DAG/barrier engines, product swarm UI, global archive ownership, concrete store deployment, or semantic relationship roles. |
| `AgentPoolStore` | Pool-scoped durable coordination port for lifecycle, membership, message status, wake status, dedupe, snapshots, and watch cursors. | `open_pool`, `snapshot`, `join_member`, `record_message`, `record_wake`, `watch`. | Global event archive ownership, scheduler/broker behavior, product workflow, or access to other pools' journals/events. |
| `RunMessage` / `WakeCondition` | Durable run-to-run communication and event-filter-based suspend/resume. | `send`, `reply_to`, `delivery_status`, `wake_on`, `timeout`. | Provider prompt injection, direct user chat ownership, or scheduling/compensation logic. |
| `SubagentSupervisor` | Higher-order helper over `AgentPool`, child `RunRequest`, stripped child package, lifecycle policy, event wrapping, and usage rollup. | `spawn_child`, `stream_child`, `cancel_child`, `rollup_usage`. | Generic coordination, direct user chat ownership, or recursive agent societies. |
| `RemoteChannelAdapter` | Inbound/outbound remote messages and source/destination metadata. | `receive`, `send`, `ack`, `dedupe`. | Agent loop policy. |
| `OutputSink` | Where final or streaming output is sent: desktop, CLI, remote reply, webhook, file. | `send_chunk`, `send_final`, `fail`. | Model/tool execution. |

## Cross-Cutting Rule

If a primitive mutates state, emits events, or sends content across a boundary, it must also expose lineage. The future SDK should be able to explain "where did this item come from, where did it go, and which policy allowed that" without parsing text labels.
