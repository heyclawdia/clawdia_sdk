# Phase 06 Output Contract Exit Report

## Status

PASS.

## Launch Targets

- `06a-output-contract.md`: implemented locally as the only Phase 06 launch target.

## Implemented Scope

- Added `OutputContract` and schema-ref DTOs under `records/output.rs`.
- Added output-contract typed IDs through the shared ID pattern: `OutputSchemaId`, `ValidatedOutputId`, `ValidationAttemptId`, and `RepairAttemptId`.
- Added typed model helper shape through `TypedOutputModel`, `OutputContract::for_type::<T>()`, `RunRequest::typed_text::<T>()`, and `Agent::typed_text_request::<T>()`.
- Added `RunRequest.output_contract` and `with_output_contract` lowering without creating a second run path.
- Normalized `RunRequest.output_contract` into the effective runtime package through `RuntimePackage::with_output_contract`, changing the package fingerprint while excluding run IDs and prompt text.
- Expanded `OutputContractSnapshot` so the package snapshot captures schema ID/version/fingerprint, dialect, mode, local validator ref, repair ref, and provider hint policy.
- Expanded `OutputContractSnapshot` to include canonicalized validation, repair, retry, content-capture, and projection policy fields so all output execution-affecting fields are runtime-package fingerprint inputs.
- Changed inline JSON schema construction so the SDK derives the schema fingerprint from canonicalized JSON instead of trusting caller-supplied hash text.

## Primitive-Lowering Evidence

- Typed output helpers build a normal `RunRequest` with `output_contract: Some(...)`; they do not execute a separate typed run path.
- `AgentRuntime::resolve_effective_package` folds request output contracts into the cloned effective package before validation and fingerprinting.
- `RuntimePackage` remains the package authority and fingerprint source; output contracts are not capabilities.
- Provider-assisted mode is only a projection hint. Tests prove the local validator ref remains the authority.
- Tests prove changes to validation limits, repair attempts, retry budget, projection hints, and content policy change the package fingerprint.
- Tests prove reordered inline JSON yields the same schema hash while changed schema content yields a different hash.

## Mockability And Testability Evidence

- Tests are deterministic and use only local DTOs, `AgentRuntime`, and fake-free package resolution; no network, process, random, clock, or remote service dependency is required.
- SDK consumers can implement `TypedOutputModel` with a content-store schema ref and assert the same helper-lowering and package-normalization behavior.
- Golden fixture `inline-json-contract.json` covers schema-ref serde and safe redaction/default policies.

## Package Architecture Evidence

- Output DTOs live in `crates/agent-sdk-core/src/records/output.rs`.
- Output-specific IDs live in `domain/ids.rs`; facade exports are in `domain/mod.rs` and `lib.rs`.
- Run helper lowering lives in `application/run.rs` and `application/agent.rs`.
- Runtime-package normalization lives in `application/runtime.rs` and `package/mod.rs`.
- Integration tests live under `tests/feature_layers/output_contract.rs`; `tests/output_contract.rs` is a root Cargo shim only.

## Validation

- `cargo fmt --check` PASS.
- `cargo test -p agent-sdk-core --test output_contract` PASS, 7 tests.
- `cargo test -p agent-sdk-core` PASS, full suite.
- `cargo test -p agent-sdk-core --no-default-features` PASS, full suite.
- `cargo tree -p agent-sdk-core --no-default-features` PASS; dependencies remain `serde`, `serde_json`, `sha2`, and `thiserror` plus transitive dependencies.
- Source-root architecture audit PASS: no files under `src/` root other than `lib.rs` and `README.md`.
- Root Cargo test shim audit PASS: no `#[test]` or test function bodies in `crates/agent-sdk-core/tests/*.rs`.
- No-live-service audit PASS for Phase 06 code/test surfaces.
- Product-neutrality audit PASS for Phase 06 code/test surfaces. Existing prose in `structured-output-contract.md` names external SDK lessons and remains source context, not an active host adapter.

## Independent Review Findings

- Initial reviewer verdict: BLOCKED.
- Finding 1: output package fingerprint dropped execution-affecting policy fields. Fixed by embedding validation, repair, retry, content-capture, and projection policy fields in `OutputContractSnapshot` plus fingerprint-change tests.
- Finding 2: inline schema hashes were trusted. Fixed by deriving `ContentHash` from canonical JSON and adding reorder-equivalence/content-change tests.
- Re-validation after fixes: `cargo fmt --check`, `cargo test -p agent-sdk-core --test output_contract`, full default test suite, no-default test suite, tree, architecture audits, no-live-service audit, and product-neutrality audit passed.
- Re-review verdict: PASS.

## Explicit Non-Scope

- No local validation execution was implemented.
- No repair loop was implemented.
- No `StructuredOutputResult<T>` or typed result publication was implemented.
- No output delivery sink behavior was implemented.

## Files Changed By This Phase

- `docs/implementation-workstreams/06-output-contract/06a-output-contract.md`
- `docs/implementation-workstreams/06-output-contract/_phase/phase-exit-report.md`
- `crates/agent-sdk-core/src/records/output.rs`
- `crates/agent-sdk-core/src/domain/ids.rs`
- `crates/agent-sdk-core/src/domain/mod.rs`
- `crates/agent-sdk-core/src/application/run.rs`
- `crates/agent-sdk-core/src/application/agent.rs`
- `crates/agent-sdk-core/src/application/runtime.rs`
- `crates/agent-sdk-core/src/package/mod.rs`
- `crates/agent-sdk-core/src/lib.rs`
- `crates/agent-sdk-core/tests/output_contract.rs`
- `crates/agent-sdk-core/tests/feature_layers/output_contract.rs`
- `crates/agent-sdk-core/tests/fixtures/output_contract/inline-json-contract.json`
- `crates/agent-sdk-core/tests/runtime/runtime_contract.rs`

## Carry-Forward Watchpoints

- Phase 07 must implement validation/repair as records and local validator ports over this contract; do not construct typed values from raw provider text.
- If a schema derive crate is introduced later, keep it optional or behind a feature and keep `TypedOutputModel` mockable without macro expansion.
- If output contract snapshots gain additional execution-affecting fields, update package fingerprint fixtures and reorder-equivalence tests.

## Reviewer

Chandrasekhar: PASS.
