# Example 08: Checkpoint Replay

## Command

```sh
cargo run -p clawdia-sdk-example-08-checkpoint-replay
```

## Expected Output

```text
output=checkpoint evidence ready; records=11; resume_allowed=true; replay_seq=11; next_loop_state=terminal:completed; checkpoint=checkpoint.example.ready
```

## What It Proves

This example runs a deterministic text request, reads durable journal evidence,
saves a checkpoint accelerator through the checkpoint store, and validates that
checkpoint shape with `ReplayReducer::new(ReplayMode::ResumeReplay)`.

## Under The Hood

The run journal remains the durable source of truth. The checkpoint store holds
an accelerator that points at the latest journal sequence it covers. The
example appends a checkpoint record, reads it back through `RunJournalReader`,
and feeds that durable checkpoint record into the replay reducer to prove that
the checkpoint is safe to use as resume-readiness evidence.

## SDK-Owned Boundary

The SDK owns checkpoint DTO invariants, checkpoint store ports, journal record
shape, replay reduction, and diagnostics for unsafe replay state.

## Host-Owned Boundary

The host owns when checkpoints are written, how checkpoint loop state maps to a
real process or session, and whether a future runtime can continue from that
accelerator. This example does not add or claim an execution resume API.

## Failure Modes

- Checkpoints with a `covers_journal_seq` beyond the replayed record sequence
  are rejected.
- Missing checkpoint stores return host-configuration diagnostics.
- Checkpoints accelerate resume decisions but do not replace journal truth or
  authorize side-effect replay by themselves.
