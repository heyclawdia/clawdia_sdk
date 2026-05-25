# Toolkit Module Layout Refactor Plan

## Objective

Refactor `agent-sdk-toolkit` so `mod.rs` files are small facades and real behavior lives in responsibility-named files. Preserve the public crate-root API and toolkit behavior while making shell, resources, discovery, pack wiring, testing stores, and reader dispatch easier for future agents to find.

## Root Cause / Problem Shape

The toolkit crate already has good operation-level files under `workspace/`, but several toolkit namespaces still keep behavior directly in `mod.rs`: `discovery`, `shell`, `resources`, `packs`, `testing`, and `workspace/readers`. That violates the repo layout rule that `mod.rs` should declare modules and re-export stable surfaces only.

## Authoritative Source Of Truth

- Launch target: `docs/implementation-workstreams/10-feature-ports/10e-tool-packs.md`
- Contract: `docs/contracts/tool-pack-contract.md`
- Public API surface: `crates/agent-sdk-toolkit/src/lib.rs` and `crates/agent-sdk-toolkit/README.md`

## Relevant Existing Context

- `AGENTS.md`: do not create branches; keep toolkit operations in files future agents can search for; preserve format-aware readers and bounded outputs.
- `README.md` and `docs/start-here.md`: `agent-sdk-toolkit` is optional and layered over `agent-sdk-core`; product-specific host behavior stays out.
- `coding_standards.md` and `docs/architecture/coding-standards.md`: source roots and facades must stay navigable; real behavior belongs in meaningful files, not catch-all `mod.rs`.
- `docs/workstreams/validation-gates.md`: implementation changes need concrete validation, package-layout evidence, and primitive-lowering evidence.
- `docs/reference/sdk-review-checklist.md`: protect simplicity, product-neutrality, optional crate boundaries, and public facade stability.
- `docs/architecture/primitive-map.md`: toolkit packs are feature-layer helpers over package, policy, content refs, effect intent/result, and typed ports.
- `docs/architecture/external-sdk-lessons.md`: keep one model-facing read/search/edit/write surface while routing through typed internal implementation files.
- `docs/reference/simplicity-audit.md`: toolkit presets/helpers must lower through the canonical contracts rather than a separate easy path.
- `docs/implementation-workstreams/10-feature-ports/10e-tool-packs.md`: owned writable surface includes optional `crates/agent-sdk-toolkit/`; do not put read/search/edit/write/shell behavior into core.
- `docs/implementation-workstreams/10-feature-ports/_phase/phase-exit-report.md`: the phase already exited with toolkit as optional and product-neutral; this refactor must preserve that evidence.

## Behavior Contract

New behavior:

- `mod.rs` files in the toolkit crate become small facades with module declarations and re-exports.
- Shell behavior is split into policy, request/result types, executor, and process execution files.
- Resource reader behavior is split into request types, executor, in-memory resolver, and policy helper files.
- Tool discovery behavior is split into index, executor, request/output types, and policy helper files.
- Pack bundle and tool snapshot construction are split into their own files.
- In-memory test stores move out of `testing/mod.rs`.
- Reader dispatch and rendered output structs move out of `workspace/readers/mod.rs`.

Preserved behavior:

- Public crate-root exports remain source-compatible.
- `agent-sdk-toolkit` remains optional and layered over `agent-sdk-core`.
- Toolkit packs keep the same policy refs, executor refs, schemas, content-ref outputs, and package lowering behavior.
- Workspace reader routing, truncation guidance, and fail-closed URI behavior are unchanged.

Removed behavior:

- None. This is a structural refactor, not a semantic change.

Tests proving this behavior:

- `cargo fmt --check`
- `cargo test -p agent-sdk-toolkit`
- `cargo test -p agent-sdk-core --test tool_pack_boundary`
- Layout audit: line counts for toolkit `mod.rs` files and direct crate-root source files.

## Scope

Writable in this task:

- `crates/agent-sdk-toolkit/src/**`
- `docs/plans/2026-05-24-toolkit-module-layout-plan.md`
- A focused docs risk note if the refactor reveals durable guidance not already captured.

Out of scope:

- Changes to `agent-sdk-core` behavior.
- New toolkit operations or public APIs.
- Product-specific host adapters, live providers, shell policy changes, or reader capability claims.
- Existing unrelated worktree changes, including `docs/architecture/architecture-proposal.md`.

## Workstreams

1. Split toolkit namespace `mod.rs` files into named implementation files while preserving public exports.
2. Run formatting and focused tests.
3. Do a review pass against the launch target, standards, public facade, and no-mini-SDK boundary.
4. Stage and commit only this refactor's files if validation passes.

## Validation Plan

- `cargo fmt --check`
- `cargo test -p agent-sdk-toolkit`
- `cargo test -p agent-sdk-core --test tool_pack_boundary`
- `find crates/agent-sdk-toolkit/src -name 'mod.rs' -exec wc -l {} +`
- `find crates -path '*/src/*.rs' -maxdepth 3 -type f | sort`
- `git diff --check`

## Risk / Gotcha Carry-Forward

- If a future toolkit operation grows, put executor, request/output types, policy helpers, and low-level adapters in named files before adding more behavior.
- Do not create new public deep-import promises while splitting files; keep stable imports through the crate root and existing public modules.
- Keep shell execution policy unchanged; this refactor must not loosen fail-closed host-execution or network checks.
- Keep resource and URI reading fail-closed unless a host resolver/policy is explicitly attached.
- Keep workspace reader dispatch one model-facing `workspace_read` surface with typed internal routes; do not split it into multiple model-facing tools just because implementation files are separate.
