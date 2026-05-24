# Stream Realtime

## Phase

[Phase 10: Feature Ports](README.md)

## Parallelism

Parallel-safe with the other Phase 10 feature-port launch targets.

## Contract Inputs

- [stream-rule-contract.md](../../contracts/stream-rule-contract.md)
- [event-schema.md](../../contracts/event-schema.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)
- [otel-mapping-contract.md](../../contracts/otel-mapping-contract.md)

## Implementation Objective

Implement stream-rule and realtime sidecars over stream deltas, policy, events, journals, and provider/realtime ports.

## Owned Implementation Surface

- stream/realtime package sidecars in `crates/agent-sdk-core/src/package/stream.rs` and `crates/agent-sdk-core/src/package/realtime.rs`
- stream/realtime durable records in `crates/agent-sdk-core/src/records/stream.rs` and `crates/agent-sdk-core/src/records/realtime.rs`
- realtime/stream adapter ports in `crates/agent-sdk-core/src/ports/realtime.rs`
- stream-rule and realtime coordination in `crates/agent-sdk-core/src/application/stream.rs` and `crates/agent-sdk-core/src/application/realtime.rs`
- root Cargo test shim `crates/agent-sdk-core/tests/stream_realtime_contract.rs`
- test body `crates/agent-sdk-core/tests/feature_layers/stream_realtime_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/stream_realtime/`
- optional `crates/agent-sdk-realtime/` only if the phase exit plan chooses a crate split

Do not add flat implementation files directly under `src/`; exports from `lib.rs`
are integration/stitching glue.

## Must Deliver

- `StreamDelta`, `StreamRule`, `StreamMatcher`, `StreamRuleEngine`, `StreamIntervention`, realtime sidecar, realtime adapter trait, and `RealtimeSessionRecord`.
- Bounded literal/regex/marker matching with privacy-visible channels only.
- Intervention intent/result mapping without `EffectKind::StreamIntervention`.
- Restart, interruption, backpressure, repeat-state, and completion-after-drain behavior.

## Validation

- `cargo test -p agent-sdk-core --test stream_realtime_contract`
- split-chunk matcher tests
- regex timeout/backtracking tests
- realtime restart/backpressure fixtures
- OTel mapping golden fixtures for implemented kinds

## Must Not

- Match hidden chain-of-thought.
- Treat final visible text as terminal run completion.
- Create a separate stream ledger or event stream.
