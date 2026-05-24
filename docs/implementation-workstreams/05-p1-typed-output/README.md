# Phase 05: P1 Typed Output

Add typed output on top of the P0 loop. All launch targets in this folder may run in parallel after Phase 04 exits.

## Launch Targets

| Title | Run in parallel? | Purpose |
| --- | --- | --- |
| [Output Contract](05a-output-contract.md) | yes | Output schema refs, typed-mode DTOs, and helper lowering. |
| [Validation Repair](05b-validation-repair.md) | yes | Local validation, bounded repair attempts, and hostile schema limits. |
| [Typed Result](05c-typed-result.md) | yes | `ValidatedOutput`, typed extraction, publication rules, and P1 fixtures. |

## Exit Gate

- [ ] Typed output lowers into the P0 run loop without a second provider or output path.
- [ ] Validation and repair are local, bounded, journaled, and observable.
- [ ] Typed results publish only after validation and policy allow it.
- [ ] Phase exit report records reviewer PASS.
