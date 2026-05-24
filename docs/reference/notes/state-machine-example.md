# State Machine Example

This note captures a conceptual state machine flow for the agent runtime loop.

### Example: State machine flow (conceptual)

A typical sequence is: Starting, then ContextAssembly, then ProviderProjection, then ModelStreaming. If the model requests a tool, the loop moves to ToolPlanning, then Approval, then ToolExecution if approved, and then back to Continue and ContextAssembly. If something fails, it transitions to Recovery or Failed. This keeps tool use, approvals, and retries consistent and replayable.
