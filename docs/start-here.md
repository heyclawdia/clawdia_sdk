# Start Here

This packet describes a Rust-first `agent_sdk` crate family and extension SDK layer.

It is intentionally standalone and product-neutral. The SDK should support demanding host workflows such as desktop chat, CLI/headless runs, realtime voice, remote channels, external runtimes, extensions, telemetry, and subagents without inheriting any one product's UI, trace store, process cache, marketplace, or workflow assumptions.

## Current Packet

- The authoritative Agent SDK packet lives in `<repo-root>`.
- The current checkout includes Rust crates under `crates/agent-sdk-core`, `crates/agent-sdk-eval`, `crates/agent-sdk-toolkit`, the optional `crates/agent-sdk-provider`, and the unpublished convenience facade `crates/clawdia-sdk`; older docs-only workstream reports are historical contract evidence, not a statement that no code exists today.
- Product-specific host-adapter material is not part of the active SDK handoff; active examples use generic host scenarios only.
- Normative implementation contracts live in [contracts](contracts/README.md).
- Completed contract-packet ownership lives in [workstreams](workstreams/README.md).
- Rust implementation launch sequencing and phase exit evidence live in [implementation-workstreams](implementation-workstreams/README.md).

## Agent Crawl Order

Use this order when an agent needs to build on the SDK instead of only reviewing
the old contract packet:

1. Read `<repo-root>/AGENTS.md`, `<repo-root>/README.md`, this file, and `<repo-root>/coding_standards.md`.
2. Confirm the current phase or task owner in [implementation-workstreams](implementation-workstreams/README.md).
3. For user-facing API ergonomics, read [API Review](implementation-workstreams/12-scenario-verification/12b-api-review.md), [Simplicity Audit](reference/simplicity-audit.md), `../crates/agent-sdk-core/README.md`, `../crates/agent-sdk-core/tests/domain/public_api.rs`, and `../crates/clawdia-sdk/README.md`.
4. For checkout-based app construction, prefer `clawdia_sdk::prelude::*` when using the unpublished facade. Split-crate users should use `agent_sdk_core::prelude::*` for common core types, explicit crate-root imports for advanced surfaces, `agent_sdk_core::ports` for host adapters, and `agent_sdk_core::testing` for deterministic conformance checks.
5. Before handoff, run the commands named by the launch target plus `scripts/public-release-audit.sh` for any release or broad documentation handoff.

## Navigation

| Note | Purpose |
| --- | --- |
| [../coding_standards.md](../coding_standards.md) | Root coding standards entry point for SDK implementation agents. |
| [architecture/coding-standards.md](architecture/coding-standards.md) | SDK engineering standards, testing discipline, performance rules, and observability requirements. |
| [agent-sdk-toolkit/README.md](agent-sdk-toolkit/README.md) | Optional adapter/toolkit roadmap for live providers, OpenAI-compatible providers, ACP, MCP, isolation runtimes, browser/web access, and local MLX/llama.cpp acceleration. |
| [architecture/external-sdk-lessons.md](architecture/external-sdk-lessons.md) | Lessons from Strands, Cursor, Claude Agent SDK, Pi, oh-my-pi, Apple Containerization, and OpenTelemetry GenAI conventions. |
| [architecture/primitive-map.md](architecture/primitive-map.md) | First-principles primitive map: ownership, responsibilities, methods, and non-ownership boundaries. |
| [architecture/observability-and-lineage.md](architecture/observability-and-lineage.md) | Source, destination, metadata, context-injection, stable event taxonomy, journal/replay, trace, privacy, telemetry, cost, and multi-agent lineage model. |
| [architecture/architecture-proposal.md](architecture/architecture-proposal.md) | Main middle-level proposal with module layout, flows, diagrams, and conceptual Rust skeletons. |
| [architecture/coverage-gap-matrix.md](architecture/coverage-gap-matrix.md) | Current coverage gaps and implementation candidates. |
| [contracts/README.md](contracts/README.md) | Normative implementation contracts for API, events, loop, package, journal/replay, policy, tools, isolation, extension, and telemetry. |
| [examples/README.md](examples/README.md) | Mermaid-heavy scenario examples for complex host workflows and SDK boundaries. |
| [workstreams/README.md](workstreams/README.md) | Completed phase-gated contract packet: what ran first, what ran in parallel, owner roles, and what closed each gate. |
| [implementation-workstreams/README.md](implementation-workstreams/README.md) | Rust coding launch map: phase dependencies, parallel-safe launch targets, implementation exit gates, and phase reports. |
| [workstreams/validation-gates.md](workstreams/validation-gates.md) | Shared validation levels, required evidence, and target commands for every workstream. |
| [reference/feature-to-primitive-matrix.md](reference/feature-to-primitive-matrix.md) | Feature-to-primitive mapping and primitive decision ladder used by Phase 00, Phase 01, and Phase 02. |
| [reference/persistence-ownership-map.md](reference/persistence-ownership-map.md) | Concrete store boundary map for journals, checkpoints, content, event cursors, agent pools, tool execution, and provider arguments. |
| [reference/dx-gap-report-agents-sdk.md](reference/dx-gap-report-agents-sdk.md) | SDK DX direction report for first-user ergonomics, install packaging, tool authoring, examples, and optional facade work. |
| [reference/facade-crate-proposal.md](reference/facade-crate-proposal.md) | Current `clawdia-sdk` convenience facade shape and carry-forward proposal boundaries. |
| [reference/dx-upgrade-risk-watchpoints.md](reference/dx-upgrade-risk-watchpoints.md) | Carry-forward risks for future facade, tool macro, install packaging, persistence, and example work. |
| [reference/sdk-review-checklist.md](reference/sdk-review-checklist.md) | SDK review rubric for simplicity, product-neutrality, observability, durability, privacy, and API quality. |
| [reference/simplicity-audit.md](reference/simplicity-audit.md) | Simplicity audit identifying what to default, merge, keep advanced-only, or preserve as essential complexity. |
| [reference/open-questions-and-ambiguities.md](reference/open-questions-and-ambiguities.md) | Decision register for first Rust-slice resolved decisions, deferred details, and non-questions. |

## Design Posture

The `agent_sdk` should be a reusable core for agent products. It should not become a host app, terminal UI, web UI, coding agent, remote messaging product, workflow engine, dashboard, deployment platform, marketplace, or self-improvement product. Those products should be built on top through typed adapters, runtime packages, policies, hooks, and event streams.

Host workflows are coverage constraints only. They prove what the SDK must make possible. They are not architecture to copy.

Product features such as proposal scoring, benchmark UI, product-specific recommendations, or automatic mutation policy stay outside the SDK. The SDK can provide generic primitives for lineage, journaled side effects, approval, replay, recovery, and inverse-candidate metadata without owning product workflows.

## Ergonomics Posture

The SDK should feel simple for normal usage and explicit for power users. Common operations should have one-line helpers and defaulted builders:

- `agent.run_text(...)` for normal text output.
- `agent.run_typed::<T>(...)` for Pydantic-like typed output where `T` provides or derives a schema.
- `agent.on(HookPoint::BeforeToolCall, hook)` or declarative hook config for lifecycle hooks that lower into `HookSpec`.
- `RuntimePackage::for_agent(...).tool_pack(...)` for common package composition.
- `StreamRule::mask_regex(...)` for common streaming safeguards.
- `ExecutionEnvironment::require(IsolationRequirement::at_least(IsolationClass::Sandbox).prefer("adapter.ref"))` for common isolated workloads.

Those helpers are thin. They lower into the same canonical contracts as advanced usage: `RunRequest`, `OutputContract`, `RuntimePackage`, `StreamRule`, `EnvironmentSpec`, `SubagentRequest`, `CoreExtensionCapabilities`, events, journal records, and policy checks. Simple APIs cannot bypass local validation, approval, redaction, lineage, or side-effect intent records.

## Primitive Kernel

The first implementation slice should use explicit readiness profiles so reserved feature contracts do not accidentally become MVP requirements:

| Profile | Proves | Must not require |
| --- | --- | --- |
| P0 text run | one fake-provider text run through the run loop, context projection, events, journal, and provider port | tools, approvals, isolation, extensions, telemetry exporters, subagents, realtime, or typed output |
| P1 typed output | `OutputContract` lowering, local validation, repair accounting, and typed result extraction over the same P0 loop | tool execution or output-channel delivery as a prerequisite |
| P2 side effects | tool/approval execution using the shared package, policy, event, journal, and `EffectIntent` spine | a second run loop, event stream, journal, package registry, policy path, or side-effect path |

The MVP Rust slice should stay small enough to prove P0, then P1:

- `Agent`, `AgentRuntime`, `RunRequest`, `RunHandle`, and `RunResult` for run lifecycle.
- `RuntimePackage`, the first-slice `CapabilitySpec` profile, typed sidecars, and source-qualified catalog snapshots for immutable per-run package state.
- `AgentMessage`, `ArtifactRef`, `ContentRef`, `ContextContribution`, `ContextItem`, `ContextProjection`, `OutputContract`, and `ValidatedOutput` for content refs, context admission/projection, and typed output.
- `EffectIntent`, `EffectResult`, `IdempotencyKey`, and `DedupeKey` for side effects.
- `AgentEvent`, `EventFrame`, `EventFilter`, `EventCursor`, `RunJournal`, `JournalRecord`, and `JournalCursor` for live observation and durable truth.
- Typed ports for provider projection/streaming, optional fake output delivery, and storage.
- `EntityRef`, `SourceRef`, `DestinationRef`, `PolicyRef`, `PrivacyClass`, `RetentionClass`, `TrustClass`, and typed IDs for lineage across every boundary.

Everything else layers over that kernel. Tool packs install callable package capabilities plus typed tool sidecars. Hooks are package sidecars plus ordered executors. Stream rules are package sidecars plus journaled interventions. Memory, tools, skills, host input, remote channels, subagents, and compaction create context candidates; only policy-admitted items become provider context. Isolation is a typed environment requirement plus adapter port. Subagents are parent-owned child runs. Extensions declare core capabilities that a host may resolve into a package. Telemetry is a projection, not durable truth. Output delivery is a sink port with journaled intent and dedupe, not product channel UX.

Reserved feature ports may exist as traits or contract sketches before implementation, but they are not required to run in the MVP slice: telemetry exporters, isolation adapters, extension bridges, subagents, realtime providers, stream rules, full tool packs, and global event archive replay.

## Core Thesis

An agent loop repeatedly coordinates input, context, model calls, model output, tools, tool results, memory/context updates, events, telemetry, and control decisions. A good SDK should make those transitions explicit, typed, observable, resumable, and replaceable.

The proposal centers on:

- An explicit `AgentLoop` state machine rather than an opaque callback chain.
- A typed `RuntimePackage` snapshot so provider-visible schemas and executable registries cannot drift.
- Opt-in SDK tool packs for common read/search/edit/write/shell/resource-reader behavior, always controlled by host policy.
- A `StreamRuleEngine` for stop-on-regex, abort-and-retry, mask, approval, and emit-only interventions during streaming.
- `ExecutionEnvironment` and `IsolationRuntime` primitives so tool and subagent workloads can run in policy-selected host, container, VM, or remote sandboxes.
- A lossless internal `AgentMessage`/`ContextItem` model plus provider-specific projection.
- First-class `AgentEventBus` lifecycle subscriptions, filters, frames, cursors, and `TelemetrySink` streams from day one.
- Message and context lineage for source, destination, injection path, policy, sensitivity, retention, trace, run, turn, and parent/child agent relationships.
- Host-owned approval, escalation, extension runtime, and UI routing boundaries.
- First-class lifecycle hooks with typed mutation rights, non-blocking/redacted defaults, and shared config/code lowering.
- Run child lifecycle policies so manual stop cleans up agent-owned work by default while explicit detach remains observable and journaled.
- Backpressure-aware streaming and realtime execution.
- Durable sessions, checkpoints, run journals, replay, idempotency, and reconnect.
- Parent-owned, depth-bounded subagents with explicit context and tool policy.

## Non-Goals

- Do not replace or migrate any existing product runtime in this phase.
- Do not introduce product-specific runtime paths, UI adapters, trace stores, or marketplace behavior into core.
- Do not design every final API detail before implementation feedback.
- Do not make provider-specific transport details the core abstraction.
- Do not let observability require raw prompt/tool/model content capture by default.
- Do not let extensions, UI components, or remote channels become shadow owners of memory, approvals, or run state.
- Do not provide self-improvement UX, proposal scoring, benchmark/evaluation product flows, or product-specific change recommendation logic.
