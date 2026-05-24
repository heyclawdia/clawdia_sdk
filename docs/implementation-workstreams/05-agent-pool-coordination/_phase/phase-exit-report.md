# Phase 05 Agent Pool Coordination Exit Report

## Status

PASS.

## Launch Targets

- `05a-agent-pool-coordination.md`: implemented locally after the Phase 05 worker handoff returned an incomplete skeleton.

## Implemented Scope

- Added `AgentPool`, `AgentPoolBuilder`, `AgentPoolMember`, `RunAddress`, `RunMessage`, `MessageReceipt`, `MessageStatus`, `WakeCondition`, and `WakeRegistration` under the `application` responsibility folder.
- Added `AgentPoolId`, `TopicId`, and `WakeConditionId` through the shared typed-ID pattern.
- Added agent-pool/topic/wake-condition ref vocabulary so the new feature has explicit domain language instead of overloading message or agent refs.
- Added agent-pool event kinds and journal payload records for pool lifecycle, run-message delivery, and wake conditions.
- Added feature-layer contract tests plus root Cargo shim, with golden fixtures under `tests/fixtures/agent_pool/`.
- Added lifecycle golden fixtures for emitted pool-created/run-joined events and `AgentPoolRecord` payloads.
- Added duplicate wake-registration test coverage proving idempotency returns the first registration without duplicate wake records.
- Updated P0 journal summary helper for the new `JournalRecordPayload` variants so later feature records do not break existing integration tests.

## Primitive-Lowering Evidence

- `AgentPool::start_run` delegates to `AgentRuntime::start_run` and then records membership; it does not own a second run loop.
- `AgentPool::send` resolves `RunAddress` against current pool members and records `RunMessageRecord` plus `EffectIntent`/`EffectResult` metadata using `ContentRef`, `PolicyRef`, `SourceRef`, `DestinationRef`, and `EntityRef`.
- `AgentPool::subscribe` compiles caller `EventFilter` values after intersecting run visibility with pool membership and policy.
- `WakeCondition` compiles to envelope-only `EventFilter` matching; tests assert indexed envelope fields and avoid payload parsing on the hot path.
- Pool timeout records a timed-out wake without calling runtime cancellation.

## Mockability And Testability Evidence

- Tests use `FakeJournalStore`, `InMemoryAgentEventBus`, `FakeContentResolver`, and `FakeProvider`; no network, clock, random, process, or remote service dependency is required.
- SDK consumers can reuse the same fake ports and fixture harness to test message delivery, visibility, wake timeout, and redaction behavior.
- Golden fixtures prove typed ID serde, pool lifecycle event/journal records, run-message event/journal records, and wake event/journal records.
- The redaction assertion checks that message bodies stay behind `ContentRef` by default.

## Package Architecture Evidence

- New source implementation lives under `crates/agent-sdk-core/src/application/agent_pool.rs`.
- Domain vocabulary changes are in `domain/ids.rs` and `domain/refs.rs`.
- Durable/observable record changes are in `records/event.rs` and `records/journal.rs`.
- Public root changes are facade wiring/re-exports only in `lib.rs` and `domain/mod.rs`.
- Integration tests live under `tests/feature_layers/agent_pool_contract.rs`; `tests/agent_pool_contract.rs` is a root Cargo shim only.

## Validation

- `cargo fmt --check` PASS.
- `cargo test -p agent-sdk-core --test agent_pool_contract` PASS, 12 tests.
- `cargo test -p agent-sdk-core` PASS, full suite.
- `cargo test -p agent-sdk-core --no-default-features` PASS, full suite.
- `cargo tree -p agent-sdk-core --no-default-features` PASS; dependencies remain `serde`, `serde_json`, `sha2`, and `thiserror` plus transitive dependencies.
- Source-root architecture audit PASS: no files under `src/` root other than `lib.rs` and `README.md`.
- Root Cargo test shim audit PASS: no `#[test]` or test function bodies in `crates/agent-sdk-core/tests/*.rs`.
- Product-neutrality audit PASS for Phase 05 surfaces.
- No-workflow-engine source audit PASS for `agent_pool.rs`; the contract test also asserts forbidden workflow/DAG/barrier/schedule/compensation public types are absent.

## Independent Review Findings

- Initial reviewer verdict: BLOCKED.
- Finding 1: missing golden coverage for emitted pool lifecycle events and `AgentPoolRecord` coverage. Fixed with `pool_lifecycle_created_and_joined_records_have_golden_fixtures` and `pool-lifecycle-*.json` fixtures.
- Finding 2: wake idempotency implemented but not proven. Fixed with `duplicate_wake_registration_is_deduped_by_idempotency_key`.
- Re-validation after fixes: `cargo fmt --check`, `cargo test -p agent-sdk-core --test agent_pool_contract`, full default test suite, no-default test suite, tree, architecture audits, product-neutrality audit, and no-workflow source audit passed.
- Re-review verdict: PASS.

## Files Changed By This Phase

- `docs/implementation-workstreams/05-agent-pool-coordination/05a-agent-pool-coordination.md`
- `docs/implementation-workstreams/05-agent-pool-coordination/_phase/phase-exit-report.md`
- `crates/agent-sdk-core/src/application/agent_pool.rs`
- `crates/agent-sdk-core/src/domain/ids.rs`
- `crates/agent-sdk-core/src/domain/refs.rs`
- `crates/agent-sdk-core/src/domain/mod.rs`
- `crates/agent-sdk-core/src/records/event.rs`
- `crates/agent-sdk-core/src/records/journal.rs`
- `crates/agent-sdk-core/src/lib.rs`
- `crates/agent-sdk-core/tests/agent_pool_contract.rs`
- `crates/agent-sdk-core/tests/feature_layers/agent_pool_contract.rs`
- `crates/agent-sdk-core/tests/fixtures/agent_pool/*.json`
- `crates/agent-sdk-core/tests/p0/p0_text_run.rs`

## Carry-Forward Watchpoints

- Wake matching currently uses the configured live event bus and can match already-buffered fake events. Before archive-backed replay hardening, decide whether production wake registration should require a registration cursor, opt-in replay, or both.
- `InMemoryAgentEventBus` still has the Phase 04 carry-forward: before true concurrent publishers, make sequence assignment and append ordering atomic together or add a concurrent publisher stress test.
- If later phases add topic mutation APIs, keep topic membership policy-gated and journal-backed; do not let topic fan-out become a hidden workflow or mailbox engine.

## Reviewer

Zeno: PASS.
