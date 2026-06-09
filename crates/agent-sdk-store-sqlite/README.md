# agent-sdk-store-sqlite

SQLite-backed durable store adapters for the Agent SDK.

The crate maps one database file to explicit SDK persistence surfaces:

| Surface | Adapter |
| --- | --- |
| `RunJournal` / `RunJournalReader` | `SqliteRunJournal` |
| `CheckpointStore` | `SqliteCheckpointStore` |
| `ContentStore` / `ContentResolver` | `SqliteContentStore` |
| `EventArchive` / `EventArchiveReader` | `SqliteEventArchive` |
| `AgentPoolStore` | `SqliteAgentPoolStore` |
| `ToolExecutionStore` | `SqliteToolExecutionStore` |
| `ProviderArgumentStore` | `SqliteProviderArgumentStore` |

## Under The Hood

`ToolExecutionStore` is a rebuildable projection over journaled tool records. It
supports run, tool-call, effect-id, idempotency-key, and journal-cursor range
lookups, but it does not approve tools, release executors, decide replay
safety, or replace `RunJournal` truth.

Live backup policy, retention, migration rollout, and product session state
remain host-owned.
