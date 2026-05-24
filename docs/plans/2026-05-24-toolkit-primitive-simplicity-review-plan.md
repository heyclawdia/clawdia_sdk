# Toolkit Primitive Simplicity Review Plan

## Objective

Review and tighten `docs/agent-sdk-toolkit/` so the planned live provider, ACP, MCP, isolation, browser/web, MLX, llama.cpp, artifact-resolution, and conformance adapters all reuse the same primitive SDK contracts instead of creating parallel mini-SDKs.

This is a documentation-only review/update. It must not create Rust source files, executable tests, package manifests, fixtures, branches, publish actions, or tags.

## Problem Shape

The existing toolkit plan correctly keeps adapter crates optional, but it still reads as a list of adapter families before it proves the common primitive surface they all share. That makes future work vulnerable to drift:

- provider adapters could grow provider-specific request/event semantics;
- ACP could become a second run loop;
- MCP could become ambient tool authority or bypass package/tool policy when an agent is isolated;
- MCP helpers could accidentally expose every tool on a server when the user only intended to attach one tool;
- browser tooling could become a provider feature or ambient host-browser bridge;
- containerized agents could become a deployment product rather than `RuntimePackage` + `IsolationRuntime` + tool packs;
- MLX or llama.cpp could be misread as a security boundary rather than provider compute;
- model/runtime downloads could become hidden runtime magic instead of explicit policy-gated side effects.

The structural fix is to add an explicit primitive ledger, canonical lowering rule, and simplicity/review gate to the toolkit docs.

## Authoritative Sources

- `AGENTS.md`: product-neutral workspace, no branches without approval, docs-only tasks must not create Rust/package/test/fixture files.
- `README.md`: first reading path, normative sources, and current implementation posture for the SDK packet.
- `docs/start-here.md`: standalone/product-neutral posture, thin ergonomic helpers, primitive kernel, and non-goals.
- `coding_standards.md`: simple helpers lower into canonical contracts; mockability is non-negotiable; package layout and SDK-consumer test helpers are gates.
- `docs/architecture/coding-standards.md`: domain-first primitives, TDD posture, deterministic fakes, reusable test kits, no ambient host state.
- `docs/reference/sdk-review-checklist.md`: simplicity, primitive fit, no mini-SDK drift, canonical lowering, host-owned boundaries, package topology, and testability.
- `docs/architecture/primitive-map.md`: feature layers must reuse the kernel before adding a primitive.
- `docs/reference/simplicity-audit.md`: simplification means preserving a small MVP primitive kernel and keeping advanced-only escape hatches out of the common path.
- `docs/workstreams/validation-gates.md`: documentation-only work must provide link/path, boundary, primitive-lowering, mockability, and no-source evidence.
- `docs/agent-sdk-toolkit/README.md` and `docs/agent-sdk-toolkit/adapter-and-runtime-plan.md`: current toolkit roadmap to review and tighten.

## Behavior Contract

New behavior:

- The toolkit roadmap explicitly names the shared primitives every adapter family must reuse.
- Each adapter family documents canonical lowering into `RuntimePackage`, typed ports, `EffectIntent` / `EffectResult`, `RunJournal`, `AgentEvent`, policy refs, content refs, idempotency/cursor rules, and conformance fakes.
- The common-path API sketches favor stateless preset/builder helpers that compose the primitive kernel instead of introducing new authority.
- Toolkit facades, if named later, are namespaces or stateless helper types only. They cannot own a run loop, provider registry, package registry, credential store, journal, event bus, policy path, context path, telemetry truth store, or side-effect path.
- Local model backends cover MLX for Apple silicon and llama.cpp/GGUF as a cross-platform provider backend, with explicit artifact resolver policy.
- Browser/web docs clarify that the SDK controls model network authority through policy and adapters rather than reinventing browser automation engines.
- ACP/MCP isolation docs clarify that isolated agents communicate through host-edge bridges or policy gateways, never raw editor handles, MCP sockets, roots, credentials, session IDs, or ambient discovery.
- MCP ergonomics clarify exact capability selection so SDK users can attach only specific tools/resources/prompts from a server, and only those capabilities become model-visible or executable.
- Adapter packaging defaults to aggregate public crates such as `agent-sdk-provider` and `agent-sdk-isolation`, with feature-gated backend modules. Per-backend crates are a later optimization only if dependency or release pressure proves the need.
- ACP conformance hard-requires fake client/server communication over the real ACP JSON-RPC transport boundary, with stdio as the first harness. In-process helper-only fakes do not count.
- Review gates reject adapter plans that create a new run loop, package registry, event stream, journal, policy path, context path, telemetry truth store, or side-effect path.
- Practical complexity guidance requires a small opinionated preset set first, stateless helpers, conformance before new variants, host-managed services where lifecycle/reporting are clearer, and a documented default deployment path for common cases.
- Toolkit open decisions are resolved into first-default choices with deferred variants and release gates, rather than remaining ambiguous implementation choices.
- ACP delegation is documented with provider-like ergonomics through an `AcpAgentProvider` facade, while still lowering to `ExternalAgentAdapter` / supervised subagent primitives so calling an external ACP agent goes through `RuntimePackage`, policy, journals, events, cancellation, and replay.
- Provider/model routing is documented as a per-agent default that freezes into the effective `RuntimePackage`, including live providers, OpenAI-compatible providers, local providers, and ACP agent providers.
- OpenAI-compatible provider support is documented with explicit compatibility profiles, endpoint policy, capability reporting, and conformance fakes so compatible endpoints cannot silently drift from OpenAI semantics.
- Provider model support is documented as a single versioned catalog source of truth for supported model names, aliases, modalities, reasoning/thinking support, image support, context limits, tool support, structured-output support, status, compatibility notes, and source/probe provenance across OpenAI, Anthropic, Kiro CLI/ACP, Bedrock, Google Cloud, and Microsoft Azure research targets.

Preserved behavior:

- `agent-sdk-core` remains product-neutral and does not import live providers, ACP, concrete isolation runtimes, browser automation, MLX, UI, or credential stores.
- Adapter crates stay optional and implement existing ports.
- Aggregate adapter crates preserve feature flags so users do not pay for providers or runtimes they do not enable.
- Live providers, real containers, and product UI remain optional smoke tests, never the first proof.
- Website access defaults remain no internet and no host browser profile.
- MLX and llama.cpp remain provider/model acceleration, not isolation.
- Model/runtime downloads remain explicit toolkit/host side effects, not silent core behavior.

Removed behavior:

- None. This pass should clarify and constrain the plan, not delete planned adapter families.

Tests/proof:

- Documentation link/path checks for the edited files.
- Product-neutrality search for product-specific names in the toolkit docs.
- Primitive-lowering audit by independent coding agent.
- `git diff --check`.
- Docs-only file audit proving no Rust/package/test/fixture files were created.

## Scope

Writable files:

- `README.md`
- `docs/start-here.md`
- `docs/plans/2026-05-24-toolkit-primitive-simplicity-review-plan.md`
- `docs/agent-sdk-toolkit/README.md`
- `docs/agent-sdk-toolkit/adapter-and-runtime-plan.md`

Out of scope:

- Rust source implementation.
- Cargo/package manifest changes.
- Executable tests or fixtures.
- Provider-specific API contracts beyond non-compiling sketches.
- Product-specific host examples.
- Branch creation, commit, push, publish, or tag actions.

## Workstreams

1. Context and standards scan: complete before editing.
2. Independent plan review: review this plan against coding standards, primitive map, and SDK review checklist before implementation.
3. Documentation implementation: add a primitive ledger, simple common API path, canonical lowering table, and adapter review gates.
4. Independent implementation review: review the final diff for simplicity, primitive fit, mockability, host boundaries, and no mini-SDK drift.
5. Validation: run docs-only audits and report skipped code tests as not applicable.

## Risk/Gotcha Carry-Forward

- Do not add new core primitives just because an adapter needs configuration. Prefer typed sidecars, existing ports, or host-owned refs.
- Do not introduce an authoritative `AgentToolkit` object. Presets/builders may be ergonomic, but they only return existing DTOs, builders, typed sidecars, or adapter registration helpers.
- Do not make `CapabilitySpec` a universal bag. Use it only for callable/discoverable capabilities with typed sidecars.
- Do not let ACP own session truth, file authority, terminal authority, approval UI, or durable memory policy.
- Do not call an external ACP agent through an ad hoc run loop. `AcpAgentProvider` is allowed as a provider-like user facade, but it must lower to the `ExternalAgentAdapter` / subagent path so prompt, cancel, stream, file/tool/terminal, approval, journal, and event semantics stay unified.
- Do not accept ACP conformance that bypasses JSON-RPC transport framing. Fake clients and fake agents must exchange encoded ACP messages over stdio or a future reviewed transport adapter.
- Do not let MCP bypass package/tool policy just because a server is reachable from inside an isolated runtime. Discovery results are candidates until filtered into `RuntimePackage` capabilities or content refs.
- Do not let MCP helper APIs attach a whole server by default. Exact tool/resource/prompt selection must be the easy path, with unselected sibling capabilities hidden from projection and denied at execution.
- Do not pass raw MCP roots, auth tokens, server handles, sockets, or session IDs into isolated processes. A policy-approved MCP relay is still the mediated SDK gateway, not a raw protocol tunnel, and must preserve package, tool-router, journal/effect, event, redaction, cancellation, and conformance semantics.
- Do not let browser access bypass `ToolPack`, `ExecutionEnvironment`, policy, content refs, effect records, or journal events.
- Do not claim host process execution or local model backends satisfy secure isolation.
- Do not let per-agent provider/model defaults mutate an active run. They are convenience inputs; the effective runtime-package route snapshot is the execution authority.
- Do not treat OpenAI-compatible providers as a single fully compatible blob. Require compatibility profiles, endpoint allowlists, typed unsupported-feature errors, and deterministic fake servers.
- Do not scatter provider model names or capabilities through adapter code. Route selection and capability checks must read the single provider model catalog or a generated typed view of it, and catalog entries must cite their source docs or host probes.
- Do not silently download model weights, provider binaries, browser images, or runtime assets from core.
- Do not reinvent browser automation when a browser engine or browser-use-style library can be an adapter behind the SDK policy boundary.
- Do not treat live provider or live runtime smoke as release confidence without deterministic fakes and conformance tests.
- Do not put credentials, editor handles, cookies, host browser profiles, raw model prompts, process IDs, or runtime session IDs into package fingerprints, journals, raw events, or fixtures.

## Review Packet

Primitive decision:

- Reused kernel primitives: `Agent`, `AgentRuntime`, `RunRequest`, `RunHandle`, `RuntimePackage`, `CapabilitySpec`, `ExecutionEnvironment`, `ProviderAdapter`, `ProviderRouteSnapshot`, `ModelRef`, provider model catalog version/hash refs, `ExternalAgentAdapter`, `IsolationRuntime`, `McpRegistry`, `ToolRouter`, `ToolPack`, `OutputSink`, `ArtifactRef`, `ContentRef`, `EffectIntent`, `EffectResult`, `RunJournal`, `AgentEvent`, policy refs, typed IDs, and lineage refs.
- New feature-layer primitives: none planned in this docs pass.
- New capability variants: none planned in this docs pass.
- Host-owned behavior kept out: credentials, provider selection UI, ACP editor UI, durable memory policy, concrete runtime installation, browser profile credentials, deployment UX, and product output channels.

Validation evidence to collect:

- Contract/unit tests: not applicable for docs-only.
- Golden fixtures: not applicable for docs-only.
- Smoke/scenario tests: named future conformance requirements only.
- Docs audits: link/path, product-neutrality, primitive-lowering, mockability, no-source, and whitespace/diff checks.

Reviewer checklist:

- Simplicity: common path stays one composition path, advanced adapters remain optional.
- Product-neutrality: toolkit docs stay free of product behavior.
- Canonical lowering: every adapter family names helper/API, lowered primitive, sidecar/capability, policy/effect/journal path, event/cursor/content refs, and fake/conformance harness before coding starts.
- Mockability / SDK test kit: each adapter family requires deterministic fakes or reusable conformance harnesses.
- Event/journal durability: every side effect lowers to existing effect and journal semantics.
- Privacy/redaction: raw content and credentials stay behind policy and refs.
- Replay/idempotency: adapter side effects name intent/result and recovery needs.
- Capability fingerprint impact: package fingerprints include execution-affecting stable refs/sidecars only, not volatile handles.
