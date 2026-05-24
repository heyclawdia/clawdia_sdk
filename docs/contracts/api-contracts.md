# API Contracts

This contract defines the first Rust-facing SDK surface. It is intentionally small enough for test-first implementation and broad enough to support the later contracts.

## Crate Boundary

Initial target crate:

```text
crates/agent-sdk-core/
  Cargo.toml
  src/
    lib.rs
    agent/
    runtime/
    run/
    domain/
    package/
    providers/
    context/
    output/
    events/
    journal/
    policy/
    ports/
    recovery/
```

The core crate must compile without importing product UI commands, external-runtime process managers, app-event store contracts, durable trace-store implementations, broad built-in tool implementations, concrete isolation runtimes, or extension installer code.

Optional crates can layer on top:

```text
crates/agent-sdk-toolkit/        read/search/edit/write/shell/resource tool packs
crates/agent-sdk-isolation/      concrete isolation adapters
crates/agent-sdk-extension/      extension protocol bridge
crates/agent-sdk-otel/           OpenTelemetry exporter helpers
crates/agent-sdk-host-adapter/  future host adapter experiments
crates/agent-sdk-workflow/       optional orchestration over core events
```

This mirrors the Pi-style boundary: core loop and contracts stay small; harness/toolkit/product layers compose around it.

## First Rust Slice Primitive Surface

The first slice has two gates: a minimal fake-provider text run, then the typed-output gate over the same kernel. Optional feature layers expand only after those gates:

- run control: `Agent`, `AgentRuntime`, `RunRequest`, `RunHandle`, `RunResult`;
- package snapshot: `RuntimePackage`, `RuntimePackageRef`, first-slice `CapabilitySpec`, typed package sidecars, `PolicyRef`;
- content, context, and output: `AgentMessage`, `ArtifactRef`, `ContentRef`, `ContextContribution`, `ContextItem`, `ContextProjection`, `OutputContract`, `ValidatedOutput`;
- observability and durability: `AgentEvent`, `EventFrame`, `EventFilter`, `EventCursor`, `RunJournal`, `JournalRecord`, `JournalCursor`;
- side effects: `EffectIntent`, `EffectResult`, `IdempotencyKey`, `DedupeKey`, `PolicyDecision`;
- ports: `ProviderAdapter`, fake `ToolExecutor`, approval policy/broker port, `OutputSink`, journal/store ports;
- boundaries: `EntityRef`, `SourceRef`, `DestinationRef`, `PrivacyClass`, `RetentionClass`, `TrustClass`, `LineageRef`, and typed IDs.

Tool packs, hooks, stream rules, memory stores, isolation adapters, subagents, extensions, telemetry exporters, and output channels are feature layers over these primitives. They may add helpers and optional crates, but they must not create a second run loop, event stream, journal, package registry, policy path, or side-effect path. Reserved feature ports can exist as traits or contracts before implementation, but they are not required to pass the MVP fake-provider run.

## Public Surface Tiers

The API docs name three tiers so the first slice stays small without losing planned features:

| Tier | Required when | Examples |
| --- | --- | --- |
| MVP public | Required for one fake-provider text or typed run. | `Agent`, `AgentRuntime`, `RunRequest`, `RunHandle`, `RunResult`, `RuntimePackage`, `CapabilitySpec`, `AgentMessage`, `ContextProjection`, `OutputContract`, `AgentEvent`, `RunJournal`, core refs/IDs, provider/fake tool/output ports. |
| Reserved public contract | Contract may be documented now, but implementation waits for the owning phase goal. | hook lifecycle, stream rules, isolation, subagents, extension capabilities, telemetry exporters, event archive replay. |
| Optional crate API | Public only from an optional crate or host adapter. | concrete toolkit tools, isolation adapters, extension JSON-RPC bridge, OTel exporter helpers, workflow/orchestration helpers. |

Reserved public contracts must not be required for the MVP crate to compile or run a fake-provider run.

## MVP Public Types

The MVP public contract targets only the names needed for one fake-provider text or typed run. Implementations may keep fields private.

| Type | Owns | Must not own |
| --- | --- | --- |
| `Agent` | Immutable agent identity, default instructions, default model route, default context/memory policy handles. | Active execution, provider clients, UI routing, approval transport. |
| `AgentBuilder` | Construction of an `Agent` from typed identity, instructions, default route refs, and context policy refs. | Runtime package mutation after build or lifecycle hook registration in the MVP path. |
| `AgentRuntime` | Active run orchestration for provider/tool/output ports, journal, policies, subscriptions, and per-run package resolution. | Product dispatch between external runtimes, desktop, CLI, remote, or scheduled surfaces; assuming one global runtime package forever. |
| `RunRequest` | One requested execution: source, destination, input parts, runtime package ref, output contract, cancellation, and host metadata. | Provider-specific wire request. |
| `RunHandle` | Event stream, cancellation handle, final result receiver, and replay cursor for one run. | UI rendering decisions. |
| `RunResult` | Terminal status, final message or structured output, usage/cost summary, and causal IDs. | Durable analytics storage. |
| `RuntimePackage` | Immutable per-run effective snapshot for provider route, callable/discoverable capabilities, typed sidecars, policies, output contracts, output sinks, lifecycle bounds, and isolation requirements. | Live discovery after the run starts or feature-specific parallel registries. |
| `AgentEvent` | Canonical live observability vocabulary. | Slow sink delivery and durable storage policy. |
| `EventFrame` | Stream item containing event, event cursor, optional archive cursor, and overflow notice. | Durable replay guarantee without journal/archive support. |
| `EventFilter` / `CompiledEventFilter` | Envelope/index filter description and optimized matcher for event subscriptions. | Payload parsing or content access authorization. |
| `AgentEventBus` | Core lifecycle event subscription, filtering, cursors, and run-scoped replay API. | Workflow/DAG/barrier engines, UI rendering, telemetry storage, or global durable event archive ownership. |
| `RunJournal` | Append-only durable run ledger port. | UI event buffering or transcript rendering. |
| `ProviderAdapter` | Provider projection and streaming transport. | Internal transcript mutation or approval decisions. |
| `ToolExecutor` | Tool attempts, concurrency, timeouts, cancellation, and result envelopes. | Policy decisions. |
| `ApprovalBroker` | Approval request lifecycle, pending decisions, timeouts, and attribution. | UI copy or out-of-band channel implementation. |

## Reserved And Optional Public Types

These names are documented so contracts can converge, but they are not required for the MVP crate to compile or for a fake-provider run to pass:

| Reserved or optional type | Activation boundary | Must not own |
| --- | --- | --- |
| `HookSpec` / `HookPoint` | Phase 04 hook sidecar work; helpers lower into package sidecars. | Ambient mutation of loop state, provider messages, approvals, memory, or host processes. |
| `RunChildLifecyclePolicy` | Child-artifact and subagent feature work. | Concrete process/container control implementation. |
| `EventArchive` | Optional indexed durable event replay port for cross-run/all-agent/filtered catch-up. | Core guarantee that every journal backend supports global queries. |
| `TelemetrySink` | Optional telemetry crate/adapter over derived events, journals, usage, and cost. | Run control flow or durable run truth. |
| `StreamRule`, `ExecutionEnvironment`, `SubagentRequest`, `CoreExtensionCapabilities` | Owning feature phases. | Parallel run loops, package registries, journals, policies, or host products. |

Public signature support types must also be exported or reachable from stable modules when they appear in public method signatures. MVP signatures should use only MVP support types; reserved support types become required only when their feature APIs are activated:

- run API: `RunRegistry`, `RunSnapshot`, `RunSummary`, `RunQuery`, `RunStatus`, `RunStatusHandle`, `RunResultHandle`, `CancellationHandle`;
- package API: `RuntimePackageRef`, `RuntimePackageResolver`, `CapabilitySpec`, `CapabilityId`, `CapabilityKind`, `CapabilitySource`, `PackageSidecarRef`, `CapabilityCatalogSnapshot`;
- content/context API: `ArtifactRef`, `ContentRef`, `ContextContribution`, `ContextSelectionDecision`, `ContextItem`, `ContextProjection`;
- effect API: `EffectIntent`, `EffectResult`, `EffectKind`, `IdempotencyKey`, `DedupeKey`;
- reserved lifecycle API: `RunChildLifecyclePolicy`, `RunChildLifecyclePolicyRef`, `ChildArtifactKind`, `ChildShutdownBehavior`, `DetachPolicy`, `ChildArtifactId`, `ProcessOwnershipPolicy`;
- reserved hook API: `HookId`, `HookSpec`, `HookPoint`, `HookInput`, `HookResponse`, `HookExecutionMode`, `HookQueueConfig`, `HookOverflowPolicy`, `HookFailurePolicy`, `HookMutationRights`, `HookExecutorRef`, `HookConfig`;
- event API: `AgentEventStream`, `AgentEventStreamHandle`, `EventStreamScope`, `EventOverflowNotice`, `EventOverflowReason`, `EventFilterId`, `EventFilterFingerprint`, `EventIndexField`, `EventArchive`, `SubscriptionOptions`, `SubscriberQueueConfig`;
- cursor/replay API: `EventCursor`, `JournalCursor`, `ArchiveCursor`, `ReplayMode`, `ReplayResult`;
- policy/config API: `RunAdvancedConfig`, `CancellationPolicy`, `PolicyDecision`, `HostMetadata`;
- lineage API: `EntityRef`, `LineageRef`, `SourceRef`, `DestinationRef`, `PrivacyClass`, `RetentionClass`, `TrustClass`.

## Conceptual API Shape

```rust
// Non-compiling contract sketch.
pub struct Agent {
    id: AgentId,
    name: AgentName,
    defaults: AgentDefaults,
}

impl Agent {
    pub fn builder() -> AgentBuilder;
    pub async fn run(&self, request: RunRequest, runtime: &AgentRuntime) -> Result<RunResult, AgentError>;
    pub fn stream(&self, request: RunRequest, runtime: &AgentRuntime) -> Result<RunHandle, AgentError>;
    pub async fn run_text(&self, input: impl Into<AgentInput>, runtime: &AgentRuntime) -> Result<String, AgentError>;
    pub async fn run_typed<T: TypedOutputModel>(&self, input: impl Into<AgentInput>, runtime: &AgentRuntime) -> Result<T, AgentError>;
    pub fn request(&self, input: impl Into<AgentInput>) -> RunRequestBuilder;
}

impl AgentBuilder {
    pub fn instructions(self, instructions: impl Into<AgentInstructions>) -> Self;
    pub fn default_route(self, route: ProviderRouteRef) -> Self;
    pub fn build(self) -> Result<Agent, AgentError>;
}

pub struct AgentRuntime {
    default_package: Option<RuntimePackage>,
    package_resolver: Arc<dyn RuntimePackageResolver>,
    providers: ProviderRegistry,
    tools: ToolRegistry,
    approval: Arc<dyn ApprovalBroker>,
    journal: Arc<dyn RunJournal>,
    events: Arc<dyn AgentEventBus>,
}

impl AgentRuntime {
    pub async fn start_run(&self, request: RunRequest) -> Result<RunHandle, AgentError>;
    pub async fn resume_run(&self, request: ResumeRequest) -> Result<RunHandle, AgentError>;
    pub async fn cancel_run(&self, run_id: RunId) -> Result<(), AgentError>;
    pub fn events(&self) -> &dyn AgentEventBus;
    pub fn subscribe_all(&self, cursor: Option<EventCursor>) -> Result<AgentEventStream, AgentError>;
    pub fn subscribe_all_with_options(&self, cursor: Option<EventCursor>, options: SubscriptionOptions) -> Result<AgentEventStream, AgentError>;
    pub fn subscribe_run(&self, run_id: RunId, cursor: Option<EventCursor>) -> Result<AgentEventStream, AgentError>;
    pub fn subscribe_run_with_options(&self, run_id: RunId, cursor: Option<EventCursor>, options: SubscriptionOptions) -> Result<AgentEventStream, AgentError>;
    pub fn subscribe_agent(&self, agent_id: AgentId, cursor: Option<EventCursor>) -> Result<AgentEventStream, AgentError>;
    pub fn subscribe_agent_with_options(&self, agent_id: AgentId, cursor: Option<EventCursor>, options: SubscriptionOptions) -> Result<AgentEventStream, AgentError>;
    pub fn subscribe_events(
        &self,
        filter: CompiledEventFilter,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError>;
}

pub struct RunRequestBuilder {
    agent_id: AgentId,
    input: AgentInput,
    source: Option<SourceRef>,
    destination: Option<DestinationRef>,
    runtime_package: Option<RuntimePackageRef>,
    output_contract: Option<OutputContract>,
    advanced: RunAdvancedConfig,
}

impl RunRequestBuilder {
    pub fn source(self, source: SourceRef) -> Self;
    pub fn destination(self, destination: DestinationRef) -> Self;
    pub fn runtime_package(self, package: RuntimePackageRef) -> Self;
    pub fn output<T: TypedOutputModel>(self) -> Self;
    pub fn output_contract(self, contract: OutputContract) -> Self;
    pub fn advanced(self, configure: impl FnOnce(&mut RunAdvancedConfig)) -> Self;
    pub fn build(self, runtime: &AgentRuntime) -> Result<RunRequest, AgentError>;
    pub async fn run(self, runtime: &AgentRuntime) -> Result<RunResult, AgentError>;
    pub async fn run_typed<T: TypedOutputModel>(self, runtime: &AgentRuntime) -> Result<T, AgentError>;
}

impl RunResult {
    pub fn structured_output<T: TypedOutputModel>(&self) -> Result<StructuredOutputResult<T>, AgentError>;
    pub fn into_typed_output<T: TypedOutputModel>(self) -> Result<T, AgentError>;
}
```

Reserved hook helpers, child lifecycle helpers, telemetry registration, event archive replay, stream rules, isolation, subagents, and extension helpers should be shown in their owning contracts, not the MVP conceptual API. When activated, each helper must lower into the same `RunRequest`, `RuntimePackage`, `AgentEvent`, `RunJournal`, `PolicyRef`, and `EffectIntent` path instead of adding a parallel behavior path.

## Ergonomic Surface And Lowering Contract

The SDK should have three layers:

| Layer | Intended caller | Example | Contract rule |
| --- | --- | --- | --- |
| Simple | Most callers | `agent.run_text("hello", &runtime)` or `agent.run_typed::<Todo>("extract", &runtime)` | Must lower into `RunRequest` and the same loop as advanced usage. |
| Builder | Common customization | `agent.request("...").source(...).output::<Todo>().run(&runtime)` | May set common fields, but cannot bypass package/policy/journal/event contracts. |
| Advanced | Hosts and power users | explicit `RunRequest`, `OutputContract`, `RuntimePackage`, policies, ports | Is the canonical contract path. |

Rules:

- Simple APIs are convenience wrappers only.
- Every wrapper has a documented canonical lowering.
- The lowered request emits the same events, journal records, telemetry, retries, and typed failures as a hand-built request.
- Event subscription helpers lower into `AgentEventBus::subscribe_filtered` or its run/agent/all shorthands.
- Event streams yield `EventFrame { event, cursor, archive_cursor, overflow }`, not bare payloads, so observers can persist cursors and detect slow-subscriber gaps.
- Simple subscription helpers use conservative default `SubscriptionOptions`; advanced subscription helpers expose `SubscriberQueueConfig` for capacity, terminal reserve, and overflow policy.
- Hook helpers and hook config lower into `HookSpec` sidecars and executor refs inside the same `RuntimePackage`; they cannot install ambient callbacks after a run starts.
- Defaults are conservative: redacted content, local validation, bounded retries, finite timeouts, no ambient tools, no ambient isolation downgrade, agent-owned child cleanup on manual cancel, and no implicit detached processes.
- Advanced config can override limits and policy refs, but it cannot disable required lineage, local validation, or side-effect intent records.

## ID And Error Requirements

All durable and cross-boundary identity fields are typed newtypes:

- `RunId`, `TurnId`, `AttemptId`, `MessageId`, `ContextItemId`, `ContextProjectionId`
- `ToolCallId`, `ApprovalRequestId`, `AgentId`, `SubagentRunId`
- `RuntimePackageId`, `RuntimePackageFingerprint`, `EventId`, `HookId`, `ChildArtifactId`
- `TraceId`, `SpanId`, `ExecutionEnvironmentId`, `StreamRuleId`, `IsolatedProcessId`

`AgentError` must distinguish:

- invalid package
- invalid state transition
- provider failure
- projection failure
- tool failure
- approval failure
- policy denial
- journal failure
- telemetry failure
- isolation failure
- structured output failure
- stream rule failure
- subagent failure
- extension failure
- cancellation
- child lifecycle failure
- hook failure
- timeout
- recovery/repair needed

Every public error carries causal IDs when available and a typed retry classification:

- `Retryable`
- `NotRetryable`
- `RepairNeeded`
- `UserActionNeeded`
- `HostConfigurationNeeded`

## Slice Order

Phase 2 should implement contracts in this order. The MVP text gate is one fake-provider text run after steps 1-6. Step 7 completes the first-slice typed-output gate over the same run loop. Later steps are feature layers.

1. Domain IDs, errors, privacy, source/destination refs.
2. Event envelope, event bus filters, cursors, and golden event records.
3. In-memory journal and replay skeleton.
4. Runtime package snapshot/fingerprint.
5. Loop state machine with fake provider.
6. Projection/message/context.
7. Structured output and optional fake output delivery sink for the typed-output gate.
8. Tools and approval.
9. Stream rules.
10. Tool packs and isolation fake adapter.
11. Subagents and extension bridge.
12. Telemetry exporters and host adapter prototypes with fakes first.

## Core Vs Toolkit Crate Boundary

Core owns:

- IDs, messages, context items, lineage.
- Agent runtime and state machine.
- Event envelope and journal traits.
- Runtime package/capability snapshot.
- Provider/tool/approval/isolation/telemetry ports.
- Fake providers/tools/stores for tests.

Toolkit and adapter crates own:

- filesystem read/search/edit/write implementations.
- shell/PTY process implementation.
- document/image/archive/SQLite/URL readers.
- Apple Containerization, Docker, Firecracker, remote sandbox adapters.
- extension SDK packaging bridge.
- Product-specific external-runtime or trace-store adapters.

The core must accept external tool packs through contracts without linking their implementation crates.

## Acceptance Tests

- `api_exports_match_contract`
- `public_signature_support_types_are_exported`
- `ids_do_not_serialize_as_ambiguous_raw_strings`
- `agent_error_preserves_causal_ids`
- `retry_classification_is_typed`
- `agent_runtime_can_start_basic_fake_run_without_product_host_imports`
- `run_handle_exposes_events_cancel_and_final_result`
- `agent_sdk_core_builds_without_toolkit_features`
- `runtime_package_accepts_external_tool_pack_contract_without_core_tool_impl`
- `run_request_resolves_effective_runtime_package_per_run`
- `run_text_lowers_to_run_request`
- `run_typed_lowers_to_run_request_with_output_contract`
- `request_builder_and_explicit_run_request_emit_equivalent_events`
- `agent_on_hook_lowers_to_hook_spec_sidecar`
- `config_hook_and_code_hook_share_runtime_package_shape`
- `manual_cancel_cascades_to_agent_owned_children_by_default`
- `run_completion_preserves_explicitly_detached_process_when_policy_allows`
- `subscribe_all_lowers_to_event_bus_all_subscription`
- `subscribe_run_lowers_to_event_bus_run_filter`
- `subscribe_agent_lowers_to_event_bus_agent_filter`
- `subscribe_events_uses_compiled_filter`
- `subscribe_with_options_applies_queue_capacity_and_terminal_reserve`
- `event_subscription_defaults_to_envelope_or_redacted_summary`
- `event_stream_frame_exposes_cursor_and_overflow_notice`
- `api_contract_examples_have_no_product_specific_ids`
- `host_surface_constructors_are_absent_from_agent_sdk_core`
- `advanced_config_cannot_disable_required_lineage`

## Complete Example

Typed shape:

```rust
// Non-compiling contract sketch.
let agent = Agent::builder()
    .id(AgentId::new("agent.default"))
    .name(AgentName::new("Workspace Assistant"))
    .default_model_route(ModelRouteId::new("host.openai.default"))
    .default_context_policy(ContextPolicyRef::new("host.standard_chat"))
    .on(HookPoint::BeforeToolCall, AuditHook::new())
    .build()?;

let runtime = AgentRuntime::builder()
    .default_package(runtime_package.clone())
    .package_resolver(Arc::new(package_resolver))
    .providers(Arc::new(provider_registry))
    .tools(Arc::new(tool_registry))
    .approval(Arc::new(approval_broker))
    .journal(Arc::new(run_journal))
    .telemetry(TelemetryFanout::new(vec![otel_sink, host_sink]))
    .build()?;

let request = RunRequest {
    run_id: RunId::new(),
    source: SourceRef::new(SourceKind::Host, SourceId::new("surface.chat"))
        .with_correlation(CorrelationKey::new("conversation.example")),
    destination: DestinationRef::new(DestinationKind::User, DestinationId::new("surface.reply")),
    input: vec![AgentMessage::user_text("Summarize this workspace")],
    runtime_package: RuntimePackageRef::from_package(&runtime_package),
    output_contract: None,
    cancellation: CancellationPolicy::default(),
    host_metadata: HostMetadata::bounded(),
};

let handle: RunHandle = runtime.start_run(request).await?;
```

Core examples use product-neutral source and destination refs. Product-specific source/destination helpers such as desktop-chat constructors belong in host-adapter crates, not `agent-sdk-core`.

Simple API:

```rust
// Non-compiling contract sketch.
let answer = agent.run_text("Summarize this workspace.", &runtime).await?;
let todo: TodoExtraction = agent.run_typed::<TodoExtraction>("Extract a todo.", &runtime).await?;
```

Advanced API:

```rust
// Non-compiling contract sketch.
let request = agent
    .request("Extract a todo.")
    .source(
        SourceRef::new(SourceKind::Host, SourceId::new("surface.chat"))
            .with_correlation(CorrelationKey::new("conversation.example")),
    )
    .destination(DestinationRef::new(DestinationKind::User, DestinationId::new("surface.reply")))
    .runtime_package(RuntimePackageRef::from_package(&runtime_package))
    .output::<TodoExtraction>()
    .advanced(|cfg| cfg.timeout_ms(60_000).content_capture(ContentCaptureMode::Off))
    .build(&runtime)?;

let result = runtime.start_run(request).await?.wait().await?;
```

Canonical lowering:

- `Agent::run_text` lowers into `RunRequestBuilder` with no `OutputContract`.
- `Agent::run_typed::<T>` lowers into `RunRequestBuilder::output::<T>`.
- `RunRequestBuilder::output::<T>` lowers into `OutputContract::for_type::<T>`.
- `RunRequestBuilder::build` resolves a per-run `RuntimePackage`, validates it, stores its fingerprint, and returns a canonical `RunRequest`.
- Reserved hook helpers and config hooks lower into `HookSpec` sidecars and executor refs in the runtime package.
- Reserved child lifecycle builder options select or tighten package-declared policy refs before the run starts.
- `RunRequestBuilder::run` always returns `RunResult`.
- Typed extraction from a generic result is explicit through `RunResult::structured_output::<T>()` or `RunResult::into_typed_output::<T>()`.
- `RunRequestBuilder::run_typed::<T>` is shorthand for `run(...).await?.into_typed_output::<T>()`.

Equivalence:

- Simple, builder, and explicit `RunRequest` paths enter `AgentRuntime::start_run`.
- They use the same state machine, package fingerprint, events, journal records, policy checks, telemetry, and error variants.
- Typed output helpers add only `OutputContract` construction and typed result extraction after validation.

Replaceable ports:

- `ProviderRegistry`, `ToolRegistry`, `ApprovalBroker`, `RunJournal`, and `CheckpointStore` are MVP traits or registries; `TelemetrySink` and `IsolationRuntimeRegistry` are reserved optional ports.
- Optional crates can provide concrete tool packs, isolation runtimes, OTel export, or host adapters without changing `agent-sdk-core`.
- `AgentRuntimeBuilder` accepts fake ports first so contract tests can run without product-host imports.

Wiring:

1. Host builds `Agent` from product configuration.
2. Host builds or references a `RuntimePackage` snapshot for the run.
3. Host composes runtime ports and optional package resolver.
4. SDK resolves the effective package before the provider call, starts a run, and returns `RunHandle`.
5. Host consumes events or waits for `RunResult`.

Events:

- `RunStarted`
- `ContextAssembled`
- `ProviderRequestProjected`
- `ModelAttemptStarted`
- `RunCompleted` or `RunFailed`

Journal:

- `RunRecord`
- `HookRecord` when hooks are registered or invoked.
- `ChildLifecycleRecord` when child work is shutdown or detached.
- `TurnRecord`
- `ContextRecord`
- `ModelAttemptRecord`
- terminal `RunRecord`

Policies and failures:

- Invalid package fails before provider calls.
- Missing provider/tool/approval/isolation ports fail with typed `HostConfigurationNeeded` or `PolicyDenied`.
- Implicit child process orphaning is denied; explicit detach requires policy, intent, acknowledgement, and reclaim metadata.
- Telemetry sink failure records an event but does not fail the run.

SDK owns / Host owns:

- SDK owns typed IDs, runtime orchestration, event vocabulary, journal ports, and result/error shape.
- SDK owns hook points, child lifecycle policy semantics, and cancellation propagation.
- Host owns product configuration, UI routing, provider credentials, concrete stores, approval transport, concrete process/isolation adapters, detached child inspectors, and adapter installation.

Tests:

- `agent_runtime_can_start_basic_fake_run_without_product_host_imports`
- `run_handle_exposes_events_cancel_and_final_result`
- `agent_sdk_core_builds_without_toolkit_features`
