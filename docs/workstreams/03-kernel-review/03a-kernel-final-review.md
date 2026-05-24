# Goal 03a: Kernel Final Review

## Phase

[Phase 03: Kernel Review](README.md)

## Owner Role

[Integration Stitching](../_roles/00-integration-stitching.md)

## Parallelism

Only goal in Phase 03. Run after Phase 01 and every Phase 02 goal exit. Do not start Phase 04 until this review passes.

## Required Reading

- `README.md`
- `docs/start-here.md`
- `coding_standards.md`
- `docs/workstreams/validation-gates.md`
- `docs/reference/sdk-review-checklist.md`
- `docs/architecture/primitive-map.md`
- phase README
- owner role doc
- Phase 01 output and all Phase 02 goal outputs

## Writable Files

- `README.md`
- `AGENTS.md`
- `docs/start-here.md`
- `docs/architecture/primitive-map.md`
- `docs/architecture/external-sdk-lessons.md`
- `docs/contracts/README.md`
- `docs/contracts/review-matrix.md`
- `docs/workstreams/README.md`
- `docs/workstreams/validation-gates.md`
- `docs/workstreams/[0-9][0-9]-*/**`
- `docs/reference/feature-to-primitive-matrix.md`
- `docs/reference/open-questions-and-ambiguities.md`
- `docs/reference/cross-cutting-proposals.md`
- narrow contract reconciliation edits allowed by [../_roles/00-integration-stitching.md](../_roles/00-integration-stitching.md)

## Primitive Focus

- Reconcile public names, IDs, event/journal alignment, runtime-package fingerprint inputs, and primitive/feature layering.
- Confirm no phase goal introduced a second run loop, package registry, event stream, journal, policy path, context projection path, or side-effect path.

## Required Output

- Phase 03 kernel review exit report.
- Accepted/rejected cross-cutting proposals.
- Updated source audit and feature-to-primitive matrix.
- Validation evidence for link, ownership, product-neutrality, and no-code audits.

## Must Not Own

Feature implementation details that belong to later owner roles, product host behavior, future Rust source, executable tests, package manifests, or scenario rewriting outside final stitching.

## Validation And Review

- Run the whole-packet docs audits required by Workstream 00.
- Apply [../../reference/sdk-review-checklist.md](../../reference/sdk-review-checklist.md).
- Do not pass if Phase 01 or any Phase 02 goal leaves a must-answer primitive ambiguity open.
