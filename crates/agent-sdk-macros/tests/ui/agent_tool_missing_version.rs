#![allow(unused_imports)]

use agent_sdk_macros::{ToolArgs, ToolOutput, agent_tool};
use agent_sdk_toolkit::ToolResult;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, ToolArgs)]
struct Args {
    query: String,
}

#[derive(Serialize, ToolOutput)]
struct Output {
    answer: String,
}

#[agent_tool(name = "lookup_docs")]
fn lookup_docs(_args: Args) -> ToolResult<Output> {
    Ok(Output {
        answer: "ok".to_string(),
    })
}

fn main() {}
