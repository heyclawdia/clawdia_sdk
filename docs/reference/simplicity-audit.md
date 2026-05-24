# Agent SDK Simplicity Audit

This audit reviews the current standalone SDK packet from a simplicity perspective. The goal is not to remove capability. The goal is to make the common path small while preserving observability, durability, policy, privacy, lineage, and extensibility underneath.

## Verdict

The architecture is feature-rich but mostly defensible because the hard parts are real: events, journal replay, policy, isolation, extensions, subagents, and host boundaries all need explicit contracts. The main simplification opportunity is not deleting features. It is enforcing a small MVP primitive kernel, layering features over that kernel, and keeping advanced-only escape hatches out of the common path.

## Simplification Decisions To Preserve

| Area | Keep simple by default | Keep explicit underneath |
| --- | --- | --- |
| Running an agent | `agent.run_text`, `agent.run_typed`, `agent.stream` | `RunRequest`, `RuntimePackage`, policy, journal, events, telemetry |
| Events | `runtime.subscribe_run`, `subscribe_agent`, `subscribe_events` | `AgentEventBus`, `EventFrame`, cursors, queue options, archive replay |
| Structured output | typed model derives/looks up schema | `OutputContract`, validation policy, repair policy, schema refs |
| Tools | opt-in `ToolPack` presets | tool specs, handlers, approval, effect lineage, isolation |
| Isolation | `IsolationRequirement::at_least(IsolationClass::Sandbox).prefer("adapter.ref")` builder | capability reports, mount/network/secrets/process lifecycle |
| Telemetry | default redacted usage/cost sink | OTel mapping, content policy, export health, cost correction |
| Extensions | manifest plus typed SDK helpers | JSON-RPC protocol, capability gating, browser-safe subpaths |
| Subagents | parent starts child with bounded request | topology, package stripping, mailbox, event wrapping, rollup |
| Output delivery | destination ref plus optional sink | delivery policy, dedupe key, intent/result records, host channel receipts |

## Opportunities To Simplify

### 0. Split MVP kernel from reserved feature ports

Status: adopted in active contracts.

The first Rust slice should prove one fake-provider text or typed run. It should not require concrete isolation adapters, extension bridges, subagents, realtime providers, full tool packs, telemetry exporters, or global event archive replay.

Keep in MVP:

- `Agent`, `AgentRuntime`, `RunRequest`, `RunHandle`, `RunResult`
- `RuntimePackage` with the first-slice `CapabilitySpec` profile
- `AgentMessage`, `ContextItem`, `ContextProjection`, `OutputContract`, `ValidatedOutput`
- `AgentEvent`, `EventFrame`, `EventCursor`, `RunJournal`, `JournalCursor`
- provider/fake tool/approval/output sink ports, source/destination refs, policy refs, privacy classes, and typed IDs

Reserve for feature workstreams:

- concrete isolation, extension, subagent, stream-rule, realtime, telemetry-export, full tool-pack, and global archive behavior.

### 1. Treat advanced event options as `SubscriptionOptions`

Status: adopted in contracts.

`SubscriberOverflowPolicy`, `SubscriberQueueConfig`, payload access, and archive replay are powerful but noisy. Normal users should call `subscribe_run` or `subscribe_agent` with defaults. Power users use `*_with_options` or compiled filters.

Keep:

- `EventFrame`
- `EventCursor`
- `ArchiveCursor`
- `EventArchive`
- `SubscriptionOptions`

Avoid:

- putting every queue and payload knob on the simple subscription methods;
- requiring users to understand archive replay for normal live observation.

### 2. Keep one canonical lowering path for ergonomics

Every simple API should lower into canonical contracts:

- `run_text` -> `RunRequest`
- `run_typed::<T>` -> `RunRequest` plus `OutputContract::for_type::<T>`
- `StreamRule::mask_regex` -> full `StreamRule`
- `IsolationRequirement::at_least(...).prefer(...).fallback(...)` -> `EnvironmentSpec`
- `ToolPack::workspace_readonly` -> `RuntimePackage` entries

Do not add separate "easy runtime" or "quick event bus" behavior.

### 3. Use presets for common policy bundles

Some contracts are necessarily detailed, especially structured output, telemetry privacy, stream rules, and isolation. Keep the details but make common use one-liners:

- `OutputContract::strict_json_schema::<T>()`
- `OutputContract::fast_lenient::<T>()`
- `TelemetryPolicy::redacted_defaults()`
- `StreamRule::stop_on_regex(...)`
- `IsolationPolicy::require_container()`

Presets must be documented as lowering to explicit structs.

### 4. Keep archive replay optional

Status: adopted in contracts.

Core run replay uses `JournalCursor`. Cross-run, all-agent, and arbitrary filtered durable replay use optional `EventArchive` with `ArchiveCursor`.

This prevents a global event database from becoming a hidden core requirement.

### 5. Avoid "manager" growth

Several primitives could turn into grab bags if implemented carelessly:

- `AgentRuntime`
- `SessionManager`
- `TelemetryFanout`
- `AntiEntropyJob`
- `ExtensionHost`

Rule: each must stay a coordinator over typed ports, not own policy, storage, UI, provider credentials, or product workflows.

### 5a. Keep `CapabilitySpec` typed

Status: adopted in runtime-package contract.

`CapabilitySpec` should be the package entry point, not the place where feature-specific blobs accumulate. Every new variant must point to a typed sidecar contract, owner role, fingerprint fields, emitted events, journal records, and acceptance tests.

### 6. Move workflow behavior out of core

Core should make "when two agents finish, spawn another" easy with event filters and idempotent `start_run`, but `wait for N`, barriers, DAGs, schedules, compensation, and durable trigger state belong in optional workflow crates or hosts.

### 7. Keep host scenarios concrete and product-neutral

Scenario examples are valuable because they stress the SDK. They must stay under `docs/examples`, use product-neutral IDs, and call out host-owned surfaces without naming a specific product adapter as implementation authority.

## Things Not To Simplify Away

- Event envelopes and typed filters. They are required for fast observability and orchestration.
- Run journal side-effect intent/result records. They are required for recovery and replay safety.
- Runtime package fingerprints. They prevent provider-visible schemas and executable registries from drifting.
- Policy layer separation. Permission, sandbox, approval, autonomy, and escalation answer different safety questions.
- Isolation capability reports. Without them fallback becomes unsafe guesswork.
- Local structured-output validation. Provider-native schema support is not authority.
- Redaction/content policy. Privacy defaults are platform-level behavior, not optional polish.

## Contract-Specific Simplicity Notes

| Contract | Simplicity action |
| --- | --- |
| API contracts | Keep simple/builder/advanced layers. Do not add a fourth surface. |
| Event schema | Keep simple subscription helpers; keep queue/archive controls advanced. |
| Run handle/reconnect | Keep `RunHandle::stream_from` as run convenience; runtime subscriptions are the broader primitive. |
| Runtime package | Provide builders/presets so callers do not hand-build snapshots for common cases. |
| Context/memory | Keep memory optional and projection-owned; no shadow memory transcript. |
| Journal/replay | Keep replay modes finite. Avoid general query language in core. |
| Structured output | Keep Pydantic-like typed path first; schema registry stays underneath. |
| Stream rules | Provide literal/regex helpers; advanced channel/repeat/privacy options remain explicit. |
| Tools/approval | Provide opt-in packs; never ambient tools. |
| Isolation | Builder syntax is good; no silent downgrade. |
| Subagents | Parent-owned child run is the primitive; no free-form agent society in core. |
| Extension SDK | Keep browser-safe helpers and Node ESM smoke tests; marketplace remains host-owned. |
| Telemetry/privacy | Redacted defaults first; raw content and sink-specific settings advanced. |
| Output delivery | Destination and sink are primitives; product channel UX stays host-owned. |

## Follow-Up Watchpoints

- If implementation agents add a new public type, ask whether it is a real primitive or a configuration detail that belongs inside a builder/options struct.
- If a workstream adds a new capability variant, require a typed sidecar contract and primitive-lowering review.
- If a contract adds a new cursor, specify scope and persistence immediately.
- If an ergonomic helper is added, add a lowering test in the same slice.
- If a fake adapter becomes complex, split the fake by contract rather than making one mega fake.
- If a workstream needs a shared rename, route it through stitching rather than editing architecture docs directly.
