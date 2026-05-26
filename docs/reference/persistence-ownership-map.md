# Persistence Ownership Map

This map clarifies which persistence surfaces are SDK primitives, which concrete
stores exist today, and where future durable adapters should live. The rule is
simple: the run journal is durable truth, and other stores are projections,
indexes, caches, or host-owned backends unless a contract explicitly says
otherwise.

## Current Store Boundaries

| Surface | Existing primitive | Current concrete support | Proper next scope | Must not own |
| --- | --- | --- | --- | --- |
| Run journal | `RunJournal`, `JournalRecord`, `JournalCursor`, effect intent/result records | `FakeJournalStore` for deterministic tests | Optional `agent-sdk-store` or toolkit file/SQLite journal adapters | Provider calls, tool execution, raw UI transcript state, or hidden side effects |
| Checkpoints | `CheckpointStore`, `RunCheckpoint`, checkpoint prune policy | `InMemoryCheckpointStore` | Optional file/SQLite checkpoint store keyed by run and journal cursor | Append-only history, content blobs, or event archive |
| Content refs | `ContentRef`, `ContentResolver`, content privacy/retention records | deterministic fake/toolkit in-memory stores for tests and examples | Optional content store adapters with redaction, retention, and hash checks | Provider-visible raw content by default or ambient filesystem access |
| Event cursors | `AgentEventBus`, `EventCursor`, `EventArchive`, archive cursors | in-memory live bus and archive-oriented contracts | Optional event archive adapter for replay, reconnect, and UI fanout | Journal truth, telemetry truth store, or product display state |
| Agent pool | `AgentPoolStore`, `AgentPoolStoreCursor`, pool records | `InMemoryAgentPoolStore` in core and `SqliteAgentPoolStore` in toolkit | Keep SQLite/file/network pool stores outside core | Workflow engine, scheduler, cross-pool broker, or global event archive |
| Tool execution | `ToolExecutionCoordinator`, `ToolExecutionRequest`, `ToolExecutionOutput`, `ToolRecord`, `EffectIntent`, `EffectResult` | coordinator and deterministic executors; no separate execution store | Optional idempotency/result cache only when it replays journaled intent/result | Direct callback log, executor-owned approvals, or bypass around journal/effect records |
| Provider tool arguments | `ProviderToolCall`, `ContentRef` argument refs, provider adapter contracts | `agent-sdk-provider` exposes `OpenAiToolArgumentSink` for host-owned raw argument storage | Content-store adapter or host policy layer, never provider adapter internals | Raw argument leakage into summaries, events, package fingerprints, or logs |

## Decisions

- Do not add one global `StorageContext` primitive. It would blur journal,
  content, checkpoint, event, pool, and tool-execution responsibilities.
- Do not put SQLite, filesystem databases, cloud stores, or product session
  stores in `agent-sdk-core`.
- Do not let a session repository replace `RunJournal`. A session layer should
  project from journals, checkpoints, content refs, and event cursors.
- Do not add a tool-execution database before idempotency, replay, and
  result-cache semantics are written against the effect spine.

## Next Adapter Shape

The next useful persistence crate should be a small optional store crate, not a
new core primitive:

1. `agent-sdk-store-file`: append-only journal files, checkpoint snapshots,
   content blobs by content hash, and event cursor archives for local examples.
2. `agent-sdk-store-sqlite`: the same surfaces with explicit tables and
   migration fixtures, plus reuse of the existing toolkit SQLite agent-pool
   adapter if the dependency boundary stays clean.
3. `agent-sdk-session`: a projection layer over stores, not an alternate source
   of truth. It should repair conversation windows and model-facing histories by
   reading journals and content refs.

Each adapter needs deterministic fixtures for append ordering, cursor replay,
checkpoint restore, missing content refs, redaction, interrupted effects, and
crash recovery before it is advertised as durable.
