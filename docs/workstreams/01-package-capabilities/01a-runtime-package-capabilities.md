# Goal 01a: Runtime Package Capabilities

## Phase

[Phase 01: Package Capabilities](README.md)

## Owner Role

[Integration Stitching](../_roles/00-integration-stitching.md)

## Parallelism

Only goal in Phase 01. Run after Phase 00 exits. Do not start Phase 02 until this goal exits.

## Required Reading

- `README.md`
- `docs/start-here.md`
- `coding_standards.md`
- `docs/workstreams/validation-gates.md`
- `docs/reference/sdk-review-checklist.md`
- `docs/architecture/primitive-map.md`
- phase README
- owner role doc
- read-only inputs below

## Writable Files

- `docs/contracts/runtime-package-schema.md`
- `docs/architecture/primitive-map.md` only for shared naming reconciliation
- `docs/contracts/review-matrix.md` only for source/validation alignment
- `docs/reference/feature-to-primitive-matrix.md`
- `docs/reference/open-questions-and-ambiguities.md`

## Read-Only Inputs

- `docs/contracts/tool-pack-contract.md`
- `docs/contracts/extension-sdk-contract.md`
- `docs/contracts/subagent-contract.md`
- `docs/contracts/isolation-runtime-contract.md`
- `docs/workstreams/_roles/01-core-api-runtime.md`
- `docs/workstreams/_roles/04-tools-approval-toolpacks.md`

## Primitive Focus

- Keep `RuntimePackage` as the immutable per-run snapshot.
- Keep `CapabilitySpec` limited to discoverable/callable capabilities with typed sidecars.
- Move provider route, output contracts, delivery sinks, hooks, guardrails, telemetry policy, and isolation requirements into typed package fields or sidecars keyed by stable IDs when they are not callable capabilities.
- Add capability projection modes and source-qualified catalog/provenance snapshots.

## Must Not Own

Tool implementation, extension subprocess runtime, provider credentials, concrete isolation runtimes, host capability install policy, or product package catalogs.

## Validation And Review

- Fingerprint inputs are deterministic and include every execution-affecting policy/sidecar field.
- Reserved capability variants name owner role, sidecar contract, events, journal records, and future tests.
- Package helpers lower to the canonical snapshot and do not create parallel registries.
