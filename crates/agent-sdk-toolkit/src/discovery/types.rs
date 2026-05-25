use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolDiscoveryRequest {
    pub query: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolDiscoveryOutput {
    pub query: String,
    pub candidates: Vec<ToolDiscoveryCandidate>,
    pub package_delta_required: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolDiscoveryCandidate {
    pub pack_id: String,
    pub tool_names: Vec<String>,
    pub package_delta_required: bool,
}
