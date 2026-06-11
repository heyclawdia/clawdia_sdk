# Phase 13 Exit Report: Release Readiness

## Status

PASS.

Post-handoff update: the first explicit publish request moved the package metadata from handoff-only to `0.1.0-alpha.1` crates.io release readiness. A later API-hygiene release updated the local crate manifests and changelog to `0.1.0-alpha.2`. The Strands gap follow-up added an optional `agent-sdk-provider` crate, and the live-provider onboarding follow-up added OpenAI, Anthropic, and Gemini adapters over `ProviderAdapter`. The current checkout is now a `0.1.0-alpha.4` release candidate for the DX quickstart, builder-first typed-tool, and durable store adapter work. The release execution uses `.github/workflows/publish-crates.yml` plus `scripts/public-release-audit.sh` so GitHub releases validate formatting, tests, public-repo sensitive-content criteria, and package metadata before publishing.

## Scope Completed

Changed release-readiness surfaces:

- `Cargo.toml`
- `crates/agent-sdk-core/Cargo.toml`
- `crates/agent-sdk-provider/Cargo.toml`
- `crates/agent-sdk-toolkit/Cargo.toml`
- `crates/agent-sdk-core/README.md`
- `crates/agent-sdk-provider/README.md`
- `crates/agent-sdk-toolkit/README.md`
- `CHANGELOG.md`
- `crates/agent-sdk-core/tests/policy_matrix.rs`
- `crates/agent-sdk-core/tests/domain/policy_matrix.rs`
- `docs/implementation-workstreams/13-release-readiness/_phase/feature-flag-matrix.md`
- `docs/implementation-workstreams/13-release-readiness/_phase/contract-to-code-traceability.md`
- `docs/implementation-workstreams/13-release-readiness/_phase/phase-exit-report.md`

## Package Metadata

- Current crates now have crate-level READMEs and descriptions.
- The original core/toolkit crates used `publish = false` for the original handoff because no publish/tag release had been requested and no live/provider/container/product-host support was included. The explicit alpha publish request removes that block while preserving the unsupported-path release notes.
- `agent-sdk-provider` is an optional aggregate adapter crate. Its current surface includes live OpenAI Responses, Anthropic Messages, and Gemini generateContent adapters, plus deterministic transport-injected harnesses. It does not claim Bedrock, local model, MCP, browser, web, journal, event, approval, or tool-executor ownership.
- `agent-sdk-store-file`, `agent-sdk-store-sqlite`, `agent-sdk-store-postgres`,
  and `agent-sdk-store-supabase` are optional durable store adapter crates.
  They map explicit SDK store ports and keep retention, backup, hosted
  provisioning, and product session state host-owned.
- `agent-sdk-macros` remains checkout-only in this repository because the
  crates.io package name is already occupied by an unrelated project.
- The invalid placeholder repository metadata was removed from the workspace package metadata.

## Feature Flag Matrix

See [feature-flag-matrix.md](feature-flag-matrix.md).

Current posture:

- `agent-sdk-core` default features are empty.
- `agent-sdk-core --no-default-features` must build and test without optional crates.
- `agent-sdk-core --all-features` currently enables only the reserved `test-support` feature and must not add live providers or host infrastructure.
- `agent-sdk-toolkit` is an optional separate crate that depends on core; core has no reverse dependency.
- `agent-sdk-provider` is an optional separate crate that depends on core and contains provider DTO mapping, live HTTP request mapping, redacted API-key wrappers, and transport-injected tests. It does not add providers, credentials, journals, events, approval, or tool execution to core.
- `agent-sdk-isolation`, `agent-sdk-otel`, `agent-sdk-extension`, and `agent-sdk-workflow` are not published or implemented as concrete optional crates in this handoff.

## Contract To Code Traceability

See [contract-to-code-traceability.md](contract-to-code-traceability.md).

Every normative contract in `docs/contracts/README.md` maps to an owning source responsibility folder plus at least one test or fixture family. Scenario references map to fake-only scenario matrix tests and the public API review suite.

## Release Notes

See [../../../../CHANGELOG.md](../../../../CHANGELOG.md).

The handoff notes explicitly state that concrete container runtimes, product UI/host adapters, network telemetry exporters, marketplace runtimes, workflow engines, and product-owned memory backends are unsupported.

## DDD, Mockability, And Package Architecture Evidence

- Source layout keeps implementation under `domain`, `package`, `records`, `ports`, `application`, and `testing`.
- `src/lib.rs` remains a public facade and rustdoc surface.
- Root integration tests remain stable Cargo target shims; the new `policy_matrix` target delegates into `tests/domain/policy_matrix.rs`.
- The policy matrix test is table-driven, deterministic, serializes/deserializes decisions, and covers missing dependencies and content-capture gates as SDK-consumer conformance surfaces.
- Public fakes and scripted adapters remain under `agent_sdk_core::testing`.
- Release notes and crate READMEs document that provider adapters are optional and concrete runtimes/product adapters remain unsupported. The provider crate README also documents that credentials remain host-resolved and do not enter runtime packages, journals, events, or content refs.

## Validation Evidence

Current `0.1.0-alpha.4` release-candidate validation on 2026-06-11:

- `cargo fmt --check` PASS.
- `git diff --check` PASS.
- `scripts/public-release-audit.sh` PASS.
- `cargo test --workspace --all-features` PASS, including core, provider,
  toolkit, eval, macros, file/SQLite/Postgres/Supabase store crates, facade,
  checkout examples, and doctests.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` PASS.
- `cargo doc --workspace --all-features --no-deps` PASS.
- `cargo publish -p agent-sdk-core --dry-run --allow-dirty` PASS through
  package verification and dry-run upload abort.

Historical release-readiness validation from the first published crate family:

- `cargo fmt --check` PASS.
- `cargo test --workspace` PASS, including `agent-sdk-core`, `agent-sdk-provider`, `agent-sdk-toolkit`, and doc tests.
- `cargo test -p agent-sdk-core --doc` PASS, 2 doc tests.
- `cargo test -p agent-sdk-core --no-default-features` PASS.
- `cargo test -p agent-sdk-core --all-features` PASS; only the reserved `test-support` feature is added.
- `cargo test -p agent-sdk-core --test contract_golden` PASS, 7 tests.
- `cargo test -p agent-sdk-core --test replay_recovery` PASS, 7 tests.
- `cargo test -p agent-sdk-core --test policy_matrix` PASS, 2 tests.
- `cargo test -p agent-sdk-core --test scenario_matrix` PASS, 7 tests.
- `cargo test -p agent-sdk-core --test public_api` PASS, 4 tests.
- `cargo test -p agent-sdk-toolkit` PASS for toolkit integration tests and doc tests.
- `cargo test -p agent-sdk-provider --quiet` PASS, including OpenAI-compatible request projection, text response, function-call tool-use, malformed response, stream terminal delta tests, and live OpenAI/Anthropic/Gemini adapter request/response mapping tests.
- `cargo clippy --workspace --all-targets -- -D warnings` PASS.
- `cargo tree -p agent-sdk-core --no-default-features` PASS; core contains only `serde`, `serde_json`, `sha2`, `thiserror`, and transitive support crates.
- `cargo tree -p agent-sdk-toolkit` PASS; toolkit depends on core plus helper dependencies such as `regex`, while core has no reverse toolkit dependency.
- `cargo tree -p agent-sdk-provider` PASS; provider's normal dependencies are core plus `serde` and `serde_json`. The default live transport shells out to system `curl` instead of adding an async runtime.
- `cargo package -p agent-sdk-core --allow-dirty --list` PASS; package includes crate README, source responsibility folders, integration-test shims/bodies, and fixtures.
- `cargo package -p agent-sdk-toolkit --allow-dirty --list` PASS; package includes crate README, toolkit modules, fixtures, and tests.
- `cargo package -p agent-sdk-provider --allow-dirty --list` PASS; package includes crate README, facade, live OpenAI/Anthropic/Gemini modules, OpenAI-compatible module, and conformance tests.
- `git diff --check` PASS.
- `scripts/public-release-audit.sh` PASS.
- Product-neutrality audit PASS: `if rg -n '(Clawdia|clawdia|Pawtrace|pawtrace|iMessage|ACP)' crates/agent-sdk-core/src crates/agent-sdk-toolkit/src crates/agent-sdk-core/tests/fixtures crates/agent-sdk-core/README.md crates/agent-sdk-toolkit/README.md CHANGELOG.md docs/implementation-workstreams/13-release-readiness/README.md docs/implementation-workstreams/13-release-readiness/13a-release-readiness.md docs/implementation-workstreams/13-release-readiness/_phase/feature-flag-matrix.md docs/implementation-workstreams/13-release-readiness/_phase/contract-to-code-traceability.md; then exit 1; else rg_status=$?; if [ "$rg_status" -eq 1 ]; then echo 'PASS: no product-specific matches in audited release surfaces'; else exit "$rg_status"; fi; fi` printed `PASS: no product-specific matches in audited release surfaces`.
- Docs link/path audit PASS: markdown links in the release-readiness surfaces were enumerated with `rg -n '\[[^]]+\]\([^)]+\)' ...`, and `test -f` passed for the Phase 13 feature matrix, traceability matrix, changelog, and crate READMEs.

## Source Layout Audit

- `find crates/agent-sdk-core/src -maxdepth 1 -type f -not -name lib.rs -not -name README.md` returned no files.
- `find crates/agent-sdk-core/tests -maxdepth 1 -type f -name '*.rs' -print -exec sh -c 'wc -l "$1"' sh {} \;` showed every root integration test target is a two-line shim, including the new `policy_matrix.rs`.
- `find crates -path '*/src/*.rs' -maxdepth 3 -type f` returned direct crate-root source files plus provider modules for auth, HTTP, live providers, OpenAI-compatible mapping, and argument sinks. `agent-sdk-provider/src/lib.rs` remains a narrow facade.
- `rg -n '#\[path = .*\]\s*pub mod|pub mod [a-zA-Z0-9_]+;' crates/agent-sdk-core/src/lib.rs` is non-empty by design because `lib.rs` is the documented public facade. This follow-up added no new public deep module alias in core.
- `rg -n '\b(Fake|Scripted)[A-Za-z0-9_]+|ConformanceHarness' crates/agent-sdk-core/src --glob '*.rs'` found fake/scripted/conformance helpers only in `src/testing` plus their documented `agent_sdk_core::testing` re-exports.
- `rg -n '\btrait\b|\bAdapter\b|\bResolver\b|\bFake\b|\bScripted\b|ConformanceHarness' crates/agent-sdk-core/src/records --glob '*.rs'` returned no matches, so durable records did not absorb port or fake behavior.
- `wc -l crates/agent-sdk-*/src/lib.rs` showed `agent-sdk-core/src/lib.rs` at 925 lines as the crate-level facade/rustdoc surface, `agent-sdk-provider/src/lib.rs` at 13 lines as a narrow optional-crate facade, and `agent-sdk-toolkit/src/lib.rs` at 66 lines as a narrow optional-crate facade.

## Independent Review

Independent reviewer Pauli initially returned BLOCKED because the product-neutrality audit evidence included the phase exit report itself, causing the quoted denylist command to self-match. The audit scope was corrected to target release surfaces and exclude the evidence report itself. The corrected command printed `PASS: no product-specific matches in audited release surfaces`.

Pauli re-reviewed the fix, reran `cargo fmt --check`, `git diff --check`, and `cargo test --workspace --quiet`, and returned PASS with no findings. Phase 13 is ready for handoff. No release publish or tag action was performed.
