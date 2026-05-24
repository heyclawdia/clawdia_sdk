# Agent SDK Examples

These examples show how the contract docs handle complex host workflows without making any product host part of SDK core. They are conceptual and documentation-only.

Each example names:

- SDK IDs.
- host adapter IDs where relevant.
- event families.
- hook points where relevant.
- journal records.
- output sinks.
- host-owned boundaries.
- acceptance tests.

## Examples

| Example | Purpose |
| --- | --- |
| [live-vs-durable-event-flow.md](live-vs-durable-event-flow.md) | Separates SDK live events, host display events, run journal, telemetry, and durable trace stores. |
| [desktop-chat-tool-approval.md](desktop-chat-tool-approval.md) | Desktop chat with context projection, model streaming, tool approval, journal, and UI fanout. |
| [realtime-voice-workflow.md](realtime-voice-workflow.md) | Wake phrase, realtime send/receive halves, interruptions, voice approval, restart gating, and UI effects. |
| [remote-headless-approval.md](remote-headless-approval.md) | Remote-channel run with source-scoped output and explicit headless approval behavior. |
| [external-runtime-session-lifecycle.md](external-runtime-session-lifecycle.md) | External runtime prewarm, restore keys, runtime session IDs, fingerprint invalidation, compaction retirement, surface close, and host shutdown. |
| [subagent-supervision-workflow.md](subagent-supervision-workflow.md) | Parent-owned subagents, package stripping, route validation, event wrapping, telemetry rollup, and read-only host display. |
| [tool-pack-isolation-anti-entropy.md](tool-pack-isolation-anti-entropy.md) | Read/search/edit/write/shell tool packs, hooks, isolation, approvals, side-effect intent, long-running process detach, telemetry, and anti-entropy repair. |
| [memory-context-compaction.md](memory-context-compaction.md) | Memory/context injection, projection audits, compaction, protected context, and resume after context pressure. |

## Example Rules

- Examples are not implementation promises by themselves. Contracts own the rules.
- If an example contradicts a contract, fix the contract or the example before coding.
- Host-owned surfaces are drawn explicitly so the SDK stays smaller than any product built on top.
