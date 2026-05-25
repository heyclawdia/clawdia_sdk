# agent-sdk-toolkit

`agent-sdk-toolkit` is an optional helper crate for concrete tool-pack behavior and adapter conformance scaffolding that should not live in `agent-sdk-core`. It layers filesystem read/search/edit/write, shell, resource-reader, discovery, and protocol-test helpers over core runtime packages, policy refs, content refs, tool ports, isolation ports, and effect lineage.

## Public Surface

SDK consumers should import through the crate root:

- `ToolkitPackBundle` and `tool_snapshot`
- `BoundedWorkspace`, `WorkspaceReadExecutor`, `WorkspaceSearchExecutor`, `WorkspaceEditExecutor`, `WorkspaceWriteExecutor`
- `WorkspaceReadDetection`, `WorkspaceFileKind`, `WorkspaceReaderStep`, `WorkspaceMediaMetadata`, `WorkspaceDocumentMetadata`, and `WorkspaceArchiveMetadata` for format-aware read output
- `ShellExecutor`, `ResourceReaderExecutor`, and `ToolDiscoveryExecutor`
- `SqliteAgentPoolStore` for file-backed `AgentPoolStore` coordination across
  independent local handles
- `JsonRpcLineCodec` and `JsonRpcLineEndpoint` for JSON-RPC stdio-style line framing
- `testing::InMemoryJsonArgumentStore` and `testing::InMemoryToolkitContentStore` for deterministic tests
- `testing::{ScriptedAcpClient, ScriptedAcpAgent, McpHostProxy, ScriptedMcpServer, IsolatedJsonRpcProcess}` for transport-level ACP/MCP conformance tests

## Workspace Readers

`workspace_read` detects file types before choosing a reader. The current implementation supports bounded UTF-8 text/Markdown/JSON reads with hashline anchors, PDF text extraction through `pdf-extract`, OCR sidecar fallback for scanned PDFs/images, image metadata through `image`, HEIC/AVIF-style dimension probing, DNG/TIFF RAW dimension/strip/embedded-preview metadata, Apple Photos `.AAE` sidecar summaries, DOCX/PPTX/XLSX OpenXML text extraction, legacy `.doc`/`.xls`/`.ppt` bounded sidecar fallbacks, ZIP/TAR/TGZ/GZIP archive listings, SQLite schema/sample reads, local `data:` URL reads, fail-closed external URI handling, and safe binary summaries.

Large safe files return bounded prefixes with `truncated: true` and guidance to use `workspace_search`/grep or a narrower/range read. Full-file parsers such as PDF, Office, archive, image, and SQLite readers downgrade to summaries for oversized inputs instead of loading the whole file.

Unsupported or partial cases return typed warnings rather than raw bytes: live OCR engines, full proprietary RAW demosaicing, full Apple Photos library adjustment application, encrypted PDFs, live network URL fetches, and high-fidelity legacy Office binary rendering need later host or optional adapter support.

## Package Boundary

The toolkit crate may provide concrete helper implementations and deterministic protocol fakes, but it must not become a hidden runtime, package registry, approval path, event stream, journal, or host product adapter. Every tool helper still lowers into core capability snapshots, policy checks, content refs, and effect intent/result records. `SqliteAgentPoolStore` is a concrete `AgentPoolStore` adapter, not a daemon or broker: it stores pool-scoped coordination records that core replays into snapshots and watches. ACP and MCP mocks exchange encoded UTF-8 JSON-RPC frames over newline-style line transports, reject embedded newlines, include strict JSON-RPC response IDs, and model required lifecycle notifications so conformance tests can prove protocol behavior without live editors, live MCP servers, or product hosts. Scripted fakes live under the `testing` namespace; production-facing wire primitives live under `protocol`.

## Unsupported In This Handoff

The toolkit does not own live shell policy for a host machine, network execution, product workspaces, remote file systems, or UI approval flows. Hosts must provide those policies and adapters explicitly.
