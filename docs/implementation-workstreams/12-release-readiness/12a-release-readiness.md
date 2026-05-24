# Release Readiness

## Phase

[Phase 12: Release Readiness](README.md)

## Parallelism

Only launch target in this phase. Run after Phase 11 scenario and API verification exits.

## Contract Inputs

- [contracts/README.md](../../contracts/README.md)
- [review-matrix.md](../../contracts/review-matrix.md)
- [validation-gates.md](../../workstreams/validation-gates.md)

## Implementation Objective

Prepare the first implementation handoff with packaging, docs, feature flags, verification evidence, and final review.

## Owned Implementation Surface

- Cargo workspace/package metadata
- crate-level README and docs generated from implementation state
- release notes or changelog path chosen by the repo
- final phase exit report under `docs/implementation-workstreams/12-release-readiness/_phase/`

## Must Deliver

- `cargo fmt --check`, full `cargo test`, optional crate tests, scenario tests, golden fixtures, and docs checks all recorded.
- Feature flag matrix showing `agent-sdk-core` default build and optional crates.
- Contract-to-code traceability matrix.
- Release notes naming unsupported live/provider/container/product-host paths.
- Final reviewer PASS before release handoff.

## Validation

- `cargo fmt --check`
- `cargo test --workspace`
- `cargo test -p agent-sdk-core --test contract_golden`
- `cargo test -p agent-sdk-core --test replay_recovery`
- `cargo test -p agent-sdk-core --test policy_matrix`
- docs link/path audit
- product-neutrality audit

## Must Not

- Publish or tag a release unless the user explicitly asks for release execution.
- Claim live-provider, concrete-container, product-UI, or host-adapter support without matching tests and docs.
