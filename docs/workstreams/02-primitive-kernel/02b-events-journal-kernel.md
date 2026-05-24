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
