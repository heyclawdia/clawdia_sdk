# agent-sdk-store-file

Filesystem-backed durable store adapters for the Agent SDK.

The crate keeps each store contract in a small responsibility module:
journal, checkpoint, content, event archive, provider arguments, agent pool,
tool execution, and a bundle for hosts that want all adapters rooted under one
directory.

| Surface | Adapter |
| --- | --- |
| `RunJournal` / `RunJournalReader` | `FileRunJournal` |
| `CheckpointStore` | `FileCheckpointStore` |
| `ContentStore` / `ContentResolver` | `FileContentStore` |
| `EventArchive` / `EventArchiveReader` | `FileEventArchive` |
| `AgentPoolStore` | `FileAgentPoolStore` |
| `ToolExecutionStore` | `FileToolExecutionStore` |
| `ProviderArgumentStore` | `FileProviderArgumentStore` |

## Under The Hood

`ToolExecutionStore` is a rebuildable projection over journaled tool records.
It supports run, tool-call, effect-id, idempotency-key, and journal-cursor
range lookups, but it does not approve tools, release executors, decide replay
safety, or replace `RunJournal` truth.

Backup policy, retention, file ownership, and product session state remain
host-owned.
