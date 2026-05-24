# Goal 04c: Telemetry Privacy

## Phase

[Phase 04: Side Effects And Policy](README.md)

## Owner Role

[Telemetry Privacy Cost](../_roles/09-telemetry-privacy-cost.md)

## Parallelism

Parallel-safe with every other goal in Phase 04 after Phase 03 exits. Do not start Phase 05 until all Phase 04 goals finish.

## Required Reading

- `README.md`
- `docs/start-here.md`
- `coding_standards.md`
- `docs/workstreams/validation-gates.md`
- `docs/reference/sdk-review-checklist.md`
- `docs/architecture/primitive-map.md`
- phase README
- owner role doc
- owner role doc read-only inputs
- read-only inputs below

## Writable Files

- `docs/contracts/otel-mapping-contract.md`
- `docs/contracts/telemetry-privacy-contract.md`

## Read-Only Inputs

- `docs/contracts/event-schema.md`
- `docs/contracts/journal-replay-schema.md`
- `docs/contracts/runtime-package-schema.md`
- `docs/architecture/observability-and-lineage.md`
- `docs/examples/live-vs-durable-event-flow.md`

## Primitive Focus

- Telemetry is a derived projection from events, journals, usage, and policy decisions.
- Raw content capture is opt-in by policy and bounded by redaction, retention, sampling, and destination permission.
- Sink failure never controls the run.
- Phase 04 owns content-capture, redaction, usage/cost, fanout backpressure, sink failure, and repair-cursor semantics. Complete OTel emitted-kind mapping after Phase 05 adds the later emitted kinds, or in the next stitching checkpoint.

## Must Not Own

Durable run truth, product dashboards, billing UX, provider credentials, or raw content defaults.

## Validation And Review

- Future tests/fixtures: OTel golden span/log fixtures, redaction matrix tests, sink failure tests, usage/cost fixtures, and fanout overflow tests.
- Docs audit: telemetry remains a derived projection and must not become a durable truth store.
- Golden span/log mapping for emitted kinds available in Phase 04; later feature-layer emitted kinds are mapped after Phase 05 or recorded as deferred with an owner.
- Redaction and content-capture policy matrix.
- Sink failure and retry behavior.
- Primitive-lowering evidence: telemetry does not create its own event stream or ledger.

## Validation Evidence

- Worker agent: Einstein (`019e586a-fb48-78b0-8e95-ea8343573368`).
- Changed files: `docs/contracts/otel-mapping-contract.md`, `docs/contracts/telemetry-privacy-contract.md`.
- `git diff --check -- docs/contracts/otel-mapping-contract.md docs/contracts/telemetry-privacy-contract.md` passed.
- `git branch --show-current` confirmed `main`; no branch was created.
- No Rust source, package manifests, executable tests, or fixtures were created.
- `markdownlint` was not installed, so Markdown lint was skipped.
- Cross-cutting proposals sent to stitching: telemetry overflow event naming and Phase 05 emitted-kind mapping owners.

## Review Packet

- Primitive decision: telemetry remains a derived projection from `AgentEvent`, `RunJournal`, usage/cost records, policy decisions, privacy/retention refs, and sink-scoped export cursors; no new capability variant.
- SDK-owned boundaries preserved: projection rules, redaction/content-capture enforcement, minimum usage/cost fields, sink failure events, overflow semantics, and repair cursors.
- Host-owned boundaries preserved: collectors, credentials, trace stores, dashboards, billing UX, rate tables, retention configuration, and unsafe repair approval.
- Reviewer checklist: PASS for simplicity, product-neutrality, event/journal durability, privacy/redaction, replay/idempotency, and capability fingerprint impact after stitching rejects a separate `TelemetryOverflowed` kind for the first slice and records Phase 05 mapping deferrals.
