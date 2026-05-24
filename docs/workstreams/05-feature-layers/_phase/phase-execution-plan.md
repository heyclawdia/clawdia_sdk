# Phase 05 Execution Plan: Feature Layers

## Launch Basis

- Phase: [05 Feature Layers](../README.md)
- Dependency: [Phase 04 Side Effects And Policy](../../04-side-effects-policy/README.md) exited, passed reviewer gate, and was committed before Phase 05 launch.
- Required shared reading: repository `README.md`, `docs/start-here.md`, `coding_standards.md`, [validation gates](../../validation-gates.md), [SDK review checklist](../../../reference/sdk-review-checklist.md), [primitive map](../../../architecture/primitive-map.md), this phase README, each goal doc, each owner role, and listed read-only inputs.
- Mode: documentation-only. Do not create Rust source files, executable tests, package manifests, or fixtures.

## Parallel Workers

| Goal | Agent | Writable contract scope | Integration notes |
| --- | --- | --- | --- |
| [05a Streaming Realtime](../05a-streaming-realtime.md) | Dirac (`019e5882-0304-7403-9657-4f98501a21fe`) | `docs/contracts/stream-rule-contract.md` | Prove stream rules, realtime lifecycle, interventions, restart/backpressure, and completion semantics are feature-layer state over the kernel and side-effect spine. |
| [05b Isolation Execution](../05b-isolation-execution.md) | Hooke (`019e5882-03af-74c0-8498-3c533011f99d`) | `docs/contracts/isolation-runtime-contract.md` | Prove isolation is `ExecutionEnvironment` plus adapter capabilities, policy, journal, events, cleanup, and no silent downgrade to host execution. |
| [05c Subagents](../05c-subagents.md) | Parfit (`019e5882-0444-70d0-aa7d-6256e620f278`) | `docs/contracts/subagent-contract.md` | Prove subagents are parent-owned child-run helpers over AgentPool with stripped packages, explicit handoff, wrapped events, child journals, run-message/wake clarification, lifecycle, and usage rollup. |
| [05d Extension SDK](../05d-extension-sdk.md) | Kuhn (`019e5882-055b-7a20-b164-9b8d6239c123`) | `docs/contracts/extension-sdk-contract.md` | Prove core extension capabilities are SDK-facing package material while host manifests, runtime, install, marketplace, browser-safe packaging, and action UI stay outside core. |

## Stitching Duties

- Collect worker handoffs in the required format from [validation-gates.md](../../validation-gates.md).
- Inspect changed contract files for shared event/journal names, package fingerprint inputs, OTel emitted-kind mapping closure, and feature-layer boundaries.
- Reconcile any accepted Phase 05 cross-cutting decisions into shared reference docs only through the integration/stitching role.
- Produce [phase-exit-report.md](phase-exit-report.md) with goal status, proposal decisions, shared names/IDs, validation evidence, reviewer verdict, and next-phase readiness.
- Update the Phase 05 README exit checklist only after the phase report and reviewer gate pass.

## Required Phase Audits

- `git diff --check`
- Whole-packet Markdown link/path audit
- No-code audit for Rust source, package manifests, executable tests, and fixtures
- Workstream ownership and changed-file writable-scope audit
- Product-neutrality audit for added lines and contract index
- Primitive/no-mini-SDK audit for parallel run, package, event, journal, policy, context, side-effect, telemetry, or host-adapter paths
- Proposal/blocker audit for accepted, rejected, deferred, and unresolved Phase 05 cross-cutting items
- Phase README exit-gate audit after integration

## Reviewer Gate

After worker integration and audits pass, spawn a dedicated reviewer agent to inspect Phase 05 changes, this execution plan, the phase exit report, worker evidence, proposal decisions, and audit outputs. Do not start Phase 06 until the reviewer returns PASS or all blocking findings are resolved and re-reviewed.
