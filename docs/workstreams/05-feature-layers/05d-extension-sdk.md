# Goal 05d: Extension SDK

## Phase

[Phase 05: Feature Layers](README.md)

## Owner Role

[Extension Sdk Packaging](../_roles/08-extension-sdk-packaging.md)

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

- `docs/contracts/extension-sdk-contract.md`

## Read-Only Inputs

- `docs/contracts/runtime-package-schema.md`
- `docs/contracts/tool-approval-contract.md`
- `docs/contracts/hook-lifecycle-contract.md`
- `docs/architecture/architecture-proposal.md`
- `docs/architecture/primitive-map.md`

## Primitive Focus

- Split core extension capabilities from host extension manifest/runtime concerns.
- Extension-declared tools, hooks, providers, subagents, and actions resolve into runtime-package sidecars/capabilities only after host policy.
- Browser-safe helper exports, package compatibility, trust state, action permissions, runtime, and install metadata remain host/optional-extension packaging concerns, not core capability fields.

## Must Not Own

Marketplace UX, extension installation, subprocess lifecycle in core, app-event storage, provider credentials, memory authority, or self-approval.

## Validation And Review

- Future tests/fixtures: extension manifest fixtures, JSON-RPC fixtures, package-subpath smoke tests, browser-safe bundle checks, and denied-action tests.
- Docs audit: extension core capabilities must lower into SDK-facing package fields without importing host manifest/runtime authority into core.
- Core capability helpers and explicit core capability declarations lower to the same SDK-facing capability fields.
- Extension action crosses host approval.
- Browser-safe exports prove no native/process/fs dependencies.
- Core has no extension runtime imports.

## Validation Evidence

- Worker agent: Kuhn (`019e5882-055b-7a20-b164-9b8d6239c123`).
- Changed file: `docs/contracts/extension-sdk-contract.md`.
- Scoped docs audit confirmed extension declarations lower into SDK-facing `CoreExtensionCapabilities`, runtime-package sidecars/capability refs, policy refs, approval records, journal-backed `EffectIntent` / `EffectResult`, events, and typed refs.
- Named future manifest, JSON-RPC, packaging smoke, browser-safe, denied-action, redaction, event, and OTel projection fixtures without creating executable fixtures in this documentation-only phase.
- Cross-cutting proposals sent to stitching: accept `ExtensionActionStarted`, `ExtensionActionCompleted`, and `ExtensionActionFailed`; close the Phase 04 OTel extension deferral; and keep host manifest/runtime/install/marketplace/browser-safe/trust/app-event transport fields outside core package authority.
- No Rust source, package manifests, executable tests, or fixtures were created.

## Review Packet

- Primitive decision: extensions provide SDK-facing capability declarations and policy-crossing action requests; host manifests and extension runtime concerns remain outside `agent-sdk-core`.
- SDK-owned boundaries preserved: core capability shapes, helper lowering, runtime-package resolution refs, policy-crossing event/journal/effect records, no self-approval, and typed refs.
- Host-owned boundaries preserved: install flow, marketplace UX, subprocess runtime, app-event transport/fanout, browser-safe packaging validation, trust/action permission state, provider credentials, memory authority, and UI surfaces.
- Reviewer checklist: PASS for simplicity, product-neutrality, event/journal durability, privacy/redaction, replay/idempotency, extension action effect ordering, and capability fingerprint impact after stitching accepted terminal action events.
