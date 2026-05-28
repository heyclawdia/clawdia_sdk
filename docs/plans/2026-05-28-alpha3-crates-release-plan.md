# 2026-05-28 Alpha 3 Crates Release Plan

## Objective

Publish the `0.1.0-alpha.3` SDK crate family to crates.io through the repository release workflow after confirming the newly added toolkit environment-runtime helper is documented in code and the release workflow publishes every current crate.

## Root Cause / Problem Shape

The checkout is already a `0.1.0-alpha.3` release candidate and contains the local-only commit `b57c0a7 Add typed toolkit environment runtimes`. The publish workflow still only publishes the older two-crate set (`agent-sdk-core` and `agent-sdk-toolkit`), while the current workspace contains four publishable crates:

- `agent-sdk-core`
- `agent-sdk-eval`
- `agent-sdk-provider`
- `agent-sdk-toolkit`

Triggering the release without updating the workflow would leave the `0.1.0-alpha.3` family incomplete on crates.io.

## Relevant Existing Context

- `AGENTS.md`: no branches unless explicitly approved; public release must run `scripts/public-release-audit.sh`; do not publish if the audit fails.
- `README.md`: checkout is a `0.1.0-alpha.3` release candidate and the post-publish dependency shape includes core, eval, toolkit, and provider.
- `docs/start-here.md`: release/broad handoff checks must use the release-readiness target and public audit.
- `coding_standards.md` and `docs/architecture/coding-standards.md`: public APIs need clear rustdoc, stable crate metadata, product-neutral boundaries, and `cargo clippy --workspace --all-targets -- -D warnings`.
- `docs/workstreams/validation-gates.md`: implementation/release handoff requires exact validation evidence, primitive-lowering evidence, and preserved SDK/host-owned boundaries.
- `docs/implementation-workstreams/13-release-readiness/13a-release-readiness.md`: release readiness owns package metadata, crate READMEs/docs, changelog, and final handoff evidence; publishing is allowed because the user explicitly requested release execution.
- `docs/reference/sdk-review-checklist.md`: review must check simplicity, product-neutrality, public facade stability, Rust API documentation, privacy, durability, and optional-crate boundaries.
- `CHANGELOG.md`: `0.1.0-alpha.3` release notes exist but do not yet mention typed toolkit environment runtimes.
- `.github/workflows/publish-crates.yml`: currently validates and publishes only core/toolkit.

## Behavior Contract

### New Behavior

- The publish workflow validates package dry-runs and publishes all four `0.1.0-alpha.3` crates in dependency order:
  1. `agent-sdk-core`
  2. `agent-sdk-eval`
  3. `agent-sdk-provider`
  4. `agent-sdk-toolkit`
- The workflow keeps idempotent "publish if missing" behavior for reruns.
- The workflow parses local `cargo pkgid` output that uses `#<version>` as well
  as registry-style output that uses `@<version>`, so crates.io lookups use the
  real version string.
- `EnvironmentRuntime` and `AgentWorkspaceEnvironmentProfile::runtime` have rustdoc that makes clear they are data-only aliases/lowering helpers and do not register or run isolation adapters.
- Release notes mention the typed toolkit environment runtime helper.

### Preserved Behavior

- `agent-sdk-core` remains product-neutral and independent of optional crates.
- Toolkit environment helpers continue lowering into core `ExecutionEnvironment`, `IsolationRequirement`, and `IsolationRuntimeRef` contracts.
- Hosts and optional runtime adapter crates still own adapter registration, policy enforcement, process/container startup, networking, and cleanup.
- Release workflow still runs the public audit, formatting check, and workspace tests before publishing.

### Removed Behavior

- None. The workflow's two-crate publish list is extended rather than replaced with a different release mechanism.

### Tests / Proof

- `scripts/public-release-audit.sh`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo doc --workspace --no-deps`
- `cargo publish -p agent-sdk-core --dry-run`
- `cargo publish -p agent-sdk-eval --dry-run` and `cargo publish -p agent-sdk-provider --dry-run` when the matching core version is already visible on crates.io; otherwise the real publish job validates them after core propagation.
- `cargo publish -p agent-sdk-toolkit --dry-run` when core and eval are already visible on crates.io; otherwise the real publish job validates it after dependency propagation.
- GitHub release/workflow evidence after the release is created.
- crates.io confirmation for all four `0.1.0-alpha.3` crates.

## Workstreams

1. Documentation polish: improve rustdoc for `EnvironmentRuntime` and `AgentWorkspaceEnvironmentProfile::runtime`; add changelog coverage.
2. Release workflow fix: update `.github/workflows/publish-crates.yml` to dry-run dependencies when their registry prerequisites are visible and publish core, eval, provider, and toolkit in dependency order with registry propagation retries where needed.
3. Verification: run public audit, formatting, clippy, tests, docs, and dry-run packaging for each crate.
4. Release execution: push the release-ready commit(s), create the `v0.1.0-alpha.3` prerelease, watch the publish workflow, and confirm crates.io availability.

## Risk / Gotcha Carry-Forward

- Do not publish if `scripts/public-release-audit.sh` flags personal data, local paths, credentials, or missing ignore coverage.
- Do not publish toolkit before both `agent-sdk-core` and `agent-sdk-eval` are visible on crates.io; toolkit has registry-pinned dependencies on both.
- Do not assume `agent-sdk-provider` is transitively published by toolkit; it is an independent optional adapter crate.
- Do not derive crates.io version checks with an `@`-only parser; local workspace package IDs use `#<version>`.
- Keep `EnvironmentRuntime` data-only. If a future runtime crate registers real adapters, route that through core isolation runtime ports and host policy, not this enum.
- Keep release workflow rerunnable. Existing crates at the same version should be skipped rather than failing the entire publish job.
