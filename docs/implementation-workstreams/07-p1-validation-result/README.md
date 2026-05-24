# Phase 07: P1 Validation Result

Implement validation/repair and typed-result records over the Phase 06 output contract. Both launch targets may run in parallel after Phase 06 exits because neither owns the P1 loop integration.

## Launch Targets

| Title | Run in parallel? | Purpose |
| --- | --- | --- |
| [Validation Repair](07a-validation-repair.md) | yes | Local validation, bounded repair attempts, and hostile schema limits. |
| [Typed Result](07b-typed-result.md) | yes | `ValidatedOutput`, typed extraction DTOs, publication records, and ordering checks with fake validation reports. |

## Exit Gate

- [x] Validation and repair are local, bounded, journaled, and observable.
- [x] Typed-result records compile and publish only from validated output evidence.
- [x] Both targets use Phase 06 `OutputContract` shapes and do not create a second run loop.
- [x] Phase exit report records reviewer PASS.
