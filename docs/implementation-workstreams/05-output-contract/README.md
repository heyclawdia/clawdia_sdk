# Phase 05: Output Contract

Define the output-contract surface after P0 exists and before validation/result work depends on it.

## Launch Targets

| Title | Run in parallel? | Purpose |
| --- | --- | --- |
| [Output Contract](05a-output-contract.md) | only target | Output schema refs, typed-mode DTOs, helper lowering, and package fingerprint normalization. |

## Exit Gate

- [ ] `OutputContract` and schema refs compile and have serde/fixture coverage.
- [ ] Helper lowering produces canonical `RunRequest` and runtime-package sidecar/fingerprint inputs.
- [ ] No validation, repair, typed-result publication, or output delivery behavior is implemented in this phase.
- [ ] Phase exit report records reviewer PASS.
