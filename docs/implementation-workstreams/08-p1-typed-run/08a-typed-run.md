# Typed Run

## Phase

[Phase 08: P1 Typed Run](README.md)

## Parallelism

Only launch target in this phase. It integrates Phase 06 and Phase 07 outputs before P2 side effects start.

## Contract Inputs

- [structured-output-contract.md](../../contracts/structured-output-contract.md)
- [api-contracts.md](../../contracts/api-contracts.md)
- [event-schema.md](../../contracts/event-schema.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)

## Implementation Objective

Prove `agent.run_typed::<T>` and explicit `RunRequest.output_contract` run through the same P0 loop, validate locally, repair within policy, and publish a typed result only after durable validation evidence exists.

## Owned Implementation Surface

- `crates/agent-sdk-core/tests/p1_typed_output.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/p1/`
- minimum integration glue in existing Phase 04/06/07 application, records,
  package, and facade modules only where required by the P1 path
- any additional touched file must be named in the Phase 08 exit report as
  integration glue and checked against the SDK package architecture gate

## Must Deliver

- End-to-end P1 fake-provider tests for valid output, invalid output, repair success, repair exhaustion, and typed extraction.
- P1 event and journal golden fixtures.
- Runtime-package fingerprint test proving output-contract normalization participates in the effective package.
- `RunResult` typed extraction over `ValidatedOutput`.
- `agent.run_typed::<T>` and explicit `RunRequest.output_contract` must use
  the same P0 loop, effective runtime package, provider projection, local
  validation, repair, structured-output event, journal, and typed-result path.
- Typed values must be produced only by deserializing the validated canonical
  content ref through the Phase 07 `TypedOutputDeserializer<T>` seam. Do not
  construct a `StructuredOutputResult<T>` from a caller-provided `T`.
- Output delivery sinks are not required for P1 typed result extraction.

## Validation

- `cargo test -p agent-sdk-core --test p1_typed_output`
- P1 golden event and journal fixtures
- helper lowering and explicit request path equivalence tests
- test that output delivery sinks are not required for typed result extraction
- named tests:
  - `run_typed_valid_output_uses_p0_loop_and_returns_typed_value`
  - `explicit_output_contract_and_typed_helper_share_runtime_path`
  - `invalid_output_repairs_and_publishes_typed_result_after_evidence`
  - `repair_exhaustion_returns_structured_output_failure_without_typed_value`
  - `typed_extraction_requires_canonical_validated_content_ref`
  - `output_delivery_sink_not_required_for_p1_typed_result`
  - `output_contract_changes_runtime_package_fingerprint_for_typed_run`
  - `p1_structured_output_events_and_journal_match_golden_fixtures`
- required fixtures:
  - `crates/agent-sdk-core/tests/fixtures/p1/typed-run-events.json`
  - `crates/agent-sdk-core/tests/fixtures/p1/typed-run-journal.json`
  - `crates/agent-sdk-core/tests/fixtures/p1/repair-success-events.json`
  - `crates/agent-sdk-core/tests/fixtures/p1/repair-success-journal.json`
  - `crates/agent-sdk-core/tests/fixtures/p1/repair-exhausted-events.json`
  - `crates/agent-sdk-core/tests/fixtures/p1/repair-exhausted-journal.json`
- every structured-output journal-backed event must be emitted only after the
  matching `JournalRecordPayload::StructuredOutput` append has produced a
  journal cursor.
- fixtures must cover `StructuredOutputRequested`,
  `StructuredOutputValidationStarted`, `StructuredOutputValidationFailed`,
  `StructuredOutputRepairRequested`, `StructuredOutputValidated`, and
  `StructuredOutputFailed` where applicable.

## Must Not

- Add tool execution, approvals, output delivery, isolation, subagents, extensions, realtime, or telemetry exporters as P1 prerequisites.
- Add a second run loop, package registry, event stream, journal, validation
  path, or typed-output shortcut.
