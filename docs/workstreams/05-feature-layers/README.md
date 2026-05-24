# Phase 05: Feature Layers

Run every goal in this folder in parallel after [Phase 04](../04-side-effects-policy/README.md) exits.

This phase layers streaming, realtime, isolation, subagents, and extensions over the kernel and side-effect spine.

## Goals

| Goal | Run in parallel? | Owner role | Purpose |
| --- | --- | --- | --- |
| [05a Streaming Realtime](05a-streaming-realtime.md) | yes | [05 Streaming](../_roles/05-streaming-realtime-rules.md) | Stream rules, interruptions, realtime lifecycle, and completion semantics. |
| [05b Isolation Execution](05b-isolation-execution.md) | yes | [06 Isolation](../_roles/06-isolation-execution.md) | Execution environments, adapter capability, process lifecycle, and cleanup. |
| [05c Subagents](05c-subagents.md) | yes | [07 Subagents](../_roles/07-subagents-coordination.md) | Parent-owned child runs, handoff policy, mailbox, and usage rollup. |
| [05d Extension SDK](05d-extension-sdk.md) | yes | [08 Extension SDK](../_roles/08-extension-sdk-packaging.md) | Extension manifests, core capability mapping, browser-safe exports, and action policy. |

## Exit Gate

- [x] Every feature is represented as package sidecars/capabilities, ports, events, journals, and policy refs over the kernel.
- [x] Realtime and streaming completion distinguish final visible text from terminal run completion.
- [x] Isolation never silently downgrades to host execution.
- [x] Subagents default to isolated child context and explicit handoff policy.
- [x] Extensions split core SDK capabilities from host extension manifest/runtime concerns.
- [x] Stitching checkpoint complete: blocking cross-cutting proposals from Phase 05 are accepted, rejected, or explicitly deferred before Phase 06 starts.

Exit evidence: [Phase 05 exit report](_phase/phase-exit-report.md).

## Next Phase

After every goal in this folder exits, run [Phase 06: Scenario Coverage](../06-scenario-coverage/README.md).
