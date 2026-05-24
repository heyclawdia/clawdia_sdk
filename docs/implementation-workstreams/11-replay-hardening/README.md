# Phase 11: Replay Hardening

Strengthen cross-cutting proof after feature ports exist. All launch targets in this folder may run in parallel after Phase 10 exits.

## Launch Targets

| Title | Run in parallel? | Purpose |
| --- | --- | --- |
| [Golden Coverage](11a-golden-coverage.md) | yes | Fill missing event, journal, package, OTel, and extension fixtures. |
| [Replay Recovery](11b-replay-recovery.md) | yes | Checkpoint, resume, anti-entropy, repair, cursor, and unsafe-pending behavior. |
| [Privacy Performance](11c-privacy-performance.md) | yes | Redaction, bounded queues, hot-path allocation, content-capture, and slow-sink behavior. |

## Exit Gate

- [x] Every implemented emitted kind and journal record has golden coverage.
- [x] Replay and recovery tests cover unsafe pending side effects and cursor compatibility.
- [x] Privacy and performance gates prevent raw-content defaults and slow-subscriber blocking.
- [x] Phase exit report records reviewer PASS.
