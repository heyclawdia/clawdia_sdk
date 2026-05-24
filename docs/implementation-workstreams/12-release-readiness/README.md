# Phase 12: Release Readiness

Run final implementation handoff checks after scenario and API verification exit.

## Launch Targets

| Title | Run in parallel? | Purpose |
| --- | --- | --- |
| [Release Readiness](12a-release-readiness.md) | only target | Packaging, feature flags, docs, changelog, verification matrix, and final reviewer gate. |

## Exit Gate

- [ ] Full workspace verification passes or every skipped command has an explicit non-release blocker.
- [ ] Feature flag matrix proves core/default and optional crates remain separated.
- [ ] Contract-to-code traceability and release notes are complete.
- [ ] Final reviewer PASS is recorded before any release handoff.
