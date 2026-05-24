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
- `docs/contracts/agent-pool-contract.md`

## Read-Only Inputs

- `docs/contracts/api-contracts.md`
- `docs/contracts/agent-pool-contract.md`
- `docs/contracts/runtime-package-schema.md`
- `docs/contracts/event-schema.md`
- `docs/contracts/journal-replay-schema.md`
- `docs/contracts/context-memory-contract.md`
- `docs/examples/subagent-supervision-workflow.md`

## Primitive Focus

- AgentPool is the generic coordination primitive for run messages, wake
  conditions, and pool membership.
- Subagents are parent-owned child-run helpers over AgentPool with stripped
  runtime packages, wrapped events, child journals, usage rollup, and lifecycle
  policy.
- Add explicit `ContextHandoffPolicy`: `none`, `summary_only`, `selected_refs`, `full_history_with_policy`.

## Must Not Own

User-chat conversation promotion, product routing, recursive agent societies, or host inspector UI.

## Validation And Review

- Future tests/fixtures: child package-diff fixtures, handoff policy matrix tests,
  agent-pool run-message/wake fixtures, event wrapping fixtures, and usage
  rollup tests.
- Docs audit: subagents must remain parent-owned child runs, not a separate runtime ledger or direct user-chat surface.
- Child package strips recursive subagent tools by default.
- Default handoff is `ContextHandoffPolicy::None`; summary or selected refs require explicit policy.
- Parent cancel, completion, detach, run-message clarification, wake, and usage
  rollup are journaled.

## Validation Evidence

- Worker agent: Parfit (`019e5882-0444-70d0-aa7d-6256e620f278`).
- Changed file: `docs/contracts/subagent-contract.md`.
- Scoped docs audit confirmed subagents lower into `AgentPool`, generic
  `RunMessage`/`WakeCondition` values, parent-owned child `RunRequest` values,
  stripped child `RuntimePackage` snapshots, linked `RunJournal` refs, wrapped
  `AgentEvent` frames, policy refs, content refs, lifecycle records, and
  usage/cost rollups.
- Named future agent-pool/depth, package-diff, handoff, run-message/wake,
  event-wrapping, usage-rollup, redaction, event, and OTel projection fixtures
  without creating executable fixtures in this documentation-only phase.
- Cross-cutting proposals sent to stitching: close the Phase 04 OTel subagent
  deferral, include child package and agent-pool message/wake fingerprint inputs
  in runtime-package records, align run-message/wake DTO names, and keep child
  lifecycle events shared with isolation/stitching.
- No Rust source, package manifests, executable tests, or fixtures were created.

## Review Packet

- Primitive decision: AgentPool is the coordination primitive, and subagents are
  parent-owned child-run helpers over it; neither creates direct user-chat
  conversations, recursive agent societies, or a separate child runtime ledger.
- SDK-owned boundaries preserved: child package stripping,
  `ContextHandoffPolicy::None` default, agent-pool run-message/wake
  communication, event wrapping, child journal linkage, cancellation/detach
  records, and usage rollup.
- Host-owned boundaries preserved: inspector UI, promotion to conversation, rate tables/billing UI, detached-child dashboards, concrete child-run adapter/process management, and product workflows over subagent events.
- Reviewer checklist: PASS for simplicity, product-neutrality, event/journal durability, privacy/redaction, replay/idempotency, package fingerprint impact, and no user-chat promotion after stitching accepted shared child-lifecycle ownership.
