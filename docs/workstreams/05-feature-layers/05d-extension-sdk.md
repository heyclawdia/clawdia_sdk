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

## Writable Files

- `docs/contracts/extension-sdk-contract.md`

## Primitive Focus

- Split core extension capabilities from host extension manifest/runtime concerns.
- Extension-declared tools, hooks, providers, subagents, and actions resolve into runtime-package sidecars/capabilities only after host policy.
- Browser-safe helper exports, package compatibility, trust state, action permissions, runtime, and install metadata remain host/optional-extension packaging concerns, not core capability fields.

## Must Not Own

Marketplace UX, extension installation, subprocess lifecycle in core, app-event storage, provider credentials, memory authority, or self-approval.

## Validation And Review

- Core capability helpers and explicit core capability declarations lower to the same SDK-facing capability fields.
- Extension action crosses host approval.
- Browser-safe exports prove no native/process/fs dependencies.
- Core has no extension runtime imports.
