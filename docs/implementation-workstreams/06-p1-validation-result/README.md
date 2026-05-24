# Phase 06: P1 Validation Result

Implement validation/repair and typed-result records over the Phase 05 output contract. Both launch targets may run in parallel after Phase 05 exits because neither owns the P1 loop integration.

## Launch Targets

| Title | Run in parallel? | Purpose |
| --- | --- | --- |
| [Validation Repair](06a-validation-repair.md) | yes | Local validation, bounded repair attempts, and hostile schema limits. |
| [Typed Result](06b-typed-result.md) | yes | `ValidatedOutput`, typed extraction DTOs, publication records, and ordering checks with fake validation reports. |

## Exit Gate

- [ ] Validation and repair are local, bounded, journaled, and observable.
- [ ] Typed-result records compile and publish only from validated output evidence.
- [ ] Both targets use Phase 05 `OutputContract` shapes and do not create a second run loop.
- [ ] Phase exit report records reviewer PASS.
