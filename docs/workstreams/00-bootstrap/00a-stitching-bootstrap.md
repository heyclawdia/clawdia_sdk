# Goal 00a: Stitching Bootstrap

## Phase

[Phase 00: Bootstrap](README.md)

## Owner Role

[Integration Stitching](../_roles/00-integration-stitching.md)

## Parallelism

Only goal in Phase 00. Run this before Phase 01.

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

- `AGENTS.md`
- `README.md`
- `docs/start-here.md`
- `docs/architecture/primitive-map.md`
- `docs/architecture/external-sdk-lessons.md`
- `docs/contracts/review-matrix.md`
- `docs/workstreams/README.md`
- `docs/workstreams/validation-gates.md`
- `docs/workstreams/[0-9][0-9]-*/**`
- `docs/reference/cross-cutting-proposals.md`
- `docs/reference/feature-to-primitive-matrix.md`
- `docs/reference/open-questions-and-ambiguities.md`

## Read-Only Inputs

- `docs/architecture/architecture-proposal.md`
- `docs/architecture/coding-standards.md`
- `docs/architecture/observability-and-lineage.md`
- `docs/architecture/coverage-gap-matrix.md`
- `docs/contracts/README.md`
- `docs/contracts/runtime-package-schema.md`
- all other contract, example, plan, risk, and note docs not listed as writable

## Bootstrap Plan

- Confirm the Phase 00 exit gate matches the active launch structure.
- Fill any phase-goal overlay gaps so every goal names owner, writable files, read-only inputs, parallelism, and validation.
- Preserve the primitive decision ladder and feature-to-primitive matrix as the shared anti-mini-SDK gate.
- Preserve the external source audit format and current source rows without importing source-specific product behavior.
- Finish with link/path, ownership, primitive, product-neutrality, no-code, and Phase 00 exit evidence.

## Plan Review Check

- Coding standards and repo-local instructions read before edits: `AGENTS.md`, `coding_standards.md`, `docs/architecture/coding-standards.md`.
- Architecture/source-of-truth docs read before edits: `README.md`, `docs/start-here.md`, `docs/architecture/primitive-map.md`, `docs/workstreams/README.md`, `docs/workstreams/validation-gates.md`, `docs/reference/sdk-review-checklist.md`.
- Boundary check: Phase 00 remains documentation-only and does not add Rust source, package manifests, executable tests, fixtures, or product host adapters.
- Simplicity check: new launch criteria must reuse the primitive kernel and block parallel run loops, package registries, event streams, journals, policy paths, context projection paths, side-effect paths, telemetry truth stores, and host adapter products.

## Primitive Focus

- Kernel primitives reused: `Agent`, `RunRequest`, `RuntimePackage`, `AgentEvent`, `RunJournal`, `PolicyRef`, `SourceRef`, `DestinationRef`, typed IDs.
- Feature-layer primitives introduced or refined: primitive decision ladder, `ArtifactRef` / `ContentRef`, `EffectIntent` / `EffectResult`, `EntityRef`, context contribution pipeline.
- Optional adapter or host-owned boundary preserved: provider adapters, memory backends, extension runtimes, concrete isolation, channel transports, product UI.

## Required Output

- Feature-to-primitive matrix with one row per active feature.
- External source audit format with URL, date checked, accepted lesson, rejected behavior, and SDK decision.
- Phase goal overlay created or updated.
- Review criteria that block mini SDKs inside workstreams.

## Must Not Own

Feature-specific contract details beyond narrow stitching decisions, future Rust source, executable tests, package manifests, product host adapters, or non-stitching implementation files.

## Validation And Review

- Link audit over touched markdown.
- Phase/goals audit proving every goal names owner, writable files, read-only inputs, parallelism, and validation.
- Primitive audit proving context is only the provider projection path, not a universal SDK abstraction.
- Review packet using [../validation-gates.md](../validation-gates.md).

## Validation Evidence

Changed files:

- `docs/workstreams/00-bootstrap/README.md`
- `docs/workstreams/00-bootstrap/00a-stitching-bootstrap.md`
- `docs/contracts/README.md`
- goal launch docs under `docs/workstreams/02-primitive-kernel/`, `docs/workstreams/03-kernel-review/`, `docs/workstreams/04-side-effects-policy/`, `docs/workstreams/05-feature-layers/`, `docs/workstreams/06-scenario-coverage/`, and `docs/workstreams/07-final-review/`

Tests/fixtures:

- Documentation-only goal. No Rust source, executable tests, package manifests, JSON/YAML fixtures, or golden fixture files were created.
- Future tests/fixtures are now named in affected goal validation sections where they were previously implicit.

Commands run:

- Workstream shape audit over `docs/workstreams/[0-9][0-9]-*/*.md` and `docs/workstreams/_roles/*.md`.
- Local markdown link audit over `docs/**/*.md`.
- External URL liveness audit over every HTTP(S) markdown link in `docs/**/*.md`.
- Workstream ownership audit over owner-role writable scopes.
- Contract-index product-neutrality audit over `docs/contracts/README.md`.
- Primitive/context audit over `docs/architecture/primitive-map.md`, `docs/workstreams/validation-gates.md`, `docs/reference/feature-to-primitive-matrix.md`, and `docs/reference/open-questions-and-ambiguities.md`.
- No-mini-SDK audit over validation/reference docs.
- No-code audit over the workspace file tree.

Skipped tests and why:

- Rust compile, unit, golden, property, smoke, and scenario tests are skipped because this Phase 00 goal is documentation-only and the workspace intentionally has no Rust crate yet.

Events/journal/telemetry touched:

- No event, journal, or telemetry contract semantics changed. Phase 00 only tightened launch metadata, validation wording, and stale external links.

SDK-owned boundaries preserved:

- Shared primitive names, the feature-to-primitive matrix, no-mini-SDK gate, context projection gate, and runtime-package/capability discipline remain SDK-owned.

Host-owned boundaries preserved:

- Product UI, host adapters, credentials, marketplaces, dashboards, channel UX, concrete isolation runtimes, and product workflows remain outside core.

Primitive-lowering evidence:

- The Phase 00 exit evidence explicitly verifies that context is a projection pipeline and that future workstreams must lower through the primitive kernel instead of creating private run/package/event/journal/policy/context/effect/telemetry paths.

Simplicity notes:

- The only added launch-doc structure is read-only input metadata and explicit future validation evidence; no new primitive, registry, side-effect path, or feature layer was introduced.

Cross-cutting proposal blocks:

- None. This bootstrap pass accepted no new cross-cutting proposal and introduced no unresolved shared rename.

## Review Packet

Primitive decision:

- Reused kernel primitives: `Agent`, `RunRequest`, `RuntimePackage`, `AgentEvent`, `RunJournal`, `PolicyRef`, `SourceRef`, `DestinationRef`, `ContentRef`, `EffectIntent`, typed IDs.
- New feature-layer primitives: none in this pass.
- New capability variants: none in this pass.
- Host-owned behavior kept out: product host adapters, UI, credentials, marketplace/install flows, dashboards, concrete runtimes, and channel UX.

Validation evidence:

- Contract/unit tests: not applicable until Rust code exists; future tests are named in goal docs.
- Golden fixtures: not applicable until Rust code exists; fixture requirements are named in goal docs.
- Smoke/scenario tests: not applicable until Rust code exists; future smoke/scenario expectations are named in goal docs.
- Docs audits: link, external URL, workstream shape, ownership, contract-index product-neutrality, primitive/context, no-mini-SDK, and no-code audits all passed.

Reviewer checklist:

- Simplicity: pass. Added metadata clarifies ownership and validation without adding parallel concepts.
- Product-neutrality: pass. Stale source links were repaired; no product-specific host adapter became normative.
- Event/journal durability: pass. Launch docs continue to require intent/result, event, and journal fixtures before code exists.
- Privacy/redaction: pass. Context projection and telemetry/raw-content boundaries remain explicit.
- Replay/idempotency: pass. No replay semantics changed; future journal/replay evidence remains required by owner roles.
- Capability fingerprint impact: none. No `CapabilitySpec` variant, sidecar, or fingerprint input changed.
