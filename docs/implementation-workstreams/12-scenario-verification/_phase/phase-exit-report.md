# Phase 12 Exit Report: Scenario Verification

## Status

PASS.

## Scope Completed

Changed scenario surfaces:

- `crates/agent-sdk-core/tests/scenario_matrix.rs`
- `crates/agent-sdk-core/tests/scenarios/mod.rs`
- `crates/agent-sdk-core/tests/fixtures/scenarios/scenario-matrix-v1.json`

Changed API review surfaces:

- `crates/agent-sdk-core/src/lib.rs`
- `crates/agent-sdk-core/tests/public_api.rs`
- `crates/agent-sdk-core/tests/domain/public_api.rs`

Scenario coverage:

- Desktop/web chat approval maps `AgentRuntime`, `RunRequest`, `ContextAssembler`, `RuntimePackage`, `ProviderAdapter`, `ApprovalBroker`, `ToolExecutor`, and `OutputSink` while keeping UI, prompt copy, conversation storage, and display transport host-owned.
- CLI/headless approval maps source-scoped approval and denial behavior through SDK policy and approval primitives while terminal prompts, scheduler behavior, escalation transport, and approval UX remain host-owned.
- Remote output dedupe maps remote source/destination refs, output delivery, dedupe keys, journal replay, and output sink behavior while transport credentials, message persistence, ack lookup, and retry scheduling remain host-owned.
- Realtime/stream safeguard maps realtime sidecars, realtime adapters, stream deltas, stream interventions, approval, and content refs while media permissions, wake/listening UI, rendering, and token transport remain host-owned.
- Structured output maps `OutputContract`, validation, repair, stream rules, validated output, output delivery, and effect intent while schema UI, business scoring, form rendering, and sink credentials remain host-owned.
- Memory/context compaction maps contributions, assembly, projection audit, memory port, checkpoint store, and provider resume while memory backends, browsing UI, ingestion, and extension proposal source remain host-owned.
- Tool-pack/isolation repair maps tool packs, capabilities, hooks, isolation runtime, effect intent/result, and anti-entropy repair while installed packs, workspace policy, concrete runtimes, and repair scheduling remain host-owned.
- Subagent supervision maps agent pool, run messages, wake conditions, supervisor, request, package stripping, and context handoff while progress display, conversation promotion, route registry, and detached-child dashboards remain host-owned.
- Extension action boundary maps core capabilities, catalog snapshots, approval, effect intent/result, and extension action records while manifests, runtime, trust state, and host action adapters remain host-owned.
- Live-vs-durable event flow maps event bus, frames, cursors, journal, telemetry sink, and optional archive while display bridges, bounded stores, trace stores, and UI selectors remain host-owned.

API review coverage:

- Public facade exports are tested for `Agent`, `AgentRuntime`, `RunRequest`, `RunHandle`, `RunResult`, runtime packages, events, journal, context, output, policy, and ports.
- Crate-level rustdoc examples cover common helper lowering and explicit advanced `RunRequest` construction.
- Public API tests prove typed helpers lower into canonical `RunRequest` plus `OutputContract`.
- Helper and explicit request paths are tested for equivalent policy, provider, journal, event, validation, redaction, lineage, and terminal telemetry behavior.
- The crate root documents the SemVer posture for the supported import surface.

## DDD, Mockability, And Product-Neutrality Evidence

- `scenario_matrix.rs` and `public_api.rs` are two-line root shims into domain-oriented test folders.
- Scenario tests use fake, scripted, or in-memory ports only.
- The scenario matrix explicitly names host-owned boundaries for UI, prompts, transports, credentials, stores, schedulers, route registries, and concrete runtimes.
- API tests use `FakeProvider`, `FakeJournalStore`, `InMemoryAgentEventBus`, and `FakeContentResolver` to prove consumer-testable helper behavior.
- Product-specific public facade and scenario fixture audits returned no matches.

## Validation Evidence

- `cargo test -p agent-sdk-core --test scenario_matrix --test public_api` passed.
- `cargo test -p agent-sdk-core --doc` passed.
- `cargo test -p agent-sdk-core --no-default-features` passed.
- `cargo test --workspace` passed.
- `cargo fmt --check` passed.
- `git diff --check` passed.
- `find crates/agent-sdk-core/src -maxdepth 1 -type f -not -name lib.rs -not -name README.md -print` returned no files.
- `find crates/agent-sdk-core/tests -maxdepth 1 -type f -name '*.rs' -exec wc -l {} + | sort -n` confirmed all root integration tests are two-line shims.
- Product/provider denylist audit over crate docs, scenario fixtures, and Phase 12 docs returned no matches outside this evidence report.

## Independent Review

Independent reviewer Einstein returned BLOCKED because the scenario durability
gate did not cover every fake/scripted effect execution. The blocker was fixed
by deriving executed effect kinds from the steps themselves, requiring both
`FakePortCall` and approval-dispatch steps with effects to have prior matching
journal intents, and adding the missing provider-request and approval-dispatch
intent steps to the scenario matrix.

Einstein re-reviewed the fix and returned PASS. Phase 12 may advance to Phase 13.
