# Agent SDK Ergonomics Layer Plan

> Historical plan only. It may contain stale product-specific paths or superseded workstream names. Do not use it as implementation authority; start from [../../start-here.md](../../start-here.md), [../../contracts/README.md](../../contracts/README.md), and [../../workstreams/README.md](../../workstreams/README.md).

## Objective

Add a thin ergonomic layer to the Agent SDK contracts so common usage stays simple while the authoritative wire, event, journal, validation, and policy contracts remain unchanged.

## Behavior Contract

New behavior:

- Structured output supports a Pydantic-like flow: caller passes a typed model, the SDK derives or looks up the schema, projects only schema/schema refs to the provider, validates locally, and constructs the typed result only after validation succeeds.
- `OutputContract`, `ValidationPolicy`, and `RepairPolicy` gain conservative builders, presets, helper constructors, and advanced overrides.
- API docs state the SDK-wide ergonomics rule: simple one-liners are wrappers over stable contracts, never separate behavior paths.
- Primitive docs state that normal users should get sensible defaults, while power users can override policies, limits, adapters, and failure behavior through builders.

Preserved behavior:

- Schema-first validation remains the source of truth.
- Provider-native JSON/schema modes are optimizations, not validation authority.
- Events, journals, retry accounting, redaction, and lineage stay identical whether the caller uses the one-liner or full `OutputContract`.
- Host-owned boundaries remain host-owned.

Removed behavior:

- Common typed-output examples should no longer require manual construction of every validation/repair field.

Validation:

- `git diff --check`.
- Structural audit that every touched doc includes the required ergonomics proof template:
  - simple API
  - advanced builder/config API
  - canonical lowered contract
  - event/journal equivalence
  - failure behavior
  - explicit `SDK owns / Host owns` boundary
  - acceptance tests
- Structured-output audit that the contract defines `TypedOutputModel` / `OutputSchemaProvider`, schema registry behavior, schema ID/version/fingerprint rules, typed result construction boundary, `run_typed::<T>()` or `.output::<T>()`, presets, defaulted builders, helper constructors, and advanced overrides.
- Coverage audit that every file in the matrix below contains the relevant ergonomic layer and does not introduce a separate behavior path.

## Risk / Gotcha Carry-Forward

- Ergonomic APIs must not become a second source of truth. They must lower into `RunRequest`, `OutputContract`, `RuntimePackage`, events, and journal records.
- Presets must be explicit and testable. Avoid hidden provider-specific behavior that changes validation semantics.
- Typed models should identify schemas by stable type/schema IDs and versions, not by fragile display names.

## Required Typed Output Contract

The structured-output docs must define a contract equivalent to:

- `TypedOutputModel`: implemented or derived by caller-owned types.
- `OutputSchemaProvider`: returns schema ID, schema version, dialect, schema ref or schema bytes, and schema fingerprint.
- `OutputSchemaRegistry`: resolves typed models to registered schema refs and validates fingerprint drift.
- `OutputContract::for_type::<T>()`: lowers typed models into the same canonical `OutputContract`.
- `RunRequestBuilder::output::<T>()` or `Agent::run_typed::<T>()`: ergonomic one-liner that still emits the same events, journal records, retry attempts, and typed failures.
- `StructuredOutputResult<T>` construction boundary: parse into `T` only after local schema validation and semantic validation succeed.

## Ergonomics Proof Template

Every touched contract example must show:

- `Simple API`: the intended one-liner or minimal builder.
- `Advanced API`: the explicit builder/config path for power users.
- `Canonical lowering`: the exact stable contract produced by the simple API.
- `Equivalence`: same events, journal records, retries, policies, telemetry, and failures as the canonical path.
- `SDK owns / Host owns`: boundary block proving no product-specific behavior entered core.
- `Tests`: acceptance tests or golden fixtures for both simple and advanced paths.

## Coverage Matrix

| File | Ergonomic layer required |
| --- | --- |
| `docs/contracts/api-contracts.md` | `Agent::run_text`, `Agent::run_typed::<T>`, `RunRequestBuilder`, advanced `AgentRuntimeBuilder`, canonical lowering rules |
| `docs/contracts/structured-output-contract.md` | typed model trait, schema provider/registry, presets, defaulted validation/repair builders, advanced override block, typed result boundary |
| `docs/contracts/runtime-package-schema.md` | capability/tool-pack/package builder presets and explicit advanced canonical builder |
| `docs/contracts/tool-pack-contract.md` | one-line tool-pack inclusion helpers plus explicit tool spec/policy builder path |
| `docs/contracts/stream-rule-contract.md` | stop/mask/emit helper constructors plus explicit matcher/action/policy builder |
| `docs/contracts/isolation-runtime-contract.md` | `isolated(\"name\")`-style helper lowering into `EnvironmentSpec` plus advanced adapter requirements |
| `docs/contracts/subagent-contract.md` | simple `agent.as_tool` / `spawn_child` helpers lowering into `SubagentRequest` and package stripping rules |
| `docs/contracts/extension-sdk-contract.md` | manifest/capability helpers lowering into explicit manifest capability fields |
| `docs/contracts/telemetry-privacy-contract.md` | default telemetry fanout/content policy helpers lowering into explicit sink/content-capture policy |
| `docs/architecture/primitive-map.md` | simple-vs-advanced design principle across primitives |
| `docs/start-here.md` | high-level SDK ergonomics posture: easy common path, stable contract path, advanced override path |
