# Phase 04: P0 Text Run

Integrate the first fake-provider text run. This phase is intentionally single-target because it depends on all Phase 02 and Phase 03 outputs.

## Launch Targets

| Title | Run in parallel? | Purpose |
| --- | --- | --- |
| [Text Run](04a-text-run.md) | only target | Prove one fake-provider text run through runtime package, context projection, provider port, events, journal, and handle result. |

## Exit Gate

- [x] P0 fake-provider text run passes through the canonical loop.
- [x] P0 does not require tools, approvals, isolation, extensions, subagents, realtime, telemetry exporters, or typed output.
- [x] Events and journal records prove durable lifecycle and terminal result.
- [x] Phase exit report records reviewer PASS.
