# agent-sdk-store-postgres

Scripted Postgres-style durable store adapters for the Agent SDK.

This crate is an adapter contract over a host-owned SQL transport. It proves SQL
statement shape, parameter binding, row decoding, and explicit surface mapping
without provisioning a live database.

| Surface | Adapter |
| --- | --- |
| `RunJournal` / `RunJournalReader` | `PostgresRunJournal` |
| `CheckpointStore` | `PostgresCheckpointStore` |
| `ContentStore` / `ContentResolver` | `PostgresContentStore` |
| `EventArchive` / `EventArchiveReader` | `PostgresEventArchive` |
| `AgentPoolStore` | `PostgresAgentPoolStore` |
| `ToolExecutionStore` | `PostgresToolExecutionStore` |
| `ProviderArgumentStore` | `PostgresProviderArgumentStore` |

## Under The Hood

Live connection pools, migrations, RLS, backups, retention policy, and database
provisioning are host-owned. `ToolExecutionStore` is only a rebuildable
projection over journaled tool records. It supports run, tool-call, effect-id,
idempotency-key, and journal-cursor range lookups; `RunJournal` remains durable
truth.
