# Example 06: Typed Output And Events

## Command

```sh
cargo run -p clawdia-sdk-example-06-typed-output-and-events
```

## Expected Output

```text
typed_title=Review Phase 16; priority=high; validation_reports=1; events=11; records=16; report_records=16
```

## What It Proves

This example runs `AgentApp::run_typed::<TodoExtraction>` through the
canonical runtime with a deterministic fake provider and file-backed stores. It
then reads the same run through the live event helper, durable journal reader,
and report projection helper.

## Under The Hood

`TodoExtraction` implements `TypedOutputModel`, so the helper lowers into a
normal `RunRequest` with an `OutputContract`. The runtime validates the
provider output locally, writes structured-output evidence into the journal,
publishes live event frames, and lets `RunReport` project from durable journal
records.

## SDK-Owned Boundary

The SDK owns typed-output lowering, validation evidence, live event frames,
journal records, file-backed adapter contracts, and report projection.

## Host-Owned Boundary

The host owns real provider credentials, schema authoring policy, prompt copy,
raw output retention decisions, UI rendering, and any production trace store.

## Failure Modes

- Invalid provider JSON fails local typed-output validation before typed
  publication.
- Missing stores make `journal_records_for_run` and `run_report_from_stores`
  return host-configuration diagnostics.
- Live event frames are observation only. Durable evidence still comes from the
  journal reader.
