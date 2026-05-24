# Phase 01: Package Capabilities

Run this phase after [Phase 00](../00-bootstrap/README.md) exits. It has one goal because later kernel goals depend on the runtime-package and capability shape.

## Goals

| Goal | Run in parallel? | Owner role | Purpose |
| --- | --- | --- | --- |
| [01a Runtime Package Capabilities](01a-runtime-package-capabilities.md) | only goal | [00 Integration](../_roles/00-integration-stitching.md) | Simplify runtime package, capability, sidecar, catalog, and fingerprint shape before kernel API/event/context goals start. |

## Exit Gate

- [ ] Runtime-package fingerprint inputs are stable enough for Phase 02 goals to consume.
- [ ] `CapabilitySpec` is limited to discoverable/callable capabilities, with non-callable execution policy in typed sidecars or package fields.
- [ ] Reserved capability variants name owner role, sidecar contract, events, journal records, and future validation.

## Next Phase

After this phase exits, run every goal in [Phase 02: Primitive Kernel](../02-primitive-kernel/README.md) in parallel.
