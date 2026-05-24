# Goal 02b: Events Journal Kernel

## Phase

[Phase 02: Primitive Kernel](README.md)

## Owner Role

[Events Journal Replay](../_roles/02-events-journal-replay.md)

## Parallelism

Parallel-safe with every other goal in Phase 02 after Phase 01 exits. Do not start Phase 03 until all Phase 02 goals finish.

## Required Reading

- `README.md`
- `docs/start-here.md`
- `coding_standards.md`
- `docs/workstreams/validation-gates.md`
- `docs/reference/sdk-review-checklist.md`
- `docs/architecture/primitive-map.md`
- phase README
- owner role doc
- read-only inputs below

## Writable Files

- `docs/contracts/event-schema.md`
- `docs/contracts/journal-replay-schema.md`

## Read-Only Inputs

- `docs/contracts/api-contracts.md`
- `docs/contracts/runtime-package-schema.md`
- `docs/contracts/context-memory-contract.md`
- `docs/contracts/tool-approval-contract.md`
- `docs/contracts/output-delivery-contract.md`
- `docs/architecture/observability-and-lineage.md`

## Primitive Focus

- Add generic `EntityRef`, `subject_ref`, `related_refs`, and `causal_refs` so new features do not add endless optional envelope IDs.
- Keep hot-path indexed fields universal.
- Add `EffectIntent` and `EffectResult` as a shared side-effect vocabulary across tools, output delivery, memory writes, extension actions, child starts, provider calls, and process actions.
- Keep live events distinct from durable journal truth.

## Must Not Own

Feature-specific payload semantics, telemetry sink storage, host display events, global workflow engines, or raw content capture policy.

## Validation And Review

- Future tests/fixtures: golden event envelope fixtures, journal record fixtures, replay/resume fixtures, and filter table tests.
- Docs audit: every event/journal addition must preserve live-event versus durable-journal separation.
- Event filters use envelope/index fields, not payload parsing.
- Every side effect has intent-before-effect and terminal result record coverage.
- Replay and resume use journal records and content refs, not live event assumptions.

## Validation Evidence

Changed files:

- No event/journal contract edits were required for this phase pass; `docs/contracts/event-schema.md` and `docs/contracts/journal-replay-schema.md` were reviewed as the authoritative Phase 02 outputs.

Tests/fixtures:

- No Rust source, package manifests, executable tests, or fixtures were created; this was a documentation-only contract pass.
- Future golden event, journal, replay, queue, and redaction tests remain named in this goal and [02 Events Journal Replay](../_roles/02-events-journal-replay.md).

Commands run:

- `git diff --check`
- local Markdown link audit over all `.md` files
- no-code audit for `.rs`, `Cargo.toml`, executable tests, and fixtures
- Phase 02 writable-scope audit
- product-neutrality keyword audit over added lines
- primitive/no-mini-SDK audit over Phase 02 contracts

Skipped tests and why:

- Golden fixtures and replay tests are skipped because the Rust crate and fixture tree do not exist yet.

Events/journal/telemetry touched:

- Existing event and journal contracts already define `EntityRef`, `subject_ref`, `related_refs`, `causal_refs`, event frames, cursor compatibility, archive boundaries, `EffectIntent`, `EffectResult`, replay modes, checkpoints, anti-entropy, and live-versus-durable separation.
- Telemetry remains derived from events/journals and is not durable run truth.

SDK-owned boundaries preserved:

- SDK owns event family/kind strings, event frame/cursor semantics, run-scoped journal replay, journal envelopes, effect intent/result semantics, and replay reducers.

Host-owned boundaries preserved:

- Host owns sink retention, UI display events, trace-store schema, global event archive implementation, external workflow orchestration, unsafe repair approval, and physical storage.

Primitive-lowering evidence:

- Event filters stay envelope/index based. Durable replay uses `RunJournal`/`JournalCursor`; global filtered durable replay requires optional `EventArchive`/`IndexedJournalView`.
- Side effects map through `EffectIntent` and `EffectResult`; no feature-specific ledger was accepted.

Simplicity notes:

- No alternate event bus, journal, archive guarantee, workflow engine, telemetry truth store, or payload-parsing hot path was introduced.

Cross-cutting proposal blocks:

- Accepted: keep `EventArchive` optional and keep core durable replay run-scoped unless an indexed host/archive port is configured.
- Rejected: none.
- Deferred: emitted-kind fixture files and schema DTO implementation are blocked until crates and fixture layout exist.

## Review Packet

Primitive decision:

- Reused kernel primitives: `AgentEvent`, `EventEnvelope`, `EventFrame`, `EventCursor`, `EntityRef`, `RunJournal`, `JournalCursor`, `EffectIntent`, `EffectResult`, `ContentRef`, and typed IDs.
- New feature-layer primitives: none.
- New capability variants: none.
- Host-owned behavior kept out: UI event stores, global workflow engines, trace-store schemas, raw content capture policy, and physical journal storage.

Validation evidence:

- Contract/unit tests: future tests named; no code exists yet.
- Golden fixtures: future fixture matrix named; not created in this documentation-only pass.
- Smoke/scenario tests: not applicable until crates exist.
- Docs audits: link, writable-scope, no-code, product-neutrality, and primitive/no-mini-SDK audits.

Reviewer checklist:

- Simplicity: PASS, one live event stream plus one durable journal path.
- Product-neutrality: PASS, no product-specific references added.
- Event/journal durability: PASS, live events remain distinct from durable journal truth.
- Privacy/redaction: PASS, default payload access is envelope/redacted-summary/content-ref based.
- Replay/idempotency: PASS, cursor/replay modes and side-effect reconciliation remain explicit.
- Capability fingerprint impact: PASS, no capability variants or package fingerprint fields changed.
