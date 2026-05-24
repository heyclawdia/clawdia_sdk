# Phase 06 Execution Plan: Scenario Coverage

## Launch Basis

- Phase: [06 Scenario Coverage](../README.md)
- Dependency: [Phase 05 Feature Layers](../../05-feature-layers/README.md) exited, passed reviewer gate, and was committed before Phase 06 launch.
- Required shared reading: repository `README.md`, `docs/start-here.md`, `coding_standards.md`, [validation gates](../../validation-gates.md), [SDK review checklist](../../../reference/sdk-review-checklist.md), [primitive map](../../../architecture/primitive-map.md), this phase README, owner role, all prior phase exit reports, and listed read-only inputs.
- Mode: documentation-only. Do not create Rust source files, executable tests, package manifests, or fixtures.

## Workstream

| Goal | Mode | Writable scenario scope | Integration notes |
| --- | --- | --- | --- |
| [06a Generic Scenario Coverage](../06a-generic-scenario-coverage.md) | serial single-goal phase | `docs/examples/*.md` | Prove desktop/web chat, CLI/headless, realtime, remote channel, external runtime, app/live event projection, telemetry, structured output, isolation, hook lifecycle, long-running detach, stream rules, subagents, memory/context, and output-delivery scenarios compose from SDK primitives plus host-owned adapters. |

## Scenario Editing Duties

- Add or update scenario coverage mapping in `docs/examples/README.md` with scenario, SDK primitives, host-owned boundaries, events/journals/telemetry/recovery, and validation.
- Update existing scenario files only where they need current Phase 05 event/journal names or stronger SDK/host boundary proof.
- Add a product-neutral example if a listed owner-role scenario is not covered by existing examples.
- Record missing primitives as proposal blocks in Phase 06 evidence, not as shared contract edits.

## Required Phase Audits

- `git diff --check`
- Whole-packet Markdown link/path audit
- No-code audit for Rust source, package manifests, executable tests, and fixtures
- Product-neutrality audit for examples and added lines
- Changed-file writable-scope audit for `docs/examples/*.md` plus phase report/checklist files under integration/stitching scope
- Scenario mapping audit: every owner-role scenario appears in the examples coverage matrix
- Boundary audit: scenario examples identify SDK-owned primitives and host-owned UI/storage/transport/credential/dashboard/workflow boundaries
- Coverage audit: every scenario names events, journal records, policy decisions, telemetry/cost records, recovery behavior, and host-owned storage/transport
- Proposal/blocker audit for accepted, rejected, deferred, and unresolved Phase 06 items
- Phase README exit-gate audit after reviewer gate

## Reviewer Gate

After scenario edits and audits pass, spawn a dedicated reviewer agent to inspect Phase 06 changes, this execution plan, the phase exit report, scenario coverage, boundary proof, and audit outputs. Do not start Phase 07 until the reviewer returns PASS or all blocking findings are resolved and re-reviewed.
