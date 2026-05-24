# Goal 04b: Output Delivery

## Phase

[Phase 04: Side Effects And Policy](README.md)

## Owner Role

[Output Delivery Channels](../_roles/11-output-delivery-channels.md)

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

- `docs/contracts/output-delivery-contract.md`

## Read-Only Inputs

- `docs/contracts/event-schema.md`
- `docs/contracts/journal-replay-schema.md`
- `docs/contracts/telemetry-privacy-contract.md`
- `docs/architecture/primitive-map.md`
- `docs/examples/remote-headless-approval.md`

## Primitive Focus

- Output delivery is a host sink effect with `DestinationRef`, `OutputSink`, policy refs, dedupe keys, events, and journal records.
- Streaming chunk delivery and final delivery share one destination/dedupe/privacy path.

## Must Not Own

Product channel UI, remote credentials, notification copy, offline retry product policy, or workflow orchestration.

## Validation And Review

- Future tests/fixtures: delivery intent/result fixtures, destination privacy matrix tests, sink-missing tests, and dedupe table tests.
- Docs audit: output delivery must remain a host sink over shared effect, journal, event, privacy, and dedupe primitives.
- Delivery intent precedes sink call.
- Missing required sink is typed `HostConfigurationNeeded`.
- Default delivery uses content refs or redacted summaries.
- No channel-specific run path exists.

## Validation Evidence

- Worker agent: Pascal (`019e586a-dd56-76d3-8636-457dad9a5a0b`).
- Changed file: `docs/contracts/output-delivery-contract.md`.
- `git diff --check -- docs/contracts/output-delivery-contract.md` passed.
- Scope/no-code audit confirmed only the allowed contract file changed and no Rust source, package manifests, executable tests, or fixtures were created.
- Product-neutrality/no-TODO audit found no product-specific terms, `TODO`, or `FIXME` in the edited contract.
- No Markdown links were introduced by the worker.
- Cross-cutting proposals: none blocking.

## Review Packet

- Primitive decision: reuse `DestinationRef`, `OutputSink`, `PolicyRef`, `EffectIntent`, `EffectResult`, `RunJournal`, `AgentEvent`, content refs, and dedupe keys; no `CapabilitySpec` variant.
- SDK-owned boundaries preserved: delivery policy, intent/result/dedupe/reconciliation records, typed sink absence, privacy checks, and replay rules.
- Host-owned boundaries preserved: channel UI, credentials, notification copy, durable channel stores, ack lookup, and offline scheduling.
- Reviewer checklist: PASS for simplicity, product-neutrality, event/journal durability, privacy/redaction, replay/idempotency, and capability fingerprint impact.
