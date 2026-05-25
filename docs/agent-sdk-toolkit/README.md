# Agent SDK Toolkit Adapter Roadmap

This folder plans the optional toolkit and adapter crates that should make the core Agent SDK useful in real hosts without moving host behavior into `agent-sdk-core`.

The core crate is already the primitive kernel: `Agent`, `AgentRuntime`, `RunRequest`, `RuntimePackage`, provider ports, tool ports, isolation ports, journals, events, policy, and the public `agent_sdk_core::testing` namespace. The next layer should be adapter crates that implement those ports with real providers, OpenAI-compatible providers, ACP interoperability, MCP gateways, containerized execution, browser/web access, local accelerators such as MLX and llama.cpp, artifact resolvers, and reusable conformance tests.

## Documents

| Document | Purpose |
| --- | --- |
| [adapter-and-runtime-plan.md](adapter-and-runtime-plan.md) | Phased plan, crate map, high-level APIs, security posture, and validation gates for live providers, OpenAI-compatible providers, ACP, MCP gateways, containerized agents, web access, local MLX/llama.cpp acceleration, and artifact resolution. |
| [workspace-toolkit-plan.md](workspace-toolkit-plan.md) | Workspace read/search/edit/write design, format-aware reader pipeline, implemented PDF/image/RAW/Office/archive behavior, and validation matrix for essential agent file tools. |

## Non-Negotiable Boundaries

- `agent-sdk-core` remains product-neutral and does not import live providers, ACP, concrete containers, product UI, browser automation, or host credential stores.
- Optional adapter crates implement existing ports and lower helpers into `RuntimePackage`, `CapabilitySpec`, `ExecutionEnvironment`, `EffectIntent`, `RunJournal`, and `AgentEvent`.
- Prefer aggregate public adapter crates such as `agent-sdk-provider` and `agent-sdk-isolation` with feature-gated backend modules. Split backends into separate crates only when dependency weight, platform constraints, licensing, release cadence, or SemVer pressure justifies it.
- Website access, shell execution, provider calls, ACP session sends, MCP calls/resource reads, and container/process starts are side effects with policy, journal, event, redaction, and replay semantics.
- Model/runtime artifact downloads are explicit, policy-gated toolkit or host-adapter side effects. Core never silently downloads model weights, provider binaries, browser images, or runtime assets.
- Browser/web access controls model network authority through tool-pack, isolation, credential, action, and observation policy. Browser engines or browser-use-style libraries may sit behind adapters; the SDK does not reimplement browser automation.
- Workspace reading is format-aware. `workspace_read` must detect common text, Markdown, JSON, PDF, image, RAW/DNG, HEIC/AVIF metadata, Office OpenXML, legacy Office fallbacks, ZIP/TAR/TGZ/GZIP archives, SQLite, safe local data URLs, and fallback binary inputs, then route through bounded parser modules instead of treating every file as lossy text.
- Oversized safe workspace files should return a bounded prefix with truncation guidance instead of failing by default. Full-file parser adapters must downgrade to summaries when the parser would need more than the policy cap.
- ACP conformance requires fake clients and fake agents to communicate through the ACP JSON-RPC transport, with stdio as the mandatory first local harness. In-process helper-only fakes do not prove ACP behavior.
- ACP agents may be defined with provider-like ergonomics, but the facade lowers to external-agent primitives rather than a raw model-provider path.
- MCP should be easy to use as a tool source without exposing a whole server. Hosts can attach exact tools, resources, or prompts from a given MCP server, and only those filtered capabilities are model-visible or executable.
- ACP and MCP access across isolation must go through host-edge bridges or policy gateways. Isolated agents do not inherit raw editor handles, MCP server sockets, roots, credentials, session IDs, or ambient discovery.
- Every adapter ships deterministic fakes or conformance harnesses so SDK consumers can test their own integrations without live services.
- OpenAI-compatible providers require explicit endpoint policy, compatibility profiles, capability reports, and fake dialect tests before use.
- Supported provider/model metadata lives in one versioned model-catalog file. Model names, aliases, modalities, reasoning/thinking support, image/audio/tool/structured-output capabilities, limits, status, compatibility notes, and source/probe provenance must not be hardcoded across provider adapters.
- Prebuilt access profiles may make common cases one line, but they never default to `~`, ambient network, host browser credentials, shell, or write access. Hosts provide a workspace ref or explicit policy, and runtime auto-selection must satisfy isolation requirements before optimizing for speed.

## Primitive Simplicity Gates

- Toolkit facades are stateless namespaces or builder helpers only. They may return existing SDK builders, typed sidecars, adapter registrations, `ToolPack`s, or `IsolationRequirement`s; they must not own a run loop, provider registry, package registry, credential store, journal, event bus, policy path, context path, telemetry truth store, or side-effect path.
- Every adapter family must publish a canonical-lowering matrix before implementation. The matrix must name the helper/API, lowered primitive, sidecar or capability, policy/effect/journal path, event/cursor/content refs, and fake/conformance harness.
- A new core primitive, `CapabilitySpec` variant, event family, journal record, or package fingerprint input is not accepted from toolkit work unless it passes the primitive decision ladder and is added to the owning contract before code starts.
- Common APIs should be one-step presets where possible, but the preset output is always ordinary SDK configuration over the same primitive kernel.

## Practical Complexity Guidance

- Ship a small set of opinionated presets first, then expand only when real host or SDK-consumer use cases prove the need.
- Keep preset helpers stateless and minimal. Do not introduce a central authoritative toolkit object.
- Require deterministic conformance and fake-harness coverage before adding new provider, ACP, MCP, browser, or isolation variants.
- Prefer host-managed services for risky local integrations when capability reporting, lifecycle control, credential handling, and cleanup are clearer outside the isolated workload.
- Document one default deployment path for common cases so most users can start from a safe preset instead of assembling providers, MCP, isolation, browser access, and artifact policy manually.
