# Phase 01: Package Capabilities

Run this phase after [Phase 00](../00-bootstrap/README.md) exits. It has one goal because later kernel goals depend on the runtime-package and capability shape.

## Goals

| Goal | Run in parallel? | Owner role | Purpose |
| --- | --- | --- | --- |
| [01a Runtime Package Capabilities](01a-runtime-package-capabilities.md) | only goal | [00 Integration](../_roles/00-integration-stitching.md) | Simplify runtime package, capability, sidecar, catalog, and fingerprint shape before kernel API/event/context goals start. |

## Exit Gate

- [x] Runtime-package fingerprint inputs are stable enough for Phase 02 goals to consume.
- [x] `CapabilitySpec` is limited to discoverable/callable capabilities, with non-callable execution policy in typed sidecars or package fields.
- [x] Reserved capability variants name owner role, sidecar contract, events, journal records, and future validation.

## Exit Evidence

- `docs/contracts/runtime-package-schema.md` now has a reserved capability variant readiness table naming owner role, typed sidecar contract, fingerprint fields, emitted events, journal records, and future validation for every non-P0/P1 variant.
- `FingerprintInputManifest` now records included groups, excluded volatile groups, readiness profile, and reserved feature status as an audit surface over the canonical package DTO.
- Shared references now point Phase 02 readers at the reserved-variant readiness gate in the primitive map, review matrix, feature-to-primitive matrix, and decision register.
- Documentation-only audits passed for local markdown links, runtime-package readiness, no-code constraints, ownership, product-neutrality, and no-mini-SDK package boundaries.

## Next Phase

After this phase exits, run every goal in [Phase 02: Primitive Kernel](../02-primitive-kernel/README.md) in parallel.
