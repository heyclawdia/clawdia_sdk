# Goal 02c: Context Output Projection

## Phase

[Phase 02: Primitive Kernel](README.md)

## Owner Role

[Context Structured Output](../_roles/03-context-structured-output.md)

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

- `docs/contracts/content-artifact-ref-contract.md`
- `docs/contracts/context-memory-contract.md`
- `docs/contracts/structured-output-contract.md`

## Read-Only Inputs

- `docs/contracts/runtime-package-schema.md`
- `docs/contracts/event-schema.md`
- `docs/contracts/journal-replay-schema.md`
- `docs/contracts/output-delivery-contract.md`
- `docs/architecture/primitive-map.md`

## Primitive Focus

- Define `ContextContribution` or `ContextCandidate` -> admitted `ContextItem` -> provider-ready `ContextProjection`.
- Use `ArtifactRef` / `ContentRef` for content-bearing data that is not automatically provider-visible.
- Add context selection/admission decisions with reasons, policy refs, provenance, trust, privacy, retention, and budget metadata.
- Keep typed output local validation as `OutputContract` plus `ValidatedOutput`, not a provider-only feature.

## Must Not Own

Memory browsing UI, durable memory backend implementation, tool execution, host context ranking products, or output channel delivery.

## Validation And Review

- Missing memory means no memory retrieval.
- Memory/tool/skill/host/subagent/compaction results may create candidates, but only policy-admitted items are projected.
- Projection audit records included, omitted, compacted, redacted, policy-denied, and budget-denied decisions.
- Events and telemetry default to content refs or redacted summaries.
