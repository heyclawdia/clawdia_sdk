# SDK Simplicity And Agent Guidance Audit Plan

## Objective

Audit the current Agent SDK repository after the alpha Rust crates landed, then make narrowly scoped improvements that simplify the SDK for downstream users and make the repository easier for future agents to crawl, verify, and build on without changing runtime behavior.

## Launch Target

Primary launch target: `docs/implementation-workstreams/12-scenario-verification/12b-api-review.md`.

This target owns public API simplicity, crate-level Rustdoc examples, public export tests, and SDK review/simplicity alignment.

Stitching follow-through is explicit and limited to files owned by the integration/stitching role or release-readiness docs. The task is a whole-repo audit, so current-state navigation may be updated only in the files listed below.

## Planned Writable Files

API review-owned:

- `crates/agent-sdk-core/src/lib.rs`
- `crates/agent-sdk-core/tests/domain/public_api.rs`
- `crates/agent-sdk-core/README.md`
- `docs/implementation-workstreams/12-scenario-verification/_phase/phase-exit-report.md`

Integration/stitching-owned current guidance:

- `README.md`
- `AGENTS.md`
- `docs/start-here.md`
- `docs/architecture/coding-standards.md`

Plan/review evidence:

- `docs/plans/2026-05-25-sdk-simplicity-agent-guidance-audit-plan.md`

Explicitly not writable for this task:

- `docs/agent-sdk-toolkit/README.md`
- runtime behavior modules outside `crates/agent-sdk-core/src/lib.rs`
- package manifests, release workflows, tags, commits, or publish scripts

## Relevant Existing Context

- `AGENTS.md`: no branch without explicit approval; preserve product-neutral SDK boundaries; public release audits must pass; implementation files should stay in responsibility folders; toolkit operations need searchable operation names and tests.
- `README.md` and `docs/start-here.md`: current repo has `agent-sdk-core` and `agent-sdk-toolkit` crates plus completed contract and implementation workstream maps.
- `coding_standards.md` and `docs/architecture/coding-standards.md`: common APIs must remain thin lowering layers, public fakes belong under `agent_sdk_core::testing`, source/test layout must stay navigable, and tests are mandatory for implementation confidence.
- `docs/workstreams/validation-gates.md`: implementation goals need tests/fixtures/commands, primitive-lowering evidence, host-owned boundary evidence, and source-layout evidence.
- `docs/reference/sdk-review-checklist.md`: review must cover simplicity, product-neutrality, canonical lowering, event/journal durability, privacy, mockability, package topology, and documentation clarity.
- `docs/reference/simplicity-audit.md`: simplify by defaults, presets, builders, canonical lowering, and a small common path; do not delete essential event/journal/policy/privacy complexity.
- `docs/architecture/primitive-map.md`: simple, builder, and advanced layers all lower into canonical contracts; new concepts must pass the primitive decision ladder.
- `docs/implementation-workstreams/12-scenario-verification/12b-api-review.md`: public exports, Rustdoc examples, and public API tests are the owned API simplicity surface.
- `docs/implementation-workstreams/13-release-readiness/_phase/phase-exit-report.md`: current release evidence says alpha crates exist and full workspace validation previously passed.
- Current audit scan: historical docs under `docs/workstreams/` still contain docs-only/no-code wording from the contract packet. That is valid as historical evidence, but current navigation should make the active crate/build path unmistakable.

## Problem Shape

The SDK already has strong contracts and tests, but a downstream human or agent has to stitch together too much context:

- the common app-building import path is documented through long import lists rather than a small prelude-style path;
- root navigation explains the repo shape but does not provide a crisp "crawl this first, build this next, verify with these commands" protocol;
- historical contract-packet docs can look current unless the reader notices implementation-workstream status;
- the public audit must avoid changing behavior, creating a new runtime path, or broadening unsupported live/provider/product-host claims.

## Behavior Contract

New behavior:

- SDK consumers get a concrete `agent_sdk_core::prelude` module for common core app-building imports. It re-exports stable existing public facade items only; it does not define behavior, wrappers, builders, ports, or alternate execution paths.
- Agents crawling the repo get an explicit current-state protocol: historical contract docs versus active crates, first files to read, what to edit, and which verification commands prove readiness.
- Public docs point at the canonical build/verification evidence without implying unsupported live provider, container, product UI, workflow-engine, or host-adapter support.

Preserved behavior:

- Runtime behavior, event/journal semantics, policy/redaction behavior, package fingerprints, provider/tool/output paths, and crate dependency boundaries do not change.
- Ergonomic helpers remain thin lowering layers over `RunRequest`, `OutputContract`, `RuntimePackage`, events, journals, policy, telemetry, lineage, and redaction.
- `agent-sdk-core` remains independent of `agent-sdk-toolkit`.
- Product-specific host behavior stays out of core and active SDK docs.

Removed behavior:

- None. This is an additive simplification and guidance pass.

Tests proving the contract:

- Public API tests cover the simplified import surface and preserve canonical helper lowering.
- Rustdoc/doc tests compile.
- Full workspace tests and public-release audit pass.
- Source-layout and root-shim audits still show no implementation sprawl.

## Scope

In scope:

- Add a small public import convenience surface if it is purely additive and reuses existing exports.
- Update root/current-state guidance for humans and agents.
- Update crate/user-facing docs to show common path, advanced path, testing path, and unsupported boundaries.
- Add or update tests for any public API documentation/import change.
- Record validation evidence.

Out of scope:

- Changing runtime behavior, records, event names, journal shapes, package fingerprint inputs, side-effect ordering, or policy outcomes.
- Adding live providers, host adapters, concrete runtimes, product-specific examples, workflow engines, or marketplace/install behavior.
- Editing user-existing dirty changes unless required and carefully scoped.
- Creating a branch, commit, tag, release, or publish action.

## Workstreams

1. API simplification: add `agent_sdk_core::prelude` as a facade-only re-export module over existing stable public items; update public API tests and Rustdoc.
2. Agent crawl guidance: update root/current-state docs so future agents know the active crate path, historical docs boundary, edit ownership, and verification gates.
3. Validation evidence: run formatting, tests, source-layout audits, public release audit, and diff hygiene.
4. Review loop: plan review before edits and implementation review after edits; resolve blocking findings.

## Public Import Surface

`agent_sdk_core::prelude` will re-export common app-building items already exported from the crate root:

- run and runtime: `Agent`, `AgentBuilder`, `AgentRuntime`, `RunHandle`, `RunRequest`, `RunResult`, `RunStatus`;
- package: `RuntimePackage`, `RuntimePackageBuilder`, `CapabilitySpec`;
- content/context/output: `AgentMessage`, `ContextContribution`, `ContextItem`, `ContextProjection`, `OutputContract`, `OutputSchemaId`, `OutputSchemaRef`, `SchemaVersion`, `TypedOutputModel`, `ValidatedOutput`;
- observability and durability: `AgentEvent`, `EventFilter`, `EventFrame`, `EventCursor`, `JournalRecord`, `RunJournal`;
- policy and lineage: `AgentId`, `RunId`, `SourceKind`, `SourceRef`, `DestinationKind`, `DestinationRef`, `EntityRef`, `PolicyDecision`, `PolicyKind`, `PolicyOutcome`, `PolicyRef`, `PolicyStage`, `PrivacyClass`, `RetentionClass`, `TrustClass`;
- host ports and errors commonly needed by applications: `AgentError`, `AgentEventBus`, `ContentResolver`, `ProviderAdapter`, `RuntimePolicyPort`.

It intentionally will not re-export every feature-layer type, testing fake, or toolkit helper. Advanced users should still import from the crate root, `agent_sdk_core::ports`, `agent_sdk_core::testing`, or documented feature modules.

## Validation Plan

- `cargo fmt --check`
- `cargo test --workspace`
- `cargo test -p agent-sdk-core --test public_api`
- `cargo test -p agent-sdk-core --doc`
- `git diff --check`
- `scripts/public-release-audit.sh`
- Source-layout audits from `docs/workstreams/validation-gates.md`
- Targeted product-neutrality audit for touched source/docs
- Targeted search for stale current-state wording in root/current navigation docs

## Risk/Gotcha Carry-Forward

- The `prelude` module must only re-export stable existing public facade items. It must not create a new behavior path, hide policy/runtime requirements, or imply deep implementation modules are stable.
- Historical phase reports should not be rewritten as if their original evidence happened after code existed. Current navigation should clarify that they are historical contract evidence.
- Avoid touching the existing dirty toolkit README/deleted TUI plan unless the audit proves it is necessary; those changes predate this task.
- Any public API addition needs SemVer/posture documentation and a public API test.
- Guidance for agents should prefer exact repo paths and commands, but avoid local absolute paths and product-specific host assumptions.
