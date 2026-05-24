# Goal 05c: Subagents

## Phase

[Phase 05: Feature Layers](README.md)

## Owner Role

[Subagents Coordination](../_roles/07-subagents-coordination.md)

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

- `docs/contracts/subagent-contract.md`

## Read-Only Inputs

- `docs/contracts/api-contracts.md`
- `docs/contracts/runtime-package-schema.md`
- `docs/contracts/event-schema.md`
- `docs/contracts/journal-replay-schema.md`
- `docs/contracts/context-memory-contract.md`
- `docs/examples/subagent-supervision-workflow.md`

## Primitive Focus

- Subagents are parent-owned child runs with stripped runtime packages, wrapped events, child journals, usage rollup, and lifecycle policy.
- Add explicit `ContextHandoffPolicy`: `none`, `summary_only`, `selected_refs`, `full_history_with_policy`.

## Must Not Own

User-chat conversation promotion, product routing, recursive agent societies, or host inspector UI.

## Validation And Review

- Future tests/fixtures: child package-diff fixtures, handoff policy matrix tests, mailbox/clarification fixtures, event wrapping fixtures, and usage rollup tests.
- Docs audit: subagents must remain parent-owned child runs, not a separate runtime ledger or direct user-chat surface.
- Child package strips recursive subagent tools by default.
- Default handoff is `ContextHandoffPolicy::None`; summary or selected refs require explicit policy.
- Parent cancel, completion, detach, mailbox, clarification, and usage rollup are journaled.
