# Agent and AgentRuntime Example

This note shows how an `Agent` and `AgentRuntime` tie together in a typical flow.

## Example Flow

1. Create an `AgentRuntime` with providers, policies, session and journal, and telemetry.
2. Create an `Agent` that defines identity and defaults.
3. Call `agent.run(...)` for a simple request, or `agent.stream(...)` if the host wants incremental events.
4. The runtime starts a run and drives the loop:
   - assemble turn context
   - project to provider request
   - stream model output
   - plan and execute tools when required by policy
   - append journal events and continue until completion
5. The host receives the final result, and with streaming, also receives intermediate events through the `RunHandle`.

## Pseudocode Sketch

```text
runtime = AgentRuntime(...)
agent = Agent(...)

handle = agent.stream(input, runtime)
for event in handle.events:
  render(event)

result = handle.final_result
```

## Key Idea

The `Agent` defines what should happen; the `AgentRuntime` provides the environment that makes it happen, including providers, policies, and observability, and it drives execution through the loop and tooling.
