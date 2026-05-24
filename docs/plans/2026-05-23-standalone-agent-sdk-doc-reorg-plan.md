# Standalone Agent SDK Doc Reorg Plan

> Historical plan only. This file describes an earlier reorg and may contain stale product-specific paths or superseded workstream names. Do not use it as implementation authority; start from [../start-here.md](../start-here.md), [../contracts/README.md](../contracts/README.md), and [../workstreams/README.md](../workstreams/README.md).

Date: 2026-05-23

## Objective

Move the Agent SDK architecture packet out of the Clawdia application checkout and make `/Users/clawdia/clawdia_sdk` the authoritative documentation workspace for the new Rust-first Agent SDK.

This SDK is a new standalone design. Clawdia is only a host-adapter/use-case reference that proves coverage. Clawdia product behavior must not become SDK-owned behavior.

## Relevant Existing Context

- `/Users/clawdia/goals/agent_sdk_phase1.md`: Phase 1 is Markdown-only, Rust-first, product-neutral, and explicitly says current Clawdia behavior is a coverage constraint rather than the architecture to copy.
- `/Volumes/Clawdia/docs/architecture/agent-sdk/architecture-proposal.md`: the current middle-level proposal for the new standalone `agent_sdk`; this is the main source document to preserve and reorganize.
- `/Volumes/Clawdia/docs/architecture/agent-sdk/coding-standards.md`: requires product-neutral SDK boundaries, DDD, typed snapshots, explicit state machines, event/journal contracts, test-first implementation, and host-owned Clawdia decisions.
- `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/README.md`: contracts are normative for implementation and already require complete examples, SDK/host boundary notes, and ergonomic helper lowering.
- `/Volumes/Clawdia/docs/potential_problems/agent-sdk-phase1-2026-05-21.md`: watchpoints include overfitting to Clawdia internals, event schema sprawl, host-boundary erosion, Node ESM assumptions, tool packs becoming a product harness, and containerization hard-dependency drift.
- Prior memory notes confirm the user preference not to create branches and to use the coding-orchestrator workflow for complex Clawdia-adjacent work.

## Behavior Contract

New behavior:

- `/Users/clawdia/clawdia_sdk` exists outside the Clawdia checkout and contains the consolidated Agent SDK docs.
- The root README provides a core SDK diagram, reading path, and parallel implementation map.
- Docs are organized so different agents can work concurrently on independent contracts/workstreams.
- One clearly named integration/stitching workstream owns cross-contract consistency and must not be parallelized away.
- Clawdia-specific material is moved into a host-reference area and described as adapter coverage, not SDK ownership.
- A dedicated uncertainty register lists what is still not clear before coding.
- The old Clawdia doc location is replaced with a lightweight pointer/stub so future agents do not keep editing stale in-repo copies.

Preserved behavior:

- Existing architecture, contract, example, risk, and plan content remains available.
- Mermaid diagrams and conceptual Rust sketches remain Markdown-only.
- Existing host-boundary guidance stays intact.
- No Rust source, executable tests, fixtures, manifests, or runtime behavior are created.

Removed behavior:

- The full Agent SDK design packet no longer lives inside `/Users/clawdia/clawdia/docs/architecture/agent-sdk`.
- The Clawdia docs index no longer presents the Clawdia checkout as the authoritative home for this SDK packet.

Validation:

- File inventory proves the standalone workspace contains the migrated architecture/contracts/examples/plans/risk references.
- A stale-copy allowlist proves `/Users/clawdia/clawdia/docs/architecture/agent-sdk` and `/Volumes/Clawdia/docs/architecture/agent-sdk` contain only pointer/stub material after migration:
  - allowed files: `README.md` only;
  - allowed directories: none except empty directories created by the filesystem during the move;
  - allowed text in the stub: pointer to `/Users/clawdia/clawdia_sdk`, source-of-truth warning, and migration date;
  - disallowed text in the stub: normative contract tables, `## Complete Example`, `pub struct`, `pub enum`, and old navigation to local contract/example files.
- The Clawdia `docs/README.md` index must link to `/Users/clawdia/clawdia_sdk/README.md` as the external authoritative workspace and must not present `docs/architecture/agent-sdk/` as the authoritative packet.
- Link audit over `/Users/clawdia/clawdia_sdk` finds no broken relative Markdown links for migrated docs.
- Structural audit proves each workstream document names owner, inputs, outputs, disjoint write scope, dependencies, validation, and integration handoff.
- `git diff --check` passes in `/Users/clawdia/clawdia` for the pointer/index changes.

## Target Folder Layout

```text
/Users/clawdia/clawdia_sdk/
  AGENTS.md
  README.md
  docs/
    start-here.md
    architecture/
      architecture-proposal.md
      primitive-map.md
      observability-and-lineage.md
      external-sdk-lessons.md
      coding-standards.md
      coverage-gap-matrix.md
    contracts/
      README.md
      api-contracts.md
      event-schema.md
      run-handle-reconnect-contract.md
      loop-state-machine.md
      runtime-package-schema.md
      journal-replay-schema.md
      tool-approval-contract.md
      structured-output-contract.md
      stream-rule-contract.md
      tool-pack-contract.md
      isolation-runtime-contract.md
      subagent-contract.md
      extension-sdk-contract.md
      otel-mapping-contract.md
      telemetry-privacy-contract.md
      review-matrix.md
    examples/
      README.md
      ...
    host-adapters/
      clawdia/
        README.md
        current-coverage.md
        flow-examples.md
        host-integration-map.md
    workstreams/
      README.md
      00-integration-stitching.md
      01-core-api-runtime.md
      02-events-journal-replay.md
      03-context-structured-output.md
      04-tools-approval-toolpacks.md
      05-streaming-realtime-rules.md
      06-isolation-execution.md
      07-subagents-coordination.md
      08-extension-sdk-packaging.md
      09-telemetry-privacy-cost.md
      10-host-adapter-coverage.md
    reference/
      plans/
      risks/
      notes/
      source-migration-map.md
      open-questions-and-ambiguities.md
      cross-cutting-proposals.md
```

## Source-To-Target Migration Map

Every source file must be moved to one of these targets or explicitly excluded. No source file is intentionally excluded.

Architecture:

| Source | Target |
| --- | --- |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/README.md` | `/Users/clawdia/clawdia_sdk/docs/start-here.md` plus summarized root `/Users/clawdia/clawdia_sdk/README.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/architecture-proposal.md` | `/Users/clawdia/clawdia_sdk/docs/architecture/architecture-proposal.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/coding-standards.md` | `/Users/clawdia/clawdia_sdk/docs/architecture/coding-standards.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/primitive-map.md` | `/Users/clawdia/clawdia_sdk/docs/architecture/primitive-map.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/observability-and-lineage.md` | `/Users/clawdia/clawdia_sdk/docs/architecture/observability-and-lineage.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/external-sdk-lessons.md` | `/Users/clawdia/clawdia_sdk/docs/architecture/external-sdk-lessons.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/coverage-gap-matrix.md` | `/Users/clawdia/clawdia_sdk/docs/architecture/coverage-gap-matrix.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/current-clawdia-coverage.md` | `/Users/clawdia/clawdia_sdk/docs/host-adapters/clawdia/current-coverage.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/clawdia-flow-examples.md` | `/Users/clawdia/clawdia_sdk/docs/host-adapters/clawdia/flow-examples.md` |

Contracts:

| Source | Target |
| --- | --- |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/README.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/README.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/api-contracts.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/api-contracts.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/event-schema.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/event-schema.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/run-handle-reconnect-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/run-handle-reconnect-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/loop-state-machine.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/loop-state-machine.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/runtime-package-schema.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/runtime-package-schema.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/journal-replay-schema.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/journal-replay-schema.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/tool-approval-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/tool-approval-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/structured-output-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/structured-output-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/stream-rule-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/stream-rule-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/tool-pack-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/tool-pack-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/isolation-runtime-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/isolation-runtime-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/subagent-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/subagent-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/extension-sdk-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/extension-sdk-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/otel-mapping-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/otel-mapping-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/telemetry-privacy-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/telemetry-privacy-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/review-matrix.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/review-matrix.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/clawdia-host-integration-map.md` | `/Users/clawdia/clawdia_sdk/docs/host-adapters/clawdia/host-integration-map.md` |

Examples and notes:

| Source | Target |
| --- | --- |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/examples/README.md` | `/Users/clawdia/clawdia_sdk/docs/examples/README.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/examples/*.md` | `/Users/clawdia/clawdia_sdk/docs/examples/*.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/notes/agent-runtime-diagram.md` | `/Users/clawdia/clawdia_sdk/docs/reference/notes/agent-runtime-diagram.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/notes/agent-runtime-example.md` | `/Users/clawdia/clawdia_sdk/docs/reference/notes/agent-runtime-example.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/notes/state-machine-example.md` | `/Users/clawdia/clawdia_sdk/docs/reference/notes/state-machine-example.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/notes/agent-start-flow.excalidraw` | `/Users/clawdia/clawdia_sdk/docs/reference/notes/agent-start-flow.excalidraw` |

Plans and risks:

| Source | Target |
| --- | --- |
| `/Volumes/Clawdia/docs/plans/*agent-sdk*.md` | `/Users/clawdia/clawdia_sdk/docs/reference/plans/*.md` |
| `/Volumes/Clawdia/docs/potential_problems/agent-sdk-phase1-2026-05-21.md` | `/Users/clawdia/clawdia_sdk/docs/reference/risks/agent-sdk-phase1-2026-05-21.md` |

Clawdia checkout stubs:

| Source | Target |
| --- | --- |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/README.md` after move | pointer-only stub to `/Users/clawdia/clawdia_sdk/README.md` |
| `/Volumes/Clawdia/docs/README.md` Agent SDK rows | external pointer rows only; no local authoritative packet links |

## Parallelization Model

Most implementation agents can work in parallel by owning one workstream document and its referenced contract files. The only serialized role is `00-integration-stitching.md`, because it owns cross-contract naming, IDs, event/journal alignment, package fingerprints, and public API coherence.

Parallel agents must not edit the same contract at the same time. If a workstream needs a cross-cutting rename or event-shape change, it records a proposal for the stitching agent instead of directly changing every file.

`docs/workstreams/README.md` must include a parallel ownership matrix with these fields for each agent/workstream:

- owner role;
- writable files;
- read-only dependencies;
- cross-cutting proposals file;
- integration/stitching dependency;
- validation evidence required before handoff.

The stitching owner has final authority over shared indices, public naming, ID taxonomy, event and journal schema alignment, runtime package fingerprint inputs, cross-contract link paths, and final whole-packet validation. Other workstreams propose cross-cutting changes in `docs/reference/cross-cutting-proposals.md` instead of editing shared files directly.

## Required Host-Material Demotion

Clawdia material must be useful but not normative for SDK core.

- `clawdia-host-integration-map.md` moves from `docs/contracts/` to `docs/host-adapters/clawdia/`.
- `current-clawdia-coverage.md` and `clawdia-flow-examples.md` move to `docs/host-adapters/clawdia/`.
- `docs/contracts/README.md` must remove `clawdia-host-integration-map.md` from the normative contract table and link it only as host-adapter reference.
- `docs/contracts/review-matrix.md` must separate SDK contract rows from host-adapter coverage rows.
- Phase 2 handoff and follow-up plan copies under `docs/reference/plans/` must point to the new standalone paths and describe Clawdia host docs as coverage/adapters, not implementation authority.
- Ergonomics and contract-example references must keep SDK contracts as canonical and treat Clawdia examples as scenario tests or adapter coverage.

## Workstreams

1. Bootstrap standalone workspace and migration map.
2. Copy/move architecture, contracts, examples, plans, and risk notes into the new layout.
3. Write root README, `start-here`, core diagram, and AGENTS guidance.
4. Create per-workstream handoff docs for maximum parallelization.
5. Create uncertainty register for things still not clear before coding.
6. Replace old Clawdia in-repo packet with an external pointer and update the Clawdia docs index.
7. Validate inventories, links, structural workstream completeness, and Clawdia diff whitespace.

## Risk / Gotcha Carry-Forward

- Do not reframe the new Agent SDK as an old Clawdia SDK. The user clarified that `architecture-proposal.md` is for the new standalone Agent SDK.
- Do not lose Clawdia examples. Move them into host-adapter reference material so they remain useful without owning SDK architecture.
- Do not parallelize the integration/stitching role. Cross-file consistency needs one accountable owner.
- Do not leave two authoritative doc copies behind. Stale Clawdia copies would confuse future agents.
- Do not create code or executable tests in this pass.

## Review Status

Independent plan review passed after adding the source-to-target migration map, Clawdia host demotion rules, stale-copy validation, and parallel ownership matrix. Implementation review later found overlapping workstream write sets; the packet now assigns shared architecture and reference docs only to the serialized stitching owner.
