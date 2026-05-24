# Owner Role 07: Subagents And Coordination

## Owner Role

Multi-agent coordination agent.

## Writable Files

- `docs/contracts/subagent-contract.md`

## Future Implementation Writable Scope

Once SDK code exists, this workstream may own subagent modules and tests only, for example:

- `crates/agent-sdk-core/src/subagents/**`
- `crates/agent-sdk-core/tests/subagent_*.rs`

## Read-Only Inputs

- `docs/contracts/api-contracts.md`
- `docs/contracts/runtime-package-schema.md`
- `docs/contracts/event-schema.md`
- `docs/contracts/journal-replay-schema.md`
- `docs/architecture/primitive-map.md`
- `docs/architecture/architecture-proposal.md`
- `docs/examples/subagent-supervision-workflow.md`

## Contract To Deliver

Define parent-owned child runs, topology/depth limits, child runtime package stripping, explicit `ContextHandoffPolicy`, parent mailbox, clarification flow, cancellation propagation, event wrapping, usage rollup, and no user-chat promotion by default.

## Must Not Own

Direct user chat ownership for child agents, recursive subagent tools by default, product-specific subagent UI, or ad hoc provider narrative promotion.

## Integration Handoff

Send child ID names, topology event names, child package fingerprint inputs, and clarification message shapes to the stitching owner. Put proposal text in the handoff; do not edit shared reference or architecture files unless the stitching owner delegates it.

## Required Validation

- Topology tests: bounded depth, max child count, cycle prevention, parent-owned child runs, and no direct user-chat promotion by default.
- Package tests: child runtime package strips recursive subagent tools by default and preserves allowed tool/handoff policy.
- Mailbox tests: parent-to-child messages, child-to-parent clarification, parent approval of context handoff, and no provider narrative promotion.
- Cancellation tests: parent cancellation interrupts children and seals child/parent terminal states consistently.
- Lifecycle tests: `parent_manual_cancel_cascades_to_child_processes`, `child_run_cannot_outlive_parent_without_detach_policy`, `detached_child_run_records_parent_detach_intent`, `before_subagent_start_hook_can_deny_or_narrow_child_request`.
- Event/journal tests: child events wrap parent run ID, child run ID, child agent ID, route, policy, and usage rollup.
- Cost tests: child usage rolls up once and duplicate subscribers do not duplicate child runs or usage.
- Primitive-lowering review: subagents must be parent-owned child runs using `RunRequest`, stripped `RuntimePackage`, explicit `ContextHandoffPolicy`, `AgentEvent`, `RunJournal`, and usage records; no direct user-chat promotion or separate child runtime ledger.
- Handoff evidence: topology matrix, package-diff fixture, mailbox fixture, event wrapping fixture, and usage rollup fixture.
