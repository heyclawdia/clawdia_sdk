# Phase 00: Bootstrap

Run this phase first. It has one goal, so there is nothing to parallelize yet.

## Goals

| Goal | Run in parallel? | Owner role | Purpose |
| --- | --- | --- | --- |
| [00a Stitching Bootstrap](00a-stitching-bootstrap.md) | only goal | [00 Integration](../_roles/00-integration-stitching.md) | Freeze the launch structure, primitive decision ladder, source audit format, and feature-to-primitive matrix before parallel kernel work starts. |

## Exit Gate

- [x] The phase-first workstream layout is current.
- [x] The feature-to-primitive matrix exists and names all active feature areas.
- [x] External SDK source audit format exists and has current source rows.
- [x] Validation gates block mini SDKs, context overreach, unjournaled side effects, and capability bag drift.
- [x] Phase 01 package/capability docs are ready to run before the parallel kernel phase.

## Exit Evidence

- Workstream shape audit passed: every numbered goal doc names owner, parallelism, required reading, writable files, read-only inputs, and validation.
- Link audits passed: all relative markdown links resolve, and all external source-audit URLs returned HTTP 2xx/3xx.
- Ownership audit passed: non-stitching role writable scopes are disjoint and do not write shared architecture/reference docs.
- Primitive/context audit passed: context remains `ContextContribution` -> admitted `ContextItem` -> `ContextProjection`, not a universal SDK abstraction.
- No-mini-SDK audit passed: validation/reference docs block private run loops, package registries, event streams, journals, policy paths, context paths, side-effect paths, telemetry truth stores, and capability bag drift.
- No-code audit passed: Phase 00 created no Rust source, package manifests, executable tests, JSON/YAML fixtures, or golden fixture files.

## Next Phase

After this phase exits, run [Phase 01: Package Capabilities](../01-package-capabilities/README.md).
