# Agent and Runtime Diagram

This diagram shows the relationship between an `Agent`, an `AgentRuntime`, and streaming events.

```mermaid
flowchart LR
  Host[Host Application] -->|input| Agent
  Agent -->|agent.stream(input, runtime)| Runtime[AgentRuntime]
  Runtime -->|assemble turn context
project request
execute tools
append journal| Loop[Run Loop]
  Loop --> Runtime
  Runtime -->|events| RunHandle
  RunHandle -->|intermediate events| Host
  RunHandle -->|final result| Host

  subgraph RuntimeDetails[Runtime responsibilities]
    Session[Session / Memory / Journal]
    Providers[Providers]
    Policies[Policies]
    Telemetry[Telemetry]
  end

  Runtime --- RuntimeDetails
```

Key idea: the `Agent` defines behavior and defaults, while the `AgentRuntime` provides the environment, drives execution, and manages memory-related state like session and journal.
