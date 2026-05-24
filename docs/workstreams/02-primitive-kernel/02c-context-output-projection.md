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

## Validation Evidence

Changed files:

- `docs/contracts/content-artifact-ref-contract.md`
- `docs/contracts/context-memory-contract.md`
- `docs/contracts/structured-output-contract.md`

Tests/fixtures:

- No Rust source, package manifests, executable tests, or fixtures were created; this was a documentation-only contract pass.
- Future content resolver, projection audit, schema registry, validation, repair, and structured-output fixture tests remain named in this goal and [03 Context Structured Output](../_roles/03-context-structured-output.md).

Commands run:

- `git diff --check`
- local Markdown link audit over all `.md` files
- no-code audit for `.rs`, `Cargo.toml`, executable tests, and fixtures
- Phase 02 writable-scope audit
- product-neutrality keyword audit over added lines
- primitive/no-mini-SDK audit over Phase 02 contracts

Skipped tests and why:

- Context and structured-output tests are skipped because the Rust crate and fixture tree do not exist yet.

Events/journal/telemetry touched:

- Context audit now requires `ContextRecord::ProjectionAudit` before `ProviderRequestProjected`.
- Structured output now names `ValidatedOutput` as the SDK-owned validation artifact before typed result construction or sink delivery.
- Telemetry remains derived and defaults to refs/redacted summaries.

SDK-owned boundaries preserved:

- SDK owns `ArtifactRef`, `ContentRef`, resolver policy shape, context admission/projection, projection audit, `OutputContract`, local validation, repair accounting, `ValidatedOutput`, and typed result construction.

Host-owned boundaries preserved:

- Host owns backing content stores, memory backend selection, memory browsing UI, retention implementation, semantic validator implementation, product rendering, business scoring, and output channel UX.

Primitive-lowering evidence:

- Content refs still do not imply provider visibility; refs become provider context only through `ContextContribution` admission, `ContextItem`, and `ContextProjection`.
- Typed output still lowers from `run_typed` to `OutputContract` and local `ValidatedOutput`; provider-native schema support is optimization only.

Simplicity notes:

- No shadow transcript, memory store, provider-context bag, output parser path, or sink delivery path was added.

Cross-cutting proposal blocks:

- Accepted: make `ContentResolutionPolicy`, `MissingContentPolicy`, `ContextProjectionAudit`, and `ValidatedOutput` explicit contract names so Phase 03 can reconcile IDs and support types.
- Rejected: treating raw content refs as automatically provider-visible.
- Deferred: exact fixture file names for projection audit and structured-output validation wait for the future crate layout.

## Review Packet

Primitive decision:

- Reused kernel primitives: `AgentMessage`, `ArtifactRef`, `ContentRef`, `ContextContribution`, `ContextItem`, `ContextProjection`, `ContextSelectionDecision`, `OutputContract`, `ValidatedOutput`, `RunJournal`, `AgentEvent`, policy refs, source/destination refs, and typed IDs.
- New feature-layer primitives: none.
- New capability variants: none.
- Host-owned behavior kept out: memory browsing, host ranking UX, backing content stores, product-specific forms, business scoring, and output channel UX.

Validation evidence:

- Contract/unit tests: future tests named; no code exists yet.
- Golden fixtures: future projection and structured-output fixture needs named; not created in this documentation-only pass.
- Smoke/scenario tests: not applicable until crates exist.
- Docs audits: link, writable-scope, no-code, product-neutrality, and primitive/no-mini-SDK audits.

Reviewer checklist:

- Simplicity: PASS, context remains contribution to item to projection, and typed output remains output contract to validation artifact.
- Product-neutrality: PASS, no product-specific references added.
- Event/journal durability: PASS, projection audit and structured-output records precede provider/output publication effects.
- Privacy/redaction: PASS, raw content resolution is opt-in and policy-bound.
- Replay/idempotency: PASS, missing refs and validation artifacts are represented as refs and journaled records.
- Capability fingerprint impact: PASS, no capability variants changed; future output/schema fingerprinting remains package-sidecar work.
