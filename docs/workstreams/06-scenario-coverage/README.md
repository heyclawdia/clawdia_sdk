# Phase 06: Scenario Coverage

Run this phase after every [Phase 05](../05-feature-layers/README.md) goal exits.

This phase is separated because scenario coverage depends on all feature-layer contracts.

## Goals

| Goal | Run in parallel? | Owner role | Purpose |
| --- | --- | --- | --- |
| [06a Generic Scenario Coverage](06a-generic-scenario-coverage.md) | only goal | [10 Generic Scenario Coverage](../_roles/10-generic-scenario-coverage.md) | Map generic scenarios to SDK primitives and host-owned boundaries without importing product behavior into core. |

## Exit Gate

- [ ] Every scenario maps to SDK-owned primitives and host-owned boundaries.
- [ ] Scenario gaps become proposal blocks or accepted primitive changes with owners.
- [ ] Examples remain product-neutral.

## Next Phase

After this phase exits, run [Phase 07: Final Review](../07-final-review/README.md).
