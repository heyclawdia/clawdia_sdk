# Phase 16: DX Phase II

## Purpose

Polish the Phase 15 SDK DX implementation into a coherent first-developer
experience. This phase improves onboarding, examples, diagnostics, event
observation, checkpoint/replay resume-readiness quickstarts, approval
ergonomics, and review evidence while keeping the primitive kernel as the only
behavior authority.

This phase is product-neutral and comparison-free. It must not introduce
references to outside SDKs, outside package families, or unrelated hosted
systems as implementation drivers.

## Entry Gate

Phase 16 is blocked until Phase 15 has a current exit report at
`docs/implementation-workstreams/15-dx-completion/_phase/phase-exit-report.md`
showing the Phase 15 README exit gate and reviewer PASS.

## Launch Targets

Run the single target:

- [16a DX Phase II](16a-dx-phase-ii.md)

## Exit Gate

- README, Start Here, facade docs, and example READMEs present one consistent
  first-developer sequence.
- New examples run without live credentials and cover event observation,
  checkpoint/replay resume-readiness, typed output, approval denial or approval
  success, report projection, and feature selection where implemented.
- Public diagnostics and rustdoc guide users through missing feature flags,
  missing stores, missing approval dispatch, and missing report evidence.
- Every new helper or example proves canonical lowering into existing runtime,
  package, policy, event, journal, content, store, tool, and report contracts.
- No new dependency lands in `agent-sdk-core` for provider, toolkit, macro,
  store, report, UI, live infrastructure, or product adapter behavior.
- Risk/watchpoint docs and onboarding docs reflect implemented behavior and any
  alpha breaking changes.
- Independent implementation review and first-developer simulation return PASS
  with no unresolved blocking findings.
