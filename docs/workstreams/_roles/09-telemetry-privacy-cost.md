# Owner Role 09: Telemetry, Privacy, And Cost

## Owner Role

Observability and privacy agent.

## Writable Files

- `docs/contracts/otel-mapping-contract.md`
- `docs/contracts/telemetry-privacy-contract.md`

## Future Implementation Writable Scope

Once SDK code exists, this workstream may own telemetry/privacy modules and tests only, for example:

- `crates/agent-sdk-core/src/telemetry/**`
- `crates/agent-sdk-otel/**`
- `crates/agent-sdk-core/tests/telemetry_*.rs`
- OTel/span golden fixtures

## Read-Only Inputs

- `docs/contracts/event-schema.md`
- `docs/contracts/journal-replay-schema.md`
- `docs/contracts/tool-pack-contract.md`
- `docs/architecture/observability-and-lineage.md`
- `docs/examples/live-vs-durable-event-flow.md`

## Contract To Deliver

Define OTel GenAI/MCP mapping, schema URL/version posture, content opt-in, redaction limits, sink failure behavior, usage/cost accounting, provider/tool cost records, telemetry fanout, and host projection boundaries.

## Must Not Own

Run control decisions, raw content capture by default, billing product UI, trace-store storage policy, or durable journal authority.

## Integration Handoff

Send span/event attribute names, usage/cost field names, content policy names, and sink failure event names to the stitching owner. Put proposal text in the handoff; do not edit shared reference or architecture files unless the stitching owner delegates it.

## Required Validation

- Golden span tests: GenAI agent/model/tool/MCP spans match schema URL, attributes, IDs, status, and error mapping.
- Redaction tests: raw content absent by default; opt-in content capture enforces policy, byte/token limits, media counts, and sink permission.
- Sink tests: slow/failing sink cannot crash or block run; sink health events and recovery/export cursor records are emitted.
- Usage/cost tests: provider-reported, estimated, corrected, and tool-unit costs produce durable records with rate table/version.
- Privacy audit: metadata limits, redaction policy ID, content refs, hashes, and retention class present for every exported event/span.
- Dedupe tests: MCP/tool spans dedupe repeated export and preserve causal IDs.
- Primitive-lowering review: telemetry must project from `AgentEvent`, `RunJournal`, usage records, and privacy policy refs; telemetry sinks cannot own run truth or control flow.
- Handoff evidence: OTel fixtures, redaction matrix, sink failure fixture, cost fixture, and host-owned dashboard/storage boundary notes.
