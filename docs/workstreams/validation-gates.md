# Phase Goal Validation Gates

This document defines the shared validation language for parallel Agent SDK phase goals. Each owner role still owns its specific tests, but every goal must end with concrete evidence rather than prose-only confidence.

## Evidence Levels

| Level | Evidence | Use for |
| --- | --- | --- |
| Contract compile | crate compiles, public exports resolve, docs/rustdoc examples compile when promoted from sketches | API and type surfaces |
| Unit tests | deterministic fake adapters and in-memory stores | state transitions, policy decisions, validators, queue behavior |
| Golden fixtures | checked-in JSON/serde fixtures with schema versions | events, journals, package fingerprints, OTel spans, extension protocol |
| Property/table tests | generated or table-driven cases | reducers, fingerprints, filters, regex safety, policy matrices |
| Smoke tests | package/import/runtime checks with fake implementations | extension packaging, browser-safe helpers, optional crates |
| Scenario tests | multi-component fake workflows | chat, voice, headless, subagents, isolation, recovery |
| SDK test kit | reusable fakes, conformance helpers, and weird-scenario harnesses | downstream provider/tool/sink/runtime/extension/telemetry adapter tests |
| Docs audits | link, ownership, boundary, and simplicity checks | documentation-only or integration/stitching work |

## Universal Required Evidence

Documentation-only and implementation goals have different proof surfaces.

Every documentation-only packet goal must provide:

- link/path, ownership, boundary, or simplicity audit evidence appropriate to its owner role;
- primitive-lowering review proving helpers and feature docs reuse the kernel instead of adding parallel paths;
- explicit accepted/rejected/deferred proposal blocks where cross-role decisions are involved;
- named future tests, fixtures, or smoke checks for behavior that cannot run until code exists;
- a concise statement that no Rust source, package manifests, executable tests, or fixtures were created when the task is documentation-only.

Every implementation goal must provide:

- a list of tests/fixtures added or changed;
- the exact command or verifier used;
- passing output or a concise failure explanation;
- confirmation that simple helpers lower into canonical contracts when helpers are involved;
- confirmation that new behavior reuses the primitive kernel or explains why a new primitive proposal is needed;
- confirmation that events, journal records, telemetry, redaction, policy, and host-owned boundaries were either exercised or explicitly not applicable;
- confirmation that touched public ports and adapter boundaries have deterministic fakes, conformance helpers, or an explicitly blocked/deferred test-support gap for SDK consumers;
- a handoff note naming cross-role proposals, unresolved risks, and skipped live/external tests.
- for non-stitching goals, cross-role proposals appear as proposal blocks in the handoff, not as direct edits to shared reference files.

## Primitive Gates

Every goal must pass these gates before implementation work can be considered ready:

| Gate | Required proof |
| --- | --- |
| Primitive fit | The review packet says whether the change is a kernel primitive, feature-layer primitive, optional adapter, or host-owned behavior. |
| Decision ladder | Any new primitive or capability variant answers the five-step decision ladder in [../reference/feature-to-primitive-matrix.md](../reference/feature-to-primitive-matrix.md). |
| Capability gate | New `CapabilitySpec` variants name the typed sidecar contract, owner role, fingerprint fields, emitted events, journal records, and acceptance tests. |
| Lineage gate | New records/events/context/effects carry `EntityRef`/source/destination/policy/privacy/retention or explain why not applicable. |
| Context projection gate | Memory, tool results, skills, files, subagents, and host input may create `ContextContribution` candidates, but only policy-admitted `ContextItem` values enter `ContextProjection`. |
| Effect gate | Mutating or externally visible behavior appends effect intent before execution and terminal effect result after execution, or maps one-to-one to that common shape. |
| No mini-SDK gate | The change does not create a parallel run loop, package registry, event stream, journal, policy path, context projection path, side-effect path, telemetry truth store, or host adapter product layer. |
| Mockability gate | Public ports, adapter contracts, side-effect paths, and scenario surfaces are mockable through deterministic fakes or reusable conformance helpers; weird/hostile cases are named and tested or explicitly blocked/deferred. |
| SDK package architecture gate | Source and integration-test files live under the owning SDK responsibility folder (`domain`, `package`, `records`, `ports`, `application`/`runtime`, or `testing`), with only thin public facades or Cargo test-target shims at the root unless a conventional Cargo layout choice is documented. |
| Phase exit gate | The phase README exit checklist has passed before later phase goals start. |

## SDK Package Architecture Evidence

Every implementation phase must prove the package stayed maintainable before a phase exit can pass.

Required source-layout evidence:

- `crates/agent-sdk-core/src` contains only `lib.rs`, `README.md`, and responsibility folders such as `domain`, `package`, `records`, `ports`, `application`, and `testing`.
- Full integration-test bodies live in `tests/domain`, `tests/package`, `tests/records`, `tests/ports`, `tests/runtime`, `tests/feature_layers`, or `tests/testing`; root `tests/*.rs` files are stable Cargo target shims only.
- Optional crates expose a narrow `src/lib.rs` facade and place real implementation in responsibility modules, not in one growing file.
- Generated/spec-derived material, when introduced, has a named generated/spec boundary and a hand-written public facade. Generated internals are not treated as stable deep-import paths.
- New public fakes and test helpers live in `src/testing` or `tests/testing` and exercise the same public ports, records, and package contracts as production code.
- SDK-consumer test-kit APIs are imported through a single documented public namespace, `agent_sdk_core::testing`, even when legacy flat re-exports temporarily exist.
- New public module aliases, deep-import paths, or facade exports require a SemVer/API review note in the phase exit report.

Mandatory audit commands:

```bash
find crates/agent-sdk-core/src -maxdepth 1 -type f -not -name lib.rs -not -name README.md
find crates/agent-sdk-core/tests -maxdepth 1 -type f -name '*.rs' -print -exec sh -c 'wc -l "$1"' sh {} \;
find crates -path '*/src/*.rs' -maxdepth 3 -type f
rg -n '#\\[path = .*\\]\\s*pub mod|pub mod [a-zA-Z0-9_]+;' crates/agent-sdk-core/src/lib.rs
rg -n '\\b(Fake|Scripted)[A-Za-z0-9_]+|ConformanceHarness' crates/agent-sdk-core/src --glob '*.rs'
rg -n '\\btrait\\b|\\bAdapter\\b|\\bResolver\\b|\\bFake\\b|\\bScripted\\b|ConformanceHarness' crates/agent-sdk-core/src/records --glob '*.rs'
wc -l crates/agent-sdk-*/src/lib.rs
```

The phase exit report must summarize the output and explain any non-empty or non-shim result. Reviewers must treat unexplained source/test sprawl, new public deep modules without API review, scripted helpers outside `src/testing`, or adapter/port behavior inside `records/` as blocking even when behavior tests pass.

## Common Commands Once Code Exists

These are target commands for the future Rust workspace. Names may change when the crate is created, but phase goals must keep equivalent gates.

```bash
cargo fmt --check
cargo test -p agent-sdk-core
cargo test -p agent-sdk-core --test contract_golden
cargo test -p agent-sdk-core --test replay_recovery
cargo test -p agent-sdk-core --test policy_matrix
```

Optional crate gates should stay optional:

```bash
cargo test -p agent-sdk-toolkit
cargo test -p agent-sdk-isolation
cargo test -p agent-sdk-otel
cargo test -p agent-sdk-extension
cargo test -p agent-sdk-workflow
```

## Required Handoff Format

Each worker ends with:

```text
Changed files:
Tests/fixtures:
Commands run:
Skipped tests and why:
Events/journal/telemetry touched:
SDK-owned boundaries preserved:
Host-owned boundaries preserved:
Primitive-lowering evidence:
Simplicity notes:
Cross-cutting proposal blocks:
```

## Phase Exit Report

Each phase should also produce a phase-level exit report, normally at `docs/workstreams/<NN-phase>/_phase/phase-exit-report.md`.

The report should include:

- phase objective and dependency status;
- goal-by-goal status, changed files, and review-packet links;
- accepted, rejected, deferred, and unresolved proposal blocks;
- changed shared names, IDs, event/journal terms, runtime-package fingerprint inputs, or public matrices;
- source-audit status when the phase goal requires it;
- validation commands and outcomes;
- reviewer-agent verdict and any resolved findings;
- explicit next-phase readiness statement.

The phase README exit gate should not be checked until the phase exit report proves every exit-gate item with current evidence.

## Review Packet

Each implementation goal should include a compact review packet:

```text
Primitive decision:
- Reused kernel primitives:
- New feature-layer primitives:
- New capability variants:
- Host-owned behavior kept out:

Validation evidence:
- Contract/unit tests:
- Golden fixtures:
- Smoke/scenario tests:
- Docs audits:

Reviewer checklist:
- Simplicity:
- Product-neutrality:
- Mockability / SDK test kit:
- Event/journal durability:
- Privacy/redaction:
- Replay/idempotency:
- Capability fingerprint impact:
```

New `CapabilitySpec` variants are not accepted until the review packet names the typed sidecar contract, owner role, fingerprint fields, emitted events, journal records, and acceptance tests.

## Non-Negotiable Validation Rules

- Do not use live providers, real containers, or product UI as the first proof of correctness.
- Do not accept an event contract without golden fixtures and redaction cases.
- Do not accept side-effect behavior without intent-before-effect journal tests.
- Do not accept helper APIs unless they prove canonical lowering.
- Do not accept host-adapter scenarios as proof that core owns the behavior.
- Do not accept "not covered yet" without adding a named follow-up owner and blocking status where it affects the first Rust slice.
- Do not accept a feature-specific event stream, journal, package registry, policy path, or side-effect path when the primitive kernel already has one.
- Do not accept source or test sprawl: new implementation files and test bodies must be placed in the owning SDK responsibility folder before phase exit.
