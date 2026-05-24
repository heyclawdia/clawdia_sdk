# Owner Role 00: Integration And Stitching

## Owner Role

One senior integration agent. This role is serialized and should not be split across agents.

## Writable Files

- `/Users/clawdia/clawdia_sdk/README.md`
- `/Users/clawdia/clawdia_sdk/AGENTS.md`
- `/Users/clawdia/clawdia_sdk/docs/start-here.md`
- `/Users/clawdia/clawdia_sdk/docs/architecture/architecture-proposal.md`
- `/Users/clawdia/clawdia_sdk/docs/architecture/primitive-map.md`
- `/Users/clawdia/clawdia_sdk/docs/architecture/observability-and-lineage.md`
- `/Users/clawdia/clawdia_sdk/docs/architecture/external-sdk-lessons.md`
- `/Users/clawdia/clawdia_sdk/docs/architecture/coding-standards.md`
- `/Users/clawdia/clawdia_sdk/docs/architecture/coverage-gap-matrix.md`
- `/Users/clawdia/clawdia_sdk/docs/contracts/README.md`
- `/Users/clawdia/clawdia_sdk/docs/contracts/review-matrix.md`
- `/Users/clawdia/clawdia_sdk/docs/contracts/runtime-package-schema.md`
- `/Users/clawdia/clawdia_sdk/docs/workstreams/README.md`
- `/Users/clawdia/clawdia_sdk/docs/workstreams/validation-gates.md`
- `/Users/clawdia/clawdia_sdk/docs/workstreams/[0-9][0-9]-*/**`
- `/Users/clawdia/clawdia_sdk/docs/plans/*.md`
- `/Users/clawdia/clawdia_sdk/docs/reference/source-migration-map.md`
- `/Users/clawdia/clawdia_sdk/docs/reference/open-questions-and-ambiguities.md`
- `/Users/clawdia/clawdia_sdk/docs/reference/cross-cutting-proposals.md`
- `/Users/clawdia/clawdia_sdk/docs/reference/feature-to-primitive-matrix.md`

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

- Run the whole-packet Markdown link audit over `/Users/clawdia/clawdia_sdk`.
- Run the workstream ownership audit proving no duplicated writable files and no non-stitching writes to `docs/architecture/*` or `docs/reference/*`.
- Verify `docs/reference/source-migration-map.md` matches actual file inventory.
- Verify any legacy in-product architecture packet, if present, is pointer-only and not an active SDK handoff source.
- Verify contract index excludes product-specific host-adapter references from normative contract tables.
- Verify every workstream has `## Required Validation` and names tests, fixtures, smoke checks, or docs audits.
- Verify every workstream has primitive-lowering review criteria and disjoint future implementation writable scope.
- Run whitespace checks in any legacy product checkout touched by the task.
- Handoff evidence: link-audit output, ownership-audit output, product-neutrality audit output, changed shared names/IDs, and unresolved cross-cutting proposals.
