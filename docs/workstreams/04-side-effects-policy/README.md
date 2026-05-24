# Phase 04: Side Effects And Policy

Run every goal in this folder in parallel after [Phase 03](../03-kernel-review/README.md) exits.

This phase proves side-effecting features all lower through the same policy, journal, event, telemetry/privacy, destination, and dedupe primitives.

## Goals

| Goal | Run in parallel? | Owner role | Purpose |
| --- | --- | --- | --- |
| [04a Tools Approval](04a-tools-approval.md) | yes | [04 Tools Approval](../_roles/04-tools-approval-toolpacks.md) | Tool execution, approval, tool packs, and mutating effects. |
| [04b Output Delivery](04b-output-delivery.md) | yes | [11 Output Delivery](../_roles/11-output-delivery-channels.md) | Host-owned output sinks, delivery intents/results, dedupe, and privacy. |
| [04c Telemetry Privacy](04c-telemetry-privacy.md) | yes | [09 Telemetry Privacy](../_roles/09-telemetry-privacy-cost.md) | Derived telemetry, content-capture policy, usage/cost, and sink failure. |
| [04d Hooks Lifecycle](04d-hooks-lifecycle.md) | yes | [01 Core API](../_roles/01-core-api-runtime.md) | Hook registration, mutation rights, package lowering, and policy-safe failures. |

## Exit Gate

- [ ] Every mutating side effect uses or maps to `EffectIntent` / `EffectResult`.
- [ ] Missing policy, dispatcher, adapter, sink, or journal append fails closed when required.
- [ ] Tool, hook, output, and telemetry behaviors reuse `RuntimePackage`, `PolicyRef`, `RunJournal`, and `AgentEvent`.
- [ ] Product channel UX, approval UI, telemetry dashboards, and extension runtimes remain host-owned.
- [ ] Stitching checkpoint complete: blocking cross-cutting proposals from Phase 04 are accepted, rejected, or explicitly deferred before Phase 05 starts.

## Next Phase

After every goal in this folder exits, run every goal in [Phase 05: Feature Layers](../05-feature-layers/README.md) in parallel.
