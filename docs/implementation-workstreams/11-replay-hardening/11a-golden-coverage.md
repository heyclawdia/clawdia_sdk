# Golden Coverage

## Phase

[Phase 11: Replay Hardening](README.md)

## Parallelism

Parallel-safe with [Replay Recovery](11b-replay-recovery.md) and [Privacy Performance](11c-privacy-performance.md). It owns the manifest and cross-family golden audit, not the replay/privacy fixture subtrees owned by sibling targets.

## Contract Inputs

- [event-schema.md](../../contracts/event-schema.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)
- [runtime-package-schema.md](../../contracts/runtime-package-schema.md)
- [otel-mapping-contract.md](../../contracts/otel-mapping-contract.md)

## Implementation Objective

Close fixture gaps for every implemented durable or exported schema before scenario verification starts.

## Owned Implementation Surface

- fixture manifest and cross-family golden fixture files under `crates/agent-sdk-core/tests/fixtures/golden/`
- `crates/agent-sdk-core/tests/contract_golden.rs`
- optional crate fixture tests where feature crates exist

## Must Deliver

- Fixture manifest enumerating implemented event kinds, journal record kinds, runtime package snapshots, package deltas, OTel projections, and extension protocol records.
- Golden tests that fail on unreviewed schema drift.
- Redaction cases for every fixture family that can contain content.
- Audit hooks that include replay and privacy fixtures by reference without owning their files.

## Validation

- `cargo test -p agent-sdk-core --test contract_golden`
- optional crate golden tests where relevant
- emitted-kind matrix audit
- fixture manifest completeness audit

## Must Not

- Mark a kind implemented unless its fixture and redaction case exist.
- Own scenario output fixtures; Phase 12 owns scenario fixtures.
- Overwrite replay or privacy fixture subtrees owned by sibling targets.
