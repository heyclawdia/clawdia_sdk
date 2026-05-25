# Rustdoc Coverage And SDK Usage Documentation Plan

## Objective

Add proper public documentation across `agent-sdk-core` and `agent-sdk-toolkit`
so SDK users can understand each module's role, when to use public functions
and types, what side effects are possible, and which boundaries stay host-owned.
This is a documentation-only task: no runtime behavior, package manifests,
executable tests, fixtures, or new Rust source files will be created.

## Launch Target

Primary launch target: `docs/implementation-workstreams/12-scenario-verification/12b-api-review.md`.

This is the launch target that explicitly owns public API review and Rustdoc
examples across crate modules. The user request intentionally extends that
Rustdoc/API clarity pass to both published crates, so this plan treats
source-local doc comments as the public API documentation surface while keeping
all edits documentation-only. Phase 13 release-readiness docs remain read-only
validation context unless release-handoff evidence needs a later separate task.

## Planned Writable Files

Rustdoc-only source comments and module docs covered by the API-review Rustdoc
surface:

- `crates/agent-sdk-core/src/**/*.rs`
- `crates/agent-sdk-toolkit/src/**/*.rs`

Documentation/readiness evidence:

- `docs/plans/2026-05-25-rustdoc-coverage-plan.md`

Local agent notes and ignore rule:

- `.gitignore`
- `scratchpad/` local notes, ignored by git

No new Rust source files, executable tests, package manifests, or fixtures are
in scope. Existing Rust files may receive doc comments only.

## Relevant Existing Context

- `AGENTS.md`: no branch without explicit approval; preserve product-neutral SDK
  boundaries; use implementation workstreams for active Rust work; documentation-
  only tasks must not create Rust source files, executable tests, manifests, or
  fixtures; `scratchpad/` should be agent notes only and ignored.
- `README.md` and `docs/start-here.md`: the current checkout includes two Rust
  crates, `agent-sdk-core` and optional `agent-sdk-toolkit`; historical
  `docs/workstreams/` files are contract-packet evidence, while
  `docs/implementation-workstreams/` is the active Rust launch map.
- `coding_standards.md` and `docs/architecture/coding-standards.md`: public APIs
  need clear ownership, invariants, and "must not own" boundaries; common APIs
  must stay thin lowering layers; source layout must remain navigable.
- `docs/workstreams/validation-gates.md`: documentation-only goals need docs
  audit evidence, primitive-lowering review, named skipped tests, and a statement
  that no source files/tests/manifests/fixtures were created.
- `docs/reference/sdk-review-checklist.md`: review must cover simplicity,
  product-neutrality, canonical lowering, event/journal durability, privacy,
  mockability, package topology, public facade, and documentation clarity.
- `docs/architecture/primitive-map.md`: simple helpers, builders, and advanced
  DTOs must lower into canonical contracts instead of creating parallel paths.
- `docs/reference/simplicity-audit.md`: document the small common path while
  preserving event, journal, policy, privacy, lineage, and package complexity
  underneath.
- `docs/implementation-workstreams/12-scenario-verification/12b-api-review.md`:
  owns public API review, Rustdoc examples across crate modules, doctest/rustdoc
  compile checks, and canonical helper-lowering review.
- Current lint evidence: `RUSTDOCFLAGS='-D missing-docs' cargo doc --workspace
  --no-deps` fails across both crates, with `agent-sdk-core` emitting thousands
  of missing-doc warnings and `agent-sdk-toolkit` missing docs on public modules,
  request/output types, protocol frames, and side-effect helpers.
- Current git state: re-check before implementation starts and before final
  review. At plan time, only pre-existing `docs/architecture/architecture-proposal.md`
  whitespace and this plan are dirty; any future change to that baseline must be
  treated as user/other-agent work and preserved.

## Problem Shape

The crates now expose enough public API that downstream users and future agents
need source-local Rustdoc, not only high-level README guidance. Some modules
have good crate or namespace docs, but many public structs, enums, fields,
traits, constructors, helpers, and module facades lack enough explanation for:

- what the item owns;
- when a host or optional crate should use it;
- whether it mutates state, launches work, touches I/O, dispatches approvals,
  emits events, appends journals, or is pure data/validation only;
- what it must not own, especially product UI, provider credentials, host
  adapters, workflow engines, or hidden side-effect paths.

## Behavior Contract

New behavior:

- Public Rustdoc explains each exposed module/type/function at the level needed
  by SDK consumers: purpose, usage timing, side-effect expectations, and
  SDK-owned versus host-owned boundaries.
- Toolkit docs make concrete side effects explicit: workspace reads/searches,
  edit/write planning, shell execution, resource reads, discovery activation
  deltas, JSON-RPC line transports, and scripted protocol harnesses.
- `.gitignore` ignores `scratchpad/`, and any code-improvement observations found
  during documentation are recorded as local scratchpad notes instead of source
  behavior changes.
- `RUSTDOCFLAGS='-D missing-docs' cargo doc --workspace --no-deps` becomes the
  primary docs-coverage gate.

Preserved behavior:

- Runtime behavior, public types, method signatures, serde shapes, events,
  journals, package fingerprints, policy behavior, redaction behavior, tests,
  Cargo manifests, and examples remain behaviorally unchanged.
- `agent-sdk-core` remains product-neutral and independent of optional concrete
  toolkit behavior.
- `agent-sdk-toolkit` remains an optional helper crate layered over core
  packages, policy refs, content refs, ports, and effect lineage.
- Existing dirty toolkit module-layout edits are preserved.

Removed behavior:

- None.

Tests and audits proving this behavior:

- `RUSTDOCFLAGS='-D missing-docs' cargo doc --workspace --no-deps`
- `cargo test --doc --workspace`
- `cargo fmt --check`
- `cargo test --workspace`
- `git diff --check`
- `scripts/public-release-audit.sh` or documented equivalent if the script is
  unavailable
- Source-layout audit commands from `docs/workstreams/validation-gates.md`
- Baseline/final dirty-tree review proving touched dirty files only received
  documentation-only changes and unrelated pre-existing changes were not staged
  or reverted.
- Targeted product-neutrality/search audit for local absolute paths and
  unsupported feature claims in touched tracked files

## Scope

In scope:

- Add or improve `//!` module docs and `///` public item docs in existing Rust
  source files across both crates.
- Document public fields and enum variants where they are part of the consumer
  contract.
- Add concise `# Side Effects`, `# When To Use`, or boundary notes where they
  materially help users avoid misuse.
- Add `.gitignore` coverage for `scratchpad/`.
- Create local ignored scratchpad note files for code improvement observations.

Out of scope:

- New runtime behavior, refactors, source-module reshaping, package manifests,
  tests, fixtures, examples, feature flags, release execution, tags, publishing,
  or branch creation.
- Changing pre-existing dirty code beyond doc comments needed for this task.
- Product-specific host adapter examples or claims of live provider/container/UI
  support.

## Workstreams

1. Core rustdoc pass: document `domain`, `package`, `records`, `ports`,
   `application`, and `testing` public surfaces with ownership, usage, and side
   effect boundaries.
2. Toolkit rustdoc pass: document workspace, shell, discovery, resources,
   protocol, packs, and testing public surfaces with concrete I/O/activation
   side-effect notes.
3. Scratchpad and hygiene: add `scratchpad/` to `.gitignore` and record code
   improvement observations locally without tracking them.
4. Validation and review: use missing-docs rustdoc, formatting, workspace tests,
   public audit, source-layout checks, and independent implementation review.

## Risk/Gotcha Carry-Forward

- Rustdoc must not imply support that the crates do not ship, especially live
  providers, host UI, concrete containers, product workspaces, remote channels,
  workflow engines, or network telemetry exporters.
- Side-effect docs should distinguish "builds a record/delta/request" from
  "executes an external action"; helpers that only lower into canonical DTOs must
  be documented as pure or side-effect-free where applicable.
- Toolkit shell/workspace/resource docs must avoid suggesting ambient host access.
  Hosts still own sandbox, approval, root selection, credentials, and policy.
- Do not convert documentation gaps into behavior changes. If code looks like it
  wants improvement, write a scratchpad note instead.
- Preserve existing user/agent edits in dirty files. Touch them only with
  additive doc comments needed for this task.
- Record a `git status --short --untracked-files=all` and targeted diff baseline
  immediately before implementation, then repeat before final review so the
  implementation review can separate this documentation pass from unrelated
  workspace churn.
- If strict missing-docs uncovers generated-size scale, prefer concise accurate
  docs over placeholder filler, and keep validation failures visible until fixed.
