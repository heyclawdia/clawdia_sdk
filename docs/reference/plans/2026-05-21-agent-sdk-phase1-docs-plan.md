# Agent SDK Phase 1 Documentation Plan

> Historical plan only. It may contain stale product-specific paths or superseded workstream names. Do not use it as implementation authority; start from [../../start-here.md](../../start-here.md), [../../contracts/README.md](../../contracts/README.md), and [../../workstreams/README.md](../../workstreams/README.md).

## Objective

Produce a reviewable, middle-level Markdown architecture proposal for a future Rust-first `agent_sdk` crate and extension SDK layer. This is a documentation-only phase: no Rust source, executable tests, fixtures, runtime edits, or production crate changes.

## Source Of Truth

- Primary goal file: `/Users/clawdia/goals/agent_sdk_phase1.md`.
- Repo standards: `coding-standards.md`.
- Current Clawdia behavior is a coverage constraint, not the design template.
- External SDKs are study inputs, not designs to clone.

## Relevant Existing Context

- `docs/architecture/agent-runtime-canonical.md`: execution is owned by `AgentExecutionService`; ACP and next-gen remain active adapters; approvals and runtime events flow through shared backend boundaries.
- `docs/backend/agent-core.md`: current next-gen `agent-core` already models provider-agnostic loop, tool registry/router/orchestrator, approvals, events, stop checks, bounded parallel tools, image handling, and subagent primitives.
- `docs/architecture/conversation-lifecycle.md`: history replay, image persistence, ACP live-tab lifecycle, trace linkage, scratchpad separation, and memory injection behavior.
- `docs/architecture/context-window-and-compaction.md`: shared `RuntimeExecutionPolicyService`, next-gen inline compaction, ACP retirement/replay compaction, provider usage mapping, and context-window fallback rules.
- `docs/architecture/tool-exposure-pipeline.md`: provider-visible schema export must match effective runtime registry; ACP uses bounded lazy app/MCP discovery; allowlists govern exposure/execution, not approval.
- `docs/architecture/extensibility-model.md`: Purrnel/package model, manifest-driven tools/hooks/providers, SDK subprocess extensions, host-owned app-event observation, and explicit `toolProvider` / `hookProvider` / `subagentProvider` opt-ins.
- `docs/features/memory.md`: durable memory is a backend-owned typed retrieval/ingest boundary with `memory_glance`, `memory_recall`, `memory_detail`, and `memory_store`.
- `docs/features/tool-approval.md` and `docs/features/escalation-manager.md`: shared approval broker, finite decisions, YOLO mode, source-scoped/headless approval, and out-of-band escalation boundaries.
- Risk notes read: `runtime-package-assembly-2026-04-13.md`, `approval-broker-contract-2026-04-13.md`, `app-event-stream-pubsub-2026-05-15.md`, `subagent-primitives-window-sdk-2026-05-18.md`, and `extension-sdk-runtime-fallback-2026-05-17.md`.
- Code surfaces sampled: `application/runtime_engine.rs`, `application/runtime_package_assembly_service.rs`, `infrastructure/runtime_package.rs`, `application/agent_execution_service.rs`, `crates/agent-core/src/types.rs`, runtime/event/approval/subagent grep scans.

## External Study Inputs

- Strands Agents SDK: core `Agent`, model/tool/session/conversation manager, hooks/plugins, async streaming, cancellation, bidirectional realtime events, MCP support, and multi-agent patterns.
- Cursor TypeScript SDK: programmatic local/cloud/self-hosted agents, durable agent/run split, run streaming/reconnect, MCP, skills, hooks, subagents, cloud VM/sandbox/session primitives.
- Claude Agent SDK Python: `query()` vs `ClaudeSDKClient`, bundled CLI transport, sessions, typed content blocks, permission modes, custom in-process MCP tools, hooks, subagents, compaction hooks, errors.
- Pi Agent Harness: package separation across `pi-ai`, `pi-agent-core`, `pi-coding-agent`, `pi-tui`, and chat automation; harness/product layers stay outside core.
- OpenTelemetry GenAI conventions: agent/model/tool spans, structured input/output/tool definition attributes, sensitivity and truncation cautions.

## Strands And Observability Emphasis

The user specifically called out Strands' flexible loop as a design inspiration and asked for observability from day one. The deliverable docs must therefore treat Strands as the deepest external comparison, with special focus on:

- Explicit loop phases: invocation, context assembly, model streaming, tool execution, recursive continuation, interruption, cancellation, retry, session sync, and final result.
- Hook surfaces that can inspect or mutate only specific fields, rather than giving every extension arbitrary control over the loop.
- Session managers as hook-driven persistence for messages, state, multi-agent state, and bidirectional agents.
- Streaming event shape for text, reasoning, citations, tool input deltas, tool results, stop reasons, usage, latency, and cancellation.
- Agent-as-tool and multi-agent graph/swarm events, especially node start/stop, handoff, wrapped child stream events, accumulated usage, and interruption propagation.
- OpenTelemetry spans and events for agent invocation, event-loop cycle, model invoke, tool call, messages, tool definitions, usage, metrics, and content sensitivity.
- Message and context provenance: every context item should be able to say where it came from, who injected it, who it is being sent to, which run/turn/span/message it belongs to, which policy allowed it, what sensitivity/retention it carries, and which child or parent agent caused it.

## Behavior Contract

### New Behavior

- Adds a navigable Markdown documentation set for the proposed future `agent_sdk`.
- Adds a future-SDK `coding-standards.md` based on Clawdia standards, external SDK study, TDD, DDD, and Rust-first performance constraints.
- Adds concise Clawdia use-case coverage, primitive map, external SDK comparison, observability/lineage design, architecture proposal, module layout, coverage/gap matrix, diagrams, conceptual Rust skeletons, testing strategy, performance/robustness check, risks, and later-phase doc arrangement.
- Adds or updates index/risk docs so future implementers can find the proposal and preserve key watchpoints.

### Preserved Behavior

- No current Clawdia runtime behavior changes.
- No current production crates, Rust source files, executable tests, fixtures, manifests, or package metadata changes.
- Existing runtime source-of-truth boundaries remain described as current-state constraints, not rewritten by this phase.

### Removed Behavior

- None.

### Validation

- Confirm changed/added files are Markdown-only.
- Confirm no `.rs`, test, fixture, package, or production runtime files changed.
- Run `git diff --check`.
- Run a focused content audit against the goal acceptance criteria.
- Run broader verification only if the final diff touches non-doc behavior; expected scope is docs-only.

Automated tests are not practical in this phase because the deliverable is a conceptual documentation set. The next phase should convert accepted API skeletons and behavior contracts into executable Rust tests and fixtures.

## Scope

In scope:

- Markdown documentation under `docs/architecture/`, one plan under `docs/reference/plans/`, one risk/watchpoint note under `docs/reference/risks/`, and index updates in `docs/README.md`.
- Conceptual Rust API skeletons inside Markdown fenced code blocks only.
- Mermaid diagrams inside Markdown only.
- Explicit caveats and tradeoffs.

Out of scope:

- Implementing `agent_sdk`.
- Creating Rust source files, executable tests, generated fixtures, or package manifests.
- Changing current ACP, next-gen, memory, approval, tool, extension, or runtime production behavior.
- Exhaustive final design.
- Cloning Strands, Cursor, Claude Agent SDK, Pi, or any provider-specific SDK shape.

## Proposed Doc Set

- `docs/start-here.md`: navigation and executive summary.
- `docs/architecture/coding-standards.md`: future SDK standards.
- `docs/host-adapters/clawdia/current-coverage.md`: concise use-case inventory.
- `docs/architecture/external-sdk-lessons.md`: external comparison and primitive lessons.
- `docs/architecture/primitive-map.md`: first-principles primitives and ownership.
- `docs/architecture/observability-and-lineage.md`: event, trace, message provenance, context injection, source/destination, and multi-agent handoff observability model.
- `docs/architecture/architecture-proposal.md`: middle-level architecture, flows, skeletons, diagrams, testing and migration strategy.
- `docs/architecture/coverage-gap-matrix.md`: acceptance coverage, gaps, phase-2 candidates.
- `docs/reference/risks/agent-sdk-phase1-2026-05-21.md`: future watchpoints.

## Workstreams

- Workstream A: local Clawdia coverage and current-boundary inventory.
- Workstream B: external SDK study and primitive extraction.
- Workstream C: future `agent_sdk` proposal, observability/lineage model, diagrams, Rust skeletons, and module layout.
- Workstream D: standards, risk notes, index updates, acceptance audit, and final verification.

## Verification Plan

- Independent plan review before writing the deliverable docs.
- Independent implementation/docs review after writing the deliverable docs.
- `git diff --check`.
- `git status --short`.
- Verify the runtime fallback watchpoint is represented: host-owned packaged extension SDK fallback, public subpath exports, resource/package drift, fallback ordering, browser dependency boundary, Node ESM caveat, and public-subpath smoke requirement.
- Verify headless/source-scoped approval is represented: broker-owned finite decisions, explicit dispatcher/custom handler, parked receiver semantics, exact YES/NO tokens, timeout denial, and host-owned escalation channels.
- Markdown-only guard:
  - changed files should be `.md` only.
  - no changed file under `crates/`, `apps/desktop/src-tauri/src/`, `packages/`, or executable fixture directories.
- Manual acceptance audit against `/Users/clawdia/goals/agent_sdk_phase1.md`.

## Risks And Rollback

- Risk: the proposal overfits current Clawdia internals instead of using them as coverage constraints. Mitigation: separate current coverage from future primitives and state "must not own" for important types.
- Risk: external SDK lessons are copied too directly. Mitigation: use a reusable-vs-incidental comparison table and phrase design choices as Rust-first first-principles decisions.
- Risk: doc set sprawls. Mitigation: keep a small set of linked docs with one main architecture proposal and one matrix.
- Risk: conceptual skeletons look like implementation. Mitigation: keep them fenced in Markdown and label them non-compiling sketches.
- Risk: acceptance criteria are missed because there are many required topics. Mitigation: maintain a coverage/gap matrix and final audit.

Rollback is documentation-only: remove the `docs/architecture/` folder, remove the plan and risk note, and revert the `docs/README.md` index entries.

## Risk/Gotcha Carry-Forward

- Keep `AgentExecutionService`-style execution dispatch as a host/application responsibility; the reusable SDK should expose adapter contracts, not own Clawdia UI/runtime selection.
- Keep runtime package snapshots/fingerprints deterministic and complete; tool schema export and executable tool registries must not drift.
- Keep approval as a broker/policy layer with finite decisions; raw event transport is not the approval API.
- Keep headless and source-scoped approval broker-owned and explicitly dispatched through host escalation or custom approval handlers. Headless approval parks the existing broker receiver, accepts only finite decision tokens such as exact YES/NO, denies on timeout, and never silently falls back to a desktop prompt path.
- Keep memory/context/compaction injectable and backend-owned; do not make UI or extension layers a shadow memory source.
- Keep app events bounded/ephemeral and tracing/Pawtrace-style stores durable; do not use a live event stream as analytics truth.
- Keep subagents parent-owned, depth-bounded, and recursion-disabled unless a later design explicitly adds budgets, approvals, trace shape, and recovery.
- Keep extension capability discovery explicit through provider flags/manifests; do not start arbitrary lifecycle extensions just to discover tools/hooks/subagents.
- Keep the extension SDK runtime fallback host-owned and predictable. Future SDK docs must call out public subpath export drift, resource/package drift, fallback ordering after extension-local paths, browser-safe subpath boundaries, Node ESM verification caveats, and temp-directory smoke tests for public subpaths.
- Keep external runtime adapters, especially ACP-like runtimes, behind explicit adapter contracts with restore/live-session/fingerprint boundaries.
- Keep message/context observability causal and lossless: context injection, memory recall, tool output, hook mutation, remote input, subagent instruction, and provider output should emit metadata that connects source, destination, run, turn, message, span, sensitivity, retention, and policy decision without leaking hidden content by default.

## Plan Review Status

Initial independent review found two blocking carry-forward gaps. The plan now includes extension SDK runtime fallback watchpoints, headless/source-scoped approval semantics, and the user's Strands/observability emphasis. Independent re-review returned PASS.

## Final Status

Deliverable docs drafted. Independent docs review returned PASS. Local validation passed: `git diff --check`, Markdown-only changed-file guard, no production/runtime/test path changes, and `./verify.sh` with 14 passed / 0 failed.
