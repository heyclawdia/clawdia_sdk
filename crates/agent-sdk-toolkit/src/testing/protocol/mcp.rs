use std::collections::{BTreeMap, BTreeSet};

use agent_sdk_core::{AgentError, AgentErrorKind, RetryClassification};
use serde_json::{Value, json};

use crate::protocol::{
    JsonRpcFrame, JsonRpcId, JsonRpcLineEndpoint, JsonRpcRequest, JsonRpcResponse, expect_response,
    protocol_violation,
};

enum ReceiveNext {
    Empty,
    Handled,
    Frame(JsonRpcFrame),
}

#[derive(Clone, Debug, Default)]
pub struct ScriptedMcpServer {
    tools: BTreeMap<String, Value>,
    resources: BTreeMap<String, String>,
    prompts: BTreeMap<String, Value>,
    initialized: bool,
    next_client_request: i64,
}

impl ScriptedMcpServer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tool(mut self, name: impl Into<String>, result: Value) -> Self {
        self.tools.insert(name.into(), result);
        self
    }

    pub fn resource(mut self, uri: impl Into<String>, text: impl Into<String>) -> Self {
        self.resources.insert(uri.into(), text.into());
        self
    }

    pub fn prompt(mut self, name: impl Into<String>, spec: Value) -> Self {
        self.prompts.insert(name.into(), spec);
        self
    }

    pub fn handle_next(&mut self, endpoint: &JsonRpcLineEndpoint) -> Result<bool, AgentError> {
        let frame = match receive_frame_or_parse_error(endpoint)? {
            ReceiveNext::Empty => return Ok(false),
            ReceiveNext::Handled => return Ok(true),
            ReceiveNext::Frame(frame) => frame,
        };
        let request = match frame {
            JsonRpcFrame::Request(request) => request,
            JsonRpcFrame::Notification(notification) => {
                if notification.method == "notifications/initialized" {
                    self.initialized = true;
                }
                return Ok(true);
            }
            JsonRpcFrame::Response(_) => {
                endpoint.send_error(None, -32600, "MCP server expected a request frame")?;
                return Ok(true);
            }
        };
        match request.method.as_str() {
            "initialize" => {
                endpoint.send_result(
                    request.id,
                    json!({
                        "protocolVersion": request.params
                            .get("protocolVersion")
                            .cloned()
                            .unwrap_or_else(|| json!("2025-11-25")),
                        "capabilities": {
                            "tools": {"listChanged": false},
                            "resources": {"subscribe": false, "listChanged": false},
                            "prompts": {"listChanged": false}
                        },
                        "serverInfo": {
                            "name": "agent-sdk-toolkit-mcp-fake",
                            "version": "0.0.0"
                        }
                    }),
                )?;
            }
            "tools/list" => {
                if !self.ensure_initialized(endpoint, &request)? {
                    return Ok(true);
                }
                endpoint.send_result(request.id, json!({"tools": self.tool_list()}))?;
            }
            "tools/call" => {
                if !self.ensure_initialized(endpoint, &request)? {
                    return Ok(true);
                }
                self.handle_tool_call(endpoint, request)?;
            }
            "resources/list" => {
                if !self.ensure_initialized(endpoint, &request)? {
                    return Ok(true);
                }
                endpoint.send_result(request.id, json!({"resources": self.resource_list()}))?;
            }
            "resources/read" => {
                if !self.ensure_initialized(endpoint, &request)? {
                    return Ok(true);
                }
                self.handle_resource_read(endpoint, request)?;
            }
            "prompts/list" => {
                if !self.ensure_initialized(endpoint, &request)? {
                    return Ok(true);
                }
                endpoint.send_result(request.id, json!({"prompts": self.prompt_list()}))?;
            }
            "logging/setLevel" => {
                if !self.ensure_initialized(endpoint, &request)? {
                    return Ok(true);
                }
                endpoint.send_error(
                    Some(request.id),
                    -32010,
                    "MCP logging control denied by default policy",
                )?;
            }
            _ => {
                endpoint.send_error(Some(request.id), -32601, "MCP method not found")?;
            }
        };
        Ok(true)
    }

    pub fn request_sampling(
        &mut self,
        endpoint: &JsonRpcLineEndpoint,
    ) -> Result<JsonRpcId, AgentError> {
        let id = self.next_client_request_id();
        endpoint
            .send_request(
                id.clone(),
                "sampling/createMessage",
                json!({"messages": [{"role": "user", "content": {"type": "text", "text": "sample"}}]}),
            )
            .map(|_| id)
    }

    pub fn request_elicitation(
        &mut self,
        endpoint: &JsonRpcLineEndpoint,
    ) -> Result<JsonRpcId, AgentError> {
        let id = self.next_client_request_id();
        endpoint
            .send_request(
                id.clone(),
                "elicitation/create",
                json!({"message": "need user input"}),
            )
            .map(|_| id)
    }

    pub fn response(&self, endpoint: &JsonRpcLineEndpoint) -> Result<JsonRpcResponse, AgentError> {
        expect_response(endpoint.receive_frame()?)
    }

    pub fn initialized(&self) -> bool {
        self.initialized
    }

    fn ensure_initialized(
        &self,
        endpoint: &JsonRpcLineEndpoint,
        request: &JsonRpcRequest,
    ) -> Result<bool, AgentError> {
        if self.initialized {
            return Ok(true);
        }
        endpoint.send_error(
            Some(request.id.clone()),
            -32002,
            "MCP client must send notifications/initialized before normal operation",
        )?;
        Ok(false)
    }

    fn next_client_request_id(&mut self) -> JsonRpcId {
        let id = JsonRpcId::Number(self.next_client_request);
        self.next_client_request += 1;
        id
    }

    fn tool_list(&self) -> Vec<Value> {
        self.tools
            .keys()
            .map(|name| json!({"name": name, "inputSchema": {"type": "object"}}))
            .collect()
    }

    fn resource_list(&self) -> Vec<Value> {
        self.resources
            .keys()
            .map(|uri| json!({"uri": uri, "mimeType": "text/plain"}))
            .collect()
    }

    fn prompt_list(&self) -> Vec<Value> {
        self.prompts
            .iter()
            .map(|(name, spec)| {
                let mut spec = spec.clone();
                if let Some(object) = spec.as_object_mut() {
                    object.insert("name".to_string(), json!(name));
                    return spec;
                }
                json!({"name": name})
            })
            .collect()
    }

    fn handle_tool_call(
        &self,
        endpoint: &JsonRpcLineEndpoint,
        request: JsonRpcRequest,
    ) -> Result<(), AgentError> {
        let name = request
            .params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| protocol_violation("MCP tools/call requires name"))?;
        let Some(result) = self.tools.get(name) else {
            endpoint.send_error(Some(request.id), -32602, "MCP tool is not available")?;
            return Ok(());
        };
        endpoint.send_result(
            request.id,
            json!({
                "content": [{"type": "text", "text": result.to_string()}],
                "isError": false
            }),
        )?;
        Ok(())
    }

    fn handle_resource_read(
        &self,
        endpoint: &JsonRpcLineEndpoint,
        request: JsonRpcRequest,
    ) -> Result<(), AgentError> {
        let uri = request
            .params
            .get("uri")
            .and_then(Value::as_str)
            .ok_or_else(|| protocol_violation("MCP resources/read requires uri"))?;
        let Some(text) = self.resources.get(uri) else {
            endpoint.send_error(Some(request.id), -32602, "MCP resource is not available")?;
            return Ok(());
        };
        endpoint.send_result(
            request.id,
            json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "text/plain",
                    "text": text
                }]
            }),
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct McpHostProxy {
    endpoint: JsonRpcLineEndpoint,
    allowed_tools: BTreeSet<String>,
    allowed_resources: BTreeSet<String>,
    allowed_prompts: BTreeSet<String>,
    pending_methods: BTreeMap<String, String>,
    next_id: i64,
}

impl McpHostProxy {
    pub fn new(endpoint: JsonRpcLineEndpoint) -> Self {
        Self {
            endpoint,
            allowed_tools: BTreeSet::new(),
            allowed_resources: BTreeSet::new(),
            allowed_prompts: BTreeSet::new(),
            pending_methods: BTreeMap::new(),
            next_id: 1,
        }
    }

    pub fn allow_tool(mut self, name: impl Into<String>) -> Self {
        self.allowed_tools.insert(name.into());
        self
    }

    pub fn allow_resource(mut self, uri: impl Into<String>) -> Self {
        self.allowed_resources.insert(uri.into());
        self
    }

    pub fn allow_prompt(mut self, name: impl Into<String>) -> Self {
        self.allowed_prompts.insert(name.into());
        self
    }

    pub fn endpoint(&self) -> &JsonRpcLineEndpoint {
        &self.endpoint
    }

    pub fn initialize(&mut self) -> Result<JsonRpcId, AgentError> {
        self.request(
            "initialize",
            json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {
                    "name": "agent-sdk-toolkit",
                    "version": "0.0.0"
                }
            }),
        )
    }

    pub fn initialized(&self) -> Result<(), AgentError> {
        self.endpoint
            .send_notification("notifications/initialized", json!({}))
            .map(|_| ())
    }

    pub fn list_tools(&mut self) -> Result<JsonRpcId, AgentError> {
        self.request("tools/list", json!({}))
    }

    pub fn list_resources(&mut self) -> Result<JsonRpcId, AgentError> {
        self.request("resources/list", json!({}))
    }

    pub fn list_prompts(&mut self) -> Result<JsonRpcId, AgentError> {
        self.request("prompts/list", json!({}))
    }

    pub fn call_tool(&mut self, name: &str, arguments: Value) -> Result<JsonRpcId, AgentError> {
        if !self.allowed_tools.contains(name) {
            return Err(policy_denial(format!(
                "MCP tool {name} is not selected in host proxy policy"
            )));
        }
        self.request("tools/call", json!({"name": name, "arguments": arguments}))
    }

    pub fn read_resource(&mut self, uri: &str) -> Result<JsonRpcId, AgentError> {
        if !self.allowed_resources.contains(uri) {
            return Err(policy_denial(format!(
                "MCP resource {uri} is not selected in host proxy policy"
            )));
        }
        self.request("resources/read", json!({"uri": uri}))
    }

    pub fn handle_next(&self, endpoint: &JsonRpcLineEndpoint) -> Result<bool, AgentError> {
        let frame = match receive_frame_or_parse_error(endpoint)? {
            ReceiveNext::Empty => return Ok(false),
            ReceiveNext::Handled => return Ok(true),
            ReceiveNext::Frame(frame) => frame,
        };
        let JsonRpcFrame::Request(request) = frame else {
            return Ok(true);
        };
        match request.method.as_str() {
            "sampling/createMessage" | "elicitation/create" => endpoint.send_error(
                Some(request.id),
                -32010,
                "MCP server-to-client request denied by host proxy policy",
            )?,
            _ => endpoint.send_error(Some(request.id), -32601, "MCP client method not found")?,
        };
        Ok(true)
    }

    pub fn response(&mut self) -> Result<JsonRpcResponse, AgentError> {
        let mut response = expect_response(self.endpoint.receive_frame()?)?;
        if response.id == JsonRpcId::Null {
            return Ok(response);
        }
        let Some(method) = self.pending_methods.remove(&response.id.as_key()) else {
            return Err(protocol_violation("unexpected MCP response id"));
        };
        self.apply_response_policy(&method, &mut response);
        Ok(response)
    }

    pub fn allowed_tool_names_from_response(
        &self,
        response: &JsonRpcResponse,
    ) -> Result<Vec<String>, AgentError> {
        let tools = response
            .result
            .as_ref()
            .and_then(|value| value.get("tools"))
            .and_then(Value::as_array)
            .ok_or_else(|| protocol_violation("MCP tools/list response missing tools array"))?;
        let mut names = tools
            .iter()
            .filter_map(|tool| tool.get("name").and_then(Value::as_str))
            .filter(|name| self.allowed_tools.contains(*name))
            .map(str::to_string)
            .collect::<Vec<_>>();
        names.sort();
        Ok(names)
    }

    fn request(&mut self, method: &str, params: Value) -> Result<JsonRpcId, AgentError> {
        let id = JsonRpcId::Number(self.next_id);
        self.next_id += 1;
        self.endpoint.send_request(id.clone(), method, params)?;
        self.pending_methods.insert(id.as_key(), method.to_string());
        Ok(id)
    }

    fn apply_response_policy(&self, method: &str, response: &mut JsonRpcResponse) {
        if response.error.is_some() {
            return;
        }
        let Some(result) = response.result.as_mut() else {
            return;
        };
        match method {
            "tools/list" => filter_named_array(result, "tools", "name", &self.allowed_tools),
            "resources/list" => {
                filter_named_array(result, "resources", "uri", &self.allowed_resources)
            }
            "prompts/list" => filter_named_array(result, "prompts", "name", &self.allowed_prompts),
            _ => {}
        }
    }
}

fn receive_frame_or_parse_error(endpoint: &JsonRpcLineEndpoint) -> Result<ReceiveNext, AgentError> {
    let Some(line) = endpoint.try_receive_raw_line()? else {
        return Ok(ReceiveNext::Empty);
    };
    match JsonRpcFrame::from_line(&line) {
        Ok(frame) => Ok(ReceiveNext::Frame(frame)),
        Err(error) => {
            endpoint.send_error(None, -32700, error.context().message)?;
            Ok(ReceiveNext::Handled)
        }
    }
}

fn filter_named_array(result: &mut Value, field: &str, key: &str, allowed: &BTreeSet<String>) {
    let Some(items) = result.get_mut(field).and_then(Value::as_array_mut) else {
        return;
    };
    items.retain(|item| {
        item.get(key)
            .and_then(Value::as_str)
            .is_some_and(|name| allowed.contains(name))
    });
}

fn policy_denial(message: impl Into<String>) -> AgentError {
    AgentError::new(
        AgentErrorKind::PolicyDenial,
        RetryClassification::UserActionNeeded,
        message,
    )
}
