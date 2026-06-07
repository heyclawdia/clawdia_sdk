# Rust SDK DX Upgrade Plan

## Objective

Create the first repo-grounded DX upgrade packet and first code slice for the
Rust-first Agent SDK: clean DX direction docs, add the optional `clawdia-sdk`
facade crate, keep the facade behavior-free, and carry forward risks for later
builder, typed-tool, persistence, observability, and example phases.

## Root Cause / Problem Shape

The current SDK has a strong product-neutral kernel, optional provider/toolkit
crates, typed events, journals, policy, runtime packages, deterministic fakes,
and release-readiness evidence. The first-user path still requires users to
assemble split crates and understand several canonical concepts before they see
one import path. The structural fix is not a second runtime or a
dependency-heavy core. The first code slice is a behavior-free facade over real
existing crates; later helpers are only accepted when they prove canonical
lowering into `Agent`, `RunRequest`, `RuntimePackage`, `ProviderAdapter`,
policy, journals, events, telemetry, redaction, and output contracts.

## Relevant Existing Context

- `AGENTS.md`: no branch creation without approval; keep the packet
  product-neutral; implementation work must preserve canonical lowering and
  optional crate ownership.
- `README.md`: the repository already publishes split crates
  `agent-sdk-core`, `agent-sdk-eval`, `agent-sdk-toolkit`, and
  `agent-sdk-provider`; it intentionally does not publish a crate named
  `agent-sdk`.
- `docs/start-here.md`: ergonomic helpers must stay thin and lower into
  canonical contracts rather than creating a second behavior path.
- `coding_standards.md` and `docs/architecture/coding-standards.md`: public
  APIs must preserve package ownership, Rust API quality, mockability,
  deterministic fakes, small facades, and clippy/test validation once code
  changes exist.
- `docs/workstreams/validation-gates.md`: packet goals need
  ownership/boundary/simplicity audit evidence, primitive-lowering review,
  accepted/rejected/deferred proposal blocks, and explicit implementation
  evidence once code exists.
- `docs/reference/sdk-review-checklist.md`: review must check simplicity,
  product-neutrality, canonical lowering, event/journal durability, privacy,
  public facade stability, package topology, and no mini-SDK drift.
- `docs/architecture/primitive-map.md`: simple, builder, and advanced layers
  must all lower into the same kernel primitives.
- `docs/reference/simplicity-audit.md`: common paths should be one-line helpers
  or builders, with the canonical advanced path preserved underneath.
- `docs/reference/persistence-ownership-map.md`: persistence features must stay
  split by journal, checkpoint, content, event archive, agent pool, tool
  execution, and provider argument ownership; avoid a vague global state store.
- `docs/implementation-workstreams/12-scenario-verification/_phase/phase-exit-report.md`:
  `agent_sdk_core::prelude` exists today as a facade-only import surface and
  must not be confused with a convenience crate.
- `docs/implementation-workstreams/13-release-readiness/_phase/feature-flag-matrix.md`:
  current features are intentionally narrow; unsupported optional crates include
  isolation, OTel, extension, workflow, Bedrock/local providers, MCP, browser,
  and web adapters.
- Subagent planning feedback on 2026-06-07: the safe first implementation slice
  is a thin `clawdia-sdk` facade with real feature-gated re-exports only:
  `providers`, `workspace-tools`, `evals`, and `test-support`; defer
  `AgentApp`, macros, stores, empty observability features, approval helpers,
  and runnable examples until they have lowering tests.
- Local source/tests read for grounding:
  `crates/agent-sdk-core/src/lib.rs`,
  `crates/agent-sdk-core/src/application/agent.rs`,
  `crates/agent-sdk-core/src/application/runtime.rs`,
  `crates/agent-sdk-core/tests/domain/public_api.rs`,
  `crates/agent-sdk-toolkit/src/packs/ergonomic.rs`,
  `crates/agent-sdk-provider/README.md`,
  `crates/agent-sdk-toolkit/README.md`, and
  `crates/agent-sdk-eval/README.md`.
- The report should speak in terms of the SDK direction we want, not as a public
  comparison against another SDK.

## Writable Scope

- `docs/reference/dx-gap-report-agents-sdk.md`
- `docs/reference/facade-crate-proposal.md`
- `docs/reference/dx-upgrade-risk-watchpoints.md`
- `Cargo.toml`
- `crates/clawdia-sdk/**`
- `README.md`
- `docs/start-here.md`
- this plan file

No other files are in scope for this first implementation slice.

## Behavior Contract

New behavior:

- Add a concise, current-state DX direction report that explains the local SDK's
  desired first-user experience and missing public API pieces.
- Add a facade crate proposal that answers naming, re-export scope, features,
  optional dependency boundaries, canonical lowering, and migration path.
- Add a risk/watchpoint note for future implementation phases.
- Add navigation from `README.md` and `docs/start-here.md` to the new facade
  and reference docs.
- Add `crates/clawdia-sdk` as an unpublished facade workspace member.
- Add feature-gated facade namespaces for existing crates only:
  `providers`, `workspace-tools`, `evals`, and `test-support`.
- Add facade tests proving no-default imports and feature-gated re-exports
  compile through the intended namespaces.

Preserved behavior:

- The split crates remain the only implemented dependency path.
- `agent_sdk_core::prelude` remains facade-only and does not gain behavior.
- `agent-sdk-core` remains free of provider/toolkit/persistence/telemetry/UI
  dependencies.
- Advanced APIs remain visible and canonical.
- Host-owned surfaces remain outside core.
- The new facade does not own runtime behavior, policy, journals, events, tool
  execution, stores, macros, or UI.

Removed behavior:

- None.

Tests proving behavior:

- Documentation-only validation: `git diff --check`, markdown path/link checks
  for the new local links, targeted public-release audit, and review against the
  SDK checklist.
- Facade validation: `cargo test -p clawdia-sdk --no-default-features`,
  `cargo test -p clawdia-sdk --features providers`,
  `cargo test -p clawdia-sdk --features workspace-tools`,
  `cargo test -p clawdia-sdk --features evals`,
  `cargo test -p clawdia-sdk --features test-support`,
  `cargo test -p clawdia-sdk --all-features`, and `cargo doc -p clawdia-sdk
  --no-deps`.

## Workstreams

1. DX direction report: document missing public API pieces, crate/install
   layout, examples, and migration risks based on the current local source and
   docs.
2. Facade proposal: decide whether to add a convenience facade, propose the
   crate name, identify re-exports and feature groups, and explain canonical
   lowering.
3. Risk note and navigation: capture future implementation gotchas and link the
   packet from the repo's reading path.
4. Facade crate: add a narrow unpublished facade with real re-exports only.
5. Validation: run facade checks, workspace checks, docs checks, and the public
   release audit.

## Risk / Gotcha Carry-Forward

- Do not let a convenience facade become a second runtime, package registry,
  event stream, journal, policy path, tool executor, or telemetry truth store.
- Do not put provider, hosted-store, OTel, MCP, browser, workflow, UI, or host
  adapter dependencies in `agent-sdk-core`.
- Do not publish a crate named `agent-sdk`; the repo already documents that name
  as intentionally unused here. Prefer `clawdia-sdk` only after proposal
  approval.
- Do not write this packet as a comparison report. Keep it focused on the SDK
  direction we want to build.
- Do not claim examples are runnable until they exist as checked-in examples and
  compile in CI with deterministic fakes or gated live-provider paths.
- If a future implementation adds macros, keep them optional and outside core;
  generated schema, executor refs, tool identity/version, and error conversion
  need deterministic tests.
- Do not ship empty facade features as API promises. Facade features must map to
  real dependencies or re-export namespaces.

## Validation Plan

- `git diff --check`
- Local markdown path/link check for links introduced by this pass.
- `scripts/public-release-audit.sh`
- `cargo fmt --check`
- `cargo test -p clawdia-sdk --no-default-features`
- `cargo test -p clawdia-sdk --features providers`
- `cargo test -p clawdia-sdk --features workspace-tools`
- `cargo test -p clawdia-sdk --features evals`
- `cargo test -p clawdia-sdk --features test-support`
- `cargo test -p clawdia-sdk --features all-stable`
- `cargo test -p clawdia-sdk --all-features`
- `cargo doc -p clawdia-sdk --no-deps`
- `cargo test --workspace`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo doc --workspace --all-features --no-deps`
- dependency and source-layout spot audits from `docs/workstreams/validation-gates.md`

## Review Notes

Plan review should verify that this packet:

- was grounded in the repo reading path before edits;
- keeps the facade implementation inside the behavior-free first-slice scope;
- carries forward the primitive-lowering and optional-crate boundaries;
- gives later implementers concrete tests and package boundaries; and
- records risks instead of hiding future constraints in chat context.
