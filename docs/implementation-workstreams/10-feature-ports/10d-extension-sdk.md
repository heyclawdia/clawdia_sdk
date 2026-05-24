# Extension SDK

## Phase

[Phase 10: Feature Ports](README.md)

## Parallelism

Parallel-safe with the other Phase 10 feature-port launch targets.

## Contract Inputs

- [extension-sdk-contract.md](../../contracts/extension-sdk-contract.md)
- [runtime-package-schema.md](../../contracts/runtime-package-schema.md)
- [tool-approval-contract.md](../../contracts/tool-approval-contract.md)
- [event-schema.md](../../contracts/event-schema.md)

## Implementation Objective

Implement SDK-facing extension capability and action boundaries while keeping host manifests and runtime packaging outside core authority.

## Owned Implementation Surface

- extension package sidecars/catalog snapshots in `crates/agent-sdk-core/src/package/extension.rs`
- extension durable records in `crates/agent-sdk-core/src/records/extension.rs`
- extension action/host boundary ports in `crates/agent-sdk-core/src/ports/extension.rs`
- extension action coordination and approval lowering in `crates/agent-sdk-core/src/application/extension.rs`
- root Cargo test shim `crates/agent-sdk-core/tests/extension_contract.rs`
- test body `crates/agent-sdk-core/tests/feature_layers/extension_contract.rs`
- optional package smoke tests under `crates/agent-sdk-extension/tests/` if the optional crate is created
- fixture files under `crates/agent-sdk-core/tests/fixtures/extensions/`
- optional `crates/agent-sdk-extension/` only if the phase exit plan chooses a crate split

Do not add flat implementation files directly under `src/`; exports from `lib.rs`
are integration/stitching glue.

## Must Deliver

- `CoreExtensionCapabilities`, extension catalog snapshots, action refs, action sidecars, approval integration, extension action intent/result, terminal action events, and protocol error recovery.
- Package subpath smoke tests for ESM, public exports, browser-safe helpers, and temp-directory execution if optional crate is created.
- Core-vs-host manifest audit.

## Validation

- `cargo test -p agent-sdk-core --test extension_contract`
- optional `cargo test -p agent-sdk-extension`
- extension action golden fixtures
- package/subpath smoke tests

## Must Not

- Make host manifests, runtime/install/marketplace/trust/browser-safe export metadata, or app-event transport core package authority.
- Let extensions approve themselves.
