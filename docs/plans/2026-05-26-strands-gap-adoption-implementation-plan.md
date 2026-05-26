# Strands Gap Adoption Implementation Plan

## Objective

Turn the Strands gap report into staged SDK work that keeps `agent-sdk-core` as the primitive kernel, places higher-level convenience in toolkit or optional crates, and improves adoption before adding broad live integrations.

## Relevant Existing Context

- `AGENTS.md`: no branch creation without explicit approval; preserve product-neutral core; optional behavior belongs in toolkit or separate crates; release/broad docs handoff requires public audit.
- `README.md` and `docs/start-here.md`: core owns the primitive kernel; P0/P1/P2 readiness profiles must stay layered; simple helpers must lower into canonical DTOs.
- `coding_standards.md` and `docs/architecture/coding-standards.md`: public APIs require canonical lowering, deterministic fakes, mockability, rustdoc/API hygiene, and package-layout discipline.
- `docs/workstreams/validation-gates.md`: implementation work must name primitive fit, no-mini-SDK evidence, mockability, events/journals/policy impact, and validation commands.
- `docs/architecture/primitive-map.md`: provider adapters, MCP, web, workflow, OTel, isolation, session stores, and tool ergonomics layer over existing ports and records.
- `docs/reference/persistence-ownership-map.md`: journals, checkpoints, content refs, event cursors, agent-pool state, provider tool arguments, and tool-execution records are separate persistence surfaces; durable adapters should be optional crates, not a new core storage primitive.
- `docs/agent-sdk-toolkit/README.md` and `docs/agent-sdk-toolkit/adapter-and-runtime-plan.md`: prefer aggregate optional crates such as `agent-sdk-provider`, `agent-sdk-mcp`, `agent-sdk-browser-toolkit`, `agent-sdk-isolation`, `agent-sdk-otel`, and `agent-sdk-workflow`; split per-backend provider crates only when dependency or release pressure proves it.
- `docs/reference/2026-05-26-strands-sdk-python-gap-report.md`: top gaps are runnable model-tool loop, live providers, MCP, sessions, context compaction, streaming, hooks, tool ergonomics, optional orchestration, and facade trimming.
- `CHANGELOG.md` and crate manifests: current local crate version is now `0.1.0-alpha.3` because live provider structured-output hints changed the source API; top-level docs previously still referenced `0.1.0-alpha.1` in current-release language.

## Requirement Map

| Need | Existing primitive | Proper scope | Work needed |
| --- | --- | --- | --- |
| Live-provider onboarding | `Agent`, `AgentRuntime`, `RunRequest`, `RuntimePackage`, `ProviderAdapter`, journal/event stores | docs and provider examples | Add a quickstart that starts from a real provider adapter and shows the canonical runtime path. |
| Typed-output onboarding | `OutputContract`, `TypedOutputModel`, `ValidatedOutput`, P1 typed run | docs and core tests/examples | Add a quickstart that uses helper ergonomics but shows it lowers to `RunRequest` plus `OutputContract`. |
| Tool approval onboarding | `RuntimePackage`, `CapabilitySpec`, `ToolRoute`, `ToolExecutionCoordinator`, `ApprovalPolicy`, `EffectIntent`, `RunJournal` | docs/examples now; later core/tool ergonomics | Add a quickstart showing approval and tool execution as journaled side effects, not direct callbacks. |
| Live provider adapters | `ProviderAdapter`, `ProviderRequest`, `ProviderResponse`, provider stream chunks, provider route snapshots | optional `agent-sdk-provider` crate first | Done for first live text/structured-output request mapping: OpenAI, Anthropic, and Gemini adapters live in the aggregate provider crate. Still add model catalog, streaming, provider-native function-result replay, and opt-in live smoke. |
| Model-tool loop | provider tool-use deltas, `ToolRouter`, `ToolExecutionCoordinator`, context projection, journal records | core application loop | Add provider tool-call request/response primitives first, then a canonical app-facing loop path. |
| Ergonomic tool wrappers | `ToolRoute`, `ToolRegistrySnapshot`, `ToolExecutor`, `ToolExecutionCoordinator` | core ergonomics or toolkit helper crate | Add wrappers only if they lower into route/policy/journal/effect and do not execute directly. |
| Persistence ownership | `RunJournal`, `CheckpointStore`, content refs/resolvers, event cursors/archive, `AgentPoolStore`, provider argument refs, tool records | core ports plus toolkit/store crates | Done for boundary clarification via the persistence ownership map; still add concrete file/SQLite adapters outside core where appropriate. |
| MCP and web adapters | tool ports, content refs, `EffectIntent`, isolation/network policy | optional `agent-sdk-mcp` and `agent-sdk-browser-toolkit` | Keep SSRF/resource policy fail-closed; require transport fakes and conformance. |
| Memory/context compaction | `MemoryPort`, `ContextContribution`, `ContextProjection`, compaction policy sketches | core example first; later toolkit/store adapters | Add a small example that feels usable and proves admission/projection boundaries. |
| Naming/package strategy | current split crates and occupied `agent-sdk` name | docs/release strategy | Make the split-package story unmistakable; defer meta crate naming until release strategy is explicit. |

## What To Say No To For Now

- Do not create `agent-sdk-provider-openai`, `agent-sdk-provider-anthropic`, and `agent-sdk-provider-gemini` immediately. The cleaner first move is an aggregate `agent-sdk-provider` facade with feature-gated modules, plus a later split only if dependency/platform/SemVer pressure justifies it.
- Do not put live provider dependencies, MCP clients, web fetchers, workflow engines, OTel exporters, or isolation runtimes in `agent-sdk-core`.
- Do not claim streaming-provider, MCP, or browser/web support before transport conformance, redaction, retry/error classification, cancellation, and public-release audits exist.
- Do not add ergonomic wrappers that execute tools directly; wrappers must lower into `ToolRoute`, package policy, journal intent/result, events, and effect contracts.

## First Implementation Slice

Short-term adoption and release hygiene:

- Sync current public docs so published `0.1.0-alpha.2` crates are distinguished from the local `0.1.0-alpha.3` release-candidate API; leave historical release plans intact.
- Add live-provider, typed-output, and tool-approval quickstarts under `docs/examples/`.
- Update `docs/examples/README.md` so onboarding examples appear before complex scenario examples.
- Make each quickstart name the canonical runtime path, preserved boundaries, and validation expectation.

## Later Slices

1. Done: add provider tool-call DTOs plus terminal stream tool-call deltas to the provider port and fixtures.
2. Done: add the first app-facing, non-streaming model-tool continuation loop over provider tool calls, policy, tool execution, journaled results, and provider continuation.
3. Done: add toolkit-owned `Tool`, `AsyncTool`, and `ToolPackBuilder::listen*` wrappers with tests proving canonical lowering.
4. Add streaming tool-call input deltas, bounded tool concurrency, richer hooks, and cancellation/retry policy to harden the loop.
5. Partially done: add an aggregate `agent-sdk-provider` crate with live OpenAI/Anthropic/Gemini adapters and OpenAI-compatible conformance tests; still add model catalog, streaming support, provider-native function-result replay, and live opt-in smoke.
6. Add MCP and browser/web adapter crates with fail-closed SSRF/resource policies.
7. Partially done: add a small memory/compaction quickstart; still add concrete session/persistence store adapters.
8. Done for storage boundary clarification: add a persistence ownership map; still add optional file/SQLite store and session crates.
9. Add optional OTel, isolation, and workflow crates after the core loop and adapter conformance are stable.

## Validation Plan

- `cargo fmt --check`
- `git diff --check`
- `scripts/public-release-audit.sh`
- Targeted docs search for stale current-version references.
- If Rust examples or tests are added in a later slice, run the owning crate tests named by the launch doc. For the provider slice, run `cargo test -p agent-sdk-provider`.

## Risk / Gotcha Carry-Forward

- The worktree already contains unrelated dirty Rust and docs changes; do not revert or silently format those files.
- Quickstarts must not become implementation promises beyond current support. Mark streaming provider/MCP/web examples as future optional-crate work until crates and tests exist.
- Version sync should avoid rewriting historical phase plans that intentionally describe an older release decision.
- Package naming must stay conservative until crates.io naming and dependency-weight constraints are reviewed.
