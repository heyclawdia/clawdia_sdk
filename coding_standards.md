# Agent SDK Coding Standards

This root file is the quick entry point for future SDK agents.

The authoritative standards live at [docs/architecture/coding-standards.md](docs/architecture/coding-standards.md). If the two ever disagree, update that architecture file first, then adjust this summary.

## Required Posture

- Keep `agent-sdk-core` product-neutral. Host products may validate coverage, but product behavior is not SDK core.
- Keep common APIs simple and thin. Simple helpers must lower into canonical contracts, not create a second behavior path.
- Preserve observability, journal durability, lineage, privacy, policy, and recovery in every implementation slice.
- Prefer explicit typed primitives over text-label archaeology or ambient host state.
- Fail closed when policy, dispatcher, adapter, isolation, or storage requirements are absent.
- Maintain the Rust SDK package with clear architectural ownership. Source and tests should be organized by SDK responsibility (`domain`, `package`, `records`, `ports`, `application`/`runtime`, and `testing`) while preserving a stable, discoverable public facade.
- Keep `mod.rs` and crate facades small and navigable. Real behavior belongs in meaningfully named files or responsibility folders, and future agents should be able to find read/search/edit/write/protocol behavior by filename.
- Follow the mature-SDK layout lesson captured in the architecture standards: stable package facades, separated generated/spec-derived code, explicit ports/adapters, durable records apart from runtime orchestration, and visible reusable fake/test-kit support.
- Run the Rust API Guidelines review gate for public Rust API changes: naming, conversions, common traits, rustdoc examples and failure docs, type-safe parameters, predictable methods, future-proof public types, crate metadata, dependency, and license posture.
- Treat `cargo clippy --workspace --all-targets -- -D warnings` as the API hygiene gate for implementation handoffs. Prefer structural fixes for large public `Result` errors; any intentional lint deviation must be a local `#[expect(..., reason = "...")]`, not a global allow or an undocumented suppression.
- Expose SDK-consumer test helpers through the documented `agent_sdk_core::testing` namespace. New `Fake*`, `Scripted*`, and conformance harnesses belong in `src/testing/` unless explicitly justified as production reference implementations.
- Validate behavior with fake adapters, golden fixtures, property tests, smoke tests, and contract audits before using live providers or concrete host runtimes.
- Treat mockability as a core SDK contract. Every port, adapter boundary, side-effect path, and scenario surface must be testable with deterministic fakes or a public test-support harness that SDK users can reuse for their own implementations.

## Required Reading

1. [README.md](README.md)
2. [docs/start-here.md](docs/start-here.md)
3. [docs/architecture/coding-standards.md](docs/architecture/coding-standards.md)
4. [docs/workstreams/README.md](docs/workstreams/README.md)
5. [docs/implementation-workstreams/README.md](docs/implementation-workstreams/README.md)
6. [docs/workstreams/validation-gates.md](docs/workstreams/validation-gates.md)
7. [docs/reference/sdk-review-checklist.md](docs/reference/sdk-review-checklist.md)
8. [docs/reference/simplicity-audit.md](docs/reference/simplicity-audit.md)
9. [Rust API Guidelines checklist](https://rust-lang.github.io/api-guidelines/checklist.html)

## Completion Rule

An SDK implementation launch target is not complete until its contract tests, golden fixtures, smoke tests, and cross-contract audits named in the launch doc have evidence. Passing docs review alone is not implementation confidence.

The SDK package architecture gate is part of completion: reviewers must reject changes that add new source or integration-test files outside the owning SDK responsibility folder unless the phase exit report explains a deliberate public facade, conventional Cargo layout choice, or Cargo test-target shim.
