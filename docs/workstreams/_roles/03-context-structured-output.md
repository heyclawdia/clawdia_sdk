# Owner Role 03: Context, Memory, And Structured Output

## Owner Role

Context projection, memory boundary, and typed output agent.

## Writable Files

- `docs/contracts/content-artifact-ref-contract.md`
- `docs/contracts/context-memory-contract.md`
- `docs/contracts/structured-output-contract.md`

## Future Implementation Writable Scope

Once SDK code exists, this workstream may own context and structured-output validation modules and tests only, for example:

- `crates/agent-sdk-core/src/context/**`
- `crates/agent-sdk-core/src/structured_output/**`
- `crates/agent-sdk-core/src/output/validation/**`
- `crates/agent-sdk-core/tests/context_*.rs`
- `crates/agent-sdk-core/tests/structured_output_*.rs`
- first-slice fixtures for emitted `memory_context` and `structured_output` event kinds

## Read-Only Inputs

- `docs/contracts/event-schema.md`
- `docs/contracts/journal-replay-schema.md`
- `docs/architecture/primitive-map.md`
- `docs/architecture/architecture-proposal.md`
- `docs/architecture/observability-and-lineage.md`
- `docs/examples/memory-context-compaction.md`

## Contract To Deliver

Define artifact/content refs, resolver policy, missing-ref recovery, context contributions/candidates, admission decisions, context item source/destination/injection metadata, optional memory ports, compaction boundaries, provider projection audit fields, typed output model ergonomics, schema registry behavior, validation policy, repair policy, retry bounds, and typed result construction.

## Must Not Own

Host memory browsing UI, product-specific form rendering, business scoring, or raw provider transcript storage.

## Integration Handoff

Send schema ID/version/fingerprint rules, context IDs, output event names, validation failure names, memory event/journal names, and projection audit fields to the stitching owner. Put proposal text in the handoff; do not edit shared reference or architecture files unless the stitching owner delegates it.

## Required Validation

- Lowering tests: `run_typed_lowers_to_output_contract`, `typed_model_schema_ref_is_stable`, `helper_presets_lower_to_explicit_validation_and_repair_policy`.
- Validation tests: valid output constructs typed result; invalid output returns typed validation error; provider-assisted structured output still validates locally.
- Repair tests: bounded repair retry count, repair prompt redacts invalid content, final invalid output emits failure and journal record.
- Projection tests: provider projection strips unsafe metadata, records projection audit fields, and preserves source/destination/lineage refs internally.
- Schema tests: inline schema and registry/content-ref schema paths produce the same canonical `OutputContract` fingerprint.
- Event/journal audit: structured-output requested/validation/repair/validated/failed events and records exist for every outcome.
- Memory/context audit: retrieval, contribution admission, injection, compaction, omitted-item, and no-raw-content defaults use `ArtifactRef`/`ContentRef`, `ContextContribution`, `ContextItem`, `ContextProjection`, `RunJournal`, and `AgentEvent`.
- Primitive-lowering review: context and output helpers must reuse `AgentMessage`, `ArtifactRef`/`ContentRef`, `ContextContribution`, `ContextItem`, `ContextProjection`, `OutputContract`, `ValidatedOutput`, `RunJournal`, and `AgentEvent`; no shadow transcript, memory store, provider-context bag, or output validator path.
- Handoff evidence: schema fixtures, projection audit fixture, repair matrix, memory/context fixture list, and ergonomic examples.
