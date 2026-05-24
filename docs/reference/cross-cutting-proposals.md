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

## Open Proposals

None yet.
