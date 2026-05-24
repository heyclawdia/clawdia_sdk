# Content Context

## Phase

[Phase 02: Core Records](README.md)

## Parallelism

Parallel-safe with the other Phase 02 core-record launch targets. Do not implement memory backends or provider transport here.

## Contract Inputs

- [content-artifact-ref-contract.md](../../contracts/content-artifact-ref-contract.md)
- [context-memory-contract.md](../../contracts/context-memory-contract.md)
- [structured-output-contract.md](../../contracts/structured-output-contract.md)

## Implementation Objective

Implement content refs and context projection records so provider-visible context is explicit, policy-admitted, and auditable.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/content.rs`
- `crates/agent-sdk-core/src/context.rs`
- `crates/agent-sdk-core/tests/context_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/context/`

## Must Deliver

- `ArtifactRef`, `ContentRef`, resolver policy structs, missing-ref behavior, `AgentMessage`, `ContextContribution`, `ContextItem`, `ContextProjection`, `ContextSelectionDecision`, and `ContextProjectionAudit`.
- Fake content resolver and projection audit fixtures.
- Privacy and retention metadata on all context records.

## Validation

- `cargo test -p agent-sdk-core --test context_contract`
- projection audit golden fixtures
- missing required content ref blocks provider projection
- raw content opt-in and redacted-summary tests

## Must Not

- Treat `ContentRef` as provider visibility by itself.
- Let memory, files, skills, tools, or subagents bypass context admission.
