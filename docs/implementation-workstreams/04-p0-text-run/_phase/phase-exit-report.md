# Phase 04 P0 Text Run Exit Report

## Status

Reviewer status: PASS.

Phase 04 is complete. This was the only launch target in the phase.

## Delivered Surfaces

- P0 loop driver:
  - `crates/agent-sdk-core/src/application/loop_driver.rs`
- Public run APIs:
  - `crates/agent-sdk-core/src/application/agent.rs`
  - `crates/agent-sdk-core/src/application/run.rs`
  - `crates/agent-sdk-core/src/application/runtime.rs`
  - `crates/agent-sdk-core/src/lib.rs`
- Durable/event records:
  - `crates/agent-sdk-core/src/records/event.rs`
  - `crates/agent-sdk-core/src/records/journal.rs`
  - `crates/agent-sdk-core/src/ports/event_bus.rs`
- Tests and fixtures:
  - `crates/agent-sdk-core/tests/p0/p0_text_run.rs`
  - `crates/agent-sdk-core/tests/p0_text_run.rs`
  - `crates/agent-sdk-core/tests/fixtures/p0/text-run-events.json`
  - `crates/agent-sdk-core/tests/fixtures/p0/text-run-journal.json`

## Contract Evidence

- `Agent::run_text(...)` lowers into the same canonical `RunRequest::text(...)`
  shape used by `AgentRuntime::run_text(...)`.
- `AgentRuntime::run_text(...)` enters through `AgentRuntime::start_run(...)`
  before executing the P0 loop, so package resolution, policy, provider-port
  presence, run registration, cancellation shell, and handle creation share the
  Phase 03 path.
- Context projection happens before provider request projection. The fake
  provider receives a typed `ProviderRequest`, not a raw prompt shortcut.
- A model-attempt journal record is appended before the provider call, then the
  provider response records model completion, message completion, and terminal
  run result.
- Events are emitted through the configured `AgentEventBus`; no private live
  event path was added.
- `RunHandle::wait()` returns only after terminal journal result and terminal
  event agree.
- P0 does not require tools, approvals, isolation, extensions, subagents,
  realtime, telemetry exporters, output delivery, or typed output.

## Validation Commands

- `cargo fmt --check` passed.
- `cargo test -p agent-sdk-core --test p0_text_run` passed, 3 tests.
- `cargo test -p agent-sdk-core` passed, 95 tests plus doc-tests.
- `cargo test -p agent-sdk-core --no-default-features` passed, 95 tests plus
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
  - P0 code lives under `application/`, `records/`, `ports/`, and
    `tests/p0/`.
- P0 scope audit passed with no matches:
  - `rg -n "Approval|Tool|Isolation|Subagent|Telemetry|OutputDispatch|StructuredOutput|Realtime|Extension|Hook" crates/agent-sdk-core/src/application/loop_driver.rs crates/agent-sdk-core/tests/p0 crates/agent-sdk-core/tests/fixtures/p0`

## Mockability And TDD Evidence

- Tests run entirely with `FakeProvider`, `FakeJournalStore`,
  `FakeContentResolver`, and `InMemoryAgentEventBus`.
- Golden P0 event and journal summaries prove stable emitted lifecycle and
  durable record ordering.
- Two-run cursor regression coverage proves the event bus assigns monotonic
  live stream sequences across runs for all-stream and agent-stream resume.
- The fake provider records typed `ProviderRequest` values so SDK consumers can
  reuse the same shape in their own adapter conformance tests.

## Boundary Notes

- Failure-path terminal repair remains for later phases. P0 proves the smallest
  successful fake-provider text run and preserves fail-closed start behavior
  from Phase 03.
- P0 journal payloads add minimal durable DTOs for run lifecycle, context
  projection, model attempts, and assistant message completion. Later phases
  should extend these DTOs instead of adding parallel journal record families.
- `EventKind::ProviderRequestProjected` was added to the shared event enum to
  keep P0 events aligned with the API and event contracts.

## Reviewer Request

Initial independent reviewer verdict: BLOCKED.

Resolved reviewer findings:

- Provider request side effect was originally represented only as a
  `ModelAttemptRecord`. Fixed by appending a shared
  `JournalRecordPayload::EffectIntent` with `EffectKind::ProviderRequest`
  before `provider.complete(...)` and a matching
  `JournalRecordPayload::EffectResult` after the provider returns. The P0
  golden journal fixture now includes both records.
- P0 event sequence numbers were originally run-local. Fixed by having
  `InMemoryAgentEventBus` assign monotonic live stream sequence numbers on
  publish and adding a two-run all/agent cursor resume regression test.

Re-review verdict: PASS, no blocking findings.

Non-blocking reviewer carry-forward:

- Before true concurrent publishers are introduced, make sequence assignment
  and push ordering atomic or add a concurrent publisher stress test. Current
  P0 is synchronous and the serial multi-run cursor gate passes.

Please review Phase 04 against:

- `coding_standards.md`
- `docs/architecture/coding-standards.md`
- `docs/workstreams/validation-gates.md`
- `docs/reference/sdk-review-checklist.md`
- `docs/contracts/api-contracts.md`
- `docs/contracts/loop-state-machine.md`
- `docs/contracts/runtime-package-schema.md`
- `docs/contracts/event-schema.md`
- `docs/contracts/journal-replay-schema.md`
- `docs/contracts/context-memory-contract.md`
- `docs/implementation-workstreams/04-p0-text-run/04a-text-run.md`

The review should specifically verify:

- P0 uses the canonical runtime/package/policy/context/provider/event/journal
  path and does not create a second mini-runner;
- provider execution is preceded by journaled provider-request intent;
- events are journal-backed and emitted through `AgentEventBus`;
- terminal `RunResult` agrees with event and journal state;
- P0 does not introduce P1/P2 or optional feature requirements;
- SDK package architecture, mockability/testability, product-neutrality, and
  no-live-service gates are preserved.
