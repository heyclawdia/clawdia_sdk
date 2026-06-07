use agent_sdk_macros::{ToolArgs, ToolOutput, agent_tool};
use agent_sdk_toolkit::{ToolArgs as _, ToolResult};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, ToolArgs)]
struct LookupArgs {
    query: String,
    limit: u32,
}

#[derive(Serialize, ToolOutput)]
struct LookupOutput {
    answer: String,
}

#[agent_tool(name = "lookup_docs", version = "v1")]
fn lookup_docs(args: LookupArgs) -> ToolResult<LookupOutput> {
    Ok(LookupOutput {
        answer: format!("{}:{}", args.query, args.limit),
    })
}

#[test]
fn derive_tool_args_generates_deterministic_schema() {
    let schema = LookupArgs::schema();

    assert_eq!(LookupArgs::SCHEMA_ID, "schema.lookup_args");
    assert_eq!(schema["properties"]["query"]["type"], "string");
    assert_eq!(schema["properties"]["limit"]["type"], "number");
}

#[test]
fn agent_tool_generates_tool_builder_function() {
    let tool = lookup_docs_tool().expect("macro helper builds");

    assert_eq!(
        tool.tool().unwrap().canonical_tool_name().as_str(),
        "lookup_docs"
    );
}
