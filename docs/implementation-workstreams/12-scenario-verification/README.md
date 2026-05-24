# Phase 12: Scenario Verification

Prove generic scenarios and public API readiness after replay hardening. Both launch targets may run in parallel after Phase 11 exits.

## Launch Targets

| Title | Run in parallel? | Purpose |
| --- | --- | --- |
| [Scenario Tests](12a-scenario-tests.md) | yes | Multi-component fake workflows matching generic examples. |
| [API Review](12b-api-review.md) | yes | Public API, rustdoc examples, SemVer posture, and simplicity pass. |

## Exit Gate

- [x] Generic scenario tests pass without product-specific host adapters.
- [x] Public API review passes with helpers lowering into canonical contracts.
- [x] Scenario/API evidence is ready for Phase 13 release-readiness stitching.
- [x] Phase exit report records reviewer PASS.
