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
- read-only inputs below

## Writable Files

- `docs/contracts/hook-lifecycle-contract.md`
- `docs/contracts/api-contracts.md` only for hook helper/API lowering language

## Read-Only Inputs

- `docs/contracts/runtime-package-schema.md`
- `docs/contracts/event-schema.md`
- `docs/contracts/journal-replay-schema.md`
- `docs/contracts/extension-sdk-contract.md`
- `docs/architecture/primitive-map.md`

## Primitive Focus

- Hook helpers lower into `HookSpec` and package sidecars/capabilities before a run starts.
- Hook mutation rights are typed per hook point.
- Hook responses are lifecycle-specific proposals that lower into existing domain operations; no generic event emission or SDK-effect hatch.
- Security-relevant checks fail closed or interrupt by policy; nonblocking observation hooks may fail open only when not security-critical.

## Must Not Own

Extension subprocess runtime, approval policy authority, arbitrary transcript mutation, or host process control.

## Validation And Review

- Future tests/fixtures: hook ordering tests, timeout/failure tests, mutation-rights matrix fixtures, and config/code lowering tests.
- Docs audit: hook responses must lower into lifecycle-specific domain operations, not a generic effect/event hatch.
- Hook ordering, timeout, queue, failure, and mutation-rights matrices.
- Hook response mutations are journaled before apply.
- Hook config and code-first helpers produce the same package shape.

## Validation Evidence

- Worker agent: Chandrasekhar (`019e586b-19bb-7b21-a871-77362b135b05`).
- Changed files: `docs/contracts/hook-lifecycle-contract.md`, `docs/contracts/api-contracts.md`.
- `git diff --check -- docs/contracts/hook-lifecycle-contract.md docs/contracts/api-contracts.md` passed.
- Scoped docs audits confirmed no generic hook hatches, no ambient callback registration, journal-before-apply, event/journal alignment, and product-neutrality.
- No Rust source, package manifests, executable tests, or fixtures were created.
- Cross-cutting proposals: none.

## Review Packet

- Primitive decision: reuse `RuntimePackage`, `HookSpec`, `PolicyRef`, `AgentEvent`, `RunJournal`, and `EffectIntent` / `EffectResult` through target domain operations; no new primitive or capability variant.
- SDK-owned boundaries preserved: hook points, typed responses, package lowering, mutation rights, events, journal/replay behavior, and security failure policy.
- Host-owned boundaries preserved: hook config files, executor installation, extension subprocesses, UI, and product hook libraries.
- Reviewer checklist: PASS for simplicity, product-neutrality, event/journal durability, privacy/redaction, replay/idempotency, and capability fingerprint impact.
