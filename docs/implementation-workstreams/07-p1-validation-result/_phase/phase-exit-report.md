# Phase 07 P1 Validation Result Exit Report

## Status

PASS.

## Launch Targets

- `07a-validation-repair.md`: implemented by the validation/repair worker, then stitched by the phase integration owner.
- `07b-typed-result.md`: implemented by the typed-result worker, then stitched by the phase integration owner.

## Implemented Scope

- Added local structured-output validation over Phase 06 `OutputContract` using the JSON Schema 2020-12 subset supported by the SDK.
- Added hostile schema limits, candidate byte limits, redacted validation errors, strict additional-property denial, and fail-closed handling for unsupported schema refs/dialects and unavailable semantic validators.
- Added bounded repair accounting with deterministic repair attempt IDs, repair prompt DTOs, content-ref-only/redacted/omitted candidate policies, and repair exhaustion records.
- Added `ValidatedOutput`, `ValidationReportRecord`, typed-result publication records, and `StructuredOutputResult<T>` metadata over validated output evidence.
- Added `TypedOutputDeserializer<T>` and `DecodedTypedOutput<T>` so typed extraction is fakeable and tied to the validated canonical content ref instead of accepting arbitrary caller-provided values.
- Added shared structured-output journal payload variants through `StructuredOutputRecord`.
- Added structured-output event kinds for Phase 08 runtime emission: `StructuredOutputRequested`, `StructuredOutputValidationStarted`, `StructuredOutputValidationFailed`, `StructuredOutputRepairRequested`, `StructuredOutputValidated`, and `StructuredOutputFailed`.

## Primitive-Lowering Evidence

- Validation and repair reuse the Phase 06 `OutputContract`; no second output contract, run loop, package registry, policy path, or output-delivery path was created.
- Application behavior lives in `application/validation.rs` and `application/repair.rs`; durable validation/repair DTOs live in `records/structured_output.rs`.
- Typed-result DTOs live in `records/validated_output.rs`; construction requires `ValidatedOutput` plus typed publication evidence and a decoder over `canonical_value_ref`.
- Shared journal integration uses the existing `JournalRecordPayload::StructuredOutput` path instead of a feature-specific journal.
- Root Cargo test targets are shims only; test bodies live under `tests/feature_layers/`.

## Mockability And Testability Evidence

- Local validation and repair tests run without live providers, network, random IDs, wall-clock time, process execution, or product UI.
- The validator and repair service are deterministic and can be driven with scripted `OutputCandidate` sequences.
- `TypedOutputDeserializer<T>` is a reusable SDK-consumer seam for testing typed extraction against canonical content refs.
- Tests cover weird/hostile paths: invalid JSON, missing required fields, enum mismatch, strict extra-field denial without `additionalProperties`, hostile remote `$ref`, repair success, repair exhaustion, private candidate redaction, publication before validation, empty validation refs, spoofed publication refs, spoofed validated-output refs, policy denial, and decoder content-ref mismatch.

## Package Architecture Evidence

- Source-root architecture audit PASS: `crates/agent-sdk-core/src` root contains only `README.md` and `lib.rs`.
- Validation/repair behavior is in `src/application/`.
- Durable structured-output records are in `src/records/structured_output.rs` and `src/records/validated_output.rs`.
- Shared event and journal indices remain in `src/records/event.rs` and `src/records/journal.rs`.
- Public exports are in the crate root facade.
- Integration test bodies are in `tests/feature_layers/validation_repair.rs` and `tests/feature_layers/validated_output_contract.rs`; root tests are two-line Cargo shims.

## Validation

- `cargo fmt --check` PASS.
- `cargo test -p agent-sdk-core --test validation_repair` PASS, 8 tests.
- `cargo test -p agent-sdk-core --test validated_output_contract` PASS, 12 tests.
- `cargo test -p agent-sdk-core` PASS, full suite.
- `cargo test -p agent-sdk-core --no-default-features` PASS, full suite.
- `cargo tree -p agent-sdk-core --no-default-features` PASS; dependencies remain `serde`, `serde_json`, `sha2`, and `thiserror` plus transitive dependencies.
- Source-root architecture audit PASS.
- Root Cargo test shim audit PASS.
- No-live-service/product-neutrality audit PASS for Phase 07 code/test surfaces.

## Independent Review Findings

- Reviewer Halley initial verdict: BLOCKED.
- Finding 1: durable validation/repair DTOs lived in application modules. Fixed by moving record DTOs to `records/structured_output.rs`.
- Finding 2: typed result could be minted from arbitrary caller-provided `T`. Fixed with `TypedOutputDeserializer<T>` and canonical content-ref mismatch checks.
- Finding 3: typed publication ordering allowed empty validation refs. Fixed with direct and order-helper evidence checks.
- Finding 4: strict validation allowed provider noise unless the schema repeated `additionalProperties: false`. Fixed by making strict policy deny extras by default and adding a regression test.
- Second review blocker: publication/order helper compared only publication report content-ref keys. Fixed by comparing full `ValidationReportRef` values between typed publication and validated output.
- Third review blocker: validated output still matched prior reports by content-ref key only. Fixed by storing prior `ValidationReportRecord::to_ref()` values and comparing full `ValidationReportRef` metadata at the `ValidatedOutput` step.
- Final reviewer verdict: PASS.

## Explicit Non-Scope

- Phase 08 still owns end-to-end P1 loop integration with fake-provider typed runs.
- Phase 08 still owns runtime event emission and journal append ordering from the live loop.
- Output delivery sink behavior remains out of Phase 07.
- Local `$ref` resolution remains fail-closed; bounded local-ref support would need a later design.

## Files Changed By This Phase

- `docs/implementation-workstreams/07-p1-validation-result/README.md`
- `docs/implementation-workstreams/07-p1-validation-result/_phase/phase-exit-report.md`
- `crates/agent-sdk-core/src/application/validation.rs`
- `crates/agent-sdk-core/src/application/repair.rs`
- `crates/agent-sdk-core/src/records/structured_output.rs`
- `crates/agent-sdk-core/src/records/validated_output.rs`
- `crates/agent-sdk-core/src/records/event.rs`
- `crates/agent-sdk-core/src/records/journal.rs`
- `crates/agent-sdk-core/src/lib.rs`
- `crates/agent-sdk-core/tests/validation_repair.rs`
- `crates/agent-sdk-core/tests/validated_output_contract.rs`
- `crates/agent-sdk-core/tests/feature_layers/validation_repair.rs`
- `crates/agent-sdk-core/tests/feature_layers/validated_output_contract.rs`
- `crates/agent-sdk-core/tests/p0/p0_text_run.rs`
- `crates/agent-sdk-core/tests/fixtures/validation/valid-output-record.json`
- `crates/agent-sdk-core/tests/fixtures/validation/invalid-output-record.json`
- `crates/agent-sdk-core/tests/fixtures/validation/repair-request-record.json`
- `crates/agent-sdk-core/tests/fixtures/validation/repair-exhausted-record.json`
- `crates/agent-sdk-core/tests/fixtures/validated_output/validated-output-record.json`
- `crates/agent-sdk-core/tests/fixtures/validated_output/typed-result-publication-record.json`

## Carry-Forward Watchpoints

- Phase 08 must wire these records into the runtime loop; do not infer typed output from raw provider text or provider-native schema mode.
- Phase 08 should reconcile structured-output event family taxonomy before emission so structured-output validation events do not collide with output-delivery semantics.
- If future multi-report validated outputs allow duplicate report content refs, add explicit duplicate-ref rejection instead of relying on map canonicalization.
- If bounded local `$ref` schema resolution is added later, keep remote refs denied by default and add hostile recursion/size tests.

## Reviewer

Halley: PASS.
