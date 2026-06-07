use clawdia_sdk::tools::{ToolArgs, ToolOutput, ToolResult, agent_tool};

#[derive(Clone, serde::Deserialize, serde::Serialize, ToolArgs)]
struct LookupArgs {
    query: String,
}

#[derive(Clone, serde::Serialize, ToolOutput)]
struct LookupOutput {
    answer: String,
}

#[agent_tool(name = "lookup_docs", version = "v1")]
fn lookup_docs(args: LookupArgs) -> ToolResult<LookupOutput> {
    Ok(LookupOutput {
        answer: format!("found docs for {}", args.query),
    })
}

fn main() -> Result<(), clawdia_sdk::core::AgentError> {
    let tool = lookup_docs_tool()?;
    let bundle = tool.pack_bundle(clawdia_sdk::core::SourceRef::with_kind(
        clawdia_sdk::core::SourceKind::Sdk,
        "source.example.typed_tool",
    ))?;
    let first_route = bundle.routes.first().expect("typed tool route");
    println!(
        "{}:{}",
        first_route.canonical_tool_name.as_str(),
        bundle.capabilities.len()
    );
    Ok(())
}
