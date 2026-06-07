# Phase 15: DX Completion

## Purpose

Complete the repo-grounded Rust SDK DX packet after the facade first slice. This
phase turns the previously documented DX gaps into implemented, tested,
product-neutral SDK surfaces.

## Entry Gate

Phase 15 is blocked until Phase 14 has a current exit report at
`docs/implementation-workstreams/14-evaluation-metrics/_phase/phase-exit-report.md`
showing the Phase 14 README exit gate and reviewer PASS.

## Launch Targets

Run the single target:

- [15a Full DX Completion](15a-full-dx-completion.md)

## Exit Gate

- The facade path can build and run a realistic deterministic agent through
  `clawdia-sdk`.
- Provider-visible typed tool declarations are projected into provider requests
  and provider adapter wire bodies.
- Typed tool helpers and macros lower into toolkit/core execution contracts.
- File-backed and Supabase-backed store adapters implement real SDK ports.
- Usage, cost, and run report helpers are deterministic projections.
- Runnable examples compile and run their fake paths without credentials.
- Risk/watchpoint docs and onboarding docs reflect the implemented behavior and
  any alpha breaking changes.
- Independent implementation review and developer-experience simulation return
  PASS with no unresolved blocking findings.
