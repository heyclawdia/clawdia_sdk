# Tool Packs

## Phase

[Phase 07: Feature Ports](README.md)

## Parallelism

Parallel-safe with the other Phase 07 feature-port launch targets. Depends on Phase 06 generic tool execution, not on sibling feature ports.

## Contract Inputs

- [tool-pack-contract.md](../../contracts/tool-pack-contract.md)
- [tool-approval-contract.md](../../contracts/tool-approval-contract.md)
- [runtime-package-schema.md](../../contracts/runtime-package-schema.md)

## Implementation Objective

Implement optional toolkit packs over the core tool executor without moving product coding harness behavior into core.

## Owned Implementation Surface

- optional `crates/agent-sdk-toolkit/`
- `crates/agent-sdk-core/tests/tool_pack_boundary.rs`
- toolkit fixture workspaces under `crates/agent-sdk-toolkit/tests/fixtures/` if the optional crate is created

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
