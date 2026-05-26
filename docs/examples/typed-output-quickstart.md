# Typed-Output Quickstart

Typed output should feel ergonomic, but it must still lower into the canonical
runtime path:

`agent.run_typed::<T>` -> `RunRequest` -> `OutputContract` -> provider request
hint -> local validation -> `ValidatedOutput` -> journaled events.

```rust
use agent_sdk_core::{
    Agent, AgentId, OutputContract, OutputSchemaId, OutputSchemaRef, RunId,
    RunRequest, SchemaVersion, SourceKind, SourceRef, TypedOutputModel,
};
use serde_json::json;

#[derive(Clone, Debug)]
struct Todo;

impl TypedOutputModel for Todo {
    const SCHEMA_ID: &'static str = "schema.quickstart.todo";
    const SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);

    fn schema_ref() -> OutputSchemaRef {
        OutputContract::inline_json_schema(
            OutputSchemaId::new(Self::SCHEMA_ID),
            Self::SCHEMA_VERSION,
            json!({
                "type": "object",
                "required": ["title", "priority"],
                "properties": {
                    "title": { "type": "string" },
                    "priority": { "enum": ["low", "medium", "high"] }
                }
            }),
        )
        .schema
    }
}

let agent = Agent::builder()
    .id(AgentId::new("agent.quickstart.typed"))
    .name("typed quickstart")
    .build()?;

let source = SourceRef::with_kind(SourceKind::Host, "source.quickstart.typed");

let helper_request = agent.typed_text_request::<Todo>(
    RunId::new("run.quickstart.typed"),
    source.clone(),
    "Extract the todo.",
);

let canonical_request = RunRequest::text(
    RunId::new("run.quickstart.typed"),
    agent.id().clone(),
    source,
    "Extract the todo.",
)
.with_output_contract(OutputContract::for_type::<Todo>());

assert_eq!(helper_request, canonical_request);
```

## What This Proves

- The helper is a convenience layer, not a second behavior path.
- Provider-native schema hints can help the model, but SDK-owned local
  validation remains authoritative.
- Validation failures, repair attempts, validated output, policy refs,
  redaction, events, and journal records stay on the same P1 path as an explicit
  `RunRequest`.

## Runtime Shape

Use the same live-provider runtime setup from
[live-provider-quickstart.md](live-provider-quickstart.md), but call:

```rust
let result = agent.run_typed::<Todo>(
    &runtime,
    RunId::new("run.quickstart.typed"),
    SourceRef::with_kind(SourceKind::Host, "source.quickstart.typed"),
    "Extract the todo.",
)?;
```

The result should not be published to a host surface until `ValidatedOutput` is
durable and the output-delivery policy allows publication.

When the output contract carries an inline redacted schema, live providers can
receive a native structured-output hint:

- OpenAI Responses receives `text.format`.
- Anthropic Messages receives `output_config.format`.
- Gemini generateContent receives `generationConfig.responseJsonSchema`.

Those hints improve model behavior; SDK-owned local validation is still the
authority before any typed result is published.
