# Agent SDK Validation And Simplicity Hardening Plan

> Planning note only. Current implementation authority lives in [../contracts/README.md](../contracts/README.md) and [../workstreams/README.md](../workstreams/README.md). If this plan conflicts with those docs, use the contracts and workstreams.

## Objective

Make the standalone Agent SDK packet easier for implementation agents to execute by adding explicit validation gates for every workstream, surfacing coding standards at the workspace root, and auditing the SDK contracts for simplification opportunities that preserve the intended feature set.

## Scope

- Add a root `coding_standards.md` entry point for agents.
- Add a shared validation-gates document under `docs/workstreams/`.
- Update every workstream file with concrete required tests, fixtures, audits, and handoff evidence.
- Add a simplicity audit under `docs/reference/` that identifies where the architecture can be easier without dropping observability, policy, replay, isolation, extension, subagent, or typed-output capabilities.
- Update navigation/index docs so future agents find these surfaces first.

## Behavior Contract

New behavior:

- Each workstream has a clear way to prove correctness after implementation.
- Validation names exact expected test families, fixture categories, and cross-contract audits.
- Simplicity guidance separates essential complexity from avoidable ceremony.
- Root standards are easy to find without knowing the architecture folder layout.

Preserved behavior:

- Workstream write ownership remains non-overlapping.
- The integration/stitching role remains serialized.
- SDK contracts remain product-neutral and feature-complete.
- Host scenarios remain product-neutral coverage, not SDK core.

Removed behavior:

- No workstream should rely on vague "validation evidence" wording alone.
- No future agent should need to infer standards from a product checkout.

## Validation Plan

- Markdown link audit over `/Users/clawdia/clawdia_sdk`.
- Workstream ownership audit: no duplicated writable files and no non-stitching architecture/reference writes.
- Content audit: every `docs/workstreams/[0-9][0-9]-*.md` has `## Required Validation`.
- Content audit: every workstream names tests or fixtures, not only review prose.
- Content audit: root `coding_standards.md`, `docs/workstreams/validation-gates.md`, and `docs/reference/simplicity-audit.md` are linked from navigation docs.
- Whitespace check in any legacy product checkout touched by the task.

## Risks / Gotchas

- Do not create a second source of truth for coding standards. Root `coding_standards.md` should point at and summarize `docs/architecture/coding-standards.md`.
- Do not make validation require live providers or real container runtimes. Phase-one confidence comes from fake adapters, golden fixtures, property tests, and smoke tests.
- Do not simplify by deleting capability. Prefer thinner ergonomics, reusable test harnesses, and clearer ownership boundaries.
- Do not make every worker edit shared docs. Validation gaps found by non-stitching workstreams go through cross-cutting proposals.
