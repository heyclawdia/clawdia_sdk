# Phase 09 Exit Report: P2 Side Effects

## Status

PASS.

Phase 09 integrated approval, generic tool execution, output delivery, hook lifecycle, and telemetry core over the P1 typed-run loop. The phase is ready for Phase 10 feature-port work.

## Scope Delivered

- Approval dispatch is modeled as SDK policy/broker behavior with finite decisions, timeout/cancel/unavailable outcomes, source-scoped decision validation, and fail-closed missing dispatcher behavior.
- Tool execution uses runtime-package capability authority, a public executor registry/policy port, intent-before-execution records, result refs, redacted summaries, and recovery for non-idempotent result append failures.
- Output delivery uses host sink ports, destination refs, content-mode policy, sink capability checks, dedupe proofs, reconciliation records, and repair replay that does not resend without completed dedupe proof.
- Hook lifecycle adds package hook sidecars, hook specs, ordering, timeout/cancel/failure policy, mutation-right validation, and journal-before-apply behavior for accepted behavior-changing responses.
- Telemetry core adds bounded fanout, usage/cost projection records, content-capture policy checks, sink failure/recovery records, terminal-preserving overflow behavior, and explicit proof that telemetry cannot decide run state, policy, output delivery, or side-effect status.

## Changed Implementation Glue

- `crates/agent-sdk-core/src/application/approval.rs`
- `crates/agent-sdk-core/src/application/tool.rs`
- `crates/agent-sdk-core/src/application/output_delivery.rs`
- `crates/agent-sdk-core/src/application/hooks.rs`
- `crates/agent-sdk-core/src/application/telemetry.rs`
- `crates/agent-sdk-core/src/domain/ids.rs`
- `crates/agent-sdk-core/src/domain/policy.rs`
- `crates/agent-sdk-core/src/domain/refs.rs`
- `crates/agent-sdk-core/src/package/hooks.rs`
- `crates/agent-sdk-core/src/ports/approval.rs`
- `crates/agent-sdk-core/src/ports/tool.rs`
- `crates/agent-sdk-core/src/ports/output_delivery.rs`
- `crates/agent-sdk-core/src/ports/hooks.rs`
- `crates/agent-sdk-core/src/ports/telemetry.rs`
- `crates/agent-sdk-core/src/records/approval.rs`
- `crates/agent-sdk-core/src/records/tool.rs`
- `crates/agent-sdk-core/src/records/output_delivery.rs`
- `crates/agent-sdk-core/src/records/hooks.rs`
- `crates/agent-sdk-core/src/records/telemetry.rs`
- `crates/agent-sdk-core/src/records/event.rs`
- `crates/agent-sdk-core/src/records/journal.rs`
- `crates/agent-sdk-core/src/lib.rs`

All new source remains under the SDK responsibility folders with only facade exports at `lib.rs`.

## Tests And Fixtures

- Root Cargo test targets remain two-line shims:
  - `crates/agent-sdk-core/tests/approval_contract.rs`
  - `crates/agent-sdk-core/tests/tool_execution_contract.rs`
  - `crates/agent-sdk-core/tests/output_delivery_contract.rs`
  - `crates/agent-sdk-core/tests/hook_lifecycle_contract.rs`
  - `crates/agent-sdk-core/tests/telemetry_core_contract.rs`
- Feature-layer test bodies live under `crates/agent-sdk-core/tests/feature_layers/`.
- Phase fixtures live under:
  - `crates/agent-sdk-core/tests/fixtures/approval/`
  - `crates/agent-sdk-core/tests/fixtures/tools/`
  - `crates/agent-sdk-core/tests/fixtures/output_delivery/`
  - `crates/agent-sdk-core/tests/fixtures/hooks/`
  - `crates/agent-sdk-core/tests/fixtures/telemetry/`

Named tests covered approval finite decisions and fail-closed dispatcher paths, tool policy/executor absence and non-idempotent recovery, output delivery sink capability/raw-content/dedupe/reconciliation behavior, hook mutation rights/journal-before-apply/security timeout behavior, and telemetry privacy/fanout/failure isolation.

## Validation Evidence

- `cargo fmt && cargo test -p agent-sdk-core --test approval_contract --test tool_execution_contract --test output_delivery_contract --test hook_lifecycle_contract --test telemetry_core_contract`: PASS.
- `cargo fmt && cargo test -p agent-sdk-core --test output_delivery_contract`: PASS, 13 tests after the durable dedupe fix.
- `cargo test -p agent-sdk-core`: PASS.
- `cargo test -p agent-sdk-core --no-default-features`: PASS.
- `cargo fmt --check`: PASS.
- `cargo tree -p agent-sdk-core --no-default-features`: PASS; no live provider, network, async runtime, or product dependency was introduced.
- Source-root architecture audit: PASS; no unexpected flat implementation files under `src/`.
- Root integration-test shim audit: PASS; `tests/*.rs` are two-line shims into responsibility folders.
- Product/live-service audit: PASS; grep hits are token-count fields, validation rejection of remote schema refs, finite approval token parser names, and an `example.invalid` test fixture.

## Review

Independent reviewer: Aquinas (`019e5a01-45b8-7d21-8f9e-a5da2a70f851`).

Initial verdict: BLOCKED.

Resolved findings:

- Approval and output result append failures after external contact could lose recovery. Fixed approval by appending a recovery marker and returning `RecoveryRepairNeeded`; fixed output delivery by appending typed reconciliation records and blocking unsafe success/replay.
- Hook behavior-changing responses journaled intent without a terminal result/recovery. Fixed hook response application to append the hook response record, effect intent, and terminal effect result before mutation can apply.
- P2 typed durable journal payload variants were not consistently used. Fixed approval, tool, output delivery, and hook canonical paths to append typed payloads. Output dedupe also now appends `OutputDeliveryRecord::Dedupe` before returning `Deduped`.
- Approval, tool, and telemetry fakes were private test helpers. Fixed with public reusable scripted helpers and facade exports: `ScriptedApprovalDispatcher`, `ScriptedToolExecutor`, `AllowToolPolicy`, and `ScriptedTelemetrySink`.
- Output delivery dedupe initially returned a skipped-send terminal fact without a durable journal record. Fixed by adding `OutputDeliveryDedupeRecord::to_journal_record(...)`, appending it on the dedupe short-circuit, and asserting the typed payload and `output_dispatch_deduped` event kind.

Final verdict: PASS.

## Boundary And Primitive Gates

- Primitive fit: PASS. Side effects lower into existing domain IDs/refs, runtime package authority, policy, event, journal, and typed port primitives.
- No mini-SDK: PASS. Phase 09 did not add a second run loop, package registry, event stream, journal, policy path, telemetry truth store, or side-effect queue.
- Mockability: PASS. Public scripted fakes and deterministic fixtures let SDK consumers exercise approval, tool, output delivery, hooks, telemetry, provider, journal, event, and content boundaries without live services.
- Journal/event durability: PASS. Mutating or externally visible side effects append intent before execution and terminal result, dedupe, or reconciliation records before exposing success, release, or replay-safe outcomes.
- Host-owned boundaries: PASS. Core defines contracts and ports; UI copy, concrete tools, credentials, notification UX, sink retry schedulers, OTel exporters, product adapters, and live infrastructure remain host/optional-crate owned.
- SDK package architecture gate: PASS. Source and tests follow responsibility folders and Cargo shim conventions.
- Canonical journal rule: PASS. P2 side-effect RunJournal records use typed feature payloads that embed the canonical effect intent/result where feature-specific durable replay data is needed; telemetry remains durable telemetry records/fanout, not a RunJournal payload in Phase 09.

## Carry-Forward Watchpoints

- Unknown output sink outcomes are durable through `OutputDeliveryResultRecord::reconciliation_needed`; standalone reconciliation records are used for append-failure and repair-replay cases.
- Tool denied-before-execution paths intentionally do not append requested/denied journal records in Phase 09. If Phase 11 hardening wants all denied attempts represented in RunJournal, add that as an explicit contract change with fixtures.
- If future telemetry exporters need durable replay, keep that in telemetry records/export cursors or an optional exporter crate; do not make telemetry a second event stream or run-journal truth store.
- If built-in tool packs arrive in Phase 10, keep concrete filesystem/shell/MCP behavior outside core and route it through the existing tool policy/executor ports.
