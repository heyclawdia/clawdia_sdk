# Goal 04d: Hooks Lifecycle

## Phase

[Phase 04: Side Effects And Policy](README.md)

## Owner Role

[Core Api Runtime](../_roles/01-core-api-runtime.md)

## Parallelism

Parallel-safe with every other goal in Phase 04 after Phase 03 exits. Coordinate extension-provided hooks with the extension role through handoff proposals. Do not start Phase 05 until all Phase 04 goals finish.

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

## Writable Files

- `docs/contracts/hook-lifecycle-contract.md`
- `docs/contracts/api-contracts.md` only for hook helper/API lowering language

## Primitive Focus

- Hook helpers lower into `HookSpec` and package sidecars/capabilities before a run starts.
- Hook mutation rights are typed per hook point.
- Hook responses are lifecycle-specific proposals that lower into existing domain operations; no generic event emission or SDK-effect hatch.
- Security-relevant checks fail closed or interrupt by policy; nonblocking observation hooks may fail open only when not security-critical.

## Must Not Own

Extension subprocess runtime, approval policy authority, arbitrary transcript mutation, or host process control.

## Validation And Review

- Hook ordering, timeout, queue, failure, and mutation-rights matrices.
- Hook response mutations are journaled before apply.
- Hook config and code-first helpers produce the same package shape.
