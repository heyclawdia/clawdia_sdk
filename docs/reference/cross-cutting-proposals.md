# Cross-Cutting Proposals

This file is maintained by the integration/stitching role. Parallel goals must not edit it unless their writable list explicitly says so. They should include proposal blocks in their handoff; the stitching owner reconciles accepted proposals here.

## Proposal Template

```text
Status: proposed | accepted | rejected | superseded
Proposed by:
Date:
Affected workstreams:
Affected files:
Decision owner: 00-integration-stitching

Problem:

Proposed change:

Why this is cross-cutting:

Compatibility impact:

Validation needed:
```

## Accepted Decisions

### 2026-05-24 Phase Delivery Protocol And Kernel Review Gate

Status: accepted
Proposed by: Phase 03 kernel review
Date: 2026-05-24
Affected workstreams: 03, 04, 05, 06, 07
Affected files: `docs/workstreams/README.md`, `docs/workstreams/validation-gates.md`, `docs/workstreams/[0-9][0-9]-*/README.md`, `docs/workstreams/[0-9][0-9]-*/_phase/*`
Decision owner: 00-integration-stitching

Problem:

The packet needed a durable per-phase review surface so humans and agents can see which goal workers ran, which proposals were accepted/rejected/deferred, which audits passed, and which reviewer gate allowed the next phase to start. Without that surface, parallel goal output can be correct locally but hard to review as one phase.

Proposed change:

Every multi-goal phase launches one subagent per goal file with disjoint writable scopes. Every phase writes a phase-local `_phase/phase-exit-report.md`, runs phase-level audits, and uses a dedicated reviewer agent as the gate before the next numbered phase starts. Single-goal stitching phases run serially but still use reviewer gates.

Why this is cross-cutting:

This affects the launch and completion protocol for every remaining workstream, not one contract.

Compatibility impact:

Documentation-only process change. Existing goal docs and owner-role writable scopes remain authoritative.

Validation needed:

Phase exit reports must include goal status, validation evidence, proposal decisions, reviewer verdict, and next-phase readiness. The phase README exit gate cannot be checked until that evidence exists.

### 2026-05-24 Primitive Simplification And Final Stitching

Status: accepted
Proposed by: Phase 07 final stitching review
Date: 2026-05-24
Affected workstreams: 01, 02, 03, 04, 05, 06, 07, 08, 09, 10, 11
Affected files: `docs/contracts/*`, `docs/architecture/*`, `docs/workstreams/*`, `docs/reference/*`
Decision owner: 00-integration-stitching

Problem:

The packet had several cross-cutting authority risks: runtime-package authority drift, event envelope ID growth, output-contract ownership split, context/memory journal shape drift, scalar isolation ranking, hooks as a generic effect/event control plane, read-tool audit ambiguity, extension host-manifest leakage, telemetry sink backpressure gaps, and workstream launch docs that delayed stitching until too late.

Proposed change:

Accepted final-stitching decisions:

- Gate implementation readiness as P0 text run, P1 typed output, and P2 tools/approval side effects.
- Treat the resolved per-run `RuntimePackage` as the execution authority and fingerprint source; `AgentRuntime` owns refs, resolvers, registries, and ports.
- Treat `RunRequest.output_contract` as user-facing authority that is normalized into the effective package sidecar/fingerprint before execution.
- Keep event hot-path IDs universal; feature IDs use `EntityRef` unless promoted by integration plus events/journal review.
- Standardize on `MemoryPort` and top-level `JournalRecordKind::ContextRecord` with typed context/memory payload variants, including memory write intent/result payloads.
- Compare isolation by class plus capability/trust vectors, not a global enum order.
- Keep hooks lifecycle-specific: accepted hook proposals lower into existing domain operations; no generic effect or event emission hatch.
- Require tool execution intent/result records for every tool call, including reads.
- Keep extension core capabilities SDK-facing only; host manifests own browser-safe exports, package compatibility, trust state, action permissions, runtime, install, and marketplace data.
- Make telemetry fanout bounded, nonblocking, terminal-preserving, and exporter-drained off-loop.
- Add stitching checkpoints after Phases 04, 05, and 06; align loop-state ownership; narrow Phase 07 writable scope to Role 00; split documentation validation from implementation validation.

Why this is cross-cutting:

Each item affects shared public names, runtime-package fingerprints, event/journal shapes, side-effect ordering, workstream ownership, or product-neutral boundaries.

Compatibility impact:

Documentation-only cleanup before Rust code exists. Future implementation goals should treat the accepted names and authority boundaries above as coding constraints.

Validation needed:

Whole-packet link/path audit, workstream ownership audit, product-neutrality audit, no-code audit, review-matrix audit, text audits for removed hazards, and independent implementation review.

### 2026-05-24 Phase 04 Side-Effect Policy Alignment

Status: accepted
Proposed by: Phase 04 side-effect workers and stitching checkpoint
Date: 2026-05-24
Affected workstreams: 01, 02, 04, 09, 11
Affected files: `docs/contracts/runtime-package-schema.md`, `docs/contracts/event-schema.md`, `docs/contracts/review-matrix.md`, `docs/reference/feature-to-primitive-matrix.md`, `docs/reference/open-questions-and-ambiguities.md`, `docs/workstreams/04-side-effects-policy/_phase/phase-exit-report.md`
Decision owner: 00-integration-stitching

Problem:

Phase 04 workers independently tightened tools, output delivery, telemetry, and hooks around the shared side-effect spine. Their handoffs raised three cross-cutting decisions: whether tool/approval event names need shared taxonomy changes, which tool-pack fields must affect runtime-package fingerprints, and whether telemetry overflow needs its own event kind.

Proposed change:

Accepted decisions:

- Keep existing tool, approval, output-delivery, hook, and telemetry-cost event names from `event-schema.md`; no new Phase 04 event family or rename is needed.
- Treat host/user approval dispatcher calls as `EffectKind::ApprovalDispatch` records wrapped by `ApprovalRecord { dispatch_intent }` and `ApprovalRecord { dispatch_result }` before any dispatcher access can release a tool execution.
- Include active tool-pack sidecar version/source, executor refs, policy refs, isolation/detach policy, redaction refs, and reconciliation requirements in runtime-package fingerprint inputs when those features are active.
- Keep telemetry overflow represented as `TelemetrySinkFailed` with `failure_kind = overflow` for the first slice. A future separate `TelemetryOverflowed` event kind would require an event-schema update and emitted-kind fixture.
- Defer Phase 05 OTel mappings for stream/realtime, isolation/child-lifecycle, subagent, and extension families to their respective Phase 05 owners; those owners must provide emitted-kind fixtures and redaction cases before activation.

Why this is cross-cutting:

These decisions affect shared event taxonomy, runtime-package fingerprint determinism, side-effect journal/event alignment, and telemetry projection ownership across multiple contracts.

Compatibility impact:

Documentation-only alignment before Rust code exists. Existing event names stay stable. Fingerprint tests gain more explicit active-feature inputs instead of relying on implicit tool-pack or sink state.

Validation needed:

Phase 04 exit report must prove every side-effecting feature maps to `EffectIntent` / `EffectResult`, approval dispatch is not a parallel side-effect path, missing required policy/dispatcher/adapter/sink/journal append fails closed, telemetry remains derived, product-specific host UX stays outside contracts, and Phase 05 deferrals name their owners.

### 2026-05-24 Phase 05 Feature-Layer Alignment

Status: accepted
Proposed by: Phase 05 feature-layer workers and stitching checkpoint
Date: 2026-05-24
Affected workstreams: 05, 06, 07, 08, 09
Affected files: `docs/contracts/event-schema.md`, `docs/contracts/journal-replay-schema.md`, `docs/contracts/runtime-package-schema.md`, `docs/contracts/otel-mapping-contract.md`, `docs/contracts/review-matrix.md`, `docs/reference/feature-to-primitive-matrix.md`, `docs/reference/open-questions-and-ambiguities.md`, `docs/workstreams/05-feature-layers/_phase/phase-exit-report.md`
Decision owner: 00-integration-stitching

Problem:

Phase 05 workers supplied feature-layer contracts for streaming/realtime, isolation, subagents, and extensions. Their handoffs raised shared decisions about event taxonomy, journal record names, runtime-package fingerprint inputs, effect-kind granularity, child-lifecycle ownership, and whether Phase 04 OTel deferrals could close.

Proposed change:

Accepted decisions:

- Accept `RealtimeSessionRecord` as the shared realtime journal record for connection, input/output cursor, interruption, restart, backpressure, and close state.
- Keep stream interventions as `StreamRuleRecord` intent/result payloads plus whichever provider, approval, output-delivery, or realtime effect they trigger; do not add `EffectKind::StreamIntervention` in Phase 05.
- Accept the granular Phase 05 isolation event names for capability match, downgrade approval/denial, rootfs/session/mount/network/secret preparation, process I/O/stats/signal, cleanup failure, and failure. Earlier Phase 04 draft aliases should not be emitted by future adapters.
- Accept `ChildLifecycle*` event names for child artifact shutdown, detach, acknowledgement, denial, reclaim, and failure. Isolation owns isolated-process child-artifact lifecycle use; subagents reference the shared child-lifecycle records for child-run cancellation and detach.
- Defer dedicated shared `EffectKind` variants for isolation image/rootfs/session/mount/network/secret/environment steps. Typed `IsolationRecord::*Intent/Result` payloads must map one-to-one to common effect fields until code proves narrower effect kinds are needed.
- Accept `ExtensionActionStarted`, `ExtensionActionCompleted`, and `ExtensionActionFailed` so extension action telemetry has explicit live projections in addition to journal-backed `EffectResult`.
- Close the Phase 04 OTel deferrals for `stream_rule`, `realtime`, `isolation`, `child_lifecycle`, `subagent`, and `extension` families by naming mappings, redaction defaults, journal records, and fixture gates in the Phase 05 contracts and OTel contract.
- Include Phase 05 feature sidecars and SDK-facing capability snapshots in runtime-package fingerprint inputs when their reserved feature is activated; continue excluding host manifest/runtime/install/marketplace/trust/browser-safe/app-event transport details unless represented as SDK-facing refs or policy decisions.

Why this is cross-cutting:

These decisions change shared event/journal vocabulary, package fingerprint determinism, telemetry projection rules, and the boundaries between feature-layer contracts and kernel primitives.

Compatibility impact:

Documentation-only alignment before Rust code exists. Existing draft names are clarified as pre-implementation aliases. Future code must fixture every emitted kind before activation.

Validation needed:

Phase 05 exit report must prove every feature layer lowers to runtime-package sidecars/capabilities, ports, events, journals, policy refs, and shared effect fields; OTel mappings are derived only; product-specific host UX and host manifests stay outside core; and unresolved cross-cutting proposals are accepted, rejected, or explicitly deferred before Phase 06 starts.

## Open Proposals

None yet.
