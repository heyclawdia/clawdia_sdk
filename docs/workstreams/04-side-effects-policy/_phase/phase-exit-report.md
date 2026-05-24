# Phase 04 Exit Report: Side Effects And Policy

## Phase Objective

Phase 04 proves side-effecting SDK features lower through one shared spine: `RuntimePackage`, `PolicyRef`, `EffectIntent`, `EffectResult`, `RunJournal`, `AgentEvent`, typed refs, privacy/redaction, dedupe, and host-owned adapters. Phase 03 exited before this phase started, so side-effect work was layered over the reviewed primitive kernel.

## Goal Status

| Goal | Agent | Status | Changed files | Review packet |
| --- | --- | --- | --- | --- |
| [04a Tools Approval](../04a-tools-approval.md) | Lorentz (`019e586a-b953-7e01-be4b-4b5fe64df070`) | PASS | `docs/contracts/tool-approval-contract.md`, `docs/contracts/tool-pack-contract.md` | [04a Review Packet](../04a-tools-approval.md#review-packet) |
| [04b Output Delivery](../04b-output-delivery.md) | Pascal (`019e586a-dd56-76d3-8636-457dad9a5a0b`) | PASS | `docs/contracts/output-delivery-contract.md` | [04b Review Packet](../04b-output-delivery.md#review-packet) |
| [04c Telemetry Privacy](../04c-telemetry-privacy.md) | Einstein (`019e586a-fb48-78b0-8e95-ea8343573368`) | PASS | `docs/contracts/otel-mapping-contract.md`, `docs/contracts/telemetry-privacy-contract.md` | [04c Review Packet](../04c-telemetry-privacy.md#review-packet) |
| [04d Hooks Lifecycle](../04d-hooks-lifecycle.md) | Chandrasekhar (`019e586b-19bb-7b21-a871-77362b135b05`) | PASS | `docs/contracts/hook-lifecycle-contract.md`, `docs/contracts/api-contracts.md` | [04d Review Packet](../04d-hooks-lifecycle.md#review-packet) |

## Accepted Proposals

- Existing `event-schema.md` names cover Phase 04 tool, approval, hook, output-delivery, and telemetry-cost events; no Phase 04 event rename or new family is accepted.
- Approval dispatcher calls are explicitly represented as `EffectKind::ApprovalDispatch` records through `ApprovalRecord { dispatch_intent }` and `ApprovalRecord { dispatch_result }`; they are not a parallel side-effect path.
- Active tool-pack fingerprints include sidecar version/source, executor refs, policy refs, isolation/detach policy, redaction refs, and reconciliation requirements.
- Telemetry overflow uses `TelemetrySinkFailed` with `failure_kind = overflow` for the first slice.
- Phase 05 OTel mappings for stream/realtime, isolation/child-lifecycle, subagent, and extension families are deferred to their Phase 05 owners with emitted-kind fixture and redaction-case requirements.

## Rejected Proposals

- A separate `TelemetryOverflowed` event kind is rejected for Phase 04 because overflow can be represented by `TelemetrySinkFailed` payload fields without expanding the stable event taxonomy.

## Deferred Proposals

- Phase 05 emitted-kind telemetry mappings remain deferred to the Phase 05 stream/realtime, isolation, subagent, and extension owners. The deferral is nonblocking for Phase 04 because those event families are not active Phase 04 outputs.

## Unresolved Proposals

None.

## Shared Names And Records

- Tool and approval events stay on the existing `tool` and `approval` event families.
- Output delivery uses `OutputDispatchRequested`, `OutputDispatchCompleted`, `OutputDispatchFailed`, and `OutputDispatchDeduped`, backed by output-delivery intent/result/dedupe/reconciliation records.
- Telemetry sink overflow, sink failure, and sink recovery stay in the `telemetry_cost` family through `TelemetrySinkFailed` and `TelemetrySinkRecovered`.
- Hooks use `HookRegistered`, `HookInvoked`, `HookCompleted`, `HookFailed`, `HookTimedOut`, `HookCancelled`, `HookResponseApplied`, and `HookResponseRejected`.
- Runtime-package fingerprint inputs now explicitly include active Phase 04 sidecar, executor, policy, redaction, sink, mutation-right, fanout, and reconciliation fields where applicable.

## Exit Gate Evidence

- Every mutating or externally visible side effect uses or maps to `EffectIntent` / `EffectResult`: PASS. Approval dispatch, tools, output delivery, hook mutations through target domains, and telemetry repair boundaries all reuse the shared effect/journal shape where side effects exist.
- Missing policy, dispatcher, adapter, sink, or journal append fails closed when required: PASS. Tool policy/executor/journal failures, required output sink absence, security hook failures, and telemetry sink isolation are explicitly typed.
- Tool, hook, output, and telemetry behaviors reuse `RuntimePackage`, `PolicyRef`, `RunJournal`, and `AgentEvent`: PASS.
- Product channel UX, approval UI, telemetry dashboards, and extension runtimes remain host-owned: PASS.
- Stitching checkpoint complete: PASS. Accepted, rejected, and deferred Phase 04 proposals are recorded above and in [cross-cutting proposals](../../../reference/cross-cutting-proposals.md#2026-05-24-phase-04-side-effect-policy-alignment).

## Validation Commands

- `git diff --check`: PASS
- Whole-packet Markdown link/path audit: PASS after this report was created.
- No-code audit for Rust source, package manifests, executable tests, and fixtures: PASS
- Product-neutrality added-line audit: PASS
- Workstream ownership audit: PASS for non-integration writable scopes.
- Changed-file writable-scope audit: PASS for Phase 04 worker scopes plus integration/stitching event/journal/reference files.
- Contract index product-neutrality audit: PASS
- Owner-role and goal-doc validation-section audits: PASS
- Primitive-lowering/disjoint-scope audit: PASS for non-integration owner roles.
- Primitive/no-mini-SDK audit: PASS
- Proposal/blocker audit: PASS
- Phase README exit-gate audit: PASS after reviewer gate and checklist update.

## Reviewer Gate

- First reviewer verdict: BLOCKED. Finding: approval dispatch looked like a parallel external side-effect path because dispatcher records did not clearly map to `EffectIntent` / `EffectResult`.
- Resolution: added `EffectKind::ApprovalDispatch`, required `ApprovalRecord { dispatch_intent }` before host dispatcher access, required `ApprovalRecord { dispatch_result }` for terminal dispatcher outcomes, updated approval payload `effect_ref`, and recorded the decision in shared stitching docs.
- Re-review verdict: PASS. Reviewer confirmed approval dispatch is no longer a parallel external side-effect path and Phase 04 README can be checked.

## Next-Phase Readiness

Phase 05 may start. The reviewer gate returned PASS and the Phase 04 README exit checklist is checked.
