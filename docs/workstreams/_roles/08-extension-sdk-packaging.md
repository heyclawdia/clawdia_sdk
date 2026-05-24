# Owner Role 08: Extension SDK And Packaging

## Owner Role

Extension protocol and packaging agent.

## Writable Files

- `docs/contracts/extension-sdk-contract.md`

## Future Implementation Writable Scope

Once SDK code exists, this workstream may own extension protocol/packaging modules and tests only, for example:

- `crates/agent-sdk-extension/**`
- `crates/agent-sdk-core/src/extensions/**`
- `crates/agent-sdk-core/tests/extension_*.rs`
- package/import smoke fixtures for extension SDK subpaths

## Read-Only Inputs

- `docs/contracts/runtime-package-schema.md`
- `docs/contracts/tool-approval-contract.md`
- `docs/contracts/telemetry-privacy-contract.md`
- `docs/architecture/architecture-proposal.md`
- `docs/architecture/primitive-map.md`
- `docs/examples/README.md`

## Contract To Deliver

Define `CoreExtensionCapabilities` versus `HostExtensionManifest` boundaries, optional `agent-sdk-extension` JSON-RPC process protocol, tool/hook/subagent provider adapters, app-event observation types, host action submission, browser-safe helper subpaths, Node/Bun support boundaries, and packaging smoke tests. `agent-sdk-core` must see only typed core capabilities, core `HookSpec`/`HookResponse` ports, policy-crossing requests, and event/journal records.

## Must Not Own

Extension marketplace UX, packaged runtime resource placement, approval authority, memory authority, provider routing, telemetry ownership, or `agent-sdk-core` ownership of extension subprocess/app-event runtime.

## Integration Handoff

Send capability names, JSON-RPC method names, package subpath names, and runtime-package extension fields to the stitching owner. Put proposal text in the handoff; do not edit shared reference or architecture files unless the stitching owner delegates it.

## Required Validation

- Manifest tests: core capability declaration, host manifest separation, denied undeclared capability, version mismatch, and host-owned authority boundaries.
- JSON-RPC tests: request/response shape, cancellation, timeout, error mapping, and no approval/memory/provider/telemetry ownership by extension.
- Packaging smoke tests: Node ESM import through normal `node_modules`, public subpath imports, browser-safe helper imports, and Bun/Node boundary where supported.
- Security tests: extension-submitted tool/action routes through host policy; extension cannot approve itself or bypass runtime package.
- Hook bridge tests: extension hooks lower into core `HookSpec`, respect core hook timeout/failure/mutation-right contracts, and do not import JSON-RPC/runtime code into `agent-sdk-core`.
- App-event tests: extension observation uses bounded metadata and cannot become durable app-event owner.
- Core-boundary tests: `agent-sdk-core` has no extension runtime, JSON-RPC subprocess, UI surface, or app-event transport imports.
- Compatibility audit: no `NODE_PATH` fallback documented unless verified by smoke test.
- Primitive-lowering review: extensions declare `CoreExtensionCapabilities` that hosts resolve into `RuntimePackage` sidecars/capabilities after policy checks; extension runtime cannot own approval, memory, provider routing, telemetry, or durable app events.
- Handoff evidence: manifest fixtures, JSON-RPC fixtures, import smoke command output, browser-safe bundle check, and unsupported-runtime notes.
