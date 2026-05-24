# Phase 00: Crate Foundation

Create the Rust workspace and test harness before any domain work starts.

## Launch Targets

| Title | Run in parallel? | Purpose |
| --- | --- | --- |
| [Workspace Skeleton](00a-workspace-skeleton.md) | only target | Create crate boundaries, CI commands, fixture layout, and no-product baseline checks. |

## Exit Gate

- [x] Workspace crates compile with no product-specific dependencies.
- [x] Test/fixture directories and cargo commands exist for later phases.
- [x] Core crate can compile without toolkit, isolation, extension, OTel, workflow, or host-adapter features.
- [x] Phase exit report records reviewer PASS.
