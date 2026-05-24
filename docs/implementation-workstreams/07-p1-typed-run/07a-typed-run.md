# Typed Run

## Phase

[Phase 07: P1 Typed Run](README.md)

## Parallelism

Only launch target in this phase. It integrates Phase 05 and Phase 06 outputs before P2 side effects start.

## Contract Inputs

- [structured-output-contract.md](../../contracts/structured-output-contract.md)
- [api-contracts.md](../../contracts/api-contracts.md)
- [event-schema.md](../../contracts/event-schema.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)

## Implementation Objective

Prove `agent.run_typed::<T>` and explicit `RunRequest.output_contract` run through the same P0 loop, validate locally, repair within policy, and publish a typed result only after durable validation evidence exists.

## Owned Implementation Surface

- `crates/agent-sdk-core/tests/p1_typed_output.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/p1/`
- integration glue in existing Phase 04/05/06-owned modules only where required by the P1 path

## Must Deliver

- End-to-end P1 fake-provider tests for valid output, invalid output, repair success, repair exhaustion, and typed extraction.
- P1 event and journal golden fixtures.
- Runtime-package fingerprint test proving output-contract normalization participates in the effective package.
- `RunResult` typed extraction over `ValidatedOutput`.

## Validation

- `cargo test -p agent-sdk-core --test p1_typed_output`
- P1 golden event and journal fixtures
- helper lowering and explicit request path equivalence tests
- test that output delivery sinks are not required for typed result extraction

## Must Not

- Add tool execution, approvals, output delivery, isolation, subagents, extensions, realtime, or telemetry exporters as P1 prerequisites.
