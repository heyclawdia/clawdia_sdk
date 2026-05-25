# Standalone Agent SDK

This workspace is the authoritative planning and implementation packet for a new Rust-first Agent SDK. It is intentionally product-neutral: examples may describe host shapes such as desktop apps, CLIs, schedulers, remote channels, or external runtimes, but no product host is allowed to become SDK architecture.

## Repository Shape

This is a monorepo with separable Rust crates:

- `crates/agent-sdk-core`: the lightweight SDK kernel for users who only want agent primitives, records, runtime packages, ports, deterministic fakes, and conformance helpers.
- `crates/agent-sdk-toolkit`: an optional add-on crate for users who want concrete workspace tools, shell/resource helpers, discovery, and ACP/MCP protocol conformance scaffolding.

Keep this package boundary deliberate. New capabilities with heavy parser/runtime/provider dependencies should live in optional crates layered over `agent-sdk-core`, not as default core dependencies. Core must remain usable by hosts that only need the primitive SDK contracts.

The first public alpha release is available from crates.io:

```toml
[dependencies]
agent-sdk-core = "0.1.0-alpha.1"
agent-sdk-toolkit = { version = "0.1.0-alpha.1", optional = true }
```

Consumers can also depend on either crate from the repository:

```toml
[dependencies]
agent-sdk-core = { git = "https://github.com/heyclawdia/clawdia_sdk.git", package = "agent-sdk-core" }
agent-sdk-toolkit = { git = "https://github.com/heyclawdia/clawdia_sdk.git", package = "agent-sdk-toolkit", optional = true }
```

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
6. [Contract Workstreams](docs/workstreams/README.md): completed documentation-packet ownership rules and phase structure.
7. [Implementation Workstreams](docs/implementation-workstreams/README.md): Rust coding launch map, phase exit reports, and parallel-safe phase folders.
8. [Validation Gates](docs/workstreams/validation-gates.md): shared proof requirements for each workstream.
9. [Toolkit Adapter Roadmap](docs/agent-sdk-toolkit/README.md): optional live provider, OpenAI-compatible provider, ACP, MCP, isolation, browser/web, MLX, and llama.cpp adapter plan.
10. [Feature To Primitive Matrix](docs/reference/feature-to-primitive-matrix.md): how features layer over the shared kernel.
11. [Simplicity Audit](docs/reference/simplicity-audit.md): current simplification opportunities without losing features.
12. [Decision Register](docs/reference/open-questions-and-ambiguities.md): resolved decisions, deferred details, and non-questions for the first Rust slice.

## What Is Normative

| Area | Path | Authority |
| --- | --- | --- |
| Architecture posture | [docs/architecture](docs/architecture) | SDK design direction, primitive kernel, feature layers, and conceptual skeletons |
| Implementation contracts | [docs/contracts](docs/contracts/README.md) | Normative contract packet |
| Contract workstream ownership and validation | [docs/workstreams](docs/workstreams/README.md) | Completed documentation-packet phase sequencing, owner roles, write boundaries, and validation gates |
| Implementation workstream launch map | [docs/implementation-workstreams](docs/implementation-workstreams/README.md) | Rust coding phases, parallel launch targets, phase dependencies, and implementation exit gates |
| Optional adapter/toolkit roadmap | [docs/agent-sdk-toolkit](docs/agent-sdk-toolkit/README.md) | Live provider, OpenAI-compatible provider, ACP, MCP, isolation runtime, browser/web access, MLX, and llama.cpp adapter planning |
| Standards and review | [coding_standards.md](coding_standards.md), [docs/reference/sdk-review-checklist.md](docs/reference/sdk-review-checklist.md) | Coding posture and SDK review rubric |
| Simplicity audit | [docs/reference/simplicity-audit.md](docs/reference/simplicity-audit.md) | Simplification guidance that preserves capability |
| Scenario coverage | [docs/examples](docs/examples/README.md) | Generic host workflows and boundary examples, not SDK core |

## Parallelization Rule

Every non-README launch file inside the current numbered implementation phase folder is parallel-safe with its siblings by contract. Single-target phases serialize naturally. The stitching role owns the primitive kernel, shared names, IDs, event and journal alignment, package fingerprints, public indices, phase structure, and final validation.

Agents working in parallel should only edit the files listed in their launch doc and owner role doc. Cross-cutting changes go through the stitching owner; scenario and non-stitching work should record proposals in their handoff unless their writable list explicitly includes [docs/reference/cross-cutting-proposals.md](docs/reference/cross-cutting-proposals.md).

For contract-packet review, use [docs/workstreams](docs/workstreams/README.md). For Rust implementation, launch from [docs/implementation-workstreams](docs/implementation-workstreams/README.md). Run numbered phase folders in order; every launch file inside the current numbered folder can run in parallel.

## Current Implementation Posture

The documentation contract packet has exited final review, and the first Rust implementation handoff now lives under `crates/agent-sdk-core` and `crates/agent-sdk-toolkit`. The implementation history and release-readiness evidence live in [docs/implementation-workstreams](docs/implementation-workstreams/README.md).

This alpha release includes deterministic fake/test-kit support and optional toolkit helpers, but it does not claim live provider, concrete container/runtime, product UI, remote channel, network telemetry exporter, marketplace, workflow-engine, or host-adapter support.
