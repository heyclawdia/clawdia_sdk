# Agent SDK Implementation Workstreams Plan

Date: 2026-05-24

## Objective

Finish every implementation phase in `docs/implementation-workstreams` in numeric order, using the SDK packet as the source of truth. Run sibling launch targets in parallel only after the previous phase exit gate is reviewed and passed.

This plan keeps the SDK Rust-first, product-neutral, and profile-gated:

- P0 proves one fake-provider text run.
- P1 adds typed output over the same loop.
- P2 adds tool, approval, output, hook, and telemetry side effects over the same primitive kernel.
- Reserved feature ports layer on top after P2 without becoming P0 or P1 requirements.

## Relevant Existing Context

- `AGENTS.md`: do not create branches; start from `README.md` and `docs/start-here.md`; read `coding_standards.md` and `docs/workstreams/validation-gates.md`; implementation workstreams are phase-gated; keep the packet product-neutral.
- `README.md`: `<repo-root>` is authoritative; future Rust implementation starts from `docs/implementation-workstreams`.
- `docs/start-here.md`: MVP readiness profiles are P0 text run, P1 typed output, and P2 side effects; host scenarios are coverage constraints, not core architecture.
- `coding_standards.md` and `docs/architecture/coding-standards.md`: TDD, SDK package architecture, and domain modeling are required. Implementation must start with tests, use typed domain primitives, preserve a stable public facade, keep source/tests grouped by SDK responsibility, and preserve observability, journal durability, lineage, privacy, policy, and recovery.
- SDK layout research checked on 2026-05-24: Cargo package/test layout, Rust API Guidelines, AWS SDK Rust/JavaScript modular package structure, Stripe Go backend injection/mocking, Kubernetes client-go testing package, and Twilio Go generated-client separation all support the same rule: stable public facades, separated generated/spec code, explicit ports/backends, and visible test/fake support.
- `docs/workstreams/validation-gates.md`: every implementation goal must provide tests/fixtures, exact commands, primitive-lowering evidence, boundary evidence, and a review packet.
- `docs/reference/sdk-review-checklist.md`: review must check simplicity, product-neutrality, canonical lowering, event/journal durability, privacy, replay, isolation, optional-layer boundaries, and public API stability.
- `docs/architecture/primitive-map.md`: all features must reuse `Agent`, `RunRequest`, `RuntimePackage`, `AgentEvent`, `RunJournal`, refs, policy, content refs, effect intent/result, and typed ports before adding new primitives.
- `docs/reference/open-questions-and-ambiguities.md`: resolved decisions freeze crate boundaries, phase delivery/reviewer gates, event hot path, cursor distinctions, journal atomicity, runtime package fingerprint ownership, output delivery, telemetry projection ownership, and feature-layer deferrals.
- `docs/reference/cross-cutting-proposals.md`: accepted stitching decisions prohibit second run loops, second event streams, second journals, second package registries, feature-specific policy paths, or host/product leakage.

## Behavior Contract

New behavior:

- A compileable Cargo workspace exists with `agent-sdk-core` as the default core crate and optional crate boundaries kept separate.
- Each implementation-workstream launch target is represented by Rust modules, tests, fixtures, or release/audit evidence matching its launch doc.
- Phase exit reports exist under `docs/implementation-workstreams/<NN-phase>/_phase/phase-exit-report.md`.
- Public helpers such as text and typed runs lower into canonical `RunRequest`, `RuntimePackage`, event, journal, policy, lineage, and validation paths.

Preserved behavior:

- Documentation contracts remain authoritative unless code exposes a concrete mismatch that is reconciled through the stitching process.
- Core remains product-neutral and free of host UI, live provider, concrete sandbox, marketplace, trace-store, or product workflow assumptions.
- Optional feature ports stay reserved or optional until their phase activates them.

Removed behavior:

- None planned. This is an implementation build-out from a docs-only packet.

Tests proving behavior:

- `cargo fmt --check`
- `cargo test -p agent-sdk-core`
- Targeted contract tests named by each launch doc, including package, events, journal, context, provider projection, runtime, loop state, P0 text run, output contract, validation repair, typed output, side effects, feature ports, replay, privacy/performance, scenarios, and release-readiness audits.
- Golden fixture tests for emitted events, journal records, package fingerprints, validation output, feature protocols, and release matrices where required by the launch docs.

## Scope

In scope:

- Create and fill `Cargo.toml`, `crates/agent-sdk-core/**`, optional crate stubs when required by feature flags, test fixtures, and phase exit reports.
- Add narrow documentation updates needed to reconcile code evidence, phase reports, or cross-cutting implementation decisions.
- Use fake deterministic adapters and in-memory stores as first proof.
- Treat mockability and SDK-consumer conformance helpers as required deliverables for every public port, adapter boundary, side-effect path, and scenario surface.
- Run independent plan and implementation reviews.

Out of scope:

- Product-specific host adapters or examples.
- Live providers, real container runtimes, marketplace/install flows, product UI, external trace stores, or concrete host workflows.
- Git branch creation.
- Pushing without explicit approval.

## Workstreams

1. Phase 00, Crate Foundation: create workspace, core crate, optional crate boundary strategy, test fixture layout, package/import smoke, and phase report.
2. Phase 01, Shared Kernel: parallel typed IDs/refs, errors/policy, deterministic fakes/fixtures; then phase-level audit and review.
3. Phase 02, Core Records: parallel runtime package, events/event bus, journal/effects, content/context, provider/projection; then phase-level fixtures and review.
4. Phase 03, Run Control: parallel runtime ownership, loop state, run handle/reconnect; then phase-level no-duplicate-path audit and review.
5. Phase 04, P0 Text Run: integrate one fake-provider text run through package, context, provider, events, journal, and result.
6. Phase 05, Agent Pool Coordination: add generic agent-run coordination, run messages, delivery receipts, and wake conditions without workflow-engine behavior.
7. Phase 06, Output Contract: implement schema refs, output contracts, helper lowering, and package fingerprint normalization.
8. Phase 07, P1 Validation Result: parallel validation/repair and typed-result records over Phase 06.
9. Phase 08, P1 Typed Run: integrate typed output over the same P0 loop, including invalid output, repair success, repair exhaustion, and result extraction.
10. Phase 09, P2 Side Effects: parallel approval broker, tool execution, output delivery, hook lifecycle, telemetry core; all must journal intent before external action and result after action.
11. Phase 10, Feature Ports: parallel stream/realtime, isolation, subagents, extension SDK, tool packs; keep concrete adapters optional or host-owned.
12. Phase 11, Replay Hardening: parallel golden coverage, replay/recovery, privacy/performance hardening.
13. Phase 12, Scenario Verification: parallel generic scenario tests and public API review.
14. Phase 13, Release Readiness: final package metadata, feature flag matrix, verification matrix, release handoff, and final reviewer PASS.

## Orchestration Plan

- Before each phase, read that phase README, launch docs, and contract inputs.
- For multi-target phases, spawn one worker per launch target with disjoint writable surfaces from its launch doc. Workers must not revert others' work and must finish with the `validation-gates.md` handoff format.
- The orchestrator owns integration, phase exit reports, shared naming reconciliation, final command execution, and review loops.
- Each phase gets a dedicated independent implementation reviewer using `docs/reference/sdk-review-checklist.md` and the phase report.
- Blocking review findings are fixed before starting the next phase.

## Phase 08 Execution Contract

Phase 08 is the single P1 typed-output integration target. It may edit only `crates/agent-sdk-core/tests/p1_typed_output.rs`, fixtures under `crates/agent-sdk-core/tests/fixtures/p1/`, and the minimum integration glue in existing Phase 04/06/07 application, records, package, and facade modules needed to run the P1 path. Any additional touched file must be named in the Phase 08 exit report as integration glue and must preserve the SDK package architecture gate.

Phase 08 behavior to prove before P2 starts:

- `agent.run_typed::<T>` and an explicit `RunRequest.output_contract` use the same P0 loop, effective runtime package, provider projection, local validation, repair, event, journal, and typed-result extraction path.
- Valid provider JSON produces durable `StructuredOutputRecord` evidence, emits journal-backed structured-output events, and returns a typed value only by deserializing the validated canonical content ref through the Phase 07 `TypedOutputDeserializer<T>` seam.
- Invalid provider JSON records a validation failure, builds a bounded repair request, retries through the provider as a normal model attempt, records repair evidence, and returns a typed value only after the repaired candidate validates.
- Repair exhaustion records terminal structured-output failure and returns a typed validation error without exposing best-effort parsed content.
- Output delivery sinks remain out of the P1 prerequisite path; typed result extraction uses validated output refs and does not require output dispatch.
- Output-contract normalization remains a runtime-package fingerprint input and helper lowering must not add a second package registry or feature-specific run loop.

Named Phase 08 tests in `cargo test -p agent-sdk-core --test p1_typed_output`:

- `run_typed_valid_output_uses_p0_loop_and_returns_typed_value`
- `explicit_output_contract_and_typed_helper_share_runtime_path`
- `invalid_output_repairs_and_publishes_typed_result_after_evidence`
- `repair_exhaustion_returns_structured_output_failure_without_typed_value`
- `typed_extraction_requires_canonical_validated_content_ref`
- `output_delivery_sink_not_required_for_p1_typed_result`
- `output_contract_changes_runtime_package_fingerprint_for_typed_run`
- `p1_structured_output_events_and_journal_match_golden_fixtures`

Required Phase 08 fixtures:

- `crates/agent-sdk-core/tests/fixtures/p1/typed-run-events.json`
- `crates/agent-sdk-core/tests/fixtures/p1/typed-run-journal.json`
- `crates/agent-sdk-core/tests/fixtures/p1/repair-success-events.json`
- `crates/agent-sdk-core/tests/fixtures/p1/repair-success-journal.json`
- `crates/agent-sdk-core/tests/fixtures/p1/repair-exhausted-events.json`
- `crates/agent-sdk-core/tests/fixtures/p1/repair-exhausted-journal.json`

Event and journal order gate:

- Every structured-output journal-backed event must be emitted only after the matching `JournalRecordPayload::StructuredOutput` append has produced a journal cursor.
- The Phase 08 golden fixtures must prove `StructuredOutputRequested`, `StructuredOutputValidationStarted`, `StructuredOutputValidationFailed`, `StructuredOutputRepairRequested`, `StructuredOutputValidated`, and `StructuredOutputFailed` where applicable.
- Structured-output events must remain distinct from output-delivery behavior; P1 must not require `OutputDispatchRecord` or an output sink.
- No test may construct a `StructuredOutputResult<T>` from a caller-provided `T`; all typed values must flow through validated canonical content-ref evidence and a fakeable deserializer.

## Validation Plan

- Run target-specific tests after each worker returns.
- Run phase-level `cargo fmt --check` and the relevant `cargo test` commands before each phase report is marked pass.
- Use golden fixtures for events, journals, package fingerprints, output validation, feature protocols, and release matrices.
- Require deterministic fakes, conformance helpers, and weird-scenario tests so SDK consumers can test their own providers, tools, sinks, isolation runtimes, extensions, telemetry exporters, and host adapters without live infrastructure.
- Use docs audits for link/path integrity, product-neutrality, primitive lowering, SDK-owned versus host-owned boundaries, no-mini-SDK drift, and phase exit readiness.
- Final Phase 12 validation runs the whole workspace test matrix. Skipped required release commands make Phase 12 `BLOCKED` unless the unsupported claim is removed from release scope or explicitly deferred in the release handoff.

## Risk / Gotcha Carry-Forward

- Do not let convenience helpers create a second behavior path; they must lower into canonical contracts.
- Do not make live events durable truth; journals remain the durable source for replay, resume, audit, recovery, and anti-entropy.
- Do not parse payloads, content stores, or journals on the live event filter hot path.
- Do not execute a side effect if the intent journal append fails. If result append fails after execution, enter recovery and block further unsafe effects.
- Do not silently downgrade isolation or run requested isolated work on the host without explicit policy.
- Do not emit raw prompt, model, tool, file, memory, or secret content by default.
- Do not let telemetry decide run state or become a second ledger.
- Do not add product-specific source/destination helpers to `agent-sdk-core`.
- Do not add new `CapabilitySpec` variants without sidecar contract, owner, fingerprint fields, events, journal records, and tests.
- Do not accept an adapter/port contract that SDK users cannot mock or verify with reusable test-support helpers.
- If a sibling launch target reveals a hidden dependency, move the dependent work to a later phase/report rather than coordinating through shared mutable work.

## Plan Gate

Implementation must not start until an independent plan reviewer returns PASS or PASS WITH NOTES with no blocking findings.
