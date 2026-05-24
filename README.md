# Standalone Agent SDK

This workspace is the authoritative planning packet for a new Rust-first Agent SDK. It is intentionally product-neutral: examples may describe host shapes such as desktop apps, CLIs, schedulers, remote channels, or external runtimes, but no product host is allowed to become SDK architecture.

## Core Map

```mermaid
flowchart TD
  Host["Host products and adapters<br/>desktop, CLI, schedulers, remote channels"] --> API["agent_sdk public API<br/>Agent, AgentRuntime, RunHandle"]
  API --> Kernel["Primitive kernel<br/>runs, package, content refs, context projection, effects, events, journal, ports"]
  Kernel --> Loop["Agent loop state machine<br/>context, provider, tools, approval, recovery"]
  Loop --> Domain["Feature layers<br/>tools, memory, streaming, isolation, subagents, extensions"]
  Domain --> Package["RuntimePackage snapshot<br/>provider projection and executable registry"]
  Domain --> Journal["RunJournal and checkpoints<br/>resume, replay, anti-entropy"]
  Domain --> Obs["AgentEventBus and telemetry<br/>subscriptions, privacy, OTel, usage, cost"]
  Loop --> Ports["Ports<br/>providers, realtime, tools, memory, isolation, extensions, subagents"]
  Ports --> Adapters["Host-provided adapters<br/>model APIs, MCP, containers, stores, app events"]
  Host --> Examples["Generic scenario examples<br/>host-owned surfaces stay outside core"]
```

## First Reading Path

1. [Start Here](docs/start-here.md): posture, thesis, non-goals, and navigation.
2. [Coding Standards](coding_standards.md): root standards entry point and required validation posture.
3. [Architecture Proposal](docs/architecture/architecture-proposal.md): module layout, state machine, flows, and conceptual Rust skeletons.
4. [Primitive Map](docs/architecture/primitive-map.md): ownership, responsibilities, decision ladder, and must-not-own boundaries.
5. [Contracts](docs/contracts/README.md): normative implementation contracts.
6. [Workstreams](docs/workstreams/README.md): ownership rules and phase-goal launch structure for parallel agents.
7. [Validation Gates](docs/workstreams/validation-gates.md): shared proof requirements for each workstream.
8. [Feature To Primitive Matrix](docs/reference/feature-to-primitive-matrix.md): how features layer over the shared kernel.
9. [Simplicity Audit](docs/reference/simplicity-audit.md): current simplification opportunities without losing features.
10. [Decision Register](docs/reference/open-questions-and-ambiguities.md): resolved decisions, deferred details, and non-questions for the first Rust slice.

## What Is Normative

| Area | Path | Authority |
| --- | --- | --- |
| Architecture posture | [docs/architecture](docs/architecture) | SDK design direction, primitive kernel, feature layers, and conceptual skeletons |
| Implementation contracts | [docs/contracts](docs/contracts/README.md) | Normative Phase 2 contract packet |
| Workstream ownership and validation | [docs/workstreams](docs/workstreams/README.md) | Phase sequencing, parallel goal launch docs, owner roles, write boundaries, and validation gates |
| Standards and review | [coding_standards.md](coding_standards.md), [docs/reference/sdk-review-checklist.md](docs/reference/sdk-review-checklist.md) | Coding posture and SDK review rubric |
| Simplicity audit | [docs/reference/simplicity-audit.md](docs/reference/simplicity-audit.md) | Simplification guidance that preserves capability |
| Scenario coverage | [docs/examples](docs/examples/README.md) | Generic host workflows and boundary examples, not SDK core |
| Historical context | [docs/reference](docs/reference/source-migration-map.md) | Plans, risks, notes, and migration audit |

## Parallelization Rule

Every non-README goal file inside the current numbered phase folder is parallel-safe with its siblings by contract. Single-goal phases serialize naturally. The stitching role owns the primitive kernel, shared names, IDs, event and journal alignment, package fingerprints, public indices, phase-goal structure, and final validation.

Agents working in parallel should only edit the files listed in their goal doc and owner role doc. Cross-cutting changes go through the stitching owner; scenario and non-stitching goals should record proposals in their handoff unless their writable list explicitly includes [docs/reference/cross-cutting-proposals.md](docs/reference/cross-cutting-proposals.md).

Launch Codex goals from [docs/workstreams](docs/workstreams/README.md). Run numbered phase folders in order; every goal file inside the current numbered folder can run in parallel.

## Current Implementation Posture

This is still documentation and handoff design. Do not create SDK code until the primitive kernel, contracts, workstreams, validation gates, and review criteria have been reviewed and accepted.
