# Portable Environment Gateway Plan

## Objective

Add a portable SDK surface for agent execution environments that lets callers describe workspace isolation, lifecycle, secrets, and network egress policy once, then lower that description into the existing `ExecutionEnvironment` and `RuntimePackage` isolation contracts. The first implementation slice must not ship a concrete runtime adapter, browser tooling, or new core primitive. It should make local and future remote adapters easier to implement and test through the same core contract.

## Problem Shape

The core SDK already has the right isolation spine: `ExecutionEnvironment`, `IsolationRequirement`, `NetworkIsolationPolicy`, `IsolationRuntime`, capability reports, journaled lifecycle records, and fail-closed downgrade checks. The missing piece is a developer-facing environment profile that expresses common agent-workspace intent without making users hand-build every runtime-package sidecar or write runtime-specific network policy by hand.

The structural fix is to keep core as the portable contract owner and add toolkit helpers that canonicalize egress targets without performing network or container work. Those helpers lower into the existing core `NetworkIsolationPolicy`, `ExecutionEnvironment`, and `IsolationRequirementSnapshot` types. Concrete local, server, or remote runtime adapters remain optional crates or host code that implement `IsolationRuntime` and truthfully report which environment policies they can enforce.

## Primary Authority And Writable Scope

Primary launch authority: `docs/implementation-workstreams/10-feature-ports/10e-tool-packs.md`.

Referenced isolation contract authority: `docs/implementation-workstreams/10-feature-ports/10b-isolation-port.md`. This slice does not edit core isolation files; it consumes the existing core contract from toolkit.

Writable files for this slice:

- `crates/agent-sdk-toolkit/src/environment/**`
- `crates/agent-sdk-toolkit/src/lib.rs`
- `crates/agent-sdk-toolkit/tests/**`
- `docs/plans/2026-05-28-portable-environment-gateway-plan.md`

## Relevant Existing Context

- `AGENTS.md`: keep the packet product-neutral, do not add product-specific host adapters to the active handoff, do not create branches, and preserve explicit SDK-owned versus host-owned boundaries.
- `coding_standards.md` and `docs/architecture/coding-standards.md`: helpers must lower into canonical contracts, all side-effect boundaries must be mockable, and core must fail closed when required adapters or policies are absent.
- `docs/workstreams/validation-gates.md`: implementation needs contract/unit tests, mockability evidence, primitive-lowering evidence, and no parallel run loop, package registry, event stream, journal, policy path, or side-effect path.
- `docs/reference/sdk-review-checklist.md`: isolation must be first-class, no silent host-process downgrade is allowed, and optional adapters must stay outside core.
- `docs/architecture/primitive-map.md`: `NetworkIsolationPolicy` and `ExecutionEnvironment` are the correct feature-layer primitives; concrete runtime behavior is an optional adapter behind `IsolationRuntime`.
- `docs/contracts/isolation-runtime-contract.md`: core owns schemas, downgrade semantics, event/journal names, redaction defaults, replay/recovery rules, and helper lowering; hosts and optional crates own concrete platform APIs, image stores, network plumbing, cleanup, and reclaim.
- `docs/agent-sdk-toolkit/adapter-and-runtime-plan.md`: access/profile helpers are the intended simple interface for common isolated-agent setups, with safe defaults and fail-closed runtime fallback.

## External Research Summary

Current container and orchestration ecosystems converge on the same constraint: runtime-neutral APIs can describe egress, mounts, lifecycle, and process intent, but enforcement quality depends on the selected runtime and host privileges. Network policy may be host-side, VM-level, namespace-level, proxy-mediated, or adapter-native. Domain allowlists are not equivalent to packet filters unless DNS, proxy, SNI, direct-IP bypass, and verification are part of the enforcement claim.

Design implication: the SDK should expose typed policy intent and capability checks. It should not claim that a runtime enforces a policy until an adapter capability report says so and tests prove the selected enforcement path.

## Behavior Contract

New behavior:

- Toolkit exposes typed egress target helpers that canonicalize host/port/protocol allowlist entries into core `NetworkIsolationPolicy` values without performing I/O.
- Toolkit exposes an ergonomic agent workspace environment profile with safe defaults: no network, no ambient secrets, snapshot workspace mounting, cleanup-required lifecycle, and explicit isolation class/runtime selection.
- Toolkit profile helpers can add egress allowlist entries and automatically add the corresponding isolation capability requirement.
- Toolkit profile lowering produces both an `ExecutionEnvironment` and an `IsolationRequirementSnapshot` so callers can attach the snapshot to a `RuntimePackage`.
- Tests prove that helper output is deterministic, invalid egress targets fail before lowering, package fingerprints include network policy changes, and fake runtime selection still fails closed on unsupported or weaker adapters.
- A local container smoke test is available but opt-in and skipped by default unless the local runtime CLI is installed, or an explicit CLI path is supplied, and the smoke is explicitly enabled.

Preserved behavior:

- Core does not depend on toolkit, concrete container runtimes, network libraries, browser tooling, or host package managers.
- Existing `NetworkIsolationPolicy` variants remain usable.
- Existing isolation lifecycle intent/result journaling and downgrade denial behavior remain authoritative.
- Existing fake isolation runtime tests remain deterministic and do not require real containers.
- Existing core isolation source files remain unchanged unless a later launch target explicitly owns a core contract extension.

Removed behavior:

- None.

Tests proving behavior:

- Toolkit environment profile tests for no-network defaults, allowlist lowering, invalid-target rejection, ordering/deduping, protocol/port defaults, snapshot sidecar creation, fingerprint sensitivity, and fake-runtime process start through `IsolationLifecycleCoordinator`.
- Toolkit opt-in local container smoke test that checks local CLI availability and basic command viability without becoming required CI proof.

## Workstreams

1. Toolkit egress target helpers
   - Add small typed DTOs/builders under `crates/agent-sdk-toolkit/src/environment/`.
   - Keep them data-only and serde-friendly.
   - Lower to existing core `NetworkIsolationPolicy` shapes instead of creating a second network-policy authority.
   - Validate invalid hosts, invalid ports, unsupported schemes, ordering, and deduping deterministically.

2. Toolkit environment profile
   - Add a narrow `environment` module with `AgentWorkspaceEnvironmentProfile` and result types.
   - Keep `src/lib.rs` as a facade and re-export only common public types.
   - The builder returns canonical core values, not a runtime-specific command plan.

3. Tests and fixtures
   - Use existing core public isolation contracts and fakes without editing core tests.
   - Add toolkit unit/integration tests using `FakeIsolationRuntime`.
   - Add opt-in local runtime smoke coverage that is safe to skip without hiding missing mock coverage.

4. Documentation and watchpoints
   - Update crate/toolkit docs only as needed for discoverability.
   - Keep product-neutral watchpoints in this plan and final handoff unless a later stitching-owned task explicitly moves them into shared reference docs.

## Validation Plan

Required local commands:

- `cargo fmt --check`
- `cargo test -p agent-sdk-toolkit`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`

Optional local smoke:

- Run the local container smoke only when the macOS container CLI is installed, or an explicit CLI path is supplied, and the explicit environment variable enables it. The smoke is not a substitute for deterministic fake-runtime tests.

Docs/public audit:

- No release or broad publication is being performed, so `scripts/public-release-audit.sh` is not required unless the final diff becomes a broad handoff. Run it if public docs gain new external operational instructions.

## Risks

- Domain allowlists can be misleading if a runtime only enforces IP rules. The API must distinguish requested policy from adapter-proven enforcement.
- A helper that silently falls back from container/VM to host process would violate the isolation contract. Tests must keep downgrade denial explicit.
- A toolkit profile can become a mini runtime if it starts owning process lifecycle. It must stop at canonical core lowering.
- Local smoke tests can become flaky or machine-specific. They must be opt-in and must not replace fake conformance coverage.
- Adding public core types affects SemVer/API surface. Keep names simple, derive common traits, document fallible constructors, and avoid broad dependencies.
- Toolkit public helpers still affect the crate API. Keep names simple, avoid runtime-specific concepts, and use typed errors instead of lossy strings where callers need to distinguish invalid input from setup failure.

## Risk/Gotcha Carry-Forward

- If future adapters translate domain allowlists, require a capability report field or test evidence that explains the actual enforcement mechanism.
- If a future runtime only supports IP/CIDR rules, expose that as a narrower capability instead of accepting hostnames as if they were enforced.
- If a future gateway service is added, keep it behind an optional crate or host adapter and route every process/network operation through `IsolationRuntime`, `EffectIntent`, `EffectResult`, and `RunJournal`.
- If remote execution is added, keep credentials, remote URLs, raw host paths, and session handles out of package fingerprints, journals, events, and fixtures unless represented as redacted refs under policy.
- If local container smoke is expanded, keep it opt-in and prove the same behavior with deterministic fakes first.
