# Agent SDK Primitive And Parallel Goal Hardening Plan

> Planning note only. Current implementation authority lives in [../contracts/README.md](../contracts/README.md), [../architecture/primitive-map.md](../architecture/primitive-map.md), and [../workstreams/README.md](../workstreams/README.md). If this plan conflicts with those docs, update the authority docs first.

Date: 2026-05-23

## Objective

Make the SDK packet simpler to implement without cutting features by defining a compact primitive kernel, showing how the larger feature set layers on top of that kernel, and tightening the workstreams so Codex goals can run in parallel with clear validation and review criteria.

## Behavior Contract

New behavior:

- A small MVP Rust slice names the few primitives that must exist before higher-level tools, memory, realtime, isolation, subagents, extensions, and telemetry can expand.
- The primitive map distinguishes kernel primitives, feature-layer primitives, optional adapter crates, and host-owned behavior.
- Workstreams include both documentation ownership and future implementation ownership so parallel Codex goals know exactly what they can touch once code exists.
- Each workstream handoff includes a primitive-lowering review: the worker must prove it reused kernel primitives instead of inventing a parallel concept.
- Event fixtures are required only for event kinds emitted by the slice under implementation; the full event taxonomy remains reserved until a workstream emits those kinds.
- Historical plans and coverage audits cannot override the active contracts/workstreams.

Preserved behavior:

- SDK core remains product-neutral.
- Ergonomic helpers remain thin lowering layers into canonical contracts.
- Observability, journal durability, lineage, privacy, policy, recovery, isolation, extension, subagent, stream-rule, typed-output, and telemetry features stay in the packet.
- One stitching owner remains serialized and owns shared names, IDs, event/journal alignment, runtime-package fingerprint inputs, public indices, and final validation.

Removed behavior:

- No workstream should create its own mini-runtime or hidden registry because a needed primitive was vague.
- No first slice should require the entire feature taxonomy, all runtime package variants, or every event kind before fake-provider runs can work.
- No product-specific package names, local absolute paths, or host adapter assumptions should appear in normative SDK contracts.

## Primitive Shape To Enforce

The MVP kernel should be just enough to prove one fake-provider text or typed run:

- `Agent`, `AgentRuntime`, `RunRequest`, `RunHandle`, and `RunResult` for starting, observing, cancelling, and completing runs.
- `RuntimePackage` as the per-run immutable capability snapshot.
- `CapabilitySpec` with only the first-slice variants needed for provider route and fake tool execution.
- `AgentMessage`, `ContextItem`, and `ContextProjection` for context assembly.
- `OutputContract` and `ValidatedOutput` for typed output.
- `AgentEvent`, `EventFrame`, `EventFilter`, and `EventCursor` for live observability.
- `RunJournal`, `JournalRecord`, and `JournalCursor` for durable truth.
- Typed ports for provider, fake tool execution, approval policy decisions, and host output delivery.
- `SourceRef`, `DestinationRef`, `PolicyRef`, `PrivacyClass`, and typed IDs for cross-boundary lineage.

Reserved feature ports exist in contracts but do not need concrete runtime behavior in the MVP slice: telemetry exporters, isolation adapters, extension bridges, subagents, realtime providers, stream rules, full tool packs, and global event archive replay.

Everything else must be a layer:

- Tool packs are presets that install `CapabilitySpec` plus policy and executor refs.
- Hooks are `CapabilitySpec` plus ordered hook executor refs.
- Stream rules are package capabilities plus event/journal-backed interventions.
- Structured output is `OutputContract` plus validator/repair policy over the normal run loop.
- Memory and compaction create `ContextItem` and journal records; they do not bypass context projection.
- Isolation is a typed environment requirement plus adapter port; concrete runtimes stay optional or host-owned.
- Subagents are parent-owned child runs with stripped runtime packages and wrapped events.
- Extensions declare capabilities that hosts resolve into runtime package capabilities after policy checks.
- Telemetry is a projection from events/journal/usage records, not the durable source of truth.
- Output delivery is a destination/sink port with dedupe and intent records; product channel UX stays host-owned.

`CapabilitySpec` is not a free-form bag. Every variant beyond the MVP profile must name its typed sidecar contract, owner workstream, fingerprint fields, emitted events, journal records, and acceptance tests before any adapter can emit or execute it.

## Scope

- Update navigation and start-here docs to make the primitive kernel and first Rust slice explicit.
- Harden `docs/architecture/primitive-map.md` so the feature list layers over a smaller kernel.
- Harden `docs/contracts/api-contracts.md`, `runtime-package-schema.md`, `event-schema.md`, `extension-sdk-contract.md`, the context/output-delivery contracts, and contract indices where they currently imply too much first-slice breadth or product-specific leakage.
- Update workstream docs and validation gates so parallel Codex goals have disjoint ownership, primitive-lowering checks, future code/test scopes, and review packets.
- Demote stale historical plan/workstream language in reference docs.

## Validation Plan

- Markdown link audit over `/Users/clawdia/clawdia_sdk`.
- Workstream ownership audit for duplicated writable documentation files, shared architecture/reference edits, and future implementation scope collisions.
- Product-neutrality grep for known stale host/product/local terms in normative architecture and contract docs.
- Primitive audit proving every workstream has a primitive-lowering/reuse criterion.
- Event fixture audit proving the docs say fixtures are required for emitted slice kinds, with the remaining taxonomy reserved.
- Independent plan review before editing.
- Independent implementation review after editing.

## Risks / Gotchas

- Do not simplify by deleting required features. Simplify by making features layers over fewer stable primitives.
- Do not turn `RuntimePackage` into an untyped bag. `CapabilitySpec` is the shared entry, but higher-level builders may still expose typed helpers.
- Do not let multiple parallel goals write shared proposal/reference files. Non-stitching goals produce proposal blocks in their handoff; stitching reconciles them.
- Do not make `AgentRuntime` own one global package forever. It may hold default ports and policies, but the effective package is resolved per run before execution.
- Do not let event taxonomy freeze block the first fake-provider run. Only emitted kinds need fixtures in the first slice.
- Do not let scenario coverage write core contracts directly. Scenario gaps become product-neutral primitive proposals for stitching.
