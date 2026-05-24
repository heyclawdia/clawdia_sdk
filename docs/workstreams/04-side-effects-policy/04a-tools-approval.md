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
- read-only inputs below

## Writable Files

- `docs/contracts/tool-approval-contract.md`
- `docs/contracts/tool-pack-contract.md`

## Read-Only Inputs

- `docs/contracts/runtime-package-schema.md`
- `docs/contracts/journal-replay-schema.md`
- `docs/contracts/isolation-runtime-contract.md`
- `docs/architecture/primitive-map.md`
- `docs/examples/tool-pack-isolation-anti-entropy.md`

## Primitive Focus

- Tools lower into `RuntimePackage` capabilities, executor refs, policy refs, journal records, and events.
- Every tool call records tool execution intent/result; mutating tools add side-effect metadata, idempotency, and reconciliation fields.
- Built-in packs remain optional toolkit layers, not core product behavior.

## Must Not Own

Runtime-package canonical schema, concrete shell/container execution, product approval UI, extension self-approval, or coding-agent workflow.

## Validation And Review

- Future tests/fixtures: approval policy matrix tests, dispatcher absence tests, tool-pack snapshot fixtures, and side-effect journal fixtures.
- Docs audit: tool and approval helpers must reuse the shared runtime-package/effect/journal path.
- Approval matrix and fail-closed dispatcher absence.
- Intent/result journal proof for every tool call, with intent-before-external-effect proof for mutating tools.
- Tool-pack snapshot and fingerprint proof.
- Primitive-lowering evidence: no ambient tool discovery or unjournaled side-effect path.

## Validation Evidence

- Worker agent: Lorentz (`019e586a-b953-7e01-be4b-4b5fe64df070`).
- Changed files: `docs/contracts/tool-approval-contract.md`, `docs/contracts/tool-pack-contract.md`.
- `git diff --check -- docs/contracts/tool-approval-contract.md docs/contracts/tool-pack-contract.md` passed.
- Scoped docs audit confirmed coverage for `RuntimePackage`, `CapabilitySpec`, `PolicyRef`, approval-dispatch `EffectIntent` / `EffectResult`, tool-execution `EffectIntent` / `EffectResult`, `RunJournal`, `AgentEvent`, `SourceRef`, `DestinationRef`, `ContentRef`, package deltas, fail-closed rules, idempotency, and reconciliation.
- No Rust source, package manifests, executable tests, or fixtures were created.
- Cross-cutting proposals sent to stitching: final tool/approval event names and tool-pack fingerprint inputs.

## Review Packet

- Primitive decision: reuse kernel primitives and typed package sidecars; no new kernel primitive or capability variant.
- SDK-owned boundaries preserved: policy stages, approval broker semantics, approval-dispatch intent/result, tool execution intent/result, package snapshots, and side-effect audit.
- Host-owned boundaries preserved: approval UI/copy, installed tools, concrete file/process/runtime adapters, compatibility modes, and product undo UX.
- Reviewer checklist: PASS for simplicity, product-neutrality, event/journal durability, privacy/redaction, replay/idempotency, and capability fingerprint impact after stitching reconciles fingerprint inputs.
