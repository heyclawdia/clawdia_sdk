# Rust API Guidelines Code Audit Plan

## Objective

Audit the existing Rust crates against the newly added Rust API Guidelines review gate and make focused improvements where the code has high-signal gaps.

## Launch Target

- Selected launch target: `docs/implementation-workstreams/12-scenario-verification/12b-api-review.md`
- Rationale: this is a public API quality pass covering facade shape, error ergonomics, common trait behavior, rustdoc, and SemVer posture.
- Writable scope for this pass: public API code and tests needed to prove the API review result, plus this plan. Any unrelated dirty tree state remains out of scope.

## Relevant Existing Context

- `AGENTS.md` requires no branch creation, current implementation workstream routing, public-release audit for broad public handoff, and preservation of SDK-owned versus host-owned boundaries.
- `coding_standards.md` and `docs/architecture/coding-standards.md` now require the Rust API Guidelines gate for public Rust API changes.
- `docs/reference/sdk-review-checklist.md` requires reviewers to check naming, conversions, common traits, meaningful errors, rustdoc, type-safe parameters, future-proofing, crate metadata, dependencies, and license posture.
- `docs/workstreams/validation-gates.md` requires concrete evidence, not prose-only confidence.
- `docs/architecture/primitive-map.md` requires ergonomic helpers to lower into canonical contracts and forbids parallel behavior paths.
- `docs/implementation-workstreams/12-scenario-verification/12b-api-review.md` owns public API review, rustdoc examples, helper lowering tests, and SemVer posture.
- The Rust API Guidelines checklist is the external source for the audit categories.

## Problem Shape

The initial audit showed strong baseline coverage: workspace tests pass, rustdoc builds, crate metadata is present, and missing-doc checks pass for both crates. The main high-signal issue is API ergonomics and performance around `AgentError`: Clippy reports `Result<T, AgentError>` as a large error across many public methods. That weakens the "meaningful and well-behaved error" guideline and makes the public API more expensive than necessary.

Secondary audit findings are ordinary idiomatic Rust cleanup: derive `Default` where it matches manual behavior, remove redundant clones on `Copy` values, and address any remaining non-structural lints that appear after the error-size fix.

## Behavior Contract

### New Behavior

- `AgentError` remains the public SDK error type but is cheaper to pass as a `Result` error.
- Existing constructors and accessor methods keep returning the same owned SDK values.
- Public API tests cover the error-size regression so this does not drift back.
- Clippy-warning fixes preserve behavior and do not add hidden paths.

### Preserved Behavior

- Existing serialized error shape must remain stable.
- Existing error constructors, context mutation helpers, retry/kind/context/causal-id accessors, and tests continue to work.
- No product-specific behavior enters core or toolkit.
- Simple helpers continue lowering into canonical contracts.
- Existing unrelated whitespace-only change in `docs/architecture/architecture-proposal.md` remains out of scope.

### Removed Behavior

- None intentionally. If a direct public enum field type changes to keep `AgentError` small, the API review will record it as an alpha SemVer improvement and tests will prove the stable constructor/accessor path.

### Tests Proving Behavior

- `cargo test --workspace`
- `cargo doc --workspace --no-deps`
- `RUSTFLAGS='-D missing_docs' cargo check -p agent-sdk-core --lib`
- `RUSTFLAGS='-D missing_docs' cargo check -p agent-sdk-toolkit --lib`
- `cargo clippy --workspace --all-targets -- -D warnings` or a documented subset if unrelated pre-existing lints remain.
- Targeted public API test for `AgentError` size and constructor/accessor stability.
- `scripts/public-release-audit.sh`

## Workstreams

1. Add/adjust public API tests for `AgentError` size, serialization, and accessor behavior.
2. Refactor `AgentError` representation to reduce the error variant size while preserving constructors and serialized shape.
3. Fix small idiomatic Rust lints revealed by the audit.
4. Run validation and record any remaining gaps.

## Risks

- Changing public enum field types can affect direct pattern matching users. The SDK is still alpha, and the stable path should be constructor/accessor use; the public API test will lock that down.
- Boxing internals must not change serde JSON shape. Tests must prove round-trip or exact JSON behavior.
- Clippy can surface a very broad backlog. This pass should fix structural/high-signal API issues, not turn into an unrelated cleanup sweep.

## Risk/Gotcha Carry-Forward

- If future code adds fields to `AgentError` or related context types, check `std::mem::size_of::<AgentError>()` and avoid returning a large error by value.
- If future reviewers need to add direct public enum fields, prefer constructor/accessor stability and document SemVer impact in the API review.
- If future linting becomes a release gate, decide explicitly which Clippy groups are normative; do not make private implementation tuning override SDK contract clarity.

## Audit Results

- `AgentError` was the highest-signal public API issue: `cargo clippy --workspace --all-targets -- -D warnings` reported `clippy::result_large_err` across many public `Result<T, AgentError>` APIs. The implemented fix boxes the large classified context and causal-id payloads while preserving constructors, accessors, and serialized JSON shape.
- Public API coverage now includes an `AgentError` size and serialization regression test in `crates/agent-sdk-core/tests/domain/public_api.rs`.
- Mechanical idiomatic Rust issues fixed in this pass: derivable defaults, redundant clones on `Copy` privacy/retention values, `inspect_err` instead of `map_err` for side-effect-only error observation, `to_vec()` instead of `iter().cloned().collect()`, direct string `len()`, simplified integer checks, redundant pattern matching, unit-struct default construction, and boolean assert style.
- The codebase still has a broader structural Clippy backlog if the repository chooses to make full `-D warnings` a release gate. Remaining categories are `clippy::too_many_arguments`, `clippy::large_enum_variant`, and non-`AgentError` `clippy::result_large_err` on large record/result types such as content resolution, journal payloads, output delivery, validated output publication steps, and validation error reports.
- This pass intentionally did not box all durable record enums or redesign multi-argument record constructors. Those are serialized/public contract changes that should be planned as a dedicated API/record-size workstream with fixture review.
