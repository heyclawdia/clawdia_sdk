# Tool Execution

## Phase

[Phase 09: P2 Side Effects](README.md)

## Parallelism

Parallel-safe with the other Phase 09 side-effect launch targets. Built-in tool packs wait until Phase 10.

## Contract Inputs

- [tool-approval-contract.md](../../contracts/tool-approval-contract.md)
- [tool-pack-contract.md](../../contracts/tool-pack-contract.md)
- [runtime-package-schema.md](../../contracts/runtime-package-schema.md)

## Implementation Objective

Implement generic tool routing and execution over runtime-package capabilities and the shared effect spine.

## Owned Implementation Surface

- tool domain IDs/policy additions in existing `crates/agent-sdk-core/src/domain/` modules where needed
- tool durable records in `crates/agent-sdk-core/src/records/tool.rs`
- tool registry/executor ports in `crates/agent-sdk-core/src/ports/tool.rs`
- tool routing/execution coordination in `crates/agent-sdk-core/src/application/tool.rs`
- root Cargo test shim `crates/agent-sdk-core/tests/tool_execution_contract.rs`
- test body `crates/agent-sdk-core/tests/feature_layers/tool_execution_contract.rs`
- fixture files under `crates/agent-sdk-core/tests/fixtures/tools/`

Do not add flat implementation files directly under `src/`; exports from `lib.rs`
are integration/stitching glue.

## Must Deliver

- Tool registry snapshot, router, executor port, execution strategy shell, and tool call records.
- Intent-before-executor access for every tool call, including reads.
- Tool result refs and redacted summaries.
- Missing executor/policy/journal denial paths.

## Validation

- `cargo test -p agent-sdk-core --test tool_execution_contract`
- read-tool intent/result tests
- non-idempotent result append failure enters recovery
- core without toolkit test

## Must Not

- Add filesystem, shell, edit, MCP, or workspace tool-pack behavior directly to core.
- Execute tools outside runtime-package/policy authority.
