# Agent SDK Coding Standards

These standards apply to the future Rust-first `agent_sdk` design. They are grounded in TDD, SDK package design, Rust async practice, observability requirements, and DDD where it clarifies the domain vocabulary.

## Architecture Standards

- Design from domain primitives first: run, runtime package, content/artifact refs, context contribution/admission/projection, model call, effect intent/result, policy, event, journal, and typed ports.
- Keep `agent_sdk` product-neutral. UI shells, coding-agent harnesses, remote messaging products, desktop windows, and cloud sandboxes belong in adapters or host crates.
- Keep product-specific decisions host-owned: provider selection UI, external-runtime session caches, restore keys, trace ingestion policy, extension runtime packaging, and approval transport.
- Prefer typed snapshots over ambient lookup. `RuntimePackage` should be the source of truth for model-visible tools, executable tool registry, hooks, policies, MCP exposure, subagent definitions, and extension capabilities.
- Prefer explicit state machines over hidden recursion. Recursive continuation is allowed, but it must be represented as loop state and observable events.
- Keep lossless internal records separate from provider projection. Provider adapters may drop or transform fields only through explicit projection steps.
- Treat APIs as contracts, not convenience helpers. Every public type needs a clear owner, methods, invariants, and "must not own" boundaries.
- Model failure as part of the domain: cancellation, timeout, max iterations, context overflow, interrupted tools, rejected approvals, bad provider chunks, partial streams, replay conflicts, and recovery attempts.

## Test-First Standards

The current workspace is documentation-only, but implementation should start with tests before code.

- Mockability is non-negotiable. Every public port, adapter boundary, policy edge, side-effect path, and scenario surface must be easy to fake, script, and assert against without live providers, real containers, product UI, network telemetry, wall-clock time, or random IDs.
- The SDK should ship reusable test-support helpers so downstream SDK users can validate their own providers, tools, output sinks, isolation runtimes, extension bridges, telemetry sinks, and host adapters against the same contract shapes used by the SDK.
- Tests must include weird and hostile scenarios, not only happy paths: missing adapters, malformed provider chunks, invalid schemas, slow subscribers, denied policies, journal append failures, duplicate delivery, replay gaps, cancellation races, timeout edges, and privacy/content-capture denial.
- Start with small Rust unit tests for state transitions: input accepted, model requested, tool requested, approval required, tool result appended, continue, stop, cancel, compact, retry, and recover.
- Add golden event tests for every `AgentEvent` variant so event names, causal IDs, and redaction rules remain stable.
- Add provider-projection tests that prove internal metadata is stripped or transformed exactly once before a provider call.
- Add structured-output tests that pass user/host schemas, validate locally, retry with repair prompts on invalid output, and commit only validated typed results.
- Add stream-rule tests for literal/regex matching over assistant text, provider-exposed reasoning summaries, tool-call arguments, tool results, and realtime transcript channels. Tests should prove stop, abort-and-retry, mask, approval, repeat policy, resume state, and redacted match events.
- Add built-in tool-pack contract tests with mock workspaces: read/search anchors, truncation metadata, glob handling, edit precondition failures, reversible-effect metadata, write approval, shell sandbox denial, resource URI resolution, and tool discovery activation.
- Add isolation-runtime contract tests with fake adapters: environment capability negotiation, image/rootfs resolution, mount expansion audits, network denial, process I/O redaction, signal/timeout handling, stats, cleanup, and resume after partial lifecycle states.
- Add session/replay tests using append-only journals, deterministic IDs, and mock providers.
- Add policy tests for approval modes, permission modes, sandbox modes, MCP allowlists, source-scoped approvals, and escalation timeouts.
- Add subagent tests for bounded depth, no recursive subagent tools by default, parent-owned cancellation, event wrapping, and trace lineage.
- Add realtime tests with bounded channels and fake audio/image/text streams to prove backpressure and restart behavior.
- Add extension SDK smoke tests for public subpath exports, packaged fallback ordering, browser-safe helpers, and temp-directory execution.
- Keep mock providers fast and deterministic. Live-provider tests should be opt-in and never required for local unit confidence.

## Mockability And SDK Test Kit Gate

- A contract is not complete until a host or SDK consumer can mock its required ports and run the same core conformance assertions without product-specific infrastructure.
- Every port trait should have a deterministic fake or conformance helper before concrete adapters are treated as ready.
- Fakes must be transparent: they exercise the same public contracts as production paths and must not hide behavior in fake-only shortcuts.
- Test-support utilities should normalize IDs, timestamps, ordering, redaction, and fixture output so failures are stable and reviewable.
- E2E scenario tests should compose fakes across provider, journal, event bus, policy, content, output, and side-effect boundaries so unusual interleavings are easy to reproduce.
- Any launch target that cannot yet provide reusable mock/conformance support must record that as a blocking or explicitly deferred gap in its phase exit report.

## Domain Modeling Standards

- The SDK should use domain modeling where it improves the public contract, not as ceremony. `RunId`, `TurnId`, `MessageId`, `ContextItemId`, `ToolCallId`, `SpanId`, `AgentId`, `SessionId`, `RuntimePackageId`, and `LineageRef` are vocabulary primitives because SDK users pass and persist them.
- Runtime/application code coordinates a run using domain services and ports. It should not parse strings to infer tool risk, provider routing, message source, or approval state.
- Adapter and infrastructure crates implement provider adapters, MCP clients, file stores, remote channels, OTLP exporters, process runners, and extension hosts.
- Host/application crates decide which adapters, policies, and sinks are active.
- Extension APIs expose bounded capabilities. Extensions can observe and propose, but cannot silently become approval, memory, provider-routing, or telemetry owners.

## SDK Package Architecture Gate

Rust crates and tests must be easy for SDK users and contributors to navigate. The layout should follow Cargo conventions, keep the public facade stable, and group internals by SDK responsibility. A phase is not ready for review when important concepts are scattered as unrelated flat files.

Reference points:

- [Cargo package layout](https://doc.rust-lang.org/cargo/guide/project-layout.html) keeps package roots, integration tests, examples, benches, and multi-file test modules predictable.
- [Cargo integration tests](https://doc.rust-lang.org/stable/cargo/guide/tests.html) are discovered from `tests/`; stable root test targets may delegate to submodules for larger suites.
- [Rust API Guidelines documentation](https://rust-lang.github.io/api-guidelines/documentation.html) expects crate-level docs and examples to make public APIs discoverable.

Patterns from mature SDKs:

- [AWS SDK for Rust](https://awslabs.github.io/aws-sdk-rust/) separates one crate per service from shared configuration/runtime crates; AWS also documents Smithy runtime and runtime-API crates as shared building blocks rather than service-specific logic.
- [AWS SDK for JavaScript v3](https://github.com/aws/aws-sdk-js-v3) separates generated service clients, hand-written support packages, higher-level libraries, internal/private packages, code generation, and scripts. It also treats package-level imports as the supported public surface rather than deep file paths.
- [Stripe Go](https://github.com/stripe/stripe-go) exposes a client entry point plus resource packages, keeps backend injection explicit for tests, and documents mocking through a backend interface.
- [Kubernetes client-go](https://github.com/kubernetes/client-go) keeps API access, discovery, informers, listers, REST transport, tools, examples, and testing/fake support in distinct top-level packages.
- [Twilio Go](https://github.com/twilio/twilio-go) separates the top-level client, generated REST resources, TwiML helpers, client/JWT helpers, examples, and testing workflows.

The SDK should copy the underlying lesson, not the exact folder names: keep public entry points stable, keep generated/spec-derived material separate from hand-written runtime code, keep transport/ports separate from records and package authority, and keep reusable test/fake support visible rather than hidden in ad hoc tests.

For this workspace, the definite package shape is:

```text
crates/
  agent-sdk-core/
    src/
      lib.rs              # public facade and re-exports only
      domain/             # ubiquitous language: IDs, refs, policy, privacy, errors
      package/            # RuntimePackage authority, capabilities, sidecars, fingerprints
      records/            # durable journals, events, effects, context/content/output records
      ports/              # public trait boundaries; no concrete host infrastructure
      application/        # runtime coordination, state machines, canonical lowering
      testing/            # reusable fake adapters, fixtures, conformance helpers
    tests/
      <stable-shim>.rs    # optional two-line Cargo test targets only
      domain/
      package/
      records/
      ports/
      runtime/
      feature_layers/
      testing/
      fixtures/
  agent-sdk-<optional>/
    src/lib.rs            # optional crate facade for concrete or higher-level packs
```

That shape is intentionally DDD-flavored without becoming ceremony. Domain names live in `domain`; durable facts live in `records`; side-effect and external boundaries live as `ports`; orchestration and helper lowering live in `application`; runtime-package authority lives in `package`; test-kit support lives in `testing`. A feature that cannot be placed cleanly in one of those buckets is a design review signal, not permission to add a catch-all folder.

Generated, spec-derived, or schema-derived code must not be mixed into hand-written runtime modules. If code generation becomes necessary, put generated/spec material behind a clearly named crate or module boundary, keep hand-written adapters and domain logic separate, and document the public facade that SDK users should import. Deep generated paths are not a stable public API.

Public API structure is part of the package contract:

- `src/lib.rs` is a public facade, not an implementation home. It may re-export stable public types, host-facing traits, and documented namespaces.
- New `#[path] pub mod ...` facade aliases, new deep-import module names, or newly public modules require explicit SemVer/API review in the phase exit report.
- The supported import surface must be documented before release readiness. Deep implementation paths are unstable unless the release matrix explicitly blesses them.
- SDK-consumer test helpers must live under one documented public namespace, `agent_sdk_core::testing`, with import-smoke coverage. Legacy flat exports may exist only as documented compatibility shims with a removal or stabilization decision.
- `Fake*`, `Scripted*`, and conformance harness types belong in `src/testing/` unless they are explicitly documented as production reference implementations. Ports define behavior; testing modules provide scripted implementations of those ports.
- `records/` must not own adapter traits, resolver traits, fakes, scripted helpers, or conformance harnesses. If a record module exposes a trait, the phase exit report must justify why it is a pure durable-record abstraction instead of a port.

For `agent-sdk-core`, source files should stay in these ownership folders:

| Folder | Owns | Must not own |
| --- | --- | --- |
| `src/domain/` | IDs, refs, privacy/retention/trust, typed errors, policy decisions, and other ubiquitous-language primitives. | Provider transport, run orchestration, concrete adapters, or feature workflow behavior. |
| `src/package/` | Runtime package authority, capability specs, sidecars, catalogs, deltas, and fingerprint inputs. | Live discovery, host install state, or ambient registries. |
| `src/records/` | Durable and observable records: events, journals, effects, content/context projection, and output records. | Provider clients, UI decisions, or runtime execution loops. |
| `src/ports/` | Public trait/port boundaries for providers, event buses, archives, stores, sinks, and future adapters. | Concrete host infrastructure or fake-only shortcuts. |
| `src/application/` or `src/runtime/` | Agent/runtime/run coordination, state machines, projection orchestration, recovery, and canonical lowering helpers. | Domain vocabulary definitions or infrastructure implementations. |
| `src/testing/` | Deterministic fakes, fixtures, conformance harnesses, and SDK-consumer test-kit helpers. | Production-only control flow or fake-only behavior paths that bypass public contracts. |

The crate root may keep public facade modules for compatibility and Rustdoc discovery, but those facades should point into the owning SDK responsibility folder. New implementation files belong in the folder for their SDK responsibility before code review begins.

Integration tests should mirror the same ownership:

| Folder | Owns |
| --- | --- |
| `tests/domain/` | ID/ref/policy/privacy/error contract tests. |
| `tests/package/` | Runtime package, capability, fingerprint, and catalog tests. |
| `tests/records/` | Event, journal, context/content, output, and durable-record tests. |
| `tests/ports/` | Provider/event bus/sink/archive port conformance tests. |
| `tests/runtime/` | Agent/runtime/run-handle/state-machine integration tests. |
| `tests/testing/` | Fake fixture harness and SDK test-kit tests. |

Small root-level Cargo test-target shims are allowed when launch docs call a stable `cargo test --test <name>` target. They must contain only module wiring; the real test body belongs in the matching SDK responsibility folder.

Reviewers must block:

- new flat source files under `src/` when an owning SDK responsibility folder exists;
- new full test bodies directly under `tests/` instead of the matching SDK responsibility folder;
- modules that mix domain records, application orchestration, concrete adapters, and fake behavior in one file;
- public fakes or helpers that bypass the production port or record contract they are supposed to test;
- package/fingerprint code that becomes hard to review because canonicalization or authority is split across unrelated folders.

Every implementation phase must include mandatory source-layout evidence in its exit report:

```bash
find crates/agent-sdk-core/src -maxdepth 1 -type f -not -name lib.rs -not -name README.md
find crates/agent-sdk-core/tests -maxdepth 1 -type f -name '*.rs' -print -exec sh -c 'wc -l "$1"' sh {} \;
find crates -path '*/src/*.rs' -maxdepth 3 -type f
rg -n '#\\[path = .*\\]\\s*pub mod|pub mod [a-zA-Z0-9_]+;' crates/agent-sdk-core/src/lib.rs
rg -n '\\b(Fake|Scripted)[A-Za-z0-9_]+|ConformanceHarness' crates/agent-sdk-core/src --glob '*.rs'
rg -n '\\btrait\\b|\\bAdapter\\b|\\bResolver\\b|\\bFake\\b|\\bScripted\\b|ConformanceHarness' crates/agent-sdk-core/src/records --glob '*.rs'
wc -l crates/agent-sdk-*/src/lib.rs
```

The first command must be empty. The second command must show only stable Cargo shims, normally two lines that delegate into the responsibility folders. The `lib.rs` module audit must explain any newly public deep module. The fake/scripted audit must either point to `src/testing` or identify a production reference implementation. The records audit must be empty except for durable-record-only traits justified in the report. Optional crate `src/lib.rs` files should stay as small facades; implementation creep there is blocking.

## Rust API Standards

- Use `Result<T, AgentError>` for fallible SDK operations. Error variants should preserve typed context and causal IDs.
- Prefer enums for finite state: stop reasons, approval decisions, permission outcomes, stream item kinds, recovery actions, and policy decisions.
- Use newtype IDs instead of raw strings at public boundaries.
- Use `serde` for durable contracts and maintain schema versions for journals, events, snapshots, and runtime packages.
- Prefer `Arc<dyn Trait + Send + Sync>` only at stable dynamic boundaries such as providers, stores, hooks, and telemetry sinks. Use generic/static composition inside hot paths where practical.
- Avoid cloning large message/media payloads in loops. Use IDs, `Arc`, borrowed projections, or media references where possible.
- Separate hot event emission from slow sink delivery. The loop should enqueue lightweight events and never block on network telemetry export.
- Make cancellation explicit and cheap to check. Long-running provider streams, tools, subagents, and realtime tasks must receive cancellation handles.
- Require bounded channels for streaming and realtime paths unless a host explicitly opts into an unbounded sink for testing.

## Observability Standards

Observability is a core SDK feature, not a plugin.

- Every run has a `RunId`, trace context, root agent identity, runtime package fingerprint, and host source.
- Every turn has a `TurnId`, parent `RunId`, input summary, context projection ID, and outcome.
- Every message has a `MessageId`, role, parts, lineage, sensitivity, retention, and provider-projection status.
- Every context item comes from a `ContextContribution` and carries source, injection path, selection decision, policy refs, privacy, retention, trust, and lineage: user, system, developer, memory, compaction, hook, extension, tool result, remote channel, scheduled task, subagent, external runtime, or replay.
- Every event has causal and filterable links: `run_id`, `agent_id`, optional `turn_id`, optional `attempt_id`, optional `message_id`, optional `context_item_id`, `span_id`, optional `parent_event_id`, `subject_ref`, `related_refs`, `causal_refs`, source, destination, correlation keys, tags, privacy class, and delivery semantics. Tool calls, hooks, approvals, subagents, isolated processes, extension actions, output deliveries, and effects use `EntityRef` rather than feature-specific envelope IDs.
- Events should support content elision by default. The event must still be useful when raw content is absent.
- Telemetry must align with OpenTelemetry GenAI concepts for agent, model, tool, usage, MCP, and errors while retaining SDK-specific lineage fields.
- Tool and model retries must be observable as attempts, not overwritten history.
- Multi-agent handoffs must preserve who instructed whom, what message/context was passed, and which policy allowed the handoff.
- Message metadata must never leak to provider adapters unless a projection explicitly maps a safe subset.
- Structured output validation must happen inside the SDK after provider output, even when the provider claims native schema support. Validation failures and repair retries must be observable as attempts.
- Stream-rule matching and interventions must be observable as their own events. A stopped or retried model attempt should explain which rule matched, which channel it watched, which action was applied, and how matched content was redacted.
- Isolated execution must emit environment lifecycle and process events. A shell/code-execution result should explain the adapter, capability report, image/rootfs refs, mount/network policy, process status, stats, and cleanup status without logging raw process I/O by default.

## Approval And Policy Standards

- Approval is a broker/policy decision, not a UI event.
- Permission, sandbox, MCP allowlist, allowed tools, YOLO/autonomy, and escalation are separate policy layers.
- Decisions are finite: allow, deny, ask, modify, defer, or interrupt.
- Desktop, CLI, external-runtime, headless, extension-submitted, and source-scoped contexts use the same decision model with different dispatchers.
- Headless approval parks the broker receiver, uses explicit host-owned escalation channels, accepts exact finite tokens, and denies on timeout.
- Extensions can request tools and observe decisions only through host-owned APIs. They cannot approve themselves.

## Performance And Robustness Standards

- Event emission must be low allocation and non-blocking on slow observers.
- Streaming must be backpressure-aware. If a sink is slow, apply a declared overflow policy: block, drop noncritical progress, summarize, or fail.
- Context assembly must be bounded by token, byte, item count, media budget, and time budget.
- Tool execution must have explicit concurrency, ordering, timeout, retry, and cancellation policies.
- Realtime media streams must use references or bounded buffers instead of copying large base64 payloads through every layer.
- Streaming regex/literal matchers must use bounded rolling windows, compile-time validation, timeout/backtracking protection, and channel-specific privacy controls.
- Built-in tool packs must be bounded by workspace, file size, match count, byte limits, process timeout, and cancellation policy. Mutating packs must journal intent and before/after effect metadata before applying changes.
- Isolation adapters must declare health and capabilities before use. Environment preparation, image pulls, rootfs creation, first-run kernel/init artifact fetches, process startup, stats collection, and cleanup must be bounded, cancellable where possible, and journaled.
- Journals and checkpoints should be append-first and compact later.
- Recovery should be explicit: replay from journal, resume from checkpoint, regenerate provider projection, retry safe step, or surface repair-needed.
- Invariant checking and anti-entropy jobs should be separate from the hot loop.

## Documentation Standards

- Every future implementation slice should update the owning architecture, contract, or decision-register docs before code lands.
- Conceptual Rust examples in docs must be labeled non-compiling sketches until they become crate APIs.
- Any change to public events, journals, snapshots, or extension contracts needs a compatibility note and strategy.
- Source links in docs should prefer primary docs, repos, or specifications.
