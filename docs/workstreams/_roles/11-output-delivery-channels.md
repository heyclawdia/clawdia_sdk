# Owner Role 11: Output Delivery And Channels

## Owner Role

Output delivery and host-channel boundary agent.

## Writable Files

- `docs/contracts/output-delivery-contract.md`

## Future Implementation Writable Scope

Once SDK code exists, this workstream may own output-delivery modules and tests only, for example:

- `crates/agent-sdk-core/src/channels/**`
- `crates/agent-sdk-core/src/output_delivery/**`
- `crates/agent-sdk-core/tests/output_delivery_*.rs`
- first-slice fixtures for emitted `output_delivery` event kinds

## Read-Only Inputs

- `docs/contracts/api-contracts.md`
- `docs/contracts/event-schema.md`
- `docs/contracts/journal-replay-schema.md`
- `docs/contracts/telemetry-privacy-contract.md`
- `docs/architecture/primitive-map.md`
- `docs/examples/README.md`

## Contract To Deliver

Define `DestinationRef`, `OutputSink`, output-delivery policy, dispatch intent/result records, dedupe keys, streaming/final output delivery semantics, privacy checks, and host-owned channel boundaries.

## Must Not Own

Product channel UI, remote transport credentials, notification copy, offline retry product policy, workflow orchestration, or durable channel storage.

## Integration Handoff

Send destination kind names, output delivery event names, dedupe-key fields, privacy policy refs, and journal record names to the stitching owner. Put proposal text in the handoff; do not edit shared reference or architecture files unless the stitching owner delegates it.

## Required Validation

- Lowering tests: run-level destination helpers lower into `DestinationRef` and `OutputDeliveryPolicy`.
- Intent tests: `output_delivery_intent_precedes_sink_call`, `output_delivery_failure_records_terminal_result`, `dedupe_record_prevents_duplicate_sink_call`.
- Privacy tests: raw output dispatch requires destination policy and allowed sink; default delivery uses content refs or redacted summaries.
- Optional/required sink tests: missing optional sink does not fail the run; missing required sink returns typed `HostConfigurationNeeded`.
- Streaming tests: chunk and final delivery share destination, dedupe, privacy, and journal semantics.
- Event/journal audit: every emitted `output_delivery` kind has a golden fixture and corresponding journal record.
- Primitive-lowering review: output delivery must reuse `RunRequest`, `DestinationRef`, `EffectIntent`, `EffectResult`, `RunJournal`, `AgentEvent`, `OutputSink`, and policy refs instead of creating a channel-specific run path.
- Handoff evidence: delivery matrix, dedupe fixture list, privacy matrix, emitted-kind fixture list, and host-owned channel boundary notes.
