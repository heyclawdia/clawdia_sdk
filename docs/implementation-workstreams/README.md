# Agent SDK Implementation Workstreams

Use this folder to launch, audit, and review phase-gated Rust implementation work after the contract packet exits through [Phase 07](../workstreams/07-final-review/README.md).

The rule is strict: **run phases in numeric order, and run every launch target inside the current numbered phase folder in parallel**. If one item needs another item's output, it belongs in a later phase. Sibling launch targets are parallel-safe by contract.

Launch targets use short titles such as `typed-ids`, `event-frames`, or `text-run`; they are not generic numbered labels.

## Launch Order

| Phase | Run Pattern | Purpose |
| --- | --- | --- |
| [00 Crate Foundation](00-crate-foundation/README.md) | one target | Create the Rust workspace, crate skeletons, CI/test harness, and package boundaries. |
| [01 Shared Kernel](01-shared-kernel/README.md) | all targets in parallel | Implement typed IDs, refs, errors, policy enums, fakes, and fixture harnesses used by every later phase. |
| [02 Core Records](02-core-records/README.md) | all targets in parallel | Build package, event, journal, content/context, and provider-port records over the shared kernel. |
| [03 Run Control](03-run-control/README.md) | all targets in parallel | Implement runtime ownership, loop state transitions, and reconnectable run handles. |
| [04 P0 Text Run](04-p0-text-run/README.md) | one target | Integrate the first fake-provider text run through package, context, provider, events, and journal. |
| [05 Agent Pool Coordination](05-agent-pool-coordination/README.md) | one target | Add generic agent-run coordination, run messages, delivery receipts, and wake conditions without workflow-engine behavior. |
| [06 Output Contract](06-output-contract/README.md) | one target | Add output contracts, schema refs, helper lowering, and package fingerprint normalization. |
| [07 P1 Validation Result](07-p1-validation-result/README.md) | all targets in parallel | Add local validation/repair and typed result records over the output contract. |
| [08 P1 Typed Run](08-p1-typed-run/README.md) | one target | Integrate P1 typed output over the P0 loop and prove typed result publication. |
| [09 P2 Side Effects](09-p2-side-effects/README.md) | all targets in parallel | Add approval, tool execution, output delivery, hooks, and core telemetry over the shared effect spine. |
| [10 Feature Ports](10-feature-ports/README.md) | all targets in parallel | Add reserved feature-layer ports and optional crates without making them P0/P1 requirements. |
| [11 Replay Hardening](11-replay-hardening/README.md) | all targets in parallel | Fill golden fixtures, replay/recovery coverage, performance, and privacy hardening. |
| [12 Scenario Verification](12-scenario-verification/README.md) | all targets in parallel | Prove generic scenarios and public API readiness after hardening. |
| [13 Release Readiness](13-release-readiness/README.md) | one target | Run final packaging, feature flag, docs, verification-matrix, and release-handoff checks. |
| [14 Evaluation Metrics](14-evaluation-metrics/README.md) | one target | Add optional post-hoc evaluation metrics and comparison helpers over released trace/journal primitives. |

Do not start a later phase until the previous phase README exit gate is checked and the phase exit report records reviewer PASS.

## Testing And Parallelism Strategy

The phase graph is shaped around test seams:

- Phase 00 creates the workspace, cargo commands, fixture layout, and optional-crate boundaries.
- Phase 01 creates deterministic fakes and shared types before any domain service depends on them.
- Phase 02 splits independent durable record families so package, event, journal, context, and provider DTOs can each get their own fixtures.
- Phase 03 keeps runtime control independent from the first complete run so state-machine and reconnect tests can fail locally.
- Phase 04 is the first integration gate: P0 must pass one fake-provider text run before typed output or side effects start.
- Phase 05 adds the generic `AgentPool` feature layer after the P0 loop exists, so later subagent work can reuse run messages and wake conditions instead of inventing a private mailbox.
- Phase 05 is an intentional full-packet sequencing gate, not a P0/P1 profile
  requirement. A strict minimal MVP can still prove P0/P1 without agent pools;
  this launch map places the pool after P0 so its IDs, events, journal records,
  and wake semantics are settled before later subagent and workflow-facing
  feature work depends on them.
- Phase 06 freezes output-contract DTOs and helper lowering before validators or typed results depend on them.
- Phase 07 lets validation/repair and typed-result record work run in parallel because both depend only on the Phase 06 contract, not each other.
- Phase 08 is the P1 integration gate: typed output must pass over the P0 loop before side effects start.
- Phase 09 proves P2 side effects with policy matrices and intent-before-effect journal tests.
- Phase 10 adds optional feature ports only after P2, keeping streaming, isolation, subagents, extensions, and tool packs out of the minimal core profiles.
- Phase 11 exists specifically to close cross-cutting fixture, replay, privacy, and performance gaps before release scenarios.
- Phase 12 runs scenario and API verification in parallel after the hardening phase has made the evidence stable.
- Phase 13 is a final serialized release-readiness stitching phase that consumes all earlier verification evidence.
- Phase 14 adds post-release evaluation metrics as an optional layer over the released trace and journal primitives.

If a future implementer finds a hidden dependency between two sibling launch targets, do not coordinate through shared mutable work. Move the dependent work into the next numbered phase and update this launch map.

## Phase Graph

```mermaid
flowchart TD
  P00["00 Crate Foundation"] --> P01["01 Shared Kernel<br/>parallel"]
  P01 --> P02["02 Core Records<br/>parallel"]
  P02 --> P03["03 Run Control<br/>parallel"]
  P03 --> P04["04 P0 Text Run"]
  P04 --> P05["05 Agent Pool Coordination"]
  P05 --> P06["06 Output Contract"]
  P06 --> P07["07 P1 Validation Result<br/>parallel"]
  P07 --> P08["08 P1 Typed Run"]
  P08 --> P09["09 P2 Side Effects<br/>parallel"]
  P09 --> P10["10 Feature Ports<br/>parallel"]
  P10 --> P11["11 Replay Hardening<br/>parallel"]
  P11 --> P12["12 Scenario Verification<br/>parallel"]
  P12 --> P13["13 Release Readiness"]
  P13 --> P14["14 Evaluation Metrics"]
```

## Launch Protocol

For a phase folder, launch one Codex run per non-README markdown file directly inside that folder. Point each run at one launch target:

```text
/goal Work in <repo-root> using the exact launch file path, for example docs/implementation-workstreams/01-shared-kernel/01a-typed-ids.md, as the launch doc.
Read README.md, docs/start-here.md, coding_standards.md, docs/implementation-workstreams/README.md, docs/workstreams/validation-gates.md, docs/reference/sdk-review-checklist.md, docs/architecture/primitive-map.md, the phase README, the launch doc, and all named contract inputs.
Do not create a branch.
Edit only the implementation surfaces named in the launch doc. If a named path does not exist yet, create it only when that launch doc owns it.
Preserve the primitive kernel: layer features over Agent/RunRequest/RuntimePackage/AgentEvent/RunJournal/PolicyRef/SourceRef/DestinationRef/ContentRef/EffectIntent/typed ports instead of inventing parallel concepts.
Finish with tests/fixtures, commands, primitive-lowering evidence, host-owned boundary evidence, and a review packet using docs/workstreams/validation-gates.md.
```

## Phase Exit Protocol

Each phase must leave a reviewable packet:

1. Every launch target finishes with the required handoff from [validation-gates.md](../workstreams/validation-gates.md).
2. The integration owner creates `docs/implementation-workstreams/<NN-phase>/_phase/phase-exit-report.md`.
3. Phase-level tests and audits pass, including `cargo fmt --check`, relevant `cargo test` commands, golden fixtures, scenario tests where named, link/path audits, product-neutrality checks, and no-mini-SDK checks.
4. A dedicated reviewer returns PASS or blocking findings.
5. The phase README exit gate is checked only after the reviewer PASS is recorded.

## Folder Contract

- Numbered folders are dependency phases.
- Non-README markdown files directly inside a numbered folder are launch targets.
- Launch targets in the same numbered folder are parallel-safe.
- A target that depends on a sibling's output belongs in the next phase.
- `_phase/` folders are for phase execution plans and exit reports.
- Do not add product-specific host adapters to this implementation plan unless the user explicitly asks for a separate external task.

## Required Proof

Implementation phases must produce real code evidence, not prose-only confidence:

- compile and public export checks for touched crates;
- deterministic fake-adapter unit tests;
- golden fixtures for events, journals, package fingerprints, OTel projections, and extension protocols;
- property/table tests for reducers, fingerprints, filters, validators, and policy matrices;
- smoke tests for optional crates and packaging boundaries;
- scenario tests for generic workflows; and
- docs audits for links, ownership, product-neutrality, primitive layering, and host-owned boundaries.

The first implementation slice must prove P0 before P1, and P1 before P2.
