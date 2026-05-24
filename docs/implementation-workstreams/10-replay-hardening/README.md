# Phase 10: Replay Hardening

Strengthen cross-cutting proof after feature ports exist. All launch targets in this folder may run in parallel after Phase 09 exits.

## Launch Targets

| Title | Run in parallel? | Purpose |
| --- | --- | --- |
| [Golden Coverage](10a-golden-coverage.md) | yes | Fill missing event, journal, package, OTel, and extension fixtures. |
| [Replay Recovery](10b-replay-recovery.md) | yes | Checkpoint, resume, anti-entropy, repair, cursor, and unsafe-pending behavior. |
| [Privacy Performance](10c-privacy-performance.md) | yes | Redaction, bounded queues, hot-path allocation, content-capture, and slow-sink behavior. |

## Exit Gate

- [ ] Every implemented emitted kind and journal record has golden coverage.
- [ ] Replay and recovery tests cover unsafe pending side effects and cursor compatibility.
- [ ] Privacy and performance gates prevent raw-content defaults and slow-subscriber blocking.
- [ ] Phase exit report records reviewer PASS.
