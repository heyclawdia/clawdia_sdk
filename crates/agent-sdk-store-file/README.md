# agent-sdk-store-file

Filesystem-backed durable store adapters for the Agent SDK.

The crate keeps each store contract in a small responsibility module:
journal, checkpoint, content, event archive, provider arguments, agent pool,
and a bundle for hosts that want all adapters rooted under one directory.
