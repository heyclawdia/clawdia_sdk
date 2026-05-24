# Phase 01 Exit Report: Shared Kernel

Date: 2026-05-24

## Objective And Dependency Status

Phase 01 implemented shared kernel types after Phase 00 exited with reviewer PASS WITH NOTES. The Phase 00 provider-port placeholder warning remains a Phase 02 carry-forward item: provider behavior must use typed projection/request shapes before behavior lands.

The user also made mockability and reusable SDK-consumer test support non-negotiable during this phase. The coding standards, architecture standards, validation gates, and implementation plan were updated before phase integration and review.

## Goal Status

| Goal | Status | Changed files |
| --- | --- | --- |
| `01a-typed-ids.md` | complete | `crates/agent-sdk-core/src/ids.rs`, `crates/agent-sdk-core/src/refs.rs`, `crates/agent-sdk-core/src/privacy.rs`, `crates/agent-sdk-core/tests/id_ref_contract.rs`, fixtures under `crates/agent-sdk-core/tests/fixtures/ids/`, minimal exports |
| `01b-errors-policy.md` | complete | `crates/agent-sdk-core/src/error.rs`, `crates/agent-sdk-core/src/policy.rs`, `crates/agent-sdk-core/tests/policy_contract.rs`, fixtures under `crates/agent-sdk-core/tests/fixtures/policy/`, minimal exports |
| `01c-fake-fixtures.md` | complete | `crates/agent-sdk-core/src/fakes.rs`, `crates/agent-sdk-core/tests/fake_fixture_harness.rs`, `crates/agent-sdk-core/tests/support/mod.rs`, `crates/agent-sdk-core/tests/fixtures/README.md` |

## Validation Evidence

- PASS: `cargo fmt --check`
- PASS: `cargo test -p agent-sdk-core --test id_ref_contract` (7 tests)
- PASS: `cargo test -p agent-sdk-core --test policy_contract` (9 tests)
- PASS: `cargo test -p agent-sdk-core --test fake_fixture_harness` (7 tests)
- PASS: `cargo test -p agent-sdk-core --no-default-features` (22 tests)
- PASS: `cargo tree -p agent-sdk-core --no-default-features` showed only core dependencies (`serde`, `serde_json`, `sha2`, `thiserror`) and no optional crate dependencies.
- PASS: fake-harness no-live-service audit with `rg -n "std::net|reqwest|tokio|rand|SystemTime|Instant::now|thread_rng|Command::new" crates/agent-sdk-core/src/fakes.rs crates/agent-sdk-core/tests/support crates/agent-sdk-core/tests/fake_fixture_harness.rs` returned no matches.
- PASS: product-neutrality audit with `rg -n "Clawdia|marketplace|trace-store|host-adapter|Docker|Firecracker|Vercel|Apple Containerization|live provider|product-specific" crates/agent-sdk-core Cargo.toml` returned no matches.

## Goal Handoffs

Typed IDs:

- Added typed ID/cursor newtypes, refs, source/destination/policy refs, correlation entries, and privacy/retention/trust classes with stable serde and redacted debug/display behavior.
- Added `try_new` validation helpers and validating serde for hostile cases: empty IDs, control characters, and oversized IDs.
- Added typed `AdapterRef` so public policy DTOs do not encode adapter identity as a raw string.
- Added golden fixtures for ID/ref/privacy wire forms.

Errors Policy:

- Added typed `AgentError`, causal IDs, finite `PolicyDecision`, policy stages, missing dependency fail-closed mapping, permission/sandbox/approval/escalation/privacy/content-capture policies, and deterministic policy fixtures.
- Covered weird cases for missing dispatcher, adapter, sink, store, journal append, denial, timeout-as-denial, metadata/redacted-summary not opening raw content, and raw-content capture denial.

Fake Fixtures:

- Added deterministic ID generator, deterministic clock, fake content store, fake journal store, fake event sink, fake provider shell, JSON fixture normalization, writer/readback helpers, and fixture manifest docs.
- Test utilities avoid live providers, real containers, network telemetry, random IDs, and wall-clock time.

## Mockability / SDK Test Kit Gate

- Public ID/ref/policy wire forms have golden fixtures suitable for SDK-consumer conformance tests.
- The policy surface exposes deterministic helper methods for missing-dependency and content-capture denial cases.
- Fakes exercise existing core traits (`ProviderAdapter`, `RunJournal`) and records (`EventFrame`, typed IDs) rather than private fake-only contracts.
- Fixture helpers normalize ordering and redaction so weird and hostile cases are stable in E2E tests.
- Phase 02 must keep extending the same test-support posture as provider, package, event, journal, context, and projection records become real.

## Boundary Notes

- SDK-owned boundaries preserved: typed refs/IDs, policy/error primitives, and deterministic fake harnesses live in `agent-sdk-core`.
- Host-owned boundaries preserved: no UI copy, approval transport, provider routing UI, live provider dependency, network service, concrete runtime, marketplace, product adapter, or trace store entered core.
- Optional crates remain absent from the default build.

## Review Packet

Primitive decision:

- Reused kernel primitives: typed IDs, entity/source/destination/policy refs, privacy/retention/trust classes, policy decisions, errors, deterministic fakes.
- New feature-layer primitives: none.
- New capability variants: none.
- Host-owned behavior kept out: approval transport, product autonomy modes, live provider behavior, network telemetry, concrete isolation, UI, marketplace, host adapters.

Validation evidence:

- Contract/unit tests: `id_ref_contract`, `policy_contract`, `fake_fixture_harness`.
- Golden fixtures: ID/ref/privacy fixtures and missing-dispatcher policy decision fixture.
- Smoke/scenario tests: package import smoke and fake fixture harness tests.
- Docs audits: product-neutrality and fake-harness no-live-service scans.

Reviewer checklist:

- Simplicity: small DTOs/helpers with stable serde; helpers lower to the same policy/decision structs.
- Product-neutrality: no product-specific source/destination variants or host transports.
- Mockability / SDK test kit: deterministic fakes and reusable fixture/conformance helpers are present for Phase 01 surfaces.
- Event/journal durability: fakes support append-only journal and normalized event metadata; full event/journal contracts wait for Phase 02.
- Privacy/redaction: debug/display redaction tests and content-capture denial tests pass.
- Replay/idempotency: deterministic cursors/fakes exist; replay semantics wait for later phases.
- Capability fingerprint impact: none.

## Proposal Blocks

None.

## Reviewer Verdict

First independent review returned BLOCKED with three findings:

- `ContentCapturePolicy::allows_raw_content()` treated metadata/redacted-summary capture as raw-content permission.
- `SandboxMode::RequireIsolation` used a raw `String` for adapter identity.
- ID validation was only opt-in through `try_new`.

Fixes applied:

- `allows_raw_content()` now requires `ContentCaptureMode::RawContent`, with table coverage for non-raw modes.
- `SandboxMode::RequireIsolation` now uses typed `AdapterRef`.
- ID/ref/cursor deserialization now validates durable wire values, and tests reject invalid serde values.

Re-review returned PASS WITH NOTES. The only note was that public `new(...)` constructors still bypassed validation; that note was resolved by making `new(...)` validate and by adding contract coverage.

## Next Phase Readiness

Ready for Phase 02.
