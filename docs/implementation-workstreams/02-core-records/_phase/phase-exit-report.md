# Phase 02 Exit Report: Core Records

Date: 2026-05-24

## Objective And Dependency Status

Phase 02 implemented durable and observable core records after Phase 01 exited with reviewer PASS WITH NOTES and its public-constructor validation note was resolved.

The Phase 00 provider-port placeholder warning is resolved in this phase: `ProviderAdapter` now accepts typed `ProviderRequest` values produced from admitted `ContextProjection` records instead of raw prompt strings.

The user also identified package/test organization as a blocking SDK package-architecture issue during review. Coding standards and validation gates now include an SDK package architecture gate, and the crate/test layout was refactored into responsibility folders before Phase 03 could start.

## Goal Status

| Goal | Status | Changed files |
| --- | --- | --- |
| `02a-runtime-package.md` | complete | `crates/agent-sdk-core/src/package/**`, `crates/agent-sdk-core/tests/package/runtime_package_contract.rs`, root Cargo test shim, fixtures under `crates/agent-sdk-core/tests/fixtures/package/`, exports |
| `02b-event-frames.md` | complete | `crates/agent-sdk-core/src/records/event.rs`, `crates/agent-sdk-core/src/ports/event_bus.rs`, `crates/agent-sdk-core/src/records/events.rs`, `crates/agent-sdk-core/tests/records/event_contract.rs`, root Cargo test shim, fixtures under `crates/agent-sdk-core/tests/fixtures/events/`, exports |
| `02c-run-journal.md` | complete | `crates/agent-sdk-core/src/records/journal.rs`, `crates/agent-sdk-core/src/records/effect.rs`, `crates/agent-sdk-core/tests/records/journal_contract.rs`, root Cargo test shim, fixtures under `crates/agent-sdk-core/tests/fixtures/journal/`, exports |
| `02d-content-context.md` | complete | `crates/agent-sdk-core/src/records/content.rs`, `crates/agent-sdk-core/src/records/context.rs`, `crates/agent-sdk-core/tests/records/context_contract.rs`, root Cargo test shim, fixture under `crates/agent-sdk-core/tests/fixtures/context/`, exports |
| `02e-provider-port.md` | complete | `crates/agent-sdk-core/src/ports/provider.rs`, `crates/agent-sdk-core/src/application/projection.rs`, `crates/agent-sdk-core/src/ports/providers.rs`, `crates/agent-sdk-core/tests/ports/provider_projection_contract.rs`, root Cargo test shim, fixture under `crates/agent-sdk-core/tests/fixtures/provider/`, fake provider integration |

## Validation Evidence

- PASS: `cargo fmt --check`
- PASS: `cargo test -p agent-sdk-core --test runtime_package_contract` (7 tests)
- PASS: `cargo test -p agent-sdk-core --test event_contract` (9 tests)
- PASS: `cargo test -p agent-sdk-core --test journal_contract` (6 tests)
- PASS: `cargo test -p agent-sdk-core --test context_contract` (10 tests)
- PASS: `cargo test -p agent-sdk-core --test provider_projection_contract` (4 tests)
- PASS: `cargo test -p agent-sdk-core` (60 tests)
- PASS: `cargo test -p agent-sdk-core --no-default-features` (60 tests)
- PASS: `cargo tree -p agent-sdk-core --no-default-features` showed only core dependencies (`serde`, `serde_json`, `sha2`, `thiserror`) and their transitive support crates.
- PASS: no-live-service audit with `rg -n "std::net|TcpStream|UdpSocket|reqwest|hyper|tokio|async-std|smol|rand|thread_rng|SystemTime|Instant::now|Command::new|process::Command" crates/agent-sdk-core/src crates/agent-sdk-core/tests --glob '!**/fixtures/README.md'` returned no matches.
- PASS: product-neutrality audit with `rg -n "Clawdia|ChatGPT|OpenAI|Anthropic|Claude|VS Code|Vercel|iMessage|macOS|Apple|Docker|Firecracker|trace-store|marketplace|host-adapter|live provider" crates/agent-sdk-core Cargo.toml --glob '!target/**'` returned no matches.
- PASS: raw-provider-bypass/cursor-ambiguity audit with `rg -n "legacy_prompts|complete\\(\\&self, prompt|pub struct EventCursor\\(String\\)|EventCursor\\(redacted\\)|ids::EventCursor" crates/agent-sdk-core/src crates/agent-sdk-core/tests` returned no matches.
- PASS: SDK source-root audit with `find crates/agent-sdk-core/src -maxdepth 1 -type f ! -name lib.rs ! -name README.md -print` returned no files.
- PASS: SDK root-test-body audit with `rg -n "#\\[test\\]|fn [a-zA-Z0-9_]+\\(" crates/agent-sdk-core/tests/*.rs` returned no matches; root integration files are Cargo target shims only.

## Goal Handoffs

Runtime package:

- Added `RuntimePackage`, package builders, catalog snapshots, package deltas, readiness profiles, typed sidecar and executor refs, provider capability snapshots, output sink snapshots, policy snapshots, and deterministic fingerprint inputs.
- Reserved capability variants stay inactive and cannot project or execute.
- Golden fixtures cover canonical package snapshots and fingerprints.
- Nested execution-affecting lists are canonicalized before fingerprinting, including capability sidecar refs, package sidecar refs, sidecar policy refs, and catalog candidates.

Event frames:

- Added canonical event envelopes, event frames, stream cursors, archive cursors, event filters, compiled filters, overflow notices, event bus fanout, and envelope-only payload defaults.
- Filter matching uses envelope/index fields only and does not inspect payload JSON, content stores, or journals.
- Subscription streams rewrite returned cursors to the requested logical stream scope so run/agent/filter subscribers can safely resume from the cursor they receive.
- Golden fixtures cover implemented event kinds, model-stream redaction, archive cursor distinction, and overflow repair markers.

Run journal:

- Added append-only journal records, journal cursors, checkpoints, terminal result markers, recovery markers, pending side-effect records, effect intent/result DTOs, and append-before-effect guard helpers.
- Failure cases cover append failure preventing effect execution and result append failure entering recovery before the next effect.
- Golden fixtures cover implemented journal record kinds.

Content context:

- Added artifact/content references, content resolution policy, fake content resolver, context contributions, admitted context items, projected provider context, selection decisions, and projection audit records.
- Missing required content blocks projection, optional missing content records an omission, and raw content resolution requires explicit opt-in.
- Golden fixture records redacted projection audit counts without raw content.

Provider port:

- Replaced the Phase 00 raw prompt placeholder with a typed `ProviderAdapter` port over `ProviderRequest` and `ProviderResponse`.
- Provider requests project from admitted `ContextProjection` records only and strip private metadata unless policy explicitly allows projection.
- Deterministic fake provider records typed requests and exposes reusable conformance cases without network access. The former fake-only raw prompt shortcut was removed.

## Mockability / SDK Test Kit Gate

- Runtime package, event, journal, content/context, and provider surfaces all include deterministic fixtures or fakes that SDK consumers can reuse for conformance tests.
- Weird and hostile cases are covered through no-live fake providers, missing content resolution, append failure recovery, event overflow repair, reserved capability enforcement, private metadata stripping, and no-default feature tests.
- Fakes lower through the same public traits and DTOs that real hosts implement; there are no fake-only bypass paths for provider projection, journal append, content resolution, or event fanout.

## SDK Package/Test Architecture Gate

- Standards updated: `coding_standards.md`, `docs/architecture/coding-standards.md`, and `docs/workstreams/validation-gates.md` now require SDK package/test architecture as a review gate, informed by Rust/Cargo conventions and mature SDK layouts.
- Source layout now uses SDK responsibility folders under `crates/agent-sdk-core/src/`: `domain/`, `package/`, `records/`, `ports/`, `application/`, and `testing/`.
- Test layout mirrors source responsibility under `crates/agent-sdk-core/tests/`: `domain/`, `package/`, `records/`, `ports/`, `runtime/`, and `testing/`.
- Root source is limited to `lib.rs` plus a layout README. Root integration test files are thin Cargo target shims so existing launch-doc commands remain stable while test bodies live in the owning SDK responsibility folder.

## Boundary Notes

- SDK-owned boundaries preserved: package authority, event envelopes/frames, journal truth, context projection, provider request/response DTOs, and deterministic test harnesses live in `agent-sdk-core`.
- Host-owned boundaries preserved: live provider clients, concrete streaming transports, output delivery, approval transports, isolation runtimes, telemetry exporters, memory backends, UI, marketplace, workflow engines, and host adapters were not added.
- Live event frames and durable journal records remain distinct. Journal records are append-only durable truth; event frames are observable live stream records with repair links back to journal cursors where needed.
- Runtime-package fingerprints are deterministic for implemented fields, while volatile runtime fields are excluded.
- The public top-level `EventCursor` export now points to the event stream cursor. The old ID-shaped `ids::EventCursor` type was removed; durable token IDs remain explicit as `EventCursorId`.

## Review Packet

Primitive decision:

- Reused kernel primitives: typed IDs/refs, privacy/retention/trust classes, policy decisions, errors, deterministic fakes, journal cursors, event IDs, source/destination refs.
- New core primitives: runtime package snapshots, capability specs, event envelopes/frames, event bus ports, journal records, effect intent/result spine, content resolver policy, context projection records, provider request/response DTOs.
- New feature-layer primitives: none.
- Host-owned behavior kept out: live providers, concrete transports, side-effect execution, output sinks, approval dispatch, isolation implementations, telemetry exporters, marketplace/install state, UI, and host adapters.

Validation evidence:

- Contract/unit tests: `runtime_package_contract`, `event_contract`, `journal_contract`, `context_contract`, `provider_projection_contract`, plus full crate regression coverage.
- Golden fixtures: package, event, journal, context, provider, ID/ref, and policy fixtures.
- Smoke/scenario tests: fake fixture harness, package import smoke, no-default full test pass, and conformance-style fake provider/content/journal/event cases.
- Docs audits: product-neutrality, no-live-service, raw-bypass/cursor-ambiguity, and package/test architecture scans.

Reviewer checklist:

- Simplicity: records and ports stay small, typed, and package/context/journal authority remains single-path.
- Product-neutrality: no product, provider, host, UI, marketplace, or transport-specific behavior entered core.
- Mockability / SDK test kit: reusable fakes and conformance helpers are present for all Phase 02 surfaces.
- SDK package/test architecture: source and test bodies are grouped by responsibility, with only public facade/test-target shims at roots.
- Event/journal durability: live frames and durable journal records are separate, with repair links rather than shared truth.
- Privacy/redaction: context projection strips private metadata by default, event payloads are envelope-only by default, and content projection requires explicit raw-content opt-in.
- Replay/idempotency: append-before-effect and recovery markers provide the first replay-safe effect spine; broader replay hardening remains in later phases.
- Capability fingerprint impact: implemented package fields feed deterministic fingerprints; volatile fields are excluded.

## Proposal Blocks

None.

## Reviewer Verdict

First independent review returned BLOCKED with four findings:

- Public fake provider still exposed a raw prompt shortcut.
- Event bus subscriptions returned frames with the original publication cursor scope instead of the requested subscription scope.
- Runtime-package canonicalization sorted only top-level vectors and missed nested execution-affecting lists.
- A duplicate public ID-shaped `EventCursor` type remained reachable under `ids`/`domain`.

Fixes applied:

- Removed the raw prompt fake-provider API and updated fake tests to use typed `ProviderAdapter::complete(&ProviderRequest)`.
- Rewrote subscription stream cursors to the requested logical stream scope and added resume coverage using the returned cursor.
- Canonicalized nested package DTO lists before fingerprinting and added a reorder-equivalence test.
- Removed the old ID-shaped `ids::EventCursor`; `EventCursorId` is the durable token type and top-level `EventCursor` is the stream cursor.
- Added and applied the SDK package/test architecture gate after the user's organization review note. A focused follow-up review confirmed the gate now reflects mature SDK package architecture rather than rigid DDD ceremony.

Independent re-review returned PASS with no findings. The phase exit report can be marked reviewer PASS.

## Next Phase Readiness

Ready for Phase 03.
