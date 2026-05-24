# Phase 11 Exit Report: Replay Hardening

## Status

PASS.

## Scope Completed

- Golden coverage now has a manifest-backed schema drift gate for implemented event kinds, journal record kinds, package snapshots/deltas, OTel projections, extension records, and replay/privacy fixture references.
- Replay recovery now has DDD application modules for replay reduction, checkpoint storage, anti-entropy scanning, unsafe-pending manifests, cursor compatibility, and host-archive-required boundaries for non-run durable replay.
- Privacy/performance tests now cover raw-content defaults and opt-in gates across events, journals, telemetry, context, stream rules, tools, and outputs, plus bounded queues, slow-sink isolation, hot-path envelope filtering, and terminal overflow preservation.

## DDD And SDK Layout Evidence

- Replay implementation lives under `crates/agent-sdk-core/src/application/`; no flat implementation modules were added under `src/`.
- Public API exposure is through `crates/agent-sdk-core/src/lib.rs` facade exports.
- Root integration test targets are two-line shims into domain-oriented test folders.
- Test fakes and scripted adapters remain under `agent_sdk_core::testing`; production replay/checkpoint/anti-entropy code does not depend on fake implementations.

## Validation Evidence

- `cargo test -p agent-sdk-core --test contract_golden --test replay_recovery --test privacy_performance` passed.
- `cargo test -p agent-sdk-core --test privacy_performance event_subscriber_overflow_policies_apply_distinct_semantics -- --nocapture` passed.
- `cargo test -p agent-sdk-core` passed.
- `cargo test -p agent-sdk-core --no-default-features` passed.
- `cargo test -p agent-sdk-toolkit` passed.
- `cargo test --workspace` passed.
- `cargo fmt --check` passed.
- `cargo tree -p agent-sdk-core --no-default-features` passed; core remains on serde, serde_json, sha2, thiserror, and transitive dependencies only.
- `find crates/agent-sdk-core/src -maxdepth 1 -type f -not -name lib.rs -not -name README.md -print` returned no files.
- `find crates -path '*/src/*.rs' -maxdepth 3 -type f | sort` returned only crate facade files.
- `find crates/agent-sdk-core/tests -maxdepth 1 -type f -name '*.rs' -exec wc -l {} + | sort -n` confirmed root test targets are two-line shims.
- `rg -n "reqwest|hyper|tokio|ureq|aws_sdk|aws-config|stripe|twilio|kubernetes|openai|anthropic|api_key|api key" Cargo.toml crates/agent-sdk-core/Cargo.toml crates/agent-sdk-core/src crates/agent-sdk-core/tests -g '!target/**'` found only a forbidden-marker string inside the golden redaction test.

## Independent Review

Independent reviewer Erdos returned PASS after the emitted-event and bounded
subscription queue fixes. A later review from Hooke found one additional blocker:
subscriber overflow policies were bounded but not semantically distinguished.

That blocker was fixed by making live event subscriptions reject
`BackpressureCaller`, making `DropProgress`, `SummarizeAndContinue`, and
`FailSubscriber` apply distinct behavior, and adding
`event_subscriber_overflow_policies_apply_distinct_semantics`.

Independent reviewer Aristotle returned PASS on the second blocker fix and
confirmed DDD/package organization, product neutrality, and mockability gates.
