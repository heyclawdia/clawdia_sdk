//! Scripted MCP protocol harnesses for SDK consumers. Use these fakes to test
//! tool/resource/prompt lifecycle and host filtering without live MCP servers.
//! Harness methods mutate in-memory endpoints and scripted response state only.
//!
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
/// In-memory scripted mcp server fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct ScriptedMcpServer {
    tools: BTreeMap<String, Value>,
    resources: BTreeMap<String, String>,
    prompts: BTreeMap<String, Value>,
    initialized: bool,
    next_client_request: i64,
}

impl ScriptedMcpServer {
    /// Creates a new testing::protocol::mcp value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns an updated testing::protocol::mcp value with tool applied.
    /// This is data construction only and does not execute the configured
    /// behavior.
    pub fn tool(mut self, name: impl Into<String>, result: Value) -> Self {
        self.tools.insert(name.into(), result);
        self
    }

    /// Returns an updated value with resource configured.
    /// This configures or reads the scripted protocol mock in memory.
    pub fn resource(mut self, uri: impl Into<String>, text: impl Into<String>) -> Self {
        self.resources.insert(uri.into(), text.into());
        self
    }

    /// Adds a scripted prompt definition and returns the updated fake server.
    /// This mutates only the builder's in-memory prompt map; request frames are appended later
    /// when a client asks the endpoint for prompts.
    pub fn prompt(mut self, name: impl Into<String>, spec: Value) -> Self {
        self.prompts.insert(name.into(), spec);
        self
    }

    /// Handle next.
    /// This consumes one queued JSON-RPC frame from the in-memory endpoint and mutates only
    /// scripted mock state.
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

    /// Request sampling.
    /// This appends the corresponding JSON-RPC frame to the in-memory test endpoint transcript.
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

    /// Request elicitation.
    /// This appends the corresponding JSON-RPC frame to the in-memory test endpoint transcript.
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

    /// Returns the response currently held by this value.
    /// This reads scripted protocol state or a queued response without contacting an external
    /// process.
    pub fn response(&self, endpoint: &JsonRpcLineEndpoint) -> Result<JsonRpcResponse, AgentError> {
        expect_response(endpoint.receive_frame()?)
    }

    /// Returns the initialized currently held by this value.
    /// This reads scripted protocol state or a queued response without contacting an external
    /// process.
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
/// In-memory mcp host proxy fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct McpHostProxy {
    endpoint: JsonRpcLineEndpoint,
    allowed_tools: BTreeSet<String>,
    allowed_resources: BTreeSet<String>,
    allowed_prompts: BTreeSet<String>,
    pending_methods: BTreeMap<String, String>,
    next_id: i64,
}

impl McpHostProxy {
    /// Creates a new testing::protocol::mcp value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
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

    /// Returns an updated value with allow tool configured.
    /// This configures or reads the scripted protocol mock in memory.
    pub fn allow_tool(mut self, name: impl Into<String>) -> Self {
        self.allowed_tools.insert(name.into());
        self
    }

    /// Returns an updated value with allow resource configured.
    /// This updates the scripted MCP mock allowlist in memory for later protocol calls.
    pub fn allow_resource(mut self, uri: impl Into<String>) -> Self {
        self.allowed_resources.insert(uri.into());
        self
    }

    /// Returns an updated value with allow prompt configured.
    /// This updates the scripted MCP mock allowlist in memory for later protocol calls.
    pub fn allow_prompt(mut self, name: impl Into<String>) -> Self {
        self.allowed_prompts.insert(name.into());
        self
    }

    /// Returns the endpoint currently held by this value.
    /// This reads scripted protocol state or a queued response without contacting an external
    /// process.
    pub fn endpoint(&self) -> &JsonRpcLineEndpoint {
        &self.endpoint
    }

    /// Initialize.
    /// This appends the corresponding JSON-RPC frame to the scripted MCP mock transcript and
    /// returns the request id when applicable.
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

    /// Sends the MCP `notifications/initialized` frame to the scripted server endpoint.
    /// This appends one in-memory JSON-RPC notification; it does not contact a live MCP process or
    /// mutate SDK runtime state.
    pub fn initialized(&self) -> Result<(), AgentError> {
        self.endpoint
            .send_notification("notifications/initialized", json!({}))
            .map(|_| ())
    }

    /// List tools.
    /// This appends the corresponding JSON-RPC frame to the in-memory test endpoint transcript.
    pub fn list_tools(&mut self) -> Result<JsonRpcId, AgentError> {
        self.request("tools/list", json!({}))
    }

    /// List resources.
    /// This appends the corresponding JSON-RPC frame to the scripted MCP mock transcript and
    /// returns the request id when applicable.
    pub fn list_resources(&mut self) -> Result<JsonRpcId, AgentError> {
        self.request("resources/list", json!({}))
    }

    /// List prompts.
    /// This appends the corresponding JSON-RPC frame to the scripted MCP mock transcript and
    /// returns the request id when applicable.
    pub fn list_prompts(&mut self) -> Result<JsonRpcId, AgentError> {
        self.request("prompts/list", json!({}))
    }

    /// Call tool.
    /// This appends the corresponding JSON-RPC frame to the in-memory test endpoint transcript.
    pub fn call_tool(&mut self, name: &str, arguments: Value) -> Result<JsonRpcId, AgentError> {
        if !self.allowed_tools.contains(name) {
            return Err(policy_denial(format!(
                "MCP tool {name} is not selected in host proxy policy"
            )));
        }
        self.request("tools/call", json!({"name": name, "arguments": arguments}))
    }

    /// Read resource.
    /// This appends the corresponding JSON-RPC frame to the in-memory test endpoint transcript.
    pub fn read_resource(&mut self, uri: &str) -> Result<JsonRpcId, AgentError> {
        if !self.allowed_resources.contains(uri) {
            return Err(policy_denial(format!(
                "MCP resource {uri} is not selected in host proxy policy"
            )));
        }
        self.request("resources/read", json!({"uri": uri}))
    }

    /// Handle next.
    /// This consumes one queued JSON-RPC frame from the endpoint and mutates only the scripted
    /// mock state.
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

    /// Returns the response currently held by this value.
    /// This reads scripted protocol state or a queued response without contacting an external
    /// process.
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

    /// Returns allowed tool names from response for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
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
