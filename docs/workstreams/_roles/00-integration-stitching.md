# Owner Role 00: Integration And Stitching

## Owner Role

One senior integration agent. This role is serialized and should not be split across agents.

## Writable Files

- `<repo-root>/README.md`
- `<repo-root>/AGENTS.md`
- `<repo-root>/docs/start-here.md`
- `<repo-root>/docs/architecture/architecture-proposal.md`
- `<repo-root>/docs/architecture/primitive-map.md`
- `<repo-root>/docs/architecture/observability-and-lineage.md`
- `<repo-root>/docs/architecture/external-sdk-lessons.md`
- `<repo-root>/docs/architecture/coding-standards.md`
- `<repo-root>/docs/architecture/coverage-gap-matrix.md`
- `<repo-root>/docs/contracts/README.md`
- `<repo-root>/docs/contracts/review-matrix.md`
- `<repo-root>/docs/contracts/runtime-package-schema.md`
- `<repo-root>/docs/workstreams/README.md`
- `<repo-root>/docs/workstreams/validation-gates.md`
- `<repo-root>/docs/workstreams/[0-9][0-9]-*/**`
- `<repo-root>/docs/reference/open-questions-and-ambiguities.md`
- `<repo-root>/docs/reference/cross-cutting-proposals.md`
- `<repo-root>/docs/reference/feature-to-primitive-matrix.md`

## Read-Only Inputs

All contract, example, plan, risk, and note docs not listed above.

Exception: when the user explicitly asks for a whole-packet stitching/reconciliation pass, this role may make narrow contract edits that install shared primitive, ownership, event/journal, runtime-package, or product-neutrality decisions. After that pass, individual contracts return to their assigned workstream owners.

## Responsibilities

- Keep one source of truth for the primitive kernel, public names, ID taxonomy, event families, journal record names, runtime-package/capability fingerprint fields, policy terms, and crate/module boundaries.
- Reconcile cross-cutting proposals from other workstreams.
- Maintain the numbered phase folders under `docs/workstreams/[0-9][0-9]-*/` as launch orchestration. Role docs in `docs/workstreams/_roles/` remain the writable-scope authority.
- Keep product-specific host material outside normative SDK contracts.
- Run final link, inventory, product-neutrality, and structural audits.
- Verify feature workstreams layer over the primitive kernel instead of adding parallel run, package, event, journal, policy, or side-effect paths.

## Integration Handoff

- Final accepted cross-contract naming decisions.
- Updated indices.
- Validation report.
- List of unresolved questions that must block coding or can be deferred.

## Required Validation

- Run the whole-packet Markdown link audit over `<repo-root>`.
- Run the workstream ownership audit proving no duplicated writable files and no non-stitching writes to `docs/architecture/*` or `docs/reference/*`.
- Verify contract index excludes product-specific host-adapter references from normative contract tables.
- Verify every owner role has `## Required Validation`, every goal doc has `## Validation And Review`, and both name tests, fixtures, smoke checks, or docs audits.
- Verify every workstream has primitive-lowering review criteria and disjoint future implementation writable scope.
- Handoff evidence: link-audit output, ownership-audit output, product-neutrality audit output, changed shared names/IDs, and unresolved cross-cutting proposals.
