# Goal 04a: Tools Approval

## Phase

[Phase 04: Side Effects And Policy](README.md)

## Owner Role

[Tools Approval Toolpacks](../_roles/04-tools-approval-toolpacks.md)

## Parallelism

Parallel-safe with every other goal in Phase 04 after Phase 03 exits. Do not start Phase 05 until all Phase 04 goals finish.

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

- `docs/contracts/tool-approval-contract.md`
- `docs/contracts/tool-pack-contract.md`

## Primitive Focus

- Tools lower into `RuntimePackage` capabilities, executor refs, policy refs, journal records, and events.
- Mutating tools use side-effect intent/result, idempotency, and reconciliation metadata.
- Built-in packs remain optional toolkit layers, not core product behavior.

## Must Not Own

Runtime-package canonical schema, concrete shell/container execution, product approval UI, extension self-approval, or coding-agent workflow.

## Validation And Review

- Approval matrix and fail-closed dispatcher absence.
- Intent-before-effect journal proof for mutating tools.
- Tool-pack snapshot and fingerprint proof.
- Primitive-lowering evidence: no ambient tool discovery or unjournaled side-effect path.
