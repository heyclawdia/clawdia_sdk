# Phase 08 Exit Report: P1 Typed Run

## Status

PASS.

Phase 08 integrated typed output over the same P0 loop after Phase 06 output contracts and Phase 07 validation/result records. The phase is ready for Phase 09 side-effect work.

## Scope Delivered

- `Agent::run_typed::<T>`, `AgentRuntime::run_typed::<T>`, and `RunRequest::typed_text::<T>` lower into the normal `RunRequest.output_contract` path.
- P1 structured output runs through the same P0 loop, runtime-package resolution, provider projection, provider effect intent/result, model attempt records, event bus, journal, local validation, repair, and terminal run result.
- `StructuredOutputRequested` is journaled and emitted before provider projection/execution.
- Provider requests carry a `ProviderStructuredOutputHint` derived from the output contract.
- Canonical validated JSON is stored through the content resolver with a hash derived from `ValidationSuccess.canonical_value`.
- Typed extraction uses `ValidatedOutput` + `TypedResultPublicationRecord` + `TypedOutputDeserializer`; the reusable fake deserializer resolves the canonical content ref instead of accepting caller-supplied typed bytes.
- Output delivery sinks remain optional and are not required for P1 typed result extraction.

## Changed Implementation Glue

- `crates/agent-sdk-core/src/application/agent.rs`
- `crates/agent-sdk-core/src/application/runtime.rs`
- `crates/agent-sdk-core/src/application/run.rs`
- `crates/agent-sdk-core/src/application/loop_driver.rs`
- `crates/agent-sdk-core/src/application/repair.rs`
- `crates/agent-sdk-core/src/application/projection.rs`
- `crates/agent-sdk-core/src/ports/provider.rs`
- `crates/agent-sdk-core/src/records/content.rs`
- `crates/agent-sdk-core/src/records/event.rs`
- `crates/agent-sdk-core/src/records/journal.rs`
- `crates/agent-sdk-core/src/records/structured_output.rs`
- `crates/agent-sdk-core/src/records/validated_output.rs`
- `crates/agent-sdk-core/src/lib.rs`

All new or changed source files remain under the owning SDK responsibility folders with only public facade exports at `lib.rs`.

## Tests And Fixtures

- `crates/agent-sdk-core/tests/p1_typed_output.rs` remains a root Cargo test-target shim.
- `crates/agent-sdk-core/tests/feature_layers/p1_typed_output.rs` owns the P1 test body.
- P1 golden fixtures:
  - `crates/agent-sdk-core/tests/fixtures/p1/typed-run-events.json`
  - `crates/agent-sdk-core/tests/fixtures/p1/typed-run-journal.json`
  - `crates/agent-sdk-core/tests/fixtures/p1/repair-success-events.json`
  - `crates/agent-sdk-core/tests/fixtures/p1/repair-success-journal.json`
  - `crates/agent-sdk-core/tests/fixtures/p1/repair-exhausted-events.json`
  - `crates/agent-sdk-core/tests/fixtures/p1/repair-exhausted-journal.json`

Named tests covered valid output, helper/explicit equivalence, repair success, repair exhaustion, canonical ref enforcement, no output sink, package fingerprint impact, and golden event/journal fixtures.

## Validation Evidence

- `cargo fmt --check`: PASS.
- `cargo test -p agent-sdk-core --test p1_typed_output`: PASS, 8 tests.
- `cargo test -p agent-sdk-core --test p0_text_run`: PASS, 3 tests.
- `cargo test -p agent-sdk-core`: PASS.
- `cargo test -p agent-sdk-core --no-default-features`: PASS.
- `cargo tree -p agent-sdk-core --no-default-features`: PASS; no live provider, network, async runtime, or product dependency was introduced.
- Source-root architecture audit: PASS; no unexpected flat implementation files under `src/`.
- Root integration-test shim audit: PASS.
- Product/live-service/output-delivery audit: PASS for Phase 08 surfaces.

## Review

Independent reviewer: Singer (`019e59d1-9395-7123-974c-749a9678fe6f`).

Initial verdict: BLOCKED.

Resolved findings:

- `StructuredOutputRequested` was emitted after provider completion. Fixed by appending/emitting it before provider projection/execution and by projecting `ProviderStructuredOutputHint`.
- Typed extraction could be satisfied by caller-seeded bytes. Fixed by storing canonical JSON derived from validator output in `FakeContentResolver` and resolving that content ref in the typed deserializer test seam.

Final verdict: PASS.

## Boundary And Primitive Gates

- Primitive fit: P1 typed output is a feature-layer integration over the existing run, package, event, journal, validation, repair, content, and provider primitives.
- No mini-SDK: PASS. No second run loop, package registry, event stream, journal, validation path, or output delivery path was added.
- Mockability: PASS. Fake provider, journal, event bus, content resolver, and typed deserializer prove the path without live services.
- Journal/event durability: PASS. Every structured-output event is emitted after the matching journal append has a cursor; `StructuredOutputRequested` now precedes provider projection/execution.
- Host-owned boundaries: PASS. No output sink, UI, live provider, host adapter, or product-specific behavior was added.
- SDK package architecture gate: PASS.

## Carry-Forward Watchpoints

- If future phases need full failed-attempt lineage on successful repair, extend `ValidatedOutput` to include prior failed validation refs and source attempts without duplicating validation-report refs.
- If repair retries need more explicit semantics than `ModelAttemptStarted`/`ModelMessageCompleted`, add a distinct event vocabulary through the event contract rather than ad hoc tags.
- If provider-native structured output support becomes capability-gated, keep provider hints advisory and preserve local validation as the source of truth.
