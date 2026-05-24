# Phase 05: Agent Pool Coordination

Implement the generic agent-run coordination feature layer after P0 run control
works end to end. This phase creates the `AgentPool`, `RunMessage`, and
`WakeCondition` implementation seam that later subagents and optional workflow
crates build on.

## Launch Targets

| Target | Parallel-safe? | Purpose |
| --- | --- | --- |
| [Agent Pool Coordination](05a-agent-pool-coordination.md) | yes | Add pool-scoped run membership, generic run messages, delivery receipts, wake conditions, and fixtures without adding workflow/DAG/barrier logic. |

## Exit Gate

- [x] `AgentPool`, `RunAddress`, `RunMessage`, `MessageReceipt`, and
  `WakeCondition` lower into existing `AgentRuntime`, `AgentEventBus`,
  `RunJournal`, `RuntimePackage`, `PolicyRef`, `ContentRef`, `SourceRef`,
  `DestinationRef`, and `EntityRef` primitives.
- [x] Agent-pool-specific IDs are added through the shared typed-ID pattern with
  serde fixtures, without reopening Phase 01 work.
- [x] Run-message delivery and wake-condition records have golden event and
  journal fixtures.
- [x] `RunAddress` is only an ergonomic wrapper over existing refs; it does not
  become a parallel identity system.
- [x] Pool timeouts wake waiting runs without cancelling target runs.
- [x] The phase review packet proves `AgentPool` is not a workflow engine and
  does not own DAGs, barriers, schedules, compensation, dashboards, or durable
  trigger engines.
- [x] Later phases are updated to treat subagents as helpers over this pool
  primitive.
