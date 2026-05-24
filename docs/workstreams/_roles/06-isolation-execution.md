# Owner Role 06: Isolation And Execution Environments

## Owner Role

Isolation/runtime adapter agent.

## Writable Files

- `docs/contracts/isolation-runtime-contract.md`

## Future Implementation Writable Scope

Once SDK code exists, this workstream may own isolation contracts/adapters and tests only, for example:

- `crates/agent-sdk-core/src/isolation/**`
- `crates/agent-sdk-isolation/**`
- `crates/agent-sdk-core/tests/isolation_*.rs`

## Read-Only Inputs

- `docs/contracts/tool-pack-contract.md`
- `docs/contracts/tool-approval-contract.md`
- `docs/contracts/runtime-package-schema.md`
- `docs/architecture/architecture-proposal.md`
- `docs/architecture/primitive-map.md`
- `docs/examples/tool-pack-isolation-anti-entropy.md`

## Contract To Deliver

Define `ExecutionEnvironment`, `IsolationRuntime`, adapter capability reports, image/rootfs lifecycle, mounts, network, secrets, process IO, stats, cleanup, fallback/downgrade rules, and Apple Containerization/Docker/Firecracker/remote adapter boundaries.

## Must Not Own

Concrete VM/container implementation, platform-specific runtime hard dependency, approval decisions, or product-safe-mode claims.

## Integration Handoff

Send capability field names, lifecycle event names, environment fingerprint inputs, and downgrade/fallback policy names to the stitching owner. Put proposal text in the handoff; do not edit shared reference or architecture files unless the stitching owner delegates it.

## Required Validation

- Fake adapter tests: capability report, unsupported host, missing image/rootfs support, architecture mismatch, health failure.
- Downgrade tests: container/VM request denies host-process fallback unless runtime package and policy explicitly allow downgrade.
- Mount/network/secret tests: mount expansion audit, path bounds, network allowlist/deny, secret redaction, environment variable policy.
- Process lifecycle tests: prepare, start, stdout/stderr redaction, stats, timeout, signal, exit, cleanup, and cancellation.
- Process ownership tests: `manual_cancel_sends_signal_to_agent_owned_isolated_process`, `explicit_detach_survives_parent_run_completion`, `implicit_orphan_process_is_denied_by_default`, `detached_process_has_host_ack_and_reclaim_policy`, `process_signal_intent_is_journaled_before_adapter_call`.
- Hook/isolation tests: `before_isolation_process_hook_cannot_silently_downgrade_environment`.
- Recovery tests: partial preparation, started process with missing cleanup, failed cleanup, and resume from journaled isolation state.
- Event/journal audit: every isolation lifecycle phase has event and journal record with environment ID, adapter, policy hashes, and cleanup status.
- Primitive-lowering review: isolation must reuse `ExecutionEnvironment`, `IsolationRuntime`, `PolicyRef`, `RunJournal`, and `AgentEvent`; no concrete runtime behavior enters `agent-sdk-core`.
- Handoff evidence: capability matrix, downgrade matrix, process fixture, cleanup/recovery fixtures, and skipped concrete-runtime tests.
