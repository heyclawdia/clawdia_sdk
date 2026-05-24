# Phase 04 Execution Plan: Side Effects And Policy

## Launch Basis

- Phase: [04 Side Effects And Policy](../README.md)
- Dependency: [Phase 03 Kernel Review](../../03-kernel-review/README.md) exited and was committed before Phase 04 launch.
- Required shared reading: repository `README.md`, `docs/start-here.md`, `coding_standards.md`, [validation gates](../../validation-gates.md), [SDK review checklist](../../../reference/sdk-review-checklist.md), [primitive map](../../../architecture/primitive-map.md), this phase README, each goal doc, each owner role, and listed read-only inputs.
- Mode: documentation-only. Do not create Rust source files, executable tests, package manifests, or fixtures.

## Parallel Workers

| Goal | Agent | Writable contract scope | Integration notes |
| --- | --- | --- | --- |
| [04a Tools Approval](../04a-tools-approval.md) | Lorentz (`019e586a-b953-7e01-be4b-4b5fe64df070`) | `docs/contracts/tool-approval-contract.md`, `docs/contracts/tool-pack-contract.md` | Prove tool calls and tool packs lower through runtime package, policy, effect intent/result, journal, event, source/destination/content refs where applicable, and fail closed when required. |
| [04b Output Delivery](../04b-output-delivery.md) | Pascal (`019e586a-dd56-76d3-8636-457dad9a5a0b`) | `docs/contracts/output-delivery-contract.md` | Prove output delivery is a host sink effect over destination, policy, dedupe, journal, events, privacy, and typed host configuration failures. |
| [04c Telemetry Privacy](../04c-telemetry-privacy.md) | Einstein (`019e586a-fb48-78b0-8e95-ea8343573368`) | `docs/contracts/otel-mapping-contract.md`, `docs/contracts/telemetry-privacy-contract.md` | Prove telemetry is derived from events, journals, usage/cost, and policy decisions, with opt-in raw content and isolated sink failures. |
| [04d Hooks Lifecycle](../04d-hooks-lifecycle.md) | Chandrasekhar (`019e586b-19bb-7b21-a871-77362b135b05`) | `docs/contracts/hook-lifecycle-contract.md`, `docs/contracts/api-contracts.md` only if hook helper/API lowering is needed | Prove hooks lower into package sidecars before runs, have lifecycle-specific mutation rights, journal mutations before apply, and fail closed for security-critical paths. |

## Stitching Duties

- Collect worker handoffs in the required format from [validation-gates.md](../../validation-gates.md).
- Inspect changed contract files for conflicts, duplicate primitives, missing owner boundaries, and accidental product-specific behavior.
- Reconcile any accepted Phase 04 cross-cutting decisions into shared reference docs only through the integration/stitching role.
- Produce [phase-exit-report.md](phase-exit-report.md) with goal status, proposal decisions, shared name/ID changes, validation evidence, reviewer verdict, and next-phase readiness.
- Update the Phase 04 README exit checklist only after the phase report and reviewer gate pass.

## Required Phase Audits

- `git diff --check`
- Whole-packet Markdown link/path audit
- No-code audit for Rust source, package manifests, executable tests, and fixtures
- Workstream ownership and changed-file writable-scope audit
- Product-neutrality audit for added lines and contract index
- Primitive/no-mini-SDK audit for parallel run, package, event, journal, policy, context, side-effect, telemetry, or host-adapter paths
- Proposal/blocker audit for accepted, rejected, deferred, and unresolved Phase 04 cross-cutting items
- Phase README exit-gate audit after integration

## Reviewer Gate

After worker integration and audits pass, spawn a dedicated reviewer agent to inspect Phase 04 changes, this execution plan, the phase exit report, worker evidence, proposal decisions, and audit outputs. Do not start Phase 05 until the reviewer returns PASS or all blocking findings are resolved and re-reviewed.
