# Phase 03 Execution Plan

## Objective

Complete the kernel review gate after Phase 01 and Phase 02, then establish the phase-level operating protocol requested for the rest of the SDK packet: parallel goal workers for multi-goal phases, a consolidated phase evidence surface, and a reviewer-agent gate before proceeding to the next phase.

## Behavior Contract

New behavior:

- Multi-goal phases launch one subagent per goal file with disjoint writable scopes.
- Single-goal phases run as a serialized stitching/review pass.
- Every phase ends with a consolidated exit report in that phase folder.
- A dedicated reviewer agent gates progression to the next phase.
- The phase README exit gate is checked only after goal packets, audits, and reviewer verdict all pass.

Preserved behavior:

- Numbered phases still run in order.
- Goal files and owner-role docs remain the writable-scope authority.
- Documentation-only work creates no Rust source files, executable tests, package manifests, or fixtures.
- Product-neutrality, primitive lowering, SDK-owned/host-owned boundaries, and no-mini-SDK rules remain mandatory.

Removed behavior:

- None. This adds orchestration structure without removing existing workstream rules.

Tests proving behavior:

- Documentation audits: link audit, writable-scope audit, no-code audit, product-neutrality audit, no-mini-SDK audit, and phase-exit checklist audit.
- Reviewer gate: a reviewer subagent must return PASS or blocking findings before the next phase starts.

## Relevant Existing Context

- `AGENTS.md`: no branches without explicit approval; documentation-only work must not create Rust source, executable tests, package manifests, or fixtures; use the current numbered phase and owner writable scopes.
- `README.md`: this packet is product-neutral and still documentation/handoff design.
- `docs/start-here.md`: P0/P1/P2 profile gates and thin ergonomic helpers over canonical contracts are core design constraints.
- `coding_standards.md` and `docs/architecture/coding-standards.md`: preserve product-neutral core, typed primitives, local validation, journal durability, event observability, policy failure-closed behavior, and fake/golden test posture.
- `docs/architecture/primitive-map.md`: shared kernel and feature-layer primitives are the source of truth for primitive-fit and no-mini-SDK review.
- `docs/workstreams/README.md`: phases run in numeric order; sibling goals in a phase are parallel-safe; later phases wait for the previous phase exit gate.
- `docs/workstreams/validation-gates.md`: every documentation-only goal needs concrete docs audit evidence, primitive-lowering review, proposal blocks, named future tests/fixtures, and no-code confirmation.
- `docs/workstreams/03-kernel-review/README.md`: Phase 03 is the gate between Phase 01/02 kernel outputs and Phase 04 side-effect work.
- `docs/workstreams/03-kernel-review/03a-kernel-final-review.md`: Phase 03 must reconcile names, IDs, event/journal alignment, runtime-package fingerprint inputs, source audit, feature-to-primitive matrix, and primitive layering.
- `docs/workstreams/_roles/00-integration-stitching.md`: Role 00 owns shared names, indices, phase docs, reference docs, and whole-packet audits.
- `docs/architecture/external-sdk-lessons.md`: external source-audit rows and lessons must be reviewed or updated during Phase 03.
- `docs/reference/sdk-review-checklist.md`: review must gate on simplicity, product-neutrality, event/journal durability, privacy, replay, capability fingerprint impact, and primitive fit.
- `docs/reference/simplicity-audit.md`: keep a small MVP kernel, canonical lowering, optional archive replay, and no workflow engine in core.
- `docs/reference/open-questions-and-ambiguities.md`: Phase 01/02 decisions already resolve the runtime package, event cursor, journal atomicity, context, and structured-output kernel posture.
- `docs/workstreams/01-package-capabilities/README.md` and `docs/workstreams/01-package-capabilities/01a-runtime-package-capabilities.md`: Phase 01 froze runtime-package fingerprint/capability-sidecar readiness for Phase 02/03.
- `docs/workstreams/02-primitive-kernel/README.md` and goals `02a`, `02b`, `02c`: Phase 02 froze core API, event/journal, and context/output contracts and named accepted/deferred proposals for Phase 03 stitching.

## Required Reading Verified

Before implementation edits, the orchestrator verified these Phase 03-required inputs:

- `README.md`
- `docs/start-here.md`
- `coding_standards.md`
- `docs/architecture/coding-standards.md`
- `docs/workstreams/validation-gates.md`
- `docs/reference/sdk-review-checklist.md`
- `docs/architecture/primitive-map.md`
- `docs/workstreams/03-kernel-review/README.md`
- `docs/workstreams/03-kernel-review/03a-kernel-final-review.md`
- `docs/workstreams/_roles/00-integration-stitching.md`
- Phase 01 output packet: `docs/workstreams/01-package-capabilities/README.md` and `docs/workstreams/01-package-capabilities/01a-runtime-package-capabilities.md`
- Phase 02 output packets: `docs/workstreams/02-primitive-kernel/README.md`, `02a-core-run-api.md`, `02b-events-journal-kernel.md`, and `02c-context-output-projection.md`
- Read-only/shared inputs used for review: `docs/contracts/README.md`, `docs/contracts/review-matrix.md`, `docs/contracts/runtime-package-schema.md`, `docs/architecture/external-sdk-lessons.md`, `docs/reference/feature-to-primitive-matrix.md`, `docs/reference/open-questions-and-ambiguities.md`, and `docs/reference/simplicity-audit.md`

## Scope

In scope for Phase 03:

- Reconcile Phase 01 and Phase 02 output packets.
- Update the workstream launch guidance so future phases organize per-goal outputs and reviewer gates consistently.
- Review or update the external SDK source audit.
- Update shared matrices or decision records only where Phase 01/02 accepted proposals require it.
- Produce the Phase 03 kernel review exit report.

Out of scope:

- Rust source, executable tests, package manifests, or fixtures.
- Later phase feature implementation details beyond launch/readiness guidance.
- Product host adapters or product-specific examples.

## Workstreams

1. Protocol/stitching pass:
   - Writable: `docs/workstreams/README.md`, `docs/workstreams/validation-gates.md`, Phase 03 docs, and shared reference/index docs allowed by the Phase 03 goal.
   - Output: phase-delivery protocol and Phase 03 exit report.

2. Kernel consistency pass:
   - Writable: shared matrices, `docs/architecture/external-sdk-lessons.md`, and narrow contract reconciliation edits only if needed.
   - Output: explicit accepted/rejected/deferred Phase 01/02 cross-cutting decisions, source-audit reconciliation, and feature-to-primitive matrix reconciliation.

3. Review gate:
   - Reviewer agent checks the Phase 03 diff against the plan, standards, Phase 01/02 evidence, and SDK review checklist.
   - Blocking findings must be fixed before Phase 04 starts.

## Validation Plan

- `git diff --check`
- whole-packet Markdown link audit
- no-code audit for `.rs`, `Cargo.toml`, executable tests, and fixture paths
- Phase/writable-scope audit over changed files
- workstream ownership audit proving no duplicated writable files and no non-stitching shared-doc writes outside delegated scope
- contract-index product-neutrality audit proving normative contract tables do not promote product host adapters
- owner-role and goal-doc completeness audit proving every owner role has `## Required Validation` and every goal doc has `## Validation And Review`
- primitive-lowering/disjoint-scope audit proving every workstream has primitive-lowering review criteria and disjoint future implementation writable scope
- changed shared names/IDs/proposals audit listing accepted, rejected, deferred, and unresolved cross-cutting proposals
- source-audit reconciliation over `docs/architecture/external-sdk-lessons.md`, recorded as updated or reviewed with no change required
- product-neutrality added-line audit
- primitive/no-mini-SDK audit over contracts/workstreams/reference docs
- Phase 03 exit-gate audit
- independent reviewer-agent verdict

## Risk / Gotcha Carry-Forward

- Do not create a branch.
- Do not let subagents edit overlapping files in a multi-goal phase.
- Do not let a reviewer gate become a prose-only rubber stamp; it must list checked evidence and blocking findings.
- Do not mark a phase complete if any goal packet lacks validation evidence, a review packet, or accepted/rejected/deferred proposal blocks.
- Do not start Phase 04 until Phase 03 exits and reviewer findings are resolved.
- If a later phase proposes a new primitive, capability variant, event family, journal record, policy stage, or fingerprint input, route it through stitching before proceeding.
