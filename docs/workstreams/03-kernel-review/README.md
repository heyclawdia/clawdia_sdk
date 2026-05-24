# Phase 03: Kernel Review

Run this phase after [Phase 01](../01-package-capabilities/README.md) and every [Phase 02](../02-primitive-kernel/README.md) goal exits.

This is a separate phase because it depends on all Phase 01 and Phase 02 outputs.

## Goals

| Goal | Run in parallel? | Owner role | Purpose |
| --- | --- | --- | --- |
| [03a Kernel Final Review](03a-kernel-final-review.md) | only goal | [00 Integration](../_roles/00-integration-stitching.md) | Reconcile names, IDs, events, journals, runtime-package fingerprints, and primitive-layering decisions from Phase 01 and Phase 02. |

## Exit Gate

- [x] The Phase 01 package spine and Phase 02 primitive kernel are internally consistent.
- [x] The feature-to-primitive matrix and review matrix reflect accepted Phase 01 and Phase 02 decisions.
- [x] Later phases can use the kernel without reopening must-answer primitive questions.

## Exit Evidence

- [Phase 03 exit report](_phase/phase-exit-report.md) reconciles Phase 01 runtime-package authority, Phase 02 core API/event-journal/context-output contracts, source audit, matrices, proposal decisions, and next-phase readiness.
- The phase delivery protocol is recorded in [../README.md](../README.md) and [../validation-gates.md](../validation-gates.md).
- The source audit was reviewed in [../../architecture/external-sdk-lessons.md](../../architecture/external-sdk-lessons.md) with no new source rows required.
- `docs/reference/feature-to-primitive-matrix.md` and `docs/contracts/review-matrix.md` now reflect content resolver policy, projection audit, and `ValidatedOutput` publication decisions.
- Plan reviewer and implementation reviewer both returned PASS after findings were addressed.

## Next Phase

After this phase exits, run every goal in [Phase 04: Side Effects And Policy](../04-side-effects-policy/README.md) in parallel.
