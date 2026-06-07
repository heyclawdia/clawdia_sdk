# SDK DX Direction Report

## Executive Summary

The SDK already has the right core shape: product-neutral primitives, typed
runtime packages, journals, events, policies, output contracts, provider ports,
deterministic fakes, and optional provider/toolkit/eval crates. The DX target is
to make that strength obvious in the first five minutes. A new user should see
one small app-building path for "choose a model, add tools, run, observe,
checkpoint, and evaluate" while every convenience still lowers into the
canonical SDK contracts.

The direction is simple outside and serious inside. Add an optional convenience
facade, app builder, tool authoring helpers, feature/install docs, and runnable
examples only where they preserve `Agent`, `AgentRuntime`, `RunRequest`,
`RuntimePackage`, `ProviderAdapter`, `ToolExecutionCoordinator`, policy,
journals, events, telemetry, redaction, output contracts, and checkpoints as the
source of truth.

This report describes the DX direction and the current first implementation
slice. The repository now has an unpublished `clawdia-sdk` facade crate for
imports and examples. It still does not add an app builder, tool macro,
persistence backend, runnable example workspace, or release metadata.

## Current Local Grounding

| Area | Current local state | DX implication |
| --- | --- | --- |
| Package shape | Split crates: `agent-sdk-core`, `agent-sdk-toolkit`, `agent-sdk-provider`, and `agent-sdk-eval`. The repository explicitly does not publish `agent-sdk`. | Correct boundary, but users must assemble a mental model across crates. |
| Core common imports | `agent_sdk_core::prelude::*` exists as a facade-only re-export namespace. It adds no behavior. | Good core import path, but not a whole-SDK facade. |
| Agent/run helpers | `Agent::builder`, `Agent::run_text`, `Agent::run_typed`, `Agent::typed_text_request`, `AgentRuntime::run_text`, and `AgentRuntime::run_typed` already lower into canonical requests. | Good foundation, but users still configure runtime packages/providers/journals explicitly. |
| Provider adapters | `agent-sdk-provider` exposes live-provider adapters over `ProviderAdapter`. | Useful, but model catalog, streaming profile, and provider quickstarts are still thin. |
| Tool ergonomics | `agent-sdk-toolkit` exposes data-only `Tool`, `AsyncTool`, and `ToolPackBuilder`; execution still goes through core tool routes, policy, journal, events, and effects. | Correct boundary, but no typed function macro/builder that generates schema and executor glue. |
| Persistence | Core has journal/checkpoint/content/event/agent-pool/tool/provider-argument boundaries. Toolkit has SQLite agent-pool support. | No one-command durable session/checkpoint setup yet; persistence must stay split by responsibility. |
| Token/cost | Core telemetry records and eval trace metrics can carry token totals and usage-derived counts. | No user-facing cost estimator/rate table policy layer yet. |
| Examples | Repo has docs quickstarts and scenario notes, but not the numbered runnable example directories requested by the launch prompt. | Good conceptual coverage; copy-run onboarding still needs a curated example suite. |

## DX Target Shape

| Target | SDK direction | Boundary that must stay true |
| --- | --- | --- |
| First app builder | A small `AgentApp`-style builder for the everyday path. | The builder wires canonical components and then calls the normal runtime path. |
| Optional facade | A convenience crate for onboarding and examples. | Split crates stay supported and core stays dependency-light. |
| Tool authoring | Typed function-tool builder first; optional macro later. | Generated or builder-created tools register schema/executor refs and execute only through core coordination. |
| Install clarity | Feature docs organized by user task and real current crates: core kernel, provider adapters, workspace tools, evaluation helpers, and deterministic test support. | Feature groups map to real crates and must not pull hidden dependencies into core. |
| Local persistence | Start with file/SQLite-oriented local stores where they match the persistence map; leave hosted stores as future adapter options. | No global state store; journals remain durable truth and sessions are projections. |
| Provider onboarding | Provider adapters should be swappable, testable, and shown with deterministic mock paths. | Credentials, endpoint policy, routing decisions, and host product choices stay outside adapters. |
| Observability | Events, journals, usage, cost estimates, and trace metrics should appear in examples from the first plan. | Telemetry is a projection from durable evidence, not a separate truth store. |
| HITL approvals | Tool-level interrupts produce durable requests and auditable decisions. | Core never calls product UI directly. |

## Missing Public API Pieces

| Gap | Proper scope | Required proof before implementation is complete |
| --- | --- | --- |
| `AgentApp` or equivalent first-run builder | Facade crate or small optional app-builder crate | Tests showing the builder lowers into `Agent`, `AgentRuntime`, `RunRequest`, `RuntimePackage`, provider registry, journal, event bus, policy, and output contracts. |
| Convenience facade crate | New optional `crates/clawdia-sdk`, initially `publish = false` | Feature-gated dependency tests, rustdoc examples, package list audit, public API/SemVer review. |
| Tool function builder/macro | `agent-sdk-toolkit` feature or `agent-sdk-macros` | Deterministic schema tests, sync/async execution tests, structured error conversion, durable tool records, docs examples. |
| Provider install groups | Facade plus provider crate features or namespaces | `cargo tree` evidence that unused providers are not pulled in; mocked provider request tests. |
| `run_stream` onboarding | Core runtime plus provider adapter streaming support | Event/journal cursor tests, terminal event preservation, redaction and backpressure tests. |
| Durable session/checkpoint quickstart | Optional store/session crates over existing persistence map | Append/replay/checkpoint/content/ref fixtures and crash recovery tests. |
| Token/cost summaries | Core telemetry projection plus optional cost policy/rate crate | Cost disabled-by-default tests, host-rate policy injection tests, redacted telemetry fixtures. |
| MCP/browser/web adapters | Optional adapter crates | Protocol conformance, SSRF/resource policy, timeout/cancellation, redaction, and effect-journal tests. |

## Recommended Crate And Feature Layout

Keep current split crates as the authoritative implementation path. The first
facade slice now exists as unpublished `crates/clawdia-sdk`; later helpers must
still prove canonical lowering before they are added.

| Crate or feature | Status | Responsibility |
| --- | --- | --- |
| `agent-sdk-core` | Existing | Primitive kernel, records, runtime, ports, policies, events, journals, deterministic fakes. |
| `agent-sdk-provider` | Existing | Optional provider adapters over `ProviderAdapter`. |
| `agent-sdk-toolkit` | Existing | Concrete tool packs, workspace/shell/resource helpers, protocol test helpers, toolkit fakes. |
| `agent-sdk-eval` | Existing | Optional post-hoc evaluation records and deterministic trace metrics. |
| `clawdia-sdk` | Existing, unpublished | Convenience facade over split crates, initially unpublished. |
| `agent-sdk-macros` | Proposed | Optional proc macros for tools and typed schema helpers if the builder layer is not enough. |
| `agent-sdk-store-file` | Proposed | File-backed journals, checkpoints, content blobs, and event archive adapters. |
| `agent-sdk-store-sqlite` | Proposed | SQLite-backed local store adapters with explicit tables and migration fixtures. |
| `agent-sdk-mcp` | Proposed | MCP adapter lowering remote tools/resources/prompts into SDK capabilities and effect records. |
| `agent-sdk-otel` | Proposed | OTel export projection over events/journals, not telemetry truth. |
| `agent-sdk-workflow` | Proposed | Optional orchestration over events/runs, not core workflow ownership. |

Implemented facade features:

| Feature | Dependency boundary | Readiness |
| --- | --- | --- |
| `providers` | Enables provider adapter re-exports. Provider-specific names can stay opt-in if dependency pressure appears. | Existing provider crate and facade re-export. |
| `workspace-tools` | Enables workspace read/search/edit/write, shell/resource helpers, and toolkit pack builders. | Existing toolkit crate. |
| `evals` | Enables evaluation crate re-exports. | Existing crate. |
| `test-support` | Enables deterministic core testing helpers. Toolkit/eval testing helpers are re-exported when their features are also enabled. | Existing test-support surfaces. |
| `all-stable` | Enables current stable optional facade groups, excluding test support and future adapters. | Existing grouping over real current features only. |

Future install areas:

| Area | Boundary | Required readiness before feature exposure |
| --- | --- | --- |
| Typed tool helpers | Function-tool builder and, later, optional macros. | Builder/macro layer with deterministic schema, execution, error, and docs tests. |
| Local stores | File/SQLite-oriented journal, checkpoint, content, event, tool, and provider-argument adapters. | Backends mapped to the persistence ownership map with crash/replay fixtures. |
| Observability helpers | Usage/cost summaries and exporter setup. | Projections from durable evidence with privacy/redaction fixtures. |
| Approval helpers | HITL ergonomics over core broker and durable decision records. | Host-owned UI boundary tests and journal/event evidence tests. |

## Recommended Quickstarts And Examples

Add examples in a later implementation phase only after their dependencies and
CI gates are defined:

1. `examples/01_live_provider_text_run`
2. `examples/02_typed_tool_builder`
3. `examples/03_typed_output`
4. `examples/04_tool_approval_hitl`
5. `examples/05_streaming_events`
6. `examples/06_checkpoint_resume`
7. `examples/07_token_tracking_costs`
8. `examples/08_subagent_handoff`
9. `examples/09_trace_eval`
10. `examples/10_facade_quickstart`

Each example should include a `README.md`, run command, environment variables,
expected output shape, failure modes, the SDK primitive it demonstrates, and an
SDK-owned/host-owned boundary block. Live-provider examples need mocked CI paths
or explicit live-test gates.

## Risks And Migration Concerns

- A facade can create dependency surprise if `default` pulls provider, toolkit,
  persistence, OTel, or async-runtime dependencies. Prefer a conservative
  default and explicit features.
- A facade can confuse `agent_sdk_core::prelude` with the broader convenience
  crate. Document that the prelude is core-only and behavior-free.
- An app builder can become a mini runtime if it owns package resolution,
  policy, journals, events, tool execution, provider credentials, or UI. It
  should build and wire canonical components only.
- A tool macro can hide side effects. Generated code must register schema and
  executor refs, then execution must still go through core coordination.
- Persistence features can blur durable truth. Journals remain truth; sessions
  are projections; backend crates must map to the persistence ownership map.
- The SDK is alpha, so breaking changes are acceptable, but they must be
  recorded in risk/watchpoint docs and release notes before release handoff.

## Proposal Blocks

Accepted for this packet:

- Recommend a `clawdia-sdk` convenience facade as the safest first proposal.
- Add an unpublished, behavior-free `clawdia-sdk` facade as the first code
  slice.

Rejected for this packet:

- Renaming existing crates.
- Publishing a facade crate now.
- Adding app builders, macros, stores, runnable examples, or release metadata.

Deferred:

- Macro implementation details.
- Runnable example workspaces.
- Provider streaming/model catalog support.
- Concrete persistence backend crates beyond current toolkit SQLite agent-pool
  support.

## Source Material Note

This report is grounded in the local repository state and the SDK direction we
want to build toward. It does not introduce external SDK terminology into the
implementation packet.
