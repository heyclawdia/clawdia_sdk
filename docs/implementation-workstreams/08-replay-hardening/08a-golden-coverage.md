# Golden Coverage

## Phase

[Phase 08: Replay Hardening](README.md)

## Parallelism

Parallel-safe with [Replay Recovery](08b-replay-recovery.md) and [Privacy Performance](08c-privacy-performance.md).

## Contract Inputs

- [event-schema.md](../../contracts/event-schema.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)
- [runtime-package-schema.md](../../contracts/runtime-package-schema.md)
- [otel-mapping-contract.md](../../contracts/otel-mapping-contract.md)

## Implementation Objective

Close fixture gaps for every implemented durable or exported schema before scenario release work starts.

## Owned Implementation Surface

- fixture files under `crates/agent-sdk-core/tests/fixtures/`
- `crates/agent-sdk-core/tests/contract_golden.rs`
- optional crate fixture tests where feature crates exist

## Must Deliver

- Fixture manifest enumerating implemented event kinds, journal record kinds, runtime package snapshots, package deltas, OTel projections, extension protocol records, and scenario outputs.
- Golden tests that fail on unreviewed schema drift.
- Redaction cases for every fixture family that can contain content.

## Validation

- `cargo test -p agent-sdk-core --test contract_golden`
- optional crate golden tests where relevant
- emitted-kind matrix audit
- fixture manifest completeness audit

## Must Not

- Mark a kind implemented unless its fixture and redaction case exist.
