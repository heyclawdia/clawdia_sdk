# Phase 05 Exit Report: Feature Layers

## Phase Objective

Phase 05 proves streaming/realtime, isolation, subagents, and extensions layer over the primitive kernel and Phase 04 side-effect spine. Each feature remains product-neutral and uses `RuntimePackage`, `PolicyRef`, `RunJournal`, `AgentEvent`, typed refs, ports/adapters, privacy/redaction, and shared effect fields instead of creating parallel registries, run loops, journals, event streams, policy paths, or telemetry stores.

## Goal Status

| Goal | Agent | Status | Changed files | Review packet |
| --- | --- | --- | --- | --- |
| [05a Streaming Realtime](../05a-streaming-realtime.md) | Dirac (`019e5882-0304-7403-9657-4f98501a21fe`) | PASS | `docs/contracts/stream-rule-contract.md` | [05a Review Packet](../05a-streaming-realtime.md#review-packet) |
| [05b Isolation Execution](../05b-isolation-execution.md) | Hooke (`019e5882-03af-74c0-8498-3c533011f99d`) | PASS | `docs/contracts/isolation-runtime-contract.md` | [05b Review Packet](../05b-isolation-execution.md#review-packet) |
| [05c Subagents](../05c-subagents.md) | Parfit (`019e5882-0444-70d0-aa7d-6256e620f278`) | PASS | `docs/contracts/subagent-contract.md` | [05c Review Packet](../05c-subagents.md#review-packet) |
| [05d Extension SDK](../05d-extension-sdk.md) | Kuhn (`019e5882-055b-7a20-b164-9b8d6239c123`) | PASS | `docs/contracts/extension-sdk-contract.md` | [05d Review Packet](../05d-extension-sdk.md#review-packet) |

## Accepted Proposals

- Accept `RealtimeSessionRecord` as the shared realtime journal record for connection, input/output cursor, interruption, restart, backpressure, and close state.
- Keep stream interventions as `StreamRuleRecord` intent/result payloads plus whichever provider, approval, output-delivery, or realtime effect they trigger; do not add `EffectKind::StreamIntervention` in Phase 05.
- Accept granular Phase 05 isolation event names for capability match, downgrade approval/denial, rootfs/session/mount/network/secret preparation, process I/O/stats/signal, cleanup failure, and failure. Earlier Phase 04 draft aliases should not be emitted by future adapters.
- Accept `ChildLifecycle*` event names for child artifact shutdown, detach, acknowledgement, denial, reclaim, and failure. Isolation owns isolated-process child-artifact lifecycle use; subagents reference shared child-lifecycle records for child-run cancellation and detach.
- Defer dedicated shared `EffectKind` variants for isolation image/rootfs/session/mount/network/secret/environment side effects. Typed `IsolationRecord::*Intent/Result` payloads must map one-to-one to common effect fields until code proves narrower shared kinds are needed.
- Accept `ExtensionActionStarted`, `ExtensionActionCompleted`, and `ExtensionActionFailed` as extension-family live projections backed by journaled `EffectResult`.
- Close the Phase 04 OTel deferrals for `stream_rule`, `realtime`, `isolation`, `child_lifecycle`, `subagent`, and `extension` families through explicit emitted-kind mappings, redaction defaults, journal records, and fixture gates.
- Include Phase 05 feature sidecars and SDK-facing capability snapshots in runtime-package fingerprint inputs when their reserved feature is activated; continue excluding host manifest/runtime/install/marketplace/trust/browser-safe/app-event transport details unless represented as SDK-facing refs or policy decisions.

## Rejected Proposals

- A dedicated `EffectKind::StreamIntervention` is rejected for Phase 05 because stream interventions already map through `StreamRuleRecord` plus existing provider, approval, output-delivery, or realtime side-effect records.
- Treating extension host manifest, runtime, install, marketplace, browser-safe export, raw trust, package-compatibility, or app-event transport metadata as core package authority is rejected.

## Deferred Proposals

- Dedicated shared `EffectKind` variants for every isolation image/rootfs/session/mount/network/secret/environment step are deferred until implementation demonstrates that narrower effect-kind names improve replay, telemetry, or policy review without duplicating the typed `IsolationRecord::*Intent/Result` payloads.
- Whether bidirectional realtime media transport lives in core or an optional realtime crate remains deferred; core owns channel/event/journal contracts either way.
- Exact Rust composition style for isolation, realtime, extension, subagent, and telemetry ports remains deferred to implementation planning.

## Unresolved Proposals

None.

## Shared Names And Records

- Stream/realtime shared records: `StreamRuleRecord`, `RealtimeSessionRecord`, `ContextRecord` for stream-rule injections, approval records for pauses, provider attempt records for retry/cancel, and output-delivery records for sink delivery.
- Isolation shared records: `IsolationRecord` for requested/capability/downgrade/lifecycle/process/cleanup facts and `ChildLifecycleRecord` for child-artifact shutdown, detach, acknowledgement, reclaim, and failure.
- Subagent shared records: `SubagentStartedRecord`, `SubagentHandoffRecord`, `SubagentWrappedEventRecord`, mailbox/clarification records, usage rollup records, terminal records, child journal refs, and shared child-lifecycle records.
- Extension shared records: SDK-facing capability catalog/sidecar records, hook/tool records where the extension supplies an executor, `ApprovalRecord` when an action asks, `EffectIntent { kind: ExtensionAction }`, terminal `EffectResult`, and recovery records for unsafe protocol/effect windows.
- OTel projections remain derived from event and journal facts only. The Phase 05 mapping rows do not grant permission to emit any kind before per-kind event, journal, redaction, and OTel projection fixtures exist.

## Exit Gate Evidence

- Every feature is represented as package sidecars/capabilities, ports, events, journals, and policy refs over the kernel: PASS.
- Realtime and streaming completion distinguish final visible text from terminal run completion: PASS.
- Isolation never silently downgrades to host execution: PASS.
- Subagents default to isolated child context and explicit handoff policy: PASS.
- Extensions split core SDK capabilities from host extension manifest/runtime concerns: PASS.
- Stitching checkpoint complete: PASS. Accepted, rejected, and deferred Phase 05 proposals are recorded above and in [cross-cutting proposals](../../../reference/cross-cutting-proposals.md#2026-05-24-phase-05-feature-layer-alignment).

## Validation Commands

- `git diff --check`: PASS
- Whole-packet Markdown link/path audit: PASS
- No-code audit for Rust source, package manifests, executable tests, and fixtures: PASS
- Product-neutrality added-line audit: PASS
- Workstream ownership audit: PASS for non-integration writable scopes.
- Changed-file writable-scope audit: PASS for Phase 05 worker scopes plus integration/stitching shared files.
- Contract index product-neutrality audit: PASS
- Owner-role and goal-doc validation-section audits: PASS
- Primitive-lowering/disjoint-scope audit: PASS for non-integration owner roles.
- Primitive/no-mini-SDK audit: PASS
- Proposal/blocker audit: PASS
- Phase README exit-gate audit: PASS after reviewer gate and checklist update.

## Reviewer Gate

- Reviewer verdict: PASS. Newton (`019e5898-5d8d-78d0-a1bf-15155655688c`) found no blocking issues and confirmed docs-only scope, writable-scope discipline, primitive lowering, cross-cutting decision reconciliation, extension host/core boundary, and Phase 05 exit evidence.
- Resolution: no fixes required. Reviewer noted this docs workspace has no `./verify.sh`, so the Phase 05 audit suite listed above is the applicable validation gate.

## Next-Phase Readiness

Phase 06 may start. The reviewer gate returned PASS and the Phase 05 README exit checklist is checked.
