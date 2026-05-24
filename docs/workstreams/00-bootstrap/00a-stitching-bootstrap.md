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
