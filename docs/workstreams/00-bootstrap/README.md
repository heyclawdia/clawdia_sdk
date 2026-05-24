# Phase 00: Bootstrap

Run this phase first. It has one goal, so there is nothing to parallelize yet.

## Goals

| Goal | Run in parallel? | Owner role | Purpose |
| --- | --- | --- | --- |
| [00a Stitching Bootstrap](00a-stitching-bootstrap.md) | only goal | [00 Integration](../_roles/00-integration-stitching.md) | Freeze the launch structure, primitive decision ladder, source audit format, and feature-to-primitive matrix before parallel kernel work starts. |

## Exit Gate

- [ ] The phase-first workstream layout is current.
- [ ] The feature-to-primitive matrix exists and names all active feature areas.
- [ ] External SDK source audit format exists and has current source rows.
- [ ] Validation gates block mini SDKs, context overreach, unjournaled side effects, and capability bag drift.
- [ ] Phase 01 package/capability docs are ready to run before the parallel kernel phase.

## Next Phase

After this phase exits, run [Phase 01: Package Capabilities](../01-package-capabilities/README.md).
