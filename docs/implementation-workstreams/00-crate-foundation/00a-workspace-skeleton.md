# Workspace Skeleton

## Phase

[Phase 00: Crate Foundation](README.md)

## Parallelism

Only launch target in this phase. Run before every other implementation phase.

## Contract Inputs

- [api-contracts.md](../../contracts/api-contracts.md)
- [runtime-package-schema.md](../../contracts/runtime-package-schema.md)
- [review-matrix.md](../../contracts/review-matrix.md)
- [open-questions-and-ambiguities.md](../../reference/open-questions-and-ambiguities.md)

## Implementation Objective

Create the minimal Rust workspace that later phases can fill in without changing crate boundaries or test layout.

## Owned Implementation Surface

- `Cargo.toml`
- `crates/agent-sdk-core/Cargo.toml`
- `crates/agent-sdk-core/src/lib.rs`
- `crates/agent-sdk-core/tests/`
- `crates/agent-sdk-core/tests/fixtures/`
- optional crate stubs only when needed for feature flags: `crates/agent-sdk-toolkit/`, `crates/agent-sdk-isolation/`, `crates/agent-sdk-extension/`, `crates/agent-sdk-otel/`, `crates/agent-sdk-workflow/`
- `.github/workflows/` or local verification scripts only if the repo already uses that surface or the user approves it

## Must Deliver

- Cargo workspace with `agent-sdk-core` compiling alone.
- Public module placeholders for kernel, ports, events, journal, runtime package, context, output, policy, and fakes.
- Feature flags that keep optional crates out of the core default build.
- Test fixture directories and naming conventions for later golden fixtures.
- A local verification command documented in the phase exit report.

## Validation

- `cargo fmt --check`
- `cargo test -p agent-sdk-core`
- package/import smoke that proves `agent-sdk-core` builds without optional crates
- docs audit confirming no product-specific host adapter entered core

## Must Not

- Add live providers, real container runtimes, product UI adapters, marketplace behavior, or trace-store implementations.
- Implement domain behavior beyond compileable empty surfaces and test harness scaffolding.

## Handoff Evidence

End with changed files, commands run, skipped tests, crate boundary notes, optional-feature notes, and the first phase exit report path.
