# Checkpoint Resume

Checkpoint and replay example using `AgentAppStores::file`.

Run:

```sh
cargo run -p clawdia-sdk-example-06-checkpoint-resume
```

Expected output shape:

```text
output=checkpoint ready; records=11; resume_allowed=true; next_state=terminal:completed; checkpoint=checkpoint.example.resume.ready
```

No credentials are required. The example writes a checkpoint accelerator, appends
a checkpoint journal record, reads the record back from `RunJournalReader`, and
feeds that durable record into `ReplayReducer`.

Common failures:

- A missing checkpoint store means the host did not configure
  `AgentAppStores::file` or an equivalent checkpoint adapter.
- This example does not continue a run from a checkpoint; it only proves
  resume-readiness evidence.

## Under The Hood

SDK-owned boundaries:

- Checkpoint DTO validation, journal checkpoint record, `RunJournalReader`,
  replay reducer, and checkpoint-store port.

Host-owned boundaries:

- Store provisioning, checkpoint retention/pruning, runtime restart policy, and
  actual run-continuation orchestration.

The checkpoint is an accelerator. Resume decisions are derived from journal
records through the core replay reducer.
