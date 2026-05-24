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

## Phase 01 Plan

Objective:

- Freeze the runtime-package/capability shape enough for Phase 02 kernel work to consume without reopening package authority or capability ownership.

Behavior contract:

- New behavior: the package contract gives one readiness table for every reserved `CapabilitySpec` variant, including owner role, typed sidecar contract, fingerprint fields, emitted events, journal records, and future validation.
- Preserved behavior: `RuntimePackage` remains the immutable per-run effective snapshot and fingerprint authority; `CapabilitySpec` stays limited to callable/discoverable capabilities; provider route, output contracts, output sinks, hooks, guardrails, telemetry policy, child lifecycle, and isolation remain package fields or typed sidecars unless exposed as callable/discoverable capabilities.
- Removed behavior: none. This is a docs-only tightening pass.
- Tests proving this behavior: future golden fingerprint fixtures, capability sidecar audits, package delta records, package preset lowering tests, and reserved-variant sidecar readiness tests named in the contract and review matrix.

Scope:

- Writable scope is limited to the files named in this goal plus the integration role launch/evidence docs.
- No Rust source, package manifests, executable tests, JSON/YAML fixtures, or product host adapters will be created.

Validation plan:

- Link audit over touched markdown.
- Runtime-package text audit for reserved variant readiness fields, no capability bag drift, deterministic fingerprint inputs, source-qualified catalog snapshots, and P0/P1 profile boundaries.
- Workstream ownership audit and contract-index product-neutrality audit.
- No-code audit.

Risks:

- If `CapabilitySpec` carries non-callable package concerns, later phases can build mini SDKs inside feature workstreams.
- If reserved variants do not name owner/sidecar/fingerprint/event/journal/test obligations now, later adapter docs can emit or execute capabilities before their durability/privacy gates exist.
- If fingerprint inputs remain prose-only, Phase 02 can accidentally omit output contracts, sidecars, policy refs, child lifecycle, or catalog activation data from golden fixtures.

## Plan Review Check

- Required docs read before edits: `AGENTS.md`, `README.md`, `docs/start-here.md`, `coding_standards.md`, `docs/architecture/coding-standards.md`, `docs/workstreams/validation-gates.md`, `docs/reference/sdk-review-checklist.md`, `docs/architecture/primitive-map.md`, phase README, owner role doc, and read-only inputs.
- Architecture alignment: the plan preserves typed snapshots over ambient lookup, one canonical package snapshot, and no-mini-SDK gates.
- Behavior contract clarity: the target edits are specifically the reserved-variant readiness table, fingerprint-readiness wording, review-matrix alignment, and Phase 01 exit evidence.
- Validation sufficiency: documentation-only audits cover links, ownership, package authority, product-neutrality, and no-code constraints; future code proof remains named tests/fixtures.
- Carry-forward gotchas: provider route/output/delivery/hooks/telemetry/isolation must not become capability variants by convenience, and reserved variants must not be executable until their owner workstream supplies sidecar contracts and fixtures.

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

## Validation Evidence

Changed files:

- `docs/contracts/runtime-package-schema.md`
- `docs/architecture/primitive-map.md`
- `docs/contracts/review-matrix.md`
- `docs/reference/feature-to-primitive-matrix.md`
- `docs/reference/open-questions-and-ambiguities.md`
- `docs/workstreams/01-package-capabilities/README.md`
- `docs/workstreams/01-package-capabilities/01a-runtime-package-capabilities.md`

Tests/fixtures:

- Documentation-only goal. No Rust source, executable tests, package manifests, JSON/YAML fixtures, or golden fixture files were created.
- Future proof is named through runtime-package acceptance tests for reserved variant readiness, fingerprint manifests, deterministic package fingerprints, package delta records, and canonical preset lowering.

Commands run:

- Runtime-package readiness audit over `docs/contracts/runtime-package-schema.md`.
- Local markdown link audit over `docs/**/*.md`.
- Workstream ownership audit over owner-role writable scopes.
- Contract-index product-neutrality audit over `docs/contracts/README.md`.
- No-mini-SDK package-boundary audit over runtime-package, primitive-map, matrix, and validation docs.
- No-code audit over the workspace file tree.

Skipped tests and why:

- Rust compile, unit, golden, property, smoke, and scenario tests are skipped because this Phase 01 goal is documentation-only and the workspace intentionally has no Rust crate yet.

Events/journal/telemetry touched:

- No executable event, journal, or telemetry implementation was changed. The runtime-package contract now names the expected event and journal surfaces for reserved variants before any adapter may emit or execute them.

SDK-owned boundaries preserved:

- `RuntimePackage` remains the immutable per-run effective snapshot and fingerprint source.
- `CapabilitySpec` remains limited to callable/discoverable capabilities with projection, executor, policy, and sidecar refs.
- Reserved capability variants remain inactive until their owner workstream supplies sidecar contracts and fixtures.

Host-owned boundaries preserved:

- Product UI, provider credentials, installed capability policy, concrete isolation runtimes, extension subprocess runtime, host manifests, marketplaces, dashboards, and channel UX stay outside core.

Primitive-lowering evidence:

- Provider route, output contracts, output sinks, hooks, guardrails, telemetry policy, child lifecycle, and isolation remain package fields or typed sidecars rather than capability variants unless they become explicit callable/discoverable capabilities.
- Catalog discovery remains separate from activation; activation creates a next-turn or next-run package delta.

Simplicity notes:

- The common path remains `RuntimePackage::for_agent(...).safe_defaults().build()?`; the new table is an advanced implementation gate for feature owners, not a new user-facing package concept.

Cross-cutting proposal blocks:

- None. This stitching pass accepted no new primitive proposal and introduced no unresolved shared rename.

## Review Packet

Primitive decision:

- Reused kernel primitives: `RuntimePackage`, first-slice `CapabilitySpec`, typed package sidecars, `PolicyRef`, `SourceRef`, `DestinationRef`, package fingerprint, `AgentEvent`, `RunJournal`, `EffectIntent`, typed IDs.
- New feature-layer primitives: none. Reserved variants remain gated names.
- New capability variants: none. Existing reserved variants now have readiness requirements before activation.
- Host-owned behavior kept out: installed capability policy, credentials, concrete runtimes, extension runtime/manifests, marketplace/install UX, dashboards, product UI, and channel UX.

Validation evidence:

- Contract/unit tests: not applicable until Rust code exists; future package/fingerprint/capability tests are named.
- Golden fixtures: not applicable until Rust code exists; future fingerprint and event/journal fixtures are named.
- Smoke/scenario tests: not applicable until Rust code exists; future extension/tool/isolation/subagent smoke expectations remain owner-role scoped.
- Docs audits: runtime-package readiness, local links, ownership, product-neutrality, no-mini-SDK boundary, and no-code audits passed.

Reviewer checklist:

- Simplicity: pass. The common package API is unchanged; the new table clarifies implementation gates.
- Product-neutrality: pass. No host product or adapter is promoted to core authority.
- Event/journal durability: pass. Reserved variants name expected event and journal records before activation.
- Privacy/redaction: pass. Resource/context, stream, realtime, extension, and telemetry-sensitive surfaces remain policy/ref based.
- Replay/idempotency: pass. Package delta, tool execution, child start, extension action, stream intervention, and discovery records remain journaled surfaces.
- Capability fingerprint impact: pass. `FingerprintInputManifest` makes included/excluded/reserved status auditable without becoming a second fingerprint source.
