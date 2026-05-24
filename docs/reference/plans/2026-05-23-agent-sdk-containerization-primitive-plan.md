# Agent SDK Containerization Primitive Plan

> Historical plan only. It may contain stale product-specific paths or superseded workstream names. Do not use it as implementation authority; start from [../../start-here.md](../../start-here.md), [../../contracts/README.md](../../contracts/README.md), and [../../workstreams/README.md](../../workstreams/README.md).

## Objective

Explore Apple Containerization and integrate the relevant lessons into the Agent SDK Phase 1 documentation as a first-class execution-isolation design, without implementing runtime code or hard-coding Apple-specific APIs into the core SDK.

## Relevant Existing Context

- `/Users/clawdia/goals/agent_sdk_phase1.md` defines this phase as Markdown-only architecture work.
- `coding-standards.md` requires DDD, typed contracts, one source of truth, architecture-doc updates, and no heuristic policy decisions.
- `docs/architecture/coding-standards.md` already keeps UI shells, coding-agent harnesses, remote products, and cloud sandboxes outside the SDK core while making sandbox policy a stable SDK boundary.
- `docs/architecture/architecture-proposal.md` already includes `SandboxPolicy`, tool packs, shell tools, runtime packages, approval policy, journal/replay, and host-owned boundaries.
- `docs/reference/risks/agent-sdk-phase1-2026-05-21.md` already warns against built-in tools becoming a product harness and against overpromising reversibility.

## External Source Notes

Apple Containerization is a Swift package for running Linux containers on macOS through `Virtualization.framework` on Apple silicon. It provides APIs for OCI images, registries, ext4 root filesystems, lightweight VM lifecycle, Linux processes, Rosetta 2 for linux/amd64 containers, networking, mounts, and process I/O. It executes each Linux container inside its own lightweight VM, uses `vminitd` as guest init over vsock/gRPC, and currently requires macOS 26, Apple silicon, and Xcode 26 for the documented build path.

Key design inputs:

- one-VM-per-container isolation is a strong local safety primitive for agent tools and coding workloads;
- container/process lifecycle should be typed: prepare image/rootfs, create environment, start process, stream I/O, send signals, wait, stop/delete, collect stats;
- resource limits, capabilities, `no_new_privileges`, mounts, networking, DNS, hosts, Rosetta, kernels, and rootfs mutability are policy fields, not prompt text;
- single-file mounts can expose the parent directory inside the VM holding area, so file-mount semantics must be explicit and audited;
- Apple Containerization is a macOS adapter candidate, not a portable SDK core dependency.

## Behavior Contract

New behavior:

- Phase 1 docs describe `ExecutionEnvironment` / `IsolationRuntime` primitives for agent and tool workloads.
- Apple Containerization is documented as an adapter inspiration and potential macOS implementation path.
- Runtime packages can declare isolation requirements and host-resolved adapter capabilities.
- Tool packs such as shell/edit/write can request isolated execution instead of raw host execution.
- Observability includes isolation-environment lifecycle, process I/O, resource stats, mount/network policy, cleanup, and recovery.

Preserved behavior:

- SDK remains product-neutral and Rust-first conceptually.
- Apple Containerization stays optional and host-adapter-owned.
- Approval, permission, sandbox, and runtime package policies remain the source of truth.
- This remains documentation-only.

Removed behavior:

- None.

Validation:

- Source-inspect Apple Containerization README, package layout, core lifecycle/process APIs, mount docs, and example command.
- Run `git diff --check`.
- Run a content audit for isolation/containerization vocabulary in the owning docs.

## Workstreams

- Workstream A: summarize Apple Containerization lessons in external SDK lessons.
- Workstream B: add execution-isolation primitives to architecture proposal and primitive map.
- Workstream C: add isolation event/journal/telemetry guarantees.
- Workstream D: update generic examples, standards, README, and risk note.

## Risk/Gotcha Carry-Forward

- Do not make Apple Containerization a hard SDK dependency; model it as one adapter behind portable isolation traits.
- Do not imply containers are a complete security boundary. Hosts must still apply permission, mount, network, secret, approval, and cleanup policy.
- Do not let container lifecycle become an agent product or coding harness inside core.
- Mount semantics must be explicit. Single-file mount implementations may expose more host filesystem to the guest VM than the final container path suggests.
- macOS version, Apple silicon, kernel/image availability, service readiness, registry credentials, and first-run artifact fetches are adapter health checks, not core loop assumptions.

## Review Status

Plan drafted after local context scan and Apple Containerization source inspection, then reviewed locally against the behavior contract and standards before finalizing. A later explicit user request authorized independent Phase 1 handoff review; isolation/containerization follow-up contracts are now carried into `docs/reference/plans/2026-05-23-agent-sdk-phase2-implementation-handoff-plan.md`.

## Final Status

Completed the containerization primitive pass in the owning docs:

- `docs/start-here.md` now names execution isolation as a Phase 1 primitive and lists Apple Containerization as an external design input.
- `docs/architecture/architecture-proposal.md` now includes an `isolation/` module area, `RuntimePackage.isolation`, and an `Execution Isolation And Containerized Agents` section with conceptual Rust traits.
- `docs/architecture/primitive-map.md` now includes `ExecutionEnvironment`, `IsolationRuntime`, `ContainerRuntimeAdapter`, `ProcessSpec`, `FilesystemIsolationPolicy`, `NetworkIsolationPolicy`, and `IsolationCapabilityReport`.
- `docs/architecture/external-sdk-lessons.md` now captures Apple Containerization lessons and links to README, lifecycle/process source files, and mount docs.
- `docs/architecture/observability-and-lineage.md` now includes isolation events, journal records, telemetry guarantees, and recovery behavior.
- `docs/examples/tool-pack-isolation-anti-entropy.md` now includes a containerized tool execution example.
- `docs/architecture/coding-standards.md`, generic scenario docs, and the Phase 1 risk note now carry the isolation testing and watchpoint guidance.

Validation run:

- Source-inspected `/tmp/apple-containerization-inspect` after a shallow clone from `https://github.com/apple/containerization.git`.
- Reviewed `README.md`, `Package.swift`, `LinuxContainer.swift`, `LinuxProcess.swift`, `LinuxProcessConfiguration.swift`, `RunCommand.swift`, `docs/single-file-mounts.md`, and example docs.
- `git diff --check` passed.
- Content audit found the new execution-isolation vocabulary in architecture, primitive, observability, example, standards, plan, and risk docs.

Local review result:

- PASS. The docs keep Apple Containerization adapter-owned, preserve the Rust-first/product-neutral SDK boundary, require policy and observability for mounts/network/secrets/processes, and avoid promising that containers replace approval or guarantee reversibility.
