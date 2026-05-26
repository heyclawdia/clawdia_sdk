# Session Turn Trace Primitives Plan

Date: 2026-05-26

## Objective

Make session and turn lineage first-class in the Rust core so hosts can answer:
"for this user message, which context, hooks, tools, provider attempts, output,
and terminal result were involved?" Forking and chat-copy UX remain outside this
slice.

## Current Problem Shape

`SessionId` and `TurnId` already exist in the vocabulary, and events already have
an optional `turn_id`, but the active execution path does not make them a strict
trace model:

- `RunRequest` has no `session_id` or caller-provided `turn_id`.
- journal records and event index projections do not carry direct `session_id`.
- the P0 loop emits context/model/run events with a generated turn id, but it
does not emit canonical `TurnStarted`/`TurnCompleted`/`TurnFailed` lifecycle
evidence.
- there is no SDK helper that derives a `TurnTrace`, `RunTrace`, or
`SessionTimeline` from durable journal records.

## Authoritative Source Of Truth

- `docs/contracts/api-contracts.md` owns public request and ID vocabulary.
- `docs/contracts/event-schema.md` owns live event envelopes and filters.
- `docs/contracts/journal-replay-schema.md` owns durable journal envelopes and
  replay/query evidence.
- `docs/contracts/context-memory-contract.md` owns context projection lineage and
  keeps memory/context as admitted SDK records rather than host UI strings.
- `docs/architecture/primitive-map.md` keeps sessions, turns, journals, and
  events in the primitive kernel; chat stores and trace dashboards are host
  owned.
- `docs/reference/sdk-review-checklist.md` is the review gate for
  product-neutrality, hidden state, replayability, and event/journal alignment.

## Behavior Contract

New behavior:

- `RunRequest` accepts optional `session_id` and optional caller-provided
  `turn_id`; existing `RunRequest::text(...)` and typed helpers remain source
  compatible and default both to absent.
- `AgentRuntime::start_run` snapshots `session_id` and `turn_id` with the run so
  handles, registry state, and the loop agree on lineage.
- event envelopes, event filters, journal records, journal bases, and journal
  event-index projections carry optional `session_id` as an indexed lineage
  field.
- the P0 loop chooses an effective turn id from `RunRequest.turn_id` or the
  existing deterministic run-derived fallback, then stamps the run, context,
  hook, model, effect, message, terminal, and turn lifecycle records/events with
  that turn.
- the P0 loop appends a durable `TurnLifecycleRecord` and publishes
  `TurnStarted` before context assembly.
- terminal completion/failure appends `TurnLifecycleRecord` and publishes
  `TurnCompleted` or `TurnFailed` after the run terminal record is sealed.
- `EventFilter` can match by `session_id` without payload parsing or journal
  scanning.
- `TurnTrace`, `RunTrace`, and `SessionTimeline` are derived read helpers over
  journal records. They do not create a new trace database or host conversation
  store.

Preserved behavior:

- chat/session storage, user-visible fork UX, dashboards, and cross-session
  retention policy remain host-owned.
- journal records remain replay truth; live events remain observation/index
  projections.
- existing callers that do not provide a session or turn id keep working.
- feature layers continue to lower through canonical records; no parallel tool,
  context, memory, or provider registry is introduced.

## Cardinality

- `SessionId -> many TurnId`
- `TurnId -> one or more RunId`
- `RunId -> many AttemptId / ToolCallId / EffectId / ContextItemId / MessageId`

`SessionId` may be absent for non-chat or legacy hosts, but when present it must
be copied into every journal/event envelope causally produced for that run.

## Tests And Evidence

- Add a focused P0 acceptance test: run two requests in the same session with
  different turn ids, make one turn use hook-injected context plus provider
  output, then assert `TurnTrace` returns only that turn's records and
  `SessionTimeline` groups both turns in order.
- Add record-level coverage for session-id event filtering and indexed field
  reporting.
- Update fixtures only where the canonical P0/P1 event/journal shapes change
  because of turn lifecycle records or explicit test-provided session ids.
- Run targeted core tests first, then `cargo test -p agent-sdk-core`, and finish
  with `scripts/public-release-audit.sh` if this becomes a public handoff.

## Independent Plan Review

Reviewed against `docs/reference/sdk-review-checklist.md` before implementation:

- The plan keeps product chat/fork UX out of core and only adds SDK lineage
  primitives.
- The query surface is derived from journals, so it does not introduce a trace
  database, background indexer, or hidden mutable runtime state.
- `session_id` and `turn_id` are typed IDs and stay optional for source
  compatibility, but the loop stamps them consistently once supplied or derived.
- Live event filtering remains envelope-only and adds `SessionId` as a cheap
  indexed field.
- The acceptance test uses deterministic fakes and durable records, not a
  live-service or product adapter path.

## Risk Notes

- Adding `session_id` to public structs touches several fixture helpers and
  literal constructors. Keep the field optional with `skip_serializing_if` so
  legacy JSON stays stable unless the test intentionally supplies a session.
- Turn lifecycle records change P0/P1 journal and event counts. Tests should
  assert semantic summaries rather than assume old counts where the lifecycle
  evidence is now canonical.
- Derived trace helpers must preserve ordering from `journal_seq` and must not
  parse raw payload content.
