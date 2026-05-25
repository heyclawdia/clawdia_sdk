# Crates Alpha Release Plan

## Objective

Publish the public Agent SDK crates from `<repo-root>` as the first alpha release and add a GitHub Actions release workflow that can publish the crates to crates.io using the repository secret.

## Problem Shape

Phase 13 release readiness intentionally left both packages with `publish = false` because no publish/tag release had been requested. The repo is now public and the user explicitly requested crates.io publishing, so the release boundary needs to move from handoff-only to an alpha package release without broadening SDK support claims.

## Authoritative Source Of Truth

- `README.md` and `docs/start-here.md`: product-neutral SDK workspace and crate boundary.
- `coding_standards.md` and `docs/architecture/coding-standards.md`: package metadata, public facade, mockability, and release validation posture.
- `docs/implementation-workstreams/13-release-readiness/13a-release-readiness.md`: publishing is allowed only when explicitly requested.
- `docs/implementation-workstreams/13-release-readiness/_phase/phase-exit-report.md`: prior release-readiness evidence and unsupported-path release notes.
- `docs/reference/sdk-review-checklist.md` and `docs/architecture/primitive-map.md`: keep host/product/live-provider behavior out of release claims.

## Behavior Contract

New behavior:

- `agent-sdk-core` and `agent-sdk-toolkit` are configured for a crates.io alpha release.
- The alpha version is `0.1.0-alpha.1`, matching the user's "0.1 alpha" request as a SemVer pre-release.
- `agent-sdk-toolkit` declares a versioned dependency on `agent-sdk-core` so crates.io can publish the path dependency correctly.
- GitHub Actions publishes crates on GitHub Release publication, validating the workspace before publishing `agent-sdk-core` first and then `agent-sdk-toolkit`.
- The repo secret used by the workflow is `CARGO_REGISTRY_TOKEN`.
- Public-release validation includes a personal/sensitive information scan and a `.gitignore` guard for local files, credentials, env files, private keys, and build artifacts before publishing.

Preserved behavior:

- Core remains product-neutral and independent from toolkit.
- Unsupported live provider, concrete runtime, product UI, marketplace, workflow-engine, and host-adapter claims remain unsupported in release notes and crate READMEs.
- Existing local user edits outside the release surface remain untouched.

Removed behavior:

- The package manifests no longer block publishing with `publish = false`.

Tests proving this behavior:

- `cargo fmt --check`
- `cargo test --workspace`
- `cargo publish -p agent-sdk-core --dry-run`
- `cargo test --workspace` covers toolkit locally before publication; `agent-sdk-toolkit` package verification must happen in the release workflow after `agent-sdk-core` is visible on crates.io because Cargo requires the registry dependency to exist first.
- `git diff --check`

## Scope

In scope:

- Cargo release metadata and alpha versions.
- GitHub Actions release workflow under `.github/workflows/`.
- Changelog/release-readiness docs that currently say the package is not published.
- GitHub repo secret configuration for crates.io publishing.
- Commit, push to `main`, GitHub release creation, and workflow monitoring because they are required to publish.
- Public-repo release criteria and audit automation for personal/sensitive content and `.gitignore` coverage.

Out of scope:

- New SDK primitives, feature support, live providers, concrete containers, product hosts, or optional adapter crates.
- Branch creation.
- Unrelated toolkit documentation edits already present in the working tree.

## Workstreams

1. Release metadata: update Cargo manifests and changelog/readiness wording for `0.1.0-alpha.1`.
2. Release automation: add a focused GitHub Actions workflow that validates and publishes the two crates in dependency order.
3. Secret and release execution: set `CARGO_REGISTRY_TOKEN`, validate locally, commit only release files, push to `main`, create GitHub release `v0.1.0-alpha.1`, and monitor the publish run.

## Validation Plan

- Verify repo visibility and default branch.
- Run `scripts/public-release-audit.sh` locally and in GitHub Actions.
- Run local cargo formatting and tests.
- Run local publish/package checks that do not require core to already exist on crates.io.
- After push/release, inspect the GitHub Actions run and crates.io publication status.

## Relevant Existing Context

- `README.md` states the previous posture was "not a publish/tag release"; this must be updated carefully.
- Phase 13 exit evidence says both crates used `publish = false` only because no release was requested.
- The toolkit depends on core, so publishing must happen in dependency order.
- User-level repo instructions forbid branch creation without explicit approval.
- The repository is public, so release automation must fail before publish if tracked files include likely personal information, local absolute paths, credentials, private keys, env files, or common secret token formats.

## Risk / Gotcha Carry-Forward

- If future releases add live providers or adapters, keep those in optional crates or host layers with matching tests before making release claims.
- If the publish workflow is rerun after a partial publish, handle already-published core/toolkit versions deliberately rather than changing history.
- If a future toolkit release depends on a new core alpha, keep the manifest's registry version aligned with the workspace version before publishing.
- If release validation needs to skip a command, record whether that skip blocks publishing rather than treating skipped validation as confidence.
- If future docs need local paths for agent launch instructions, keep them as placeholders such as `<repo-root>` rather than personal machine paths.
