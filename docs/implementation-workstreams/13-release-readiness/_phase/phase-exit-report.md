# Phase 13 Exit Report: Release Readiness

## Status

PASS.

## Scope Completed

Changed release-readiness surfaces:

- `Cargo.toml`
- `crates/agent-sdk-core/Cargo.toml`
- `crates/agent-sdk-toolkit/Cargo.toml`
- `crates/agent-sdk-core/README.md`
- `crates/agent-sdk-toolkit/README.md`
- `CHANGELOG.md`
- `crates/agent-sdk-core/tests/policy_matrix.rs`
- `crates/agent-sdk-core/tests/domain/policy_matrix.rs`
- `docs/implementation-workstreams/13-release-readiness/_phase/feature-flag-matrix.md`
- `docs/implementation-workstreams/13-release-readiness/_phase/contract-to-code-traceability.md`
- `docs/implementation-workstreams/13-release-readiness/_phase/phase-exit-report.md`

## Package Metadata

- Both current crates now have crate-level READMEs and descriptions.
- Both current crates use `publish = false` for this handoff because no publish/tag release was requested and no live/provider/container/product-host support is included.
- The invalid placeholder repository metadata was removed from the workspace package metadata.

## Feature Flag Matrix

See [feature-flag-matrix.md](feature-flag-matrix.md).

Current posture:

- `agent-sdk-core` default features are empty.
- `agent-sdk-core --no-default-features` must build and test without optional crates.
- `agent-sdk-core --all-features` currently enables only the reserved `test-support` feature and must not add live providers or host infrastructure.
- `agent-sdk-toolkit` is an optional separate crate that depends on core; core has no reverse dependency.
- `agent-sdk-isolation`, `agent-sdk-otel`, `agent-sdk-extension`, and `agent-sdk-workflow` are not published or implemented as concrete optional crates in this handoff.

## Contract To Code Traceability

See [contract-to-code-traceability.md](contract-to-code-traceability.md).

Every normative contract in `docs/contracts/README.md` maps to an owning source responsibility folder plus at least one test or fixture family. Scenario references map to fake-only scenario matrix tests and the public API review suite.

## Release Notes

See [../../../../CHANGELOG.md](../../../../CHANGELOG.md).

The handoff notes explicitly state that live providers, concrete container runtimes, product UI/host adapters, network telemetry exporters, marketplace runtimes, workflow engines, and product-owned memory backends are unsupported.

## DDD, Mockability, And Package Architecture Evidence

- Source layout keeps implementation under `domain`, `package`, `records`, `ports`, `application`, and `testing`.
- `src/lib.rs` remains a public facade and rustdoc surface.
- Root integration tests remain stable Cargo target shims; the new `policy_matrix` target delegates into `tests/domain/policy_matrix.rs`.
- The policy matrix test is table-driven, deterministic, serializes/deserializes decisions, and covers missing dependencies and content-capture gates as SDK-consumer conformance surfaces.
- Public fakes and scripted adapters remain under `agent_sdk_core::testing`.
- Release notes and crate READMEs document that live providers, concrete runtimes, and product adapters are unsupported.

## Validation Evidence

- `cargo fmt --check` PASS.
- `cargo test --workspace` PASS, including `agent-sdk-core`, `agent-sdk-toolkit`, and doc tests.
- `cargo test -p agent-sdk-core --doc` PASS, 2 doc tests.
- `cargo test -p agent-sdk-core --no-default-features` PASS.
- `cargo test -p agent-sdk-core --all-features` PASS; only the reserved `test-support` feature is added.
- `cargo test -p agent-sdk-core --test contract_golden` PASS, 7 tests.
- `cargo test -p agent-sdk-core --test replay_recovery` PASS, 7 tests.
- `cargo test -p agent-sdk-core --test policy_matrix` PASS, 2 tests.
- `cargo test -p agent-sdk-core --test scenario_matrix` PASS, 7 tests.
- `cargo test -p agent-sdk-core --test public_api` PASS, 4 tests.
- `cargo test -p agent-sdk-toolkit` PASS, 7 integration tests and 0 doc tests.
- `cargo tree -p agent-sdk-core --no-default-features` PASS; core contains only `serde`, `serde_json`, `sha2`, `thiserror`, and transitive support crates.
- `cargo tree -p agent-sdk-toolkit` PASS; toolkit depends on core plus helper dependencies such as `regex`, while core has no reverse toolkit dependency.
- `cargo package -p agent-sdk-core --allow-dirty --list` PASS; package includes crate README, source responsibility folders, integration-test shims/bodies, and fixtures.
- `cargo package -p agent-sdk-toolkit --allow-dirty --list` PASS; package includes crate README, toolkit modules, fixtures, and tests.
- `git diff --check` PASS.
- Product-neutrality audit PASS: `if rg -n '(Clawdia|clawdia|Pawtrace|pawtrace|iMessage|ACP)' crates/agent-sdk-core/src crates/agent-sdk-toolkit/src crates/agent-sdk-core/tests/fixtures crates/agent-sdk-core/README.md crates/agent-sdk-toolkit/README.md CHANGELOG.md docs/implementation-workstreams/13-release-readiness/README.md docs/implementation-workstreams/13-release-readiness/13a-release-readiness.md docs/implementation-workstreams/13-release-readiness/_phase/feature-flag-matrix.md docs/implementation-workstreams/13-release-readiness/_phase/contract-to-code-traceability.md; then exit 1; else rg_status=$?; if [ "$rg_status" -eq 1 ]; then echo 'PASS: no product-specific matches in audited release surfaces'; else exit "$rg_status"; fi; fi` printed `PASS: no product-specific matches in audited release surfaces`.
- Docs link/path audit PASS: markdown links in the release-readiness surfaces were enumerated with `rg -n '\[[^]]+\]\([^)]+\)' ...`, and `test -f` passed for the Phase 13 feature matrix, traceability matrix, changelog, and crate READMEs.

## Source Layout Audit

- `find crates/agent-sdk-core/src -maxdepth 1 -type f -not -name lib.rs -not -name README.md` returned no files.
- `find crates/agent-sdk-core/tests -maxdepth 1 -type f -name '*.rs' -print -exec sh -c 'wc -l "$1"' sh {} \;` showed every root integration test target is a two-line shim, including the new `policy_matrix.rs`.
- `find crates -path '*/src/*.rs' -maxdepth 3 -type f` returned only `crates/agent-sdk-core/src/lib.rs` and `crates/agent-sdk-toolkit/src/lib.rs` as direct crate-root source files.
- `rg -n '#\[path = .*\]\s*pub mod|pub mod [a-zA-Z0-9_]+;' crates/agent-sdk-core/src/lib.rs` is non-empty by design because `lib.rs` is the documented public facade. Phase 13 added no new public deep module alias.
- `rg -n '\b(Fake|Scripted)[A-Za-z0-9_]+|ConformanceHarness' crates/agent-sdk-core/src --glob '*.rs'` found fake/scripted/conformance helpers only in `src/testing` plus their documented `agent_sdk_core::testing` re-exports.
- `rg -n '\btrait\b|\bAdapter\b|\bResolver\b|\bFake\b|\bScripted\b|ConformanceHarness' crates/agent-sdk-core/src/records --glob '*.rs'` returned no matches, so durable records did not absorb port or fake behavior.
- `wc -l crates/agent-sdk-*/src/lib.rs` showed `agent-sdk-core/src/lib.rs` at 589 lines as the crate-level facade/rustdoc surface and `agent-sdk-toolkit/src/lib.rs` at 25 lines as a narrow optional-crate facade.

## Independent Review

Independent reviewer Pauli initially returned BLOCKED because the product-neutrality audit evidence included the phase exit report itself, causing the quoted denylist command to self-match. The audit scope was corrected to target release surfaces and exclude the evidence report itself. The corrected command printed `PASS: no product-specific matches in audited release surfaces`.

Pauli re-reviewed the fix, reran `cargo fmt --check`, `git diff --check`, and `cargo test --workspace --quiet`, and returned PASS with no findings. Phase 13 is ready for handoff. No release publish or tag action was performed.
