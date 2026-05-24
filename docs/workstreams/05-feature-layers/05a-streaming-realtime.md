# Goal 05a: Streaming Realtime

## Phase

[Phase 05: Feature Layers](README.md)

## Owner Role

[Streaming Realtime Rules](../_roles/05-streaming-realtime-rules.md)

## Parallelism

Parallel-safe with every other goal in Phase 05 after Phase 04 exits. Do not start Phase 06 until all Phase 05 goals finish.

## Required Reading

- `README.md`
- `docs/start-here.md`
- `coding_standards.md`
- `docs/workstreams/validation-gates.md`
- `docs/reference/sdk-review-checklist.md`
- `docs/architecture/primitive-map.md`
- phase README
- owner role doc
- owner role doc read-only inputs
- read-only inputs below

## Writable Files

- `docs/contracts/stream-rule-contract.md`

## Read-Only Inputs

- `docs/contracts/event-schema.md`
- `docs/contracts/tool-approval-contract.md`
- `docs/architecture/architecture-proposal.md`
- `docs/architecture/observability-and-lineage.md`
- `docs/examples/realtime-voice-workflow.md`

## Primitive Focus

- Stream deltas, rules, interventions, and realtime lifecycle are feature-layer state over `AgentEvent`, `RunJournal`, `RuntimePackage`, policy refs, and provider ports.
- A run is complete only after the event iterator drains and session/approval/compaction/output bookkeeping has reached terminal state.

## Must Not Own

Provider transport internals, UI rendering, product interruption UX, or durable truth outside the journal.

## Validation And Review

- Bounded matcher and repeat-state tests.
- Intervention intent/result and resume-state proof.
- Completion semantics explicitly separate final visible text from terminal run completion.
