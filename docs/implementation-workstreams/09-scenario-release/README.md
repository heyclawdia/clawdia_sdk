# Phase 09: Scenario Release

Prove the implementation is ready for a first public handoff. All launch targets in this folder may run in parallel after Phase 08 exits.

## Launch Targets

| Title | Run in parallel? | Purpose |
| --- | --- | --- |
| [Scenario Tests](09a-scenario-tests.md) | yes | Multi-component fake workflows matching generic examples. |
| [API Review](09b-api-review.md) | yes | Public API, rustdoc examples, SemVer posture, and simplicity pass. |
| [Release Readiness](09c-release-readiness.md) | yes | Packaging, feature flags, docs, changelog, and final verification matrix. |

## Exit Gate

- [ ] Generic scenario tests pass without product-specific host adapters.
- [ ] Public API review passes with helpers lowering into canonical contracts.
- [ ] Release package builds with optional crates/features separated.
- [ ] Phase exit report records reviewer PASS and final implementation readiness.
