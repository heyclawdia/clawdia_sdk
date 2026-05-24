# Phase 02: Core Records

Implement durable and observable record shapes over the shared kernel. All launch targets in this folder may run in parallel after Phase 01 exits.

## Launch Targets

| Title | Run in parallel? | Purpose |
| --- | --- | --- |
| [Runtime Package](02a-runtime-package.md) | yes | Effective package snapshot, capability specs, typed sidecars, catalogs, deltas, and fingerprints. |
| [Event Frames](02b-event-frames.md) | yes | Event envelope, event frames, cursors, filters, overflow notices, and hot-path fanout primitives. |
| [Run Journal](02c-run-journal.md) | yes | Journal record enum, append/replay cursors, checkpoints, and effect atomicity scaffolding. |
| [Content Context](02d-content-context.md) | yes | Content/artifact refs, context contributions, admitted items, projection, and projection audit records. |
| [Provider Port](02e-provider-port.md) | yes | Provider adapter trait, fake provider, provider projection request, and usage extraction shell. |

## Exit Gate

- [ ] Runtime package, events, journal, context, and provider records have compile and fixture coverage.
- [ ] Live event frames and durable journal records stay distinct.
- [ ] Runtime-package fingerprints are deterministic for implemented fields.
- [ ] Phase exit report records reviewer PASS.
