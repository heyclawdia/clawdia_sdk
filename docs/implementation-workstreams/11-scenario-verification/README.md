# Phase 11: Scenario Verification

Prove generic scenarios and public API readiness after replay hardening. Both launch targets may run in parallel after Phase 10 exits.

## Launch Targets

| Title | Run in parallel? | Purpose |
| --- | --- | --- |
| [Scenario Tests](11a-scenario-tests.md) | yes | Multi-component fake workflows matching generic examples. |
| [API Review](11b-api-review.md) | yes | Public API, rustdoc examples, SemVer posture, and simplicity pass. |

## Exit Gate

- [ ] Generic scenario tests pass without product-specific host adapters.
- [ ] Public API review passes with helpers lowering into canonical contracts.
- [ ] Scenario/API evidence is ready for Phase 12 release-readiness stitching.
- [ ] Phase exit report records reviewer PASS.
