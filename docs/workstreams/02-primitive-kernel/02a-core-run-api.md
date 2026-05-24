# Goal 02a: Core Run API

## Phase

[Phase 02: Primitive Kernel](README.md)

## Owner Role

[Core Api Runtime](../_roles/01-core-api-runtime.md)

## Parallelism

Parallel-safe with every other goal in Phase 02 after Phase 01 exits. Do not start Phase 03 until all Phase 02 goals finish.

## Required Reading

- `README.md`
- `docs/start-here.md`
- `coding_standards.md`
- `docs/workstreams/validation-gates.md`
- `docs/reference/sdk-review-checklist.md`
- `docs/architecture/primitive-map.md`
- phase README
- owner role doc
- read-only inputs below

## Writable Files

- `docs/contracts/api-contracts.md`
- `docs/contracts/run-handle-reconnect-contract.md`
- `docs/contracts/loop-state-machine.md`
- `docs/contracts/hook-lifecycle-contract.md` only where the core hook contract is explicitly in scope

## Read-Only Inputs

- `docs/contracts/runtime-package-schema.md`
- `docs/contracts/event-schema.md`
- `docs/contracts/journal-replay-schema.md`
- `docs/architecture/primitive-map.md`

## Primitive Focus

- Freeze the MVP public API as run control, package refs, context/output projection, events, journals, ports, and lineage.
- Split public API docs into MVP public surface, reserved public contracts, and optional crate APIs.
- Treat hooks as package-resolved lifecycle capabilities; do not let the first run API require every hook, stream, telemetry, isolation, subagent, channel, or extension surface.

## Must Not Own

Runtime-package fingerprint rules, event/journal taxonomy, tool packs, concrete telemetry exporters, product host adapters, or UI routing.

## Validation And Review

- Future tests: compile/API lowering tests for `run_text`, `run_typed`, `AgentRuntime::start_run`, and `RunHandle` terminal completion.
- Docs audit: simple helpers must lower into the same package, policy, journal, event, redaction, and lineage path as advanced `RunRequest`.
- `run_text` and `run_typed` lower into `RunRequest`.
- `RunHandle` completion waits for terminal run state, not only final visible text.
- Public support types required by signatures are named.
- No simple helper bypasses package, policy, journal, event, redaction, or lineage requirements.

## Validation Evidence

Changed files:

- `docs/contracts/api-contracts.md`

Tests/fixtures:

- No Rust source, package manifests, executable tests, or fixtures were created; this was a documentation-only contract pass.
- Future tests remain named in this goal and [01 Core Api Runtime](../_roles/01-core-api-runtime.md).

Commands run:

- `git diff --check`
- local Markdown link audit over all `.md` files
- no-code audit for `.rs`, `Cargo.toml`, executable tests, and fixtures
- Phase 02 writable-scope audit
- product-neutrality keyword audit over added lines
- primitive/no-mini-SDK audit over Phase 02 contracts

Skipped tests and why:

- Compile and runtime tests are skipped because the Rust crate does not exist yet.

Events/journal/telemetry touched:

- No event or journal schema changes were needed. The API review confirmed helpers lower into the existing `AgentEvent`, `RunJournal`, `EffectIntent`, policy, redaction, and typed-ID paths.

SDK-owned boundaries preserved:

- `Agent`, `AgentRuntime`, `RunRequest`, `RunHandle`, `RunResult`, content resolver support types, and structured-output support types remain SDK-owned kernel surfaces.

Host-owned boundaries preserved:

- Product routing, UI rendering, provider credentials, concrete stores, approval transport, concrete process/isolation adapters, and host-specific orchestration remain outside core.

Primitive-lowering evidence:

- Simple and builder APIs still enter `RunRequest` and `AgentRuntime::start_run`; `run_typed` still lowers through `OutputContract` and local validation.

Simplicity notes:

- No new run helper, runtime registry, event stream, journal, policy path, or side-effect path was added.

Cross-cutting proposal blocks:

- Accepted: add structured-output and content-resolution support types to the public-signature support list so Phase 03 stitching can check exported names consistently.
- Rejected: none.
- Deferred: exact Rust module layout and compile tests remain for implementation.

## Review Packet

Primitive decision:

- Reused kernel primitives: `Agent`, `AgentRuntime`, `RunRequest`, `RuntimePackage`, `AgentEvent`, `RunJournal`, `ContentRef`, `OutputContract`, `ValidatedOutput`, `EffectIntent`, and typed IDs.
- New feature-layer primitives: none.
- New capability variants: none.
- Host-owned behavior kept out: UI routing, concrete stores, provider credentials, approval transport, workflow orchestration, and product adapters.

Validation evidence:

- Contract/unit tests: future tests named; no code exists yet.
- Golden fixtures: not created in this documentation-only pass.
- Smoke/scenario tests: not applicable until crates exist.
- Docs audits: link, writable-scope, no-code, product-neutrality, and primitive/no-mini-SDK audits.

Reviewer checklist:

- Simplicity: PASS, common API remains one-line helpers over canonical requests.
- Product-neutrality: PASS, no product-specific references added.
- Event/journal durability: PASS, helpers keep the existing event/journal path.
- Privacy/redaction: PASS, content refs and redacted summaries remain defaults.
- Replay/idempotency: PASS, `RunHandle` idempotency and cursor semantics remain in the reconnect contract.
- Capability fingerprint impact: PASS, no new capability variants; support type naming only.
