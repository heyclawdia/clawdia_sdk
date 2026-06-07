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

This report describes the DX direction and the current implementation slice.
Phase 15 now implements the previously deferred first-user DX surfaces:
`AgentApp`, typed tool helpers and optional macros, file and Supabase store
adapters, usage/cost/run reports, provider-visible tool projection, and five
runnable checkout examples. Release metadata remains a separate readiness task.

## Phase 15 Implementation Update

Implemented in this checkout:

- `clawdia-sdk::AgentApp` assembles canonical `AgentRuntimeBuilder` ports and
  lowers `run_text`/`run_typed` into normal `RunRequest` execution.
- `agent-sdk-toolkit::typed_tool` provides typed argument/output traits,
  deterministic schema snapshots, typed executor glue, and package bundle
  lowering.
- `agent-sdk-macros` provides optional derives and `#[agent_tool]`, including
  facade-only path resolution through `clawdia_sdk::tools`.
- Provider adapters project provider-visible tool declarations for
  OpenAI-compatible, OpenAI Responses, Anthropic Messages, and Gemini
  generateContent request shapes, using inline redacted schemas when toolkit
  packages provide them and a schema-ref fallback otherwise.
- `agent-sdk-store-file` implements file-backed journal, checkpoint, content,
  event archive, provider-argument, and agent-pool store adapters.
- `agent-sdk-store-supabase` implements Supabase REST-backed journal,
  checkpoint, content, event archive, provider-argument, and agent-pool
  adapters through an injectable HTTP transport plus checked migrations.
- `agent-sdk-eval` now exposes `UsageReport`, `CostPolicy`, `StaticRateTable`,
  `CostReport`, `RunReport`, and `RunReportLimitations`.
- Runnable smoke examples now live under `examples/01_*` through
  `examples/05_*`.

## Current Local Grounding

| Area | Current local state | DX implication |
| --- | --- | --- |
| Package shape | Split crates: `agent-sdk-core`, `agent-sdk-toolkit`, `agent-sdk-provider`, and `agent-sdk-eval`. The repository explicitly does not publish `agent-sdk`. | Correct boundary, but users must assemble a mental model across crates. |
| Core common imports | `agent_sdk_core::prelude::*` exists as a facade-only re-export namespace. It adds no behavior. | Good core import path, but not a whole-SDK facade. |
| Agent/run helpers | `Agent::builder`, `Agent::run_text`, `Agent::run_typed`, `Agent::typed_text_request`, `AgentRuntime::run_text`, and `AgentRuntime::run_typed` already lower into canonical requests. | Good foundation, but users still configure runtime packages/providers/journals explicitly. |
| Provider adapters | `agent-sdk-provider` exposes live-provider adapters over `ProviderAdapter`. | Useful, but model catalog, streaming profile, and provider quickstarts are still thin. |
| Tool ergonomics | `agent-sdk-toolkit` exposes data-only `Tool`, `AsyncTool`, `ToolPackBuilder`, and typed tool helpers; `agent-sdk-macros` adds optional derives/attribute helpers. | Correct boundary; continue adding helper coverage without bypassing core tool coordination. |
| Persistence | Core has journal/checkpoint/content/event/agent-pool/tool/provider-argument boundaries. File and Supabase store crates implement concrete adapters over those boundaries. | Durable stores remain split by responsibility; no global state store. |
| Token/cost | Core telemetry records and eval trace metrics can carry token totals and usage-derived counts. Eval now adds usage, cost, and run-report helpers. | Keep rates host-provided and disabled unless a caller supplies a policy. |
| Examples | The checkout has five runnable Phase 15 smoke examples. | Keep examples deterministic and credential-free unless a live gate is explicit. |

## DX Target Shape

| Target | SDK direction | Boundary that must stay true |
| --- | --- | --- |
| First app builder | A small `AgentApp`-style builder for the everyday path. | The builder wires canonical components and then calls the normal runtime path. |
| Optional facade | A convenience crate for onboarding and examples. | Split crates stay supported and core stays dependency-light. |
| Tool authoring | Typed function-tool builders plus optional macro helpers. | Generated or builder-created tools register schema/executor refs and execute only through core coordination. |
| Install clarity | Feature docs organized by user task and real current crates: core kernel, provider adapters, workspace tools, evaluation helpers, and deterministic test support. | Feature groups map to real crates and must not pull hidden dependencies into core. |
| Persistence adapters | File and Supabase stores implemented where they match the persistence map; future backends must follow the same per-port ownership. | No global state store; journals remain durable truth and sessions are projections. |
| Provider onboarding | Provider adapters should be swappable, testable, and shown with deterministic mock paths. | Credentials, endpoint policy, routing decisions, and host product choices stay outside adapters. |
| Observability | Events, journals, usage, cost estimates, and trace metrics should appear in examples from the first plan. | Telemetry is a projection from durable evidence, not a separate truth store. |
| HITL approvals | Tool-level interrupts produce durable requests and auditable decisions. | Core never calls product UI directly. |

## Implemented And Remaining Public API Pieces

| Gap | Proper scope | Required proof before implementation is complete |
| --- | --- | --- |
| `AgentApp` first-run builder | Implemented in `clawdia-sdk` | Public API tests show the builder lowers into `Agent`, `AgentRuntime`, `RunRequest`, `RuntimePackage`, provider registry, journal, event bus, policy, and output contracts; the complex facade example exercises file stores, typed tools, approval dispatch, events, and run reports. |
| Convenience facade crate | Implemented as optional `crates/clawdia-sdk`, initially `publish = false` | Feature-gated dependency tests, rustdoc examples, cargo-tree audits, and public API review are Phase 15 validation gates. |
| Tool function builder/macro | Implemented in `agent-sdk-toolkit` and `agent-sdk-macros` | Deterministic schema tests, sync execution tests, structured error conversion, durable tool records, macro compile tests, and runnable examples cover the path. |
| Provider install groups | Implemented through facade features plus provider crate namespaces | `cargo tree` evidence shows core stays dependency-light; provider request tests cover mocked wire shapes. |
| `run_stream` onboarding | Remaining core/provider documentation and example work | Event/journal cursor tests, terminal event preservation, redaction, and backpressure tests must be named before adding a facade quickstart. |
| Durable session/checkpoint quickstart | File and Supabase adapters over the existing persistence map | Append/replay/checkpoint/content/ref fixtures, provider-argument readback, recovery tests, agent-pool store tests, and scripted Supabase request tests cover current stores. |
| Token/cost summaries | Implemented in optional `agent-sdk-eval` reports | Cost disabled-by-default behavior, host-rate policy injection, and redacted evidence limitations are covered by eval tests. |
| MCP/browser/web adapters | Future optional adapter crates | Protocol conformance, SSRF/resource policy, timeout/cancellation, redaction, and effect-journal tests are required before feature exposure. |

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
| `agent-sdk-macros` | Existing | Optional proc macros for typed tool schema and builder helpers. |
| `agent-sdk-store-file` | Existing | File-backed journals, checkpoints, content blobs, event archive, provider argument, and agent-pool adapters. |
| `agent-sdk-store-supabase` | Existing | Supabase REST-backed store adapters with injectable transport and SQL migration. |
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
| `macros` | Enables optional typed-tool proc macros through `clawdia_sdk::tools`. | Existing macro crate and facade re-export. |
| `file-store` | Enables file-backed durable store adapters. | Existing adapter crate and tests. |
| `supabase-store` | Enables Supabase REST-backed durable store adapters. | Existing adapter crate, scripted transport tests, and migration. |
| `stores` | Enables all current store backends. | Existing grouping over file and Supabase stores. |
| `all-stable` | Enables current stable optional facade groups, excluding test support. | Existing grouping over real current features only. |

Future optional adapter areas:

| Area | Boundary | Required readiness before feature exposure |
| --- | --- | --- |
| Additional local stores | SQLite or other local journal, checkpoint, content, event, tool, and provider-argument adapters. | Backends mapped to the persistence ownership map with crash/replay fixtures. |
| OTel/exporter helpers | Export projections over events and journals. | Projections from durable evidence with privacy/redaction fixtures and no telemetry truth store. |
| Approval ergonomics | HITL helpers over core broker and durable decision records. | Host-owned UI boundary tests and journal/event evidence tests. |
| Protocol adapters | MCP, ACP, browser, web, or remote providers. | Protocol conformance, resource policy, timeout/cancellation, redaction, and journaled effect tests. |

## Recommended Quickstarts And Examples

Phase 15 adds deterministic checkout examples:

1. `examples/01_facade_complex_agent`
2. `examples/02_typed_tool_macro`
3. `examples/03_file_store`
4. `examples/04_supabase_scripted_store`
5. `examples/05_reporting_and_eval`

Future richer examples should keep the same discipline: include a run command,
expected output shape, failure modes, the SDK primitive demonstrated, and an
SDK-owned/host-owned boundary block. Live-provider examples need mocked CI
paths or explicit live-test gates.

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
- Implement `AgentApp`, typed tool helpers/macros, provider-visible tool
  projection, file and Supabase stores, deterministic reports, and runnable
  examples as Phase 15 DX completion surfaces.

Rejected for this packet:

- Renaming existing crates.
- Publishing a facade crate now.
- Adding release metadata or live hosted infrastructure requirements.

Deferred:

- Provider streaming/model catalog support.
- Concrete persistence backend crates beyond file, Supabase, and the current
  toolkit SQLite agent-pool support.
- OTel/exporter, MCP/browser/web, and workflow adapter crates.

## Source Material Note

This report is grounded in the local repository state and the SDK direction we
want to build toward. It does not introduce external SDK terminology into the
implementation packet.
