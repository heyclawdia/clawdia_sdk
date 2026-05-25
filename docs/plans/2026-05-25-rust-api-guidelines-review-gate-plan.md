# Rust API Guidelines Review Gate Plan

## Objective

Add the most important applicable Rust API Guidelines from `rust-lang/api-guidelines` to the Agent SDK coding standards as an explicit review gate for public Rust API changes.

## Launch Target

- Selected launch target: `docs/implementation-workstreams/12-scenario-verification/12b-api-review.md`
- Rationale: this change affects public API, rustdoc, SemVer posture, and simplicity review criteria rather than runtime behavior.
- Broad stitching-owned guidance files named for this standards pass: `coding_standards.md`, `docs/architecture/coding-standards.md`, and `docs/reference/sdk-review-checklist.md`.

## Relevant Existing Context

- `README.md` and `docs/start-here.md` define this repository as the product-neutral Rust-first SDK source of truth.
- `coding_standards.md` is the root standards entry point and points to `docs/architecture/coding-standards.md` as authoritative.
- `docs/architecture/coding-standards.md` already contains Rust API standards, package architecture gates, rustdoc requirements, and SemVer review expectations.
- `docs/workstreams/validation-gates.md` requires concrete evidence and treats public API/package topology as implementation gates once code exists.
- `docs/reference/sdk-review-checklist.md` is the reviewer-facing rubric for API quality, public facade, package topology, simplicity, and product-neutrality.
- `docs/architecture/primitive-map.md` requires simple APIs to lower into canonical contracts instead of creating parallel behavior paths.
- `docs/implementation-workstreams/12-scenario-verification/12b-api-review.md` owns the public API review pass and SemVer posture.
- The Rust API Guidelines checklist is the external source for crate-level API review categories: naming, interoperability, documentation, predictability, flexibility, type safety, dependability, debuggability, future proofing, and crate necessities.

## Behavior Contract

### New Behavior

- Public Rust API review must explicitly check a condensed Rust API Guidelines gate.
- Reviewers must look for Rust naming and conversion conventions, common trait implementations, meaningful errors, rustdoc examples and failure docs, predictable constructors/methods, type-safe parameters, future-proof public types, and crate metadata/dependency/license posture.
- Standards must frame the upstream guidelines as important review considerations, not as blind mandates that override SDK primitive, privacy, policy, durability, or product-neutrality rules.

### Preserved Behavior

- The SDK remains product-neutral.
- Simple helpers must still lower into canonical contracts.
- Existing package topology, mockability, journal/event/privacy, and primitive-lowering gates remain authoritative.
- Documentation-only work remains markdown-only and must not create Rust source, manifests, executable tests, or fixtures.

### Removed Behavior

- None.

### Tests Proving Behavior

- Markdown/source search evidence will verify the new review gate appears in the root standards, authoritative architecture standards, and SDK review checklist.
- Public release audit will be run because this changes public-facing standards.
- No Rust tests are required because this is documentation-only and does not change code.

## Workstreams

1. Add a concise root standards pointer so future agents see the Rust API Guidelines gate early.
2. Add an authoritative `Rust API Guidelines Review Gate` section to `docs/architecture/coding-standards.md`.
3. Add a reviewer checklist row and output-format prompt to `docs/reference/sdk-review-checklist.md`.
4. Validate with targeted `rg` checks and the public release audit.

## Validation Plan

- `rg -n "Rust API Guidelines|C-CASE|C-COMMON-TRAITS|C-FAILURE|C-NEWTYPE|C-SEALED|C-METADATA" coding_standards.md docs/architecture/coding-standards.md docs/reference/sdk-review-checklist.md`
- `scripts/public-release-audit.sh`
- `git diff --check`

## Risks

- Over-importing the guideline checklist could make reviews noisy. The standards should identify the SDK-relevant subset and keep lower-priority items as context.
- Some upstream guidelines are crate-general rather than SDK-specific. SDK primitive-lowering, observability, durability, privacy, product-neutrality, and mockability gates remain stricter where they apply.
- Public API future-proofing must not become vague. The gate should name concrete checks such as private fields, sealed traits where appropriate, newtype encapsulation, non-exhaustive strategy, and SemVer notes.

## Risk/Gotcha Carry-Forward

- If future reviewers add a public helper, require proof that it lowers into canonical contracts and does not skip validation, policy, journal, events, telemetry, lineage, or redaction.
- If future reviewers add public DTOs, require explicit trait, serialization, visibility, and future-extension posture rather than relying on ad hoc derives.
- If future reviewers add adapter or port APIs, require deterministic fake/conformance support and avoid ambient host state or product-specific behavior.
- If future reviewers add crate metadata, docs, examples, or dependencies, check the release-readiness gate and public audit before publishing.
