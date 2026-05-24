# Owner Role 02: Events, Journal, And Replay

## Owner Role

Durability and event taxonomy agent.

## Writable Files

- `docs/contracts/event-schema.md`
- `docs/contracts/journal-replay-schema.md`

## Future Implementation Writable Scope

Once SDK code exists, this workstream may own event/journal modules and tests only, for example:

- `crates/agent-sdk-core/src/events/**`
- `crates/agent-sdk-core/src/journal/**`
- `crates/agent-sdk-core/src/replay/**`
- `crates/agent-sdk-core/tests/event_*.rs`
- `crates/agent-sdk-core/tests/journal_*.rs`
- golden fixtures for emitted event and journal record kinds

## Read-Only Inputs

- `docs/contracts/api-contracts.md`
- `docs/contracts/loop-state-machine.md`
- `docs/contracts/telemetry-privacy-contract.md`
- `docs/architecture/observability-and-lineage.md`
- `docs/examples/live-vs-durable-event-flow.md`

## Contract To Deliver

Freeze the event envelope, entity refs, event frames, event families, payload versioning, event cursors, typed/compiled filters, live subscription APIs, `EventArchive`/indexed replay port boundary, journal index projections, shared effect intent/result records, journal record kinds, replay modes, checkpoint rules, resume/cancel/failure paths, and anti-entropy behavior.

## Must Not Own

UI event storage, trace-store database schema, raw content capture policy, or provider-specific stream chunks.

## Integration Handoff

Send all new event family names, payload schema versions, event frame fields, event filter fields, subscription helper names, journal record names, and replay cursor semantics to the stitching owner. Put proposal text in the handoff; do not edit shared reference or architecture files unless the stitching owner delegates it.

## Required Validation

- Golden fixtures: one JSON fixture per emitted event kind and one fixture per journal record kind in the first slice.
- Event tests: `event_golden_payload_exists_for_each_emitted_kind`, `subscribe_all_receives_envelope_only_by_default`, `subscribe_agent_filters_by_agent_id_without_payload_deserialization`, `subscribe_filtered_uses_compiled_envelope_filter`.
- Cursor tests: `event_stream_yields_frame_with_cursor_and_overflow_notice`, `archive_replay_frame_exposes_archive_cursor`, `cursor_scope_mismatch_is_typed_error`.
- Queue tests: saturated-queue coverage for `DropNonTerminal`, `DropProgress`, `SummarizeAndContinue`, `BackpressureCaller`, and `FailSubscriber`; prove live `AgentLoop` emission never blocks.
- Journal tests: `side_effect_intent_precedes_execution`, `journal_append_failure_prevents_non_idempotent_effect`, `journal_replay_filters_by_agent_tag_privacy_without_payload_deserialization`.
- Provider journal tests: `provider_request_intent_record_precedes_model_attempt_started`.
- Hook/lifecycle journal tests: `hook_response_mutation_is_journaled_before_apply`, `manual_cancel_appends_child_shutdown_intent_before_signal`, `detach_intent_without_ack_blocks_successful_completion`, `audit_replay_does_not_reinvoke_hooks`, `audit_replay_tracks_detached_child_without_killing_it`.
- Replay matrix: resume, cancel, crash, failed provider stream, failed tool, failed approval, partial side effect, and archive unsupported cases.
- Redaction audit: every event family has privacy class, redaction policy, content-capture mode, and no raw content by default; hook and child lifecycle fixtures must prove content-ref/redacted-summary defaults.
- Primitive-lowering review: live events and journal records stay distinct; new behaviors reuse `AgentEvent`, `EventFrame`, `EventCursor`, `EntityRef`, `EffectIntent`, `EffectResult`, `RunJournal`, and `JournalCursor` instead of creating feature-specific event streams or ledgers.
- Handoff evidence: fixture list, replay matrix, queue matrix, event/journal schema versions, and any migration notes.
