# Tool Packs

## Phase

[Phase 10: Feature Ports](README.md)

## Parallelism

Parallel-safe with the other Phase 10 feature-port launch targets. Depends on Phase 09 generic tool execution, not on sibling feature ports.

## Contract Inputs

- [tool-pack-contract.md](../../contracts/tool-pack-contract.md)
- [tool-approval-contract.md](../../contracts/tool-approval-contract.md)
- [runtime-package-schema.md](../../contracts/runtime-package-schema.md)

## Implementation Objective

Implement optional toolkit packs over the core tool executor without moving product coding harness behavior into core.

## Owned Implementation Surface

- optional `crates/agent-sdk-toolkit/` for concrete read/search/edit/write/shell/resource/discovery pack helpers
- tool-pack package sidecar additions in `crates/agent-sdk-core/src/package/tool_pack.rs` only for product-neutral typed sidecar snapshots and fingerprint inputs
- tool-pack boundary/lineage records in `crates/agent-sdk-core/src/records/tool_pack.rs` only when core must serialize package sidecar or effect metadata
- tool-pack boundary ports in `crates/agent-sdk-core/src/ports/tool_pack.rs` only for product-neutral resource routing abstractions
- root Cargo test shim `crates/agent-sdk-core/tests/tool_pack_boundary.rs`
- test body `crates/agent-sdk-core/tests/feature_layers/tool_pack_boundary.rs`
- toolkit fixture workspaces under `crates/agent-sdk-toolkit/tests/fixtures/` if the optional crate is created

Do not add flat implementation files directly under `src/`; exports from `lib.rs`
are integration/stitching glue.

## Must Deliver

- Read/search/edit/write/shell/resource-reader/tool-discovery packs as optional capabilities with typed sidecars.
- Workspace bounds, file-size limits, anchor validation, preview/apply split, shell sandbox policy, resource URI routing, and effect lineage.
- Core crate tests proving `agent-sdk-core` compiles and runs without toolkit.

## Validation

- `cargo test -p agent-sdk-core --test tool_pack_boundary`
- optional `cargo test -p agent-sdk-toolkit`
- anchored edit precondition tests
- shell denial and cancellation tests
- package fingerprint fixtures for activated packs

## Must Not

- Put read/search/edit/write/shell behavior into `agent-sdk-core`.
- Promise product undo/revert or own a coding-agent UI.
