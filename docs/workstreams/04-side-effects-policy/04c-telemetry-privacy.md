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
