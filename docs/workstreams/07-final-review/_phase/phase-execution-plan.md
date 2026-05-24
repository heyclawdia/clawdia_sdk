# Phase 07 Execution Plan

## Objective

Run the final whole-packet stitching review after Phase 06. Confirm that the Agent SDK documentation packet is product-neutral, primitive-centered, documentation-only, and ready for the first Rust coding goals.

## Relevant Existing Context

- `README.md` and `docs/start-here.md`: the active packet is standalone, product-neutral, and still documentation-only.
- `coding_standards.md` and `docs/architecture/coding-standards.md`: future implementation must start with tests, keep helpers thin, preserve observability and journal durability, and avoid product-specific host behavior in core.
- `docs/workstreams/README.md`: Phase 07 is the final serialized review after Phase 06 exits.
- `docs/workstreams/validation-gates.md`: documentation-only goals require link/path, ownership, primitive-lowering, proposal, no-code, and named future-test evidence.
- `docs/reference/sdk-review-checklist.md`: final review must check simplicity, primitive fit, product-neutrality, event/journal durability, privacy, replay, and testability.
- `docs/architecture/primitive-map.md`: feature layers must reuse `Agent`, `RunRequest`, `RuntimePackage`, `AgentEvent`, `RunJournal`, policy refs, source/destination refs, content refs, effect intent/result, and typed ports.
- Phase 03 through Phase 06 exit reports: prior phase gates passed and recorded accepted, rejected, deferred, and unresolved proposal status.

## Work Plan

1. Verify Phase 00 through Phase 06 exit gates and Phase 02's three goal packets.
2. Reconcile any stale public indices found during final review.
3. Run whole-packet audits: diff hygiene, Markdown links, ownership, product-neutrality, no-code, owner/goal validation sections, primitive-lowering/disjoint scopes, no-mini-SDK language, proposal/blocker status, phase-gate status, and review-matrix coverage.
4. Write the final Phase 07 exit report and goal-level validation evidence.
5. Send the final packet to an independent reviewer and resolve any blocking findings.
6. Check the Phase 07 README exit gate only after the reviewer verdict and final audits pass.

## Scope

Writable scope is limited to the integration/stitching surfaces named by [Owner Role 00](../../_roles/00-integration-stitching.md), especially Phase 07 docs, shared indices, and narrow contract index reconciliation.

The final pass must not create Rust source files, package manifests, executable tests, fixtures, or product-specific host adapters.

## Validation Plan

- `git diff --check`
- whole-packet Markdown link/path audit
- no-code audit for `.rs`, `Cargo.toml`, executable tests, and fixture paths
- product-neutrality audit over root docs and `docs/`
- workstream ownership audit
- changed-file writable-scope audit
- contract-index product-neutrality audit
- owner-role required-validation audit
- goal-doc validation-section audit
- primitive-lowering/disjoint future scope audit
- primitive/no-mini-SDK audit
- proposal/blocker audit
- phase README exit-gate audit
- review-matrix contract row audit

## Risk/Gotcha Carry-Forward

- Do not treat generic host scenarios as normative SDK-owned behavior.
- Do not add product-specific host adapters or examples to the active packet.
- Do not create code artifacts while this remains a documentation-only phase.
- Do not let final-review index edits bypass the accepted primitive kernel or reopen settled Phase 04/05 proposal decisions.
