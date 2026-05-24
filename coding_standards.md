# Agent SDK Coding Standards

This root file is the quick entry point for future SDK agents.

The authoritative standards live at [docs/architecture/coding-standards.md](docs/architecture/coding-standards.md). If the two ever disagree, update that architecture file first, then adjust this summary.

## Required Posture

- Keep `agent-sdk-core` product-neutral. Host products may validate coverage, but product behavior is not SDK core.
- Keep common APIs simple and thin. Simple helpers must lower into canonical contracts, not create a second behavior path.
- Preserve observability, journal durability, lineage, privacy, policy, and recovery in every implementation slice.
- Prefer explicit typed primitives over text-label archaeology or ambient host state.
- Fail closed when policy, dispatcher, adapter, isolation, or storage requirements are absent.
- Validate behavior with fake adapters, golden fixtures, property tests, smoke tests, and contract audits before using live providers or concrete host runtimes.

## Required Reading

1. [README.md](README.md)
2. [docs/start-here.md](docs/start-here.md)
3. [docs/architecture/coding-standards.md](docs/architecture/coding-standards.md)
4. [docs/workstreams/README.md](docs/workstreams/README.md)
5. [docs/workstreams/validation-gates.md](docs/workstreams/validation-gates.md)
6. [docs/reference/sdk-review-checklist.md](docs/reference/sdk-review-checklist.md)
7. [docs/reference/simplicity-audit.md](docs/reference/simplicity-audit.md)

## Completion Rule

An SDK phase goal is not complete until its contract tests, golden fixtures, smoke tests, and cross-contract audits named in the goal and owner role docs have evidence. Passing docs review alone is not implementation confidence.
