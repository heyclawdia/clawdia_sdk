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

## Validation Evidence

- Worker agent: Dirac (`019e5882-0304-7403-9657-4f98501a21fe`).
- Changed file: `docs/contracts/stream-rule-contract.md`.
- Scoped docs audit confirmed the contract layers stream/realtime behavior over `RuntimePackage` sidecars, `StreamDelta`, `AgentEvent`, `RunJournal`, `PolicyRef`, typed refs, `EffectIntent` / `EffectResult`, and provider/realtime ports.
- Named future matcher, intervention, realtime, redaction, journal, event, and OTel projection fixtures without creating executable fixtures in this documentation-only phase.
- Cross-cutting proposals sent to stitching: accept `RealtimeSessionRecord`, keep stream interventions mapped through existing effects instead of a new `EffectKind`, and close stream/realtime OTel deferrals with named fixtures.
- No Rust source, package manifests, executable tests, or fixtures were created.

## Review Packet

- Primitive decision: stream rules and realtime sessions are feature-layer sidecars over the kernel, not a second run loop or provider callback path.
- SDK-owned boundaries preserved: channel/cursor semantics, bounded matchers, intervention records, realtime lifecycle records, repeat state, completion gating, redaction defaults, and event/journal names.
- Host-owned boundaries preserved: provider credentials/transport internals, realtime UX, microphone/rendering surfaces, custom matcher sandbox, rule authoring UI, approval UI, and output sink implementation.
- Reviewer checklist: PASS for simplicity, product-neutrality, event/journal durability, privacy/redaction, replay/idempotency, completion semantics, and capability fingerprint impact after stitching accepted the shared realtime journal and OTel mapping decisions.
