# Phase 02: Primitive Kernel

Run every goal in this folder in parallel after [Phase 01](../01-package-capabilities/README.md) exits.

This phase freezes the SDK kernel contracts over the runtime-package spine from Phase 01. It does not include a final review goal because that would depend on these goals; the review is separated into [Phase 03](../03-kernel-review/README.md).

## Goals

| Goal | Run in parallel? | Owner role | Purpose |
| --- | --- | --- | --- |
| [02a Core Run API](02a-core-run-api.md) | yes | [01 Core API](../_roles/01-core-api-runtime.md) | Keep the MVP public API small while reserving feature layers. |
| [02b Events Journal Kernel](02b-events-journal-kernel.md) | yes | [02 Events Journal](../_roles/02-events-journal-replay.md) | Align event envelope, entity refs, journal records, and effect durability. |
| [02c Context Output Projection](02c-context-output-projection.md) | yes | [03 Context Output](../_roles/03-context-structured-output.md) | Make context a policy-admitted projection pipeline, not a universal bag. |

## Exit Gate

- [ ] All three goal review packets pass.
- [ ] Public names, IDs, package fingerprint inputs, event/journal names, and context/output IDs are ready for stitching review.
- [ ] No goal introduced a parallel run loop, package registry, event stream, journal, policy path, context projection path, or side-effect path.

## Next Phase

After every goal in this folder exits, run [Phase 03: Kernel Review](../03-kernel-review/README.md).
