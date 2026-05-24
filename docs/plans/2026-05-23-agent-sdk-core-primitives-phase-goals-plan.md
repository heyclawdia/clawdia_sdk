# Agent SDK Core Primitives And Phased Goals Plan

> Planning note only. Current implementation authority lives in [../contracts/README.md](../contracts/README.md), [../architecture/primitive-map.md](../architecture/primitive-map.md), and [../workstreams/README.md](../workstreams/README.md). This plan hardens those docs before any Rust code exists.

Date: 2026-05-23

## Objective

Harden the SDK packet so future agents can first build a tight primitive core, then run phase-scoped parallel goals that layer features on top without each workstream inventing a mini SDK.

The user specifically called out context as an example: memory, tool results, skills, and host data can all contribute to provider context, but they must keep provenance, policy, privacy, and observability. This plan treats that as a stress test for primitive extraction, not as a rule that every SDK concept is context.

This pass is owned by the **Integration And Stitching role**. It may update shared indices, phase launch docs, primitive-map guidance, runtime-package/capability ownership, and narrow cross-contract primitive alignment. Detailed contract work that belongs to another role must be captured as a directly runnable phase goal unless the change is only a stitching reconciliation.

## Relevant Existing Context

- [../../AGENTS.md](../../AGENTS.md): documentation-only rules, no branches, exact workstream ownership, and primitive-layering discipline.
- [../../coding_standards.md](../../coding_standards.md): product-neutral core, canonical lowering, observability/journal/privacy in every slice.
- [../architecture/primitive-map.md](../architecture/primitive-map.md): current MVP primitive kernel and feature-layer map.
- [../contracts/context-memory-contract.md](../contracts/context-memory-contract.md): current context/memory contract with `ContextItem`, `ContextProjection`, `MemoryPort`, and provenance requirements.
- [../workstreams/README.md](../workstreams/README.md): current parallel workstream ownership matrix and Codex goal prompt template.
- [../workstreams/validation-gates.md](../workstreams/validation-gates.md): current validation and review packet rules.
- [../architecture/external-sdk-lessons.md](../architecture/external-sdk-lessons.md): existing external source lessons from Strands, Cursor, Claude, Pi, oh-my-pi, Apple Containerization, and OTel.
- Official references refreshed for this pass:
  - Google ADK context/state docs: context as operation-scoped background and state mutation through callback/tool contexts.
  - Strands plugins docs: plugins use low-level agent primitives; context offloading turns oversized tool results into previews plus retrieval.
  - Cursor SDK release notes: agent/run split, run-scoped streaming, reconnect, terminal states.
  - Pi docs/package catalog: small core extended by extensions, skills, prompt templates, themes, and packages.
  - Pi multiagent package docs: child agents do not inherit ambient context by default; parent remains lead and child output is evidence.
  - Claude Agent SDK loop docs: message lifecycle, tool execution, and context window are core loop concepts; tool schemas can be deferred.
  - OpenAI Agents SDK docs: SDK path fits when the application owns orchestration, tool execution, approvals, and state.

## Behavior Contract

New behavior:

- The primitive docs distinguish kernel primitives, feature-layer primitives, adapter ports, and host-owned products through a small "primitive decision ladder."
- Context contribution becomes a typed feature-layer example with source/provenance (`memory`, `tool_result`, `skill`, `file`, `host`, `remote_channel`, `subagent`, `compaction`, etc.), but provider context remains one projection path rather than a universal SDK abstraction.
- Workstreams gain a phase-first folder structure so the user can run numbered phases in order and run all goals inside a phase in parallel:
  - `docs/workstreams/README.md`
  - `docs/workstreams/00-bootstrap/`
  - `docs/workstreams/01-package-capabilities/`
  - `docs/workstreams/02-primitive-kernel/`
  - `docs/workstreams/03-kernel-review/`
  - `docs/workstreams/04-side-effects-policy/`
  - `docs/workstreams/05-feature-layers/`
  - `docs/workstreams/06-scenario-coverage/`
  - `docs/workstreams/07-final-review/`
  - `docs/workstreams/_roles/`
  - `docs/workstreams/_templates/`
- Each numbered phase folder is a dependency boundary. Every goal file directly inside that folder is parallel-safe with its siblings.
- Owner role docs under `_roles/` remain writable-scope authority but are not launch targets.
- A feature-to-primitive matrix becomes a required Phase 00 artifact: every feature maps to reused kernel primitives, any feature-layer primitive, owner role, events, journal records, validation, and host-owned boundary.
- External SDK lessons become auditable: each active source entry names source URL, date checked, accepted primitive lesson, rejected product-specific behavior, and resulting SDK decision.
- External SDK lessons are refreshed around primitives, not copied as features.

Preserved behavior:

- Existing feature envelope stays intact: memory, tools, skills/extensions, stream rules, isolation, subagents, telemetry, output delivery, events, journals, and host scenarios.
- Existing owner role docs remain authoritative for writable files.
- Non-stitching agents still use handoff proposal blocks for cross-cutting changes.
- No Rust source, manifests, executable tests, or fixtures are created in this documentation-only pass.

Removed behavior:

- No docs should imply that all SDK concepts collapse into context.
- No phase/workstream should imply it can edit shared architecture/reference/contract files outside its ownership.
- No external SDK comparison should be treated as API authority without mapping it to our primitive kernel.

## Phase Workstreams

1. **Primitive model hardening**
   - Update `primitive-map.md`, `context-memory-contract.md`, and the decision register with a primitive decision ladder and context-contribution provenance stress test.
   - Update ownership so `runtime-package-schema.md` has an explicit owner for the central capability/package contract.

2. **Phase/goal launch structure**
   - Add numbered folders directly under `docs/workstreams/` so future agents can target phase goals directly.
   - Update the workstream README and validation gates to point at those phase folders.

3. **External source lesson refresh**
   - Update external SDK lessons and review matrix so Strands, Cursor, Pi, Claude, Google ADK, and OpenAI Agents SDK map to concrete primitive decisions.

4. **Independent review and audits**
   - Fold in findings from the spawned primitive/API/phase/external reviewers.
   - Run link, ownership, phase-gate, product-neutrality, and no-code audits.

## Validation Plan

- Markdown link audit over `/Users/clawdia/clawdia_sdk`.
- Workstream ownership audit for duplicated writable files and future implementation scope collisions.
- Phase/goals audit:
  - every phase folder has a `README.md`;
  - every numbered phase folder has a `README.md`;
  - every goal doc names phase, owner role, writable docs, read-only inputs, whether it can run in parallel, and validation evidence;
  - every goal in a numbered folder is parallel-safe with its siblings;
  - later numbered phases wait for the previous phase exit gate.
- Phase 01 package-capability and Phase 02 primitive-kernel exit checklist:
  - every feature in the active packet maps to `kernel primitive`, `feature-layer primitive`, `optional adapter`, or `host-owned`;
  - the MVP slice remains one fake-provider text or typed run;
  - every new primitive has passed the primitive decision ladder;
  - every `CapabilitySpec` variant has a typed sidecar owner or is marked reserved;
  - runtime-package fingerprint inputs for MVP are stable and reserved feature inputs are gated;
  - every proposal block has an owner and phase;
  - no duplicate writable files or future code scopes exist;
  - link, product-neutrality, and no-code audits pass.
- Primitive audit:
  - docs define kernel, feature layer, adapter port, and host-owned categories;
  - context contribution provenance covers memory, tool results, skills, files, host context, remote channels, subagents, and compaction without making all behavior context;
  - every new feature still maps to an existing primitive or a proposal path.
- Product-neutrality grep for stale host/product/local terms in active architecture/contracts/workstreams.
- No-code audit proving no Rust source, manifests, executable tests, or fixtures were created.
- Independent plan review before editing beyond this plan.
- Independent implementation review after edits.

## Risk / Gotcha Carry-Forward

- If context contribution becomes too broad, it will swallow tools, memory, skills, and events into a vague "context bag." Keep it scoped to provider projection and provenance.
- If phase folders become another source of truth, future agents will not know whether to trust phase docs or workstream docs. Phase docs must orchestrate; workstream docs remain write-scope authority.
- If external SDK lessons are copied directly, the SDK will inherit product-specific assumptions. Every lesson must map to a product-neutral primitive or be rejected.
- If Phase 01 package/capability decisions and Phase 02 kernel decisions do not have hard exit gates, later parallel goals will build on unstable names and duplicate primitives.
- If context/provenance metadata is stored only in event payloads, live filtering and durable replay become slow and inconsistent. Keep source/provenance in typed refs, indexed envelope fields, and journal records where relevant.
