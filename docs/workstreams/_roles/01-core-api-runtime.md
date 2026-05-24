# Owner Role 01: Core API And Runtime

## Owner Role

Core runtime agent.

## Writable Files

- `docs/contracts/api-contracts.md`
- `docs/contracts/run-handle-reconnect-contract.md`
- `docs/contracts/hook-lifecycle-contract.md`

## Future Implementation Writable Scope

Once SDK code exists, this workstream may own core run-control modules and tests only, for example:

- `crates/agent-sdk-core/src/agent/**`
- `crates/agent-sdk-core/src/runtime/**`
- `crates/agent-sdk-core/src/run/**`
- `crates/agent-sdk-core/src/hooks/**`
- `crates/agent-sdk-core/src/domain/ids.rs`
- `crates/agent-sdk-core/tests/api_*.rs`
- `crates/agent-sdk-core/tests/run_handle_*.rs`

## Read-Only Inputs

- `docs/contracts/event-schema.md`
- `docs/contracts/journal-replay-schema.md`
- `docs/contracts/runtime-package-schema.md`
- `docs/contracts/loop-state-machine.md`
- `docs/architecture/primitive-map.md`
- `docs/architecture/architecture-proposal.md`

## Contract To Deliver

Define the MVP public `Agent`, `AgentRuntime`, `RunRequest`, `RunHandle`, `RunResult`, builder ergonomics, public-surface tiers, lifecycle hook registration as reserved sidecars, run child lifecycle policy selection, reconnect semantics, wait/status/cancel idempotency, and host-owned runtime boundaries.

## Must Not Own

Provider transport, telemetry storage, host UI routing, trace-store policy, extension marketplace behavior, or concrete isolation runtimes.

## Integration Handoff

Send proposed public type names, error names, ID names, and result shapes to the stitching owner before changing shared indices. Put proposal text in the handoff; do not edit shared reference or architecture files unless the stitching owner delegates it.

## Required Validation

- Unit tests: `api_exports_match_contract`, `public_signature_support_types_are_exported`, `agent_runtime_can_start_basic_fake_run_without_host_imports`.
- Lowering tests: `run_text_lowers_to_run_request`, `run_typed_lowers_to_run_request_with_output_contract`, `request_builder_and_explicit_run_request_emit_equivalent_events`.
- Runtime tests: `run_handle_exposes_events_cancel_and_final_result`, `run_handle_wait_is_idempotent`, `run_handle_cancel_is_idempotent`, `wait_with_timeout_does_not_cancel_run`.
- Hook tests: `agent_on_hook_lowers_to_hook_spec_sidecar`, `config_hook_and_code_hook_share_runtime_package_shape`, `hook_ordering_is_deterministic_by_point_phase_order_and_id`, `hook_execution_mode_and_queue_are_fingerprinted`, `slow_hook_does_not_block_loop`.
- Child lifecycle tests: `manual_cancel_cascades_to_agent_owned_children_by_default`, `wait_with_timeout_does_not_cancel_children`, `run_completion_terminates_non_detached_agent_owned_process_by_default`, `run_request_can_select_but_not_loosen_child_lifecycle_policy`.
- Reconnect tests: `subscriber_drop_and_resubscribe_from_cursor_catches_up`, `cursor_scope_mismatch_is_rejected_without_widening_or_narrowing`, `filtered_cursor_fingerprint_mismatch_returns_cursor_scope_mismatch`.
- Contract audit: `RunHandle`, `RunResult`, and `AgentRuntime` agree with `event-schema.md`, `journal-replay-schema.md`, and `loop-state-machine.md`.
- Primitive-lowering review: API helpers must enter `RunRequest`/`AgentRuntime::start_run` and reuse `RuntimePackage`, `AgentEvent`, `RunJournal`, `ContentRef`, `EffectIntent` where applicable, policy refs, and typed IDs; no alternate runtime path.
- Handoff evidence: commands run, fake runtime fixture names, public API diff, and any shared ID/name proposals.
