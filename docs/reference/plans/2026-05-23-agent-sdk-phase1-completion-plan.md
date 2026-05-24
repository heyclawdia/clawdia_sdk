# Agent SDK Phase 1 Completion Plan

> Historical plan only. It may contain stale product-specific paths or superseded workstream names. Do not use it as implementation authority; start from [../../start-here.md](../../start-here.md), [../../contracts/README.md](../../contracts/README.md), and [../../workstreams/README.md](../../workstreams/README.md).

## Objective

Improve the existing Agent SDK Phase 1 Markdown proposal by finding the unfinished decisions in the current documents and completing them in place, without implementing runtime code, creating Rust source files, or adding executable fixtures.

## Source Of Truth

- Primary goal file: `/Users/clawdia/goals/agent_sdk_phase1.md`.
- Existing Phase 1 docs under `docs/architecture/`.
- Risk notes for Agent SDK Phase 1 and the packaged extension SDK runtime fallback.
- Repo rule: do not create a branch without explicit user approval.

## Behavior Contract

New behavior:

- Add a clear "What changed" summary for this completion pass.
- Replace the documented open questions with explicit Phase 1 answers.
- Define a stable event taxonomy and durable journal/replay guarantees for resume, cancel, and failure paths.
- Tighten approval and policy semantics for headless runs, timeouts, escalation defaults, and missing dispatchers.
- Specify minimal extension SDK compatibility and smoke-test requirements for package roots, public subpaths, fallback ordering, and browser-safe helpers.
- Clarify security, privacy, redaction, projection audit fields, metadata limits, telemetry guarantees, and cost accounting guarantees.
- Verify Node ESM support from the packaged fallback before documenting the result.
- Integrate oh-my-pi lessons as SDK-owned tool packs for read/search/edit/write/shell/resource readers/tool discovery, without making the SDK a coding-agent product.
- Add streaming rule matching as a primitive for stop-on-literal/regex, abort-and-retry, mask, emit-only, and approval interventions across typed streaming channels.

Preserved behavior:

- No production runtime behavior changes.
- No changes to current product crates, package manifests, generated resources, or extension package contents.
- Existing user-local edits and untracked notes remain intact unless directly integrated into the requested docs.

Removed behavior:

- None. Broken or misplaced review stubs in existing Markdown may be replaced with integrated sections.

Validation:

- Run explicit Node ESM smoke checks for the packaged extension SDK fallback from outside the repo.
- Run `git diff --check`.
- Run a focused Markdown-only changed-file audit for this turn.
- Run a content audit against the user's requested Phase 1 concern list.

## Scope

In scope:

- Markdown docs under `docs/architecture/`.
- One completion plan under `docs/reference/plans/`.
- The existing Agent SDK Phase 1 risk note if a watchpoint needs updating.
- Documentation index updates if new files are added.

Out of scope:

- Implementing `agent_sdk`.
- Creating Rust/TypeScript source files, executable tests, fixtures, or generated package artifacts.
- Changing package versions, Tauri resources, runtime code, or extension SDK package exports.
- Finalizing every future API field beyond the minimal stable Phase 1 guarantees requested here.

## Workstreams

- Workstream A: verify extension SDK fallback behavior with Node ESM and capture the exact support boundary.
- Workstream B: tighten event taxonomy, journal, replay, resume, cancel, failure, telemetry, cost, privacy, and metadata guarantees.
- Workstream C: tighten policy and approval edge cases for headless, source-scoped, timeout, dispatcher absence, and escalation paths.
- Workstream D: integrate extension SDK compatibility and smoke requirements in the owning docs, then update the open-question answers.
- Workstream E: integrate oh-my-pi-inspired tool packs and streaming matcher primitives into the owning architecture, primitive, observability, example, standards, and risk docs.

## Verification Plan

- `node --version`
- Node ESM temp-directory import smoke for the packaged fallback root and public subpaths.
- Optional Bun comparison if useful to explain the fallback boundary.
- `git diff --check`
- Changed-file scope check: only Markdown files changed by this completion pass.
- Grep/content audit for: "What changed", "Open Questions", "resume", "cancel", "failure", "headless", "timeout", "dispatcher", "Node ESM", "redaction", "projection audit", "cost".
- Grep/content audit for: "ToolPack", "BuiltinToolPack", "StreamRule", "StreamRuleEngine", "regex", "read", "search", "edit", "write", and "shell".

## Risks And Rollback

- Risk: documenting Node ESM support from assumptions. Mitigation: run a real local smoke and document the observed result.
- Risk: the event taxonomy becomes too sprawling for Phase 1. Mitigation: define stable families and envelope guarantees in the observability doc, leave detailed payload schemas for Phase 2.
- Risk: policy semantics leak host UX into the SDK. Mitigation: keep dispatcher and escalation transport host-owned while making denial/timeout behavior stable.
- Risk: privacy guidance is too vague. Mitigation: specify default redaction behavior, projection audit fields, custom metadata limits, and opt-in content capture.

Rollback is documentation-only: revert this plan and the Markdown edits made by the completion pass.

## Review Status

Initial plan drafted for the completion pass and reviewed locally against the documentation-only goal constraints and the user's request to integrate decisions into the owning documents. A later explicit user request authorized an independent Phase 1 handoff review; that review found implementation-level blockers now carried into `docs/reference/plans/2026-05-23-agent-sdk-phase2-implementation-handoff-plan.md`.

## Final Status

Completed the Phase 1 completion pass in the owning docs:

- `docs/start-here.md` now summarizes what changed for Phase 1 completion.
- `docs/architecture/architecture-proposal.md` now answers the open questions directly and tightens policy/approval and extension SDK runtime semantics.
- `docs/architecture/architecture-proposal.md` now also defines opt-in built-in tool packs and streaming rule interventions.
- `docs/architecture/observability-and-lineage.md` now defines the stable event taxonomy, journal/replay guarantees, projection audit fields, metadata limits, redaction rules, and minimal telemetry/cost guarantees.
- `docs/architecture/external-sdk-lessons.md` now includes oh-my-pi read/search/edit/write/shell/tool-discovery and stream-rule lessons.
- `docs/architecture/primitive-map.md` now includes tool-pack, workspace search/edit, resource URI, tool discovery, stream delta, stream rule, matcher, engine, and intervention primitives.
- `docs/examples/*.md` now walk through generic chat, voice/realtime, CLI/headless usage, event fanout, journal behavior, telemetry, and recovery.
- `docs/examples/tool-pack-isolation-anti-entropy.md` now captures the streamed-text stop/retry need and common local-agent tool-pack coverage implication.
- `docs/reference/risks/agent-sdk-phase1-2026-05-21.md` now records the Node ESM evidence and follow-up watchpoint.

Verification run:

- `node --version` -> `v25.9.0`
- `bun --version` -> `1.3.13`
- Node ESM from `/tmp` with only the packaged fallback on `NODE_PATH` failed with `ERR_MODULE_NOT_FOUND`, so that support is documented as unsupported.
- Node ESM through normal local `node_modules` resolution succeeded for root, browser-safe helper, and process-only media subpaths in the historical smoke.
- Bun through the packaged fallback on `NODE_PATH` succeeded for root, browser-safe helper, and process-only media subpaths in the historical smoke.
- `git diff --check` passed.
- The follow-up content audit found the new `ToolPack`, `BuiltinToolPack`, `StreamRule`, `StreamRuleEngine`, regex, read/search/edit/write/shell vocabulary in the owning docs.
- `docs/architecture/coverage-gap-matrix.md` is no longer part of this completion diff.

Follow-up handoff hardening:

- `docs/reference/plans/2026-05-23-agent-sdk-phase2-implementation-handoff-plan.md` now captures the implementation-level contracts that remain intentionally beyond Phase 1's middle-level architecture scope.
- An independent review found that direct coding handoff was blocked until event payload schemas, loop transitions, runtime-package canonicalization, journal record shapes, structured output, stream rules, tool packs, isolation, extension packaging, and generic host-flow mappings were turned into explicit testable contracts. Those items are now carried forward in the Phase 2 handoff plan rather than treated as completed implementation design.
