# Owner Role 04: Tools, Approval, And Tool Packs

## Owner Role

Tooling and policy agent.

## Writable Files

- `docs/contracts/tool-approval-contract.md`
- `docs/contracts/tool-pack-contract.md`

## Future Implementation Writable Scope

Once SDK code exists, this workstream may own tool/policy modules and tests only, for example:

- `crates/agent-sdk-core/src/tools/**`
- `crates/agent-sdk-core/src/policy/**`
- `crates/agent-sdk-toolkit/**`
- `crates/agent-sdk-core/tests/tool_*.rs`
- `crates/agent-sdk-core/tests/approval_*.rs`

## Read-Only Inputs

- `docs/contracts/runtime-package-schema.md`
- `docs/contracts/journal-replay-schema.md`
- `docs/contracts/isolation-runtime-contract.md`
- `docs/architecture/primitive-map.md`
- `docs/examples/tool-pack-isolation-anti-entropy.md`

## Contract To Deliver

Define tool registry/router/executor contracts, approval broker semantics, permission/sandbox/escalation layers, built-in read/search/edit/write/shell/resource/discovery tool packs, mutating side-effect records, and reversibility limits.

## Must Not Own

Coding-agent product workflow, UI approval copy, global tool installation, extension self-approval, or concrete shell/container execution.

## Integration Handoff

Send policy decision enum names, approval event names, tool effect journal records, and package fingerprint inputs to the stitching owner. Put proposal text in the handoff; do not edit shared reference or architecture files unless the stitching owner delegates it.

## Required Validation

- Policy matrix tests: permission, sandbox, approval, autonomy, escalation, dispatcher absence, timeout, deny, and exact finite-token response.
- Fail-closed tests: `headless_missing_dispatcher_denies`, `agent_sdk_core_cannot_send_out_of_band_approval`, `extension_cannot_answer_own_approval`.
- Tool-pack snapshot tests: tool specs and handlers share one `RuntimePackage` fingerprint; no live discovery mutates a running package.
- Side-effect tests: mutating tools append intent before execution and terminal result after execution; intent append failure prevents execution.
- Built-in pack tests: read/search anchors, edit precondition failure, write approval, shell sandbox denial, resource URI resolution, bounded output/truncation metadata.
- Process ownership tests: `shell_process_defaults_to_agent_owned_cleanup`, `start_script_detach_requires_explicit_policy_and_journal_record`, `manual_cancel_terminates_agent_owned_shell_process_by_default`, `implicit_orphan_shell_process_is_denied`.
- Hook/tool tests: `before_tool_hook_can_deny_before_executor_start`, `before_tool_hook_cannot_silently_detach_process`.
- Reversibility tests: inverse candidates are recorded as advisory metadata and never promised as automatic undo.
- Primitive-lowering review: tools and approvals must reuse `RuntimePackage` capabilities, `ToolExecutor`, `ApprovalBroker`, `PolicyRef`, `EffectIntent`, `EffectResult`, `RunJournal`, and `AgentEvent`; no ambient tool discovery or unjournaled side-effect path.
- Handoff evidence: policy table, tool-pack fixture list, side-effect journal fixtures, and denied/timeout approval fixtures.
