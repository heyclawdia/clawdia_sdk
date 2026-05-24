# Goal 05b: Isolation Execution

## Phase

[Phase 05: Feature Layers](README.md)

## Owner Role

[Isolation Execution](../_roles/06-isolation-execution.md)

## Parallelism

Parallel-safe with every other goal in Phase 05 after Phase 04 exits. Do not start Phase 06 until all Phase 05 goals finish.

## Required Reading

- `README.md`
- `docs/start-here.md`
- `coding_standards.md`
- `docs/workstreams/validation-gates.md`
- `docs/reference/sdk-review-checklist.md`
- `docs/architecture/primitive-map.md`
- phase README
- owner role doc
- owner role doc read-only inputs
- read-only inputs below

## Writable Files

- `docs/contracts/isolation-runtime-contract.md`

## Read-Only Inputs

- `docs/contracts/tool-pack-contract.md`
- `docs/contracts/tool-approval-contract.md`
- `docs/contracts/runtime-package-schema.md`
- `docs/architecture/architecture-proposal.md`
- `docs/architecture/primitive-map.md`
- `docs/examples/tool-pack-isolation-anti-entropy.md`

## Primitive Focus

- Isolation is `ExecutionEnvironment` plus `IsolationRuntime` adapter port, package requirements, policy checks, events, and journal records.
- SDK-owned isolation classes are coarse containment enums; concrete adapter refs such as `mlx.macos-sandbox` are host-provided registry identifiers. Adapter selection must also compare capability/trust vectors for locality, tenancy, mount/network/secret enforcement, cleanup, and auditability.
- Concrete containers, VMs, and remote sandboxes are optional adapters.

## Must Not Own

Approval UI, tool semantics, provider routing, or concrete runtime implementation in core.

## Validation And Review

- Future tests/fixtures: fake adapter capability tests, downgrade matrix tests, mount/network/process lifecycle fixtures, cleanup/recovery fixtures, and skipped concrete-runtime smoke notes.
- Docs audit: isolation must stay `ExecutionEnvironment` plus `IsolationRuntime` adapter port; concrete runtimes remain adapter-owned.
- Unsupported adapter and class/capability/trust-vector downgrades fail closed unless explicitly policy-approved.
- Mount/network/process lifecycle is journaled.
- Cleanup and detached process behavior matches child lifecycle policy.

## Validation Evidence

- Worker agent: Hooke (`019e5882-03af-74c0-8498-3c533011f99d`).
- Changed file: `docs/contracts/isolation-runtime-contract.md`.
- Scoped docs audit confirmed isolation is represented as `ExecutionEnvironment`, `IsolationRequirementSnapshot`, `IsolationRuntime`, `RuntimePackage` sidecars, policy refs, journal records, events, typed refs, and shared effect fields.
- Named future capability, downgrade, lifecycle, process I/O, cleanup/recovery, child-lifecycle, redaction, event, and OTel projection fixtures without creating executable fixtures in this documentation-only phase.
- Cross-cutting proposals sent to stitching: accept granular Phase 05 isolation/child-lifecycle event names, keep concrete runtimes adapter-owned, split child-artifact lifecycle ownership from subagent child-run wrapping, and defer dedicated isolation `EffectKind` expansion until implementation needs it.
- No Rust source, package manifests, executable tests, or fixtures were created.

## Review Packet

- Primitive decision: isolation remains a feature layer over runtime-package sidecars, adapter ports, policy, journals, events, child lifecycle, and shared effect fields.
- SDK-owned boundaries preserved: portable class/capability/trust matching, downgrade semantics, event/journal names, redaction defaults, cleanup/reclaim records, and replay/recovery rules.
- Host-owned boundaries preserved: concrete container/VM/sandbox implementation, runtime installation, image stores, mount/network plumbing, credentials, process inspectors, and adapter-native cleanup/reclaim.
- Reviewer checklist: PASS for simplicity, product-neutrality, event/journal durability, privacy/redaction, replay/idempotency, fail-closed downgrade semantics, and capability fingerprint impact after stitching accepted the shared event names and effect-kind deferral.
