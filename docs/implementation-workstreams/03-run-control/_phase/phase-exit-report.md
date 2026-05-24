# Phase 03 Run Control Exit Report

## Status

Reviewer status: PASS.

Phase 03 is complete. The three launch targets were worked in parallel, then
integrated by the orchestrator.

## Delivered Surfaces

- Agent runtime:
  - `crates/agent-sdk-core/src/application/agent.rs`
  - `crates/agent-sdk-core/src/application/runtime.rs`
  - `crates/agent-sdk-core/src/ports/mod.rs`
  - `crates/agent-sdk-core/tests/runtime/runtime_contract.rs`
  - `crates/agent-sdk-core/tests/runtime_contract.rs`
- Loop state:
  - `crates/agent-sdk-core/src/application/loop_state.rs`
  - `crates/agent-sdk-core/src/application/recovery.rs`
  - `crates/agent-sdk-core/tests/runtime/loop_state_contract.rs`
  - `crates/agent-sdk-core/tests/loop_state_contract.rs`
- Run handle:
  - `crates/agent-sdk-core/src/application/run.rs`
  - `crates/agent-sdk-core/src/application/run_handle.rs`
  - `crates/agent-sdk-core/src/ports/subscription.rs`
  - `crates/agent-sdk-core/tests/runtime/run_handle_contract.rs`
  - `crates/agent-sdk-core/tests/run_handle_contract.rs`
- Public facade:
  - `crates/agent-sdk-core/src/lib.rs`

## Contract Evidence

- `Agent`, `AgentBuilder`, `AgentRuntime`, `AgentRuntimeBuilder`,
  `RunHandle`, runtime snapshots, cancellation handles, runtime package
  resolver, provider registry, output sink registry, loop state machine,
  recovery classifier, and subscription test source compile through the public
  facade.
- `AgentRuntime::start_run()` fails closed until required journal, event bus,
  content, policy, package, and provider ports are present.
- Runtime package resolution validates the package, checks the run agent against
  the package agent snapshot, and captures a deterministic fingerprint before a
  run registry entry is inserted.
- Cancellation is idempotent through both `RunHandle::cancel()` and
  `AgentRuntime::cancel_run()`.
- Runtime-created run handles stream through the configured `AgentEventBus`.
  `InMemorySubscriptionHub` remains a reusable test/conformance source for
  replay, cursor expiry, gap diagnostics, and weird reconnect scenarios.
- The loop state machine is pure transition-table logic. It does not call
  providers, tools, journals, event buses, UI, or host callbacks.
- `RunHandle::wait()` resolves only when handle status, sealed terminal journal
  result, and terminal event agree. Visible output alone does not resolve
  `wait()`.
- Cursor compatibility rejects widening, narrowing, or changing all/run/agent
  and filter scopes.

## Validation Commands

- `cargo fmt --check` passed.
- `cargo test -p agent-sdk-core --test runtime_contract` passed, 6 tests.
- `cargo test -p agent-sdk-core --test loop_state_contract` passed, 13 tests.
- `cargo test -p agent-sdk-core --test run_handle_contract` passed, 13 tests.
- `cargo test -p agent-sdk-core` passed, 92 tests plus doc-tests.
- `cargo test -p agent-sdk-core --no-default-features` passed, 92 tests plus
  doc-tests.
- `cargo tree -p agent-sdk-core --no-default-features` passed with only core
  dependencies: `serde`, `serde_json`, `sha2`, and `thiserror` plus transitive
  support crates.

## Audits

- Live-service audit passed with no matches:
  - `rg -n "std::net|TcpStream|UdpSocket|reqwest|hyper|tokio|async-std|smol|rand|thread_rng|SystemTime|Instant::now|Command::new|process::Command" crates/agent-sdk-core/src crates/agent-sdk-core/tests --glob '!**/fixtures/README.md'`
- Product-neutrality audit passed with no matches:
  - `rg -n "Clawdia|ChatGPT|OpenAI|Anthropic|Claude|VS Code|Vercel|iMessage|macOS|Apple|Docker|Firecracker|trace-store|marketplace|host-adapter|live provider" crates/agent-sdk-core Cargo.toml --glob '!target/**'`
- SDK package architecture audit passed:
  - source root contains only `lib.rs` and `README.md`;
  - root integration-test files are Cargo shims only;
  - Phase 03 code lives under `application/`, `ports/`, and
    `tests/runtime/`.
- Raw-provider/cursor ambiguity audit passed with no matches:
  - `rg -n "complete\(&str|legacy_prompts|raw prompt|raw_prompt|ids::EventCursor" crates/agent-sdk-core/src crates/agent-sdk-core/tests`

## Mockability And TDD Evidence

- Runtime tests use fake provider, fake journal, fake content resolver, and
  in-memory event bus without live providers or host runtimes.
- Run-handle tests exercise reconnect, duplicate subscribers, journal replay,
  gap diagnostics, cursor mismatch, idempotent wait/cancel/status, and terminal
  mismatch using deterministic in-memory stores.
- Loop-state tests cover legal transition rows, invalid transition rejection,
  cancellation coverage, guard requirements, recovery classification, and pure
  no-side-effect behavior.
- Public test helpers remain reusable by SDK consumers for their own adapter
  conformance tests.

## Boundary Notes

- This phase intentionally does not execute a provider loop. Phase 04 owns the
  first P0 fake-provider text run and will append actual lifecycle events and
  journal records.
- Runtime `stream_from_journal()` through a runtime-created handle currently
  requires an archive-backed subscription source; direct in-memory replay
  conformance is covered by `InMemorySubscriptionHub`. Phase 04 or replay
  hardening can wire a concrete archive-backed runtime source when the journal
  replay contract grows a read API.
- `LoopEventKind` names are local transition-table names. Future event-schema
  expansion should reconcile them with the shared event enum instead of adding
  a parallel emitted-event family.
- `LoopState::Failed` is modeled as a recoverable/non-terminal state while it
  can carry a failed terminal result. Replay hardening should keep or clarify
  that distinction before relying on terminal-state enumeration.

## Independent Review

Independent reviewer Kuhn reviewed Phase 03 against:

- `coding_standards.md`
- `docs/architecture/coding-standards.md`
- `docs/workstreams/validation-gates.md`
- `docs/reference/sdk-review-checklist.md`
- `docs/contracts/api-contracts.md`
- `docs/contracts/loop-state-machine.md`
- `docs/contracts/run-handle-reconnect-contract.md`
- `docs/contracts/event-schema.md`
- `docs/contracts/journal-replay-schema.md`
- `docs/contracts/runtime-package-schema.md`

The review specifically verified:

- no duplicate event, journal, package, policy, or provider path;
- runtime-created handles use the runtime event bus rather than a second live
  event path;
- loop-state logic remains pure and side-effect-free;
- `wait()` does not resolve before durable terminal agreement;
- mockability/test-support remains reusable for SDK consumers;
- SDK package architecture and root-shim rules are preserved.

Reviewer verdict: PASS, no blocking findings.

Non-blocking reviewer carry-forward:

- Runtime-created handles use the configured `AgentEventBus` for live streams.
  `stream_from_journal()` currently returns `HostConfigurationNeeded` until an
  archive-backed source exists; keep this as a Phase 04/replay-hardening
  follow-up.
- `LoopState::Failed` terminal/recovery semantics should be clarified before
  replay hardening.
