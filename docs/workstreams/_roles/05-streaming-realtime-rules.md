# Owner Role 05: Streaming, Realtime, And Stream Rules

## Owner Role

Streaming and realtime control agent.

## Writable Files

- `docs/contracts/stream-rule-contract.md`

## Future Implementation Writable Scope

Once SDK code exists, this workstream may own streaming/realtime rule modules and tests only, for example:

- `crates/agent-sdk-core/src/stream/**`
- `crates/agent-sdk-core/src/realtime/**`
- `crates/agent-sdk-core/tests/stream_rule_*.rs`
- `crates/agent-sdk-core/tests/realtime_*.rs`

## Read-Only Inputs

- `docs/contracts/event-schema.md`
- `docs/contracts/tool-approval-contract.md`
- `docs/architecture/architecture-proposal.md`
- `docs/architecture/observability-and-lineage.md`
- `docs/examples/realtime-voice-workflow.md`

## Contract To Deliver

Define stream delta channels, bounded matcher windows, literal/regex/marker matchers, stop/retry/mask/approval/emit interventions, realtime send/receive lifecycle, restart gates, privacy rules, and resume repeat-state restoration.

## Must Not Own

Provider hidden chain-of-thought, UI media rendering, unbounded transcript buffering, or approval transport.

## Integration Handoff

Send stream channel names, intervention event names, cursor semantics, and approval-trigger links to the stitching owner. Put proposal text in the handoff; do not edit shared reference or architecture files unless the stitching owner delegates it.

## Required Validation

- Matcher tests: literal, regex, marker, bounded rolling window, compile failure, timeout/backtracking protection, and channel privacy.
- Intervention tests: stop, mask, emit-only, approval request, abort-and-retry, and injection append semantics.
- Redaction tests: matched content is redacted/hash-summary by default; raw match capture requires explicit policy.
- Resume tests: repeat state restores from journal/checkpoint without duplicate interventions.
- Realtime tests: send/receive halves, interruption, restart requested/started/completed/failed, backpressure, and close events.
- Event/journal audit: every rule match/intervention/restart emits event and journal record with rule ID/version, channel, cursor, action, and privacy.
- Primitive-lowering review: stream rules must reuse `RuntimePackage` sidecars, `StreamDelta`, `AgentEvent`, `RunJournal`, policy refs, and approval ports; no provider-specific callback path that bypasses the loop.
- Handoff evidence: matcher corpus, intervention matrix, realtime scenario fixture, and skipped live-media tests.
