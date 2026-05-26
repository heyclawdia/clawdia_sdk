# Strands SDK Python Gap Report Plan

## Goal

Compare this Rust-first Agent SDK against `strands-agents/sdk-python` and produce a critical, product-neutral gap report that identifies missing primitives, toolkit capabilities, code-organization lessons, and areas where this SDK is already stronger.

## Evidence

- Local source-of-truth docs: `README.md`, `docs/start-here.md`, `coding_standards.md`, `docs/workstreams/validation-gates.md`, `docs/reference/sdk-review-checklist.md`, `docs/architecture/primitive-map.md`, and implementation-workstream docs.
- Local crate surfaces: `crates/agent-sdk-core`, `crates/agent-sdk-toolkit`, and the optional `crates/agent-sdk-provider`, with emphasis on run-loop, provider, tool, hook, event, journal, session, agent-pool, and toolkit boundaries.
- External comparison target: `strands-agents/sdk-python` at commit `f6c3b571eda8e5ae2eeb3c997db5d1f7bc2ed986`.

## Scope

- Write documentation only.
- Do not create branches.
- Do not create Rust source files, tests, fixtures, package manifests, or implementation edits.
- Keep the report focused on SDK primitives and optional toolkit layers, not product-specific host adapters.

## Review Checks

- Verify findings against current local code and docs rather than only against roadmap prose.
- Separate gaps that are real missing implementation from gaps that are intentional non-goals or optional-layer candidates.
- Preserve the core rule that ergonomic helpers must lower into canonical contracts and must not bypass policy, events, journals, telemetry, redaction, or replay.
- Run lightweight text checks so the new report does not introduce local absolute paths or obvious public-release hazards.
