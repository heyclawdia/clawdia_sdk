use std::collections::{BTreeMap, BTreeSet};

use agent_sdk_core::AgentError;
use serde_json::{Value, json};

use crate::protocol::{
    JsonRpcFrame, JsonRpcId, JsonRpcLineEndpoint, JsonRpcNotification, JsonRpcRequest,
    JsonRpcResponse, expect_notification, expect_response, protocol_violation,
};

enum ReceiveNext {
    Empty,
    Handled,
    Frame(JsonRpcFrame),
}

#[derive(Clone, Debug)]
pub struct ScriptedAcpClient {
    endpoint: JsonRpcLineEndpoint,
    next_id: i64,
}

impl ScriptedAcpClient {
    pub fn new(endpoint: JsonRpcLineEndpoint) -> Self {
        Self {
            endpoint,
            next_id: 1,
        }
    }

    pub fn endpoint(&self) -> &JsonRpcLineEndpoint {
        &self.endpoint
    }

    pub fn initialize(&mut self) -> Result<JsonRpcId, AgentError> {
        self.request(
            "initialize",
            json!({
                "protocolVersion": 1,
                "clientInfo": {
                    "name": "agent-sdk-toolkit-acp-fake",
                    "version": "0.0.0"
                },
                "clientCapabilities": {
                    "fs": {"readTextFile": false, "writeTextFile": false},
                    "terminal": false
                }
            }),
        )
    }

    pub fn new_session(&mut self, cwd: impl Into<String>) -> Result<JsonRpcId, AgentError> {
        self.request(
            "session/new",
            json!({
                "cwd": cwd.into(),
                "mcpServers": []
            }),
        )
    }

    pub fn prompt(
        &mut self,
        session_id: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Result<JsonRpcId, AgentError> {
        self.request(
            "session/prompt",
            json!({
                "sessionId": session_id.into(),
                "prompt": [{"type": "text", "text": prompt.into()}]
            }),
        )
    }

    pub fn cancel(&self, session_id: impl Into<String>) -> Result<(), AgentError> {
        self.endpoint
            .send_notification("session/cancel", json!({"sessionId": session_id.into()}))
            .map(|_| ())
    }

    pub fn handle_next(&mut self, endpoint: &JsonRpcLineEndpoint) -> Result<bool, AgentError> {
        let frame = match receive_frame_or_parse_error(endpoint)? {
            ReceiveNext::Empty => return Ok(false),
            ReceiveNext::Handled => return Ok(true),
            ReceiveNext::Frame(frame) => frame,
        };
        let JsonRpcFrame::Request(request) = frame else {
            return Ok(true);
        };
        match request.method.as_str() {
            "fs/read_text_file" => {
                endpoint.send_error(
                    Some(request.id),
                    -32003,
                    "ACP fs/read_text_file denied by client capability policy",
                )?;
            }
            "terminal/create" => {
                endpoint.send_error(
                    Some(request.id),
                    -32004,
                    "ACP terminal/create denied by client capability policy",
                )?;
            }
            "session/request_permission" => {
                endpoint.send_result(request.id, json!({"outcome": {"outcome": "cancelled"}}))?;
            }
            _ => {
                endpoint.send_error(Some(request.id), -32601, "ACP client method not found")?;
            }
        }
        Ok(true)
    }

    pub fn response(&self) -> Result<JsonRpcResponse, AgentError> {
        expect_response(self.endpoint.receive_frame()?)
    }

    pub fn notification(&self) -> Result<JsonRpcNotification, AgentError> {
        expect_notification(self.endpoint.receive_frame()?)
    }

    fn request(&mut self, method: &str, params: Value) -> Result<JsonRpcId, AgentError> {
        let id = JsonRpcId::Number(self.next_id);
        self.next_id += 1;
        self.endpoint
            .send_request(id.clone(), method, params)
            .map(|_| id)
    }
}

#[derive(Clone, Debug)]
pub struct ScriptedAcpAgent {
    session_id: String,
    prompt_outputs: BTreeMap<String, String>,
    cancelled_sessions: BTreeSet<String>,
    next_client_request: i64,
}

impl ScriptedAcpAgent {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            prompt_outputs: BTreeMap::new(),
            cancelled_sessions: BTreeSet::new(),
            next_client_request: 1,
        }
    }

    pub fn with_prompt_output(
        mut self,
        prompt: impl Into<String>,
        output: impl Into<String>,
    ) -> Self {
        self.prompt_outputs.insert(prompt.into(), output.into());
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
                if notification.method == "session/cancel" {
                    if let Some(session_id) =
                        notification.params.get("sessionId").and_then(Value::as_str)
                    {
                        self.cancelled_sessions.insert(session_id.to_string());
                    }
                    return Ok(true);
                }
                endpoint.send_error(
                    None,
                    -32600,
                    "ACP agent expected a request or known notification",
                )?;
                return Ok(true);
            }
            JsonRpcFrame::Response(_) => {
                endpoint.send_error(None, -32600, "ACP agent expected a request frame")?;
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
                            .unwrap_or_else(|| json!(1)),
                        "agentCapabilities": {
                            "auth": {},
                            "loadSession": false,
                            "mcpCapabilities": {"http": false, "sse": false},
                            "promptCapabilities": {
                                "audio": false,
                                "embeddedContext": false,
                                "image": false
                            },
                            "sessionCapabilities": {}
                        },
                        "agentInfo": {
                            "name": "agent-sdk-toolkit-acp-fake",
                            "version": "0.0.0"
                        },
                        "authMethods": []
                    }),
                )?;
            }
            "session/new" => {
                endpoint.send_result(
                    request.id,
                    json!({
                        "sessionId": self.session_id,
                        "modes": null,
                        "configOptions": null
                    }),
                )?;
            }
            "session/prompt" => self.handle_prompt(endpoint, request)?,
            _ => {
                endpoint.send_error(Some(request.id), -32601, "ACP method not found")?;
            }
        };
        Ok(true)
    }

    pub fn request_file_read(
        &mut self,
        endpoint: &JsonRpcLineEndpoint,
        session_id: impl Into<String>,
        path: impl Into<String>,
    ) -> Result<JsonRpcId, AgentError> {
        let id = self.next_client_request_id();
        endpoint
            .send_request(
                id.clone(),
                "fs/read_text_file",
                json!({"sessionId": session_id.into(), "path": path.into()}),
            )
            .map(|_| id)
    }

    pub fn request_terminal_create(
        &mut self,
        endpoint: &JsonRpcLineEndpoint,
        session_id: impl Into<String>,
        command: impl Into<String>,
    ) -> Result<JsonRpcId, AgentError> {
        let id = self.next_client_request_id();
        endpoint
            .send_request(
                id.clone(),
                "terminal/create",
                json!({
                    "sessionId": session_id.into(),
                    "command": command.into(),
                    "args": [],
                    "cwd": null
                }),
            )
            .map(|_| id)
    }

    pub fn request_permission(
        &mut self,
        endpoint: &JsonRpcLineEndpoint,
        session_id: impl Into<String>,
        tool_call_id: impl Into<String>,
    ) -> Result<JsonRpcId, AgentError> {
        let id = self.next_client_request_id();
        endpoint
            .send_request(
                id.clone(),
                "session/request_permission",
                json!({
                    "sessionId": session_id.into(),
                    "toolCall": {"toolCallId": tool_call_id.into()},
                    "options": [
                        {"optionId": "allow-once", "name": "Allow once", "kind": "allow_once"},
                        {"optionId": "reject-once", "name": "Reject", "kind": "reject_once"}
                    ]
                }),
            )
            .map(|_| id)
    }

    pub fn response(&self, endpoint: &JsonRpcLineEndpoint) -> Result<JsonRpcResponse, AgentError> {
        expect_response(endpoint.receive_frame()?)
    }

    pub fn cancelled_sessions(&self) -> BTreeSet<String> {
        self.cancelled_sessions.clone()
    }

    fn next_client_request_id(&mut self) -> JsonRpcId {
        let id = JsonRpcId::Number(self.next_client_request);
        self.next_client_request += 1;
        id
    }

    fn handle_prompt(
        &mut self,
        endpoint: &JsonRpcLineEndpoint,
        request: JsonRpcRequest,
    ) -> Result<(), AgentError> {
        let prompt = request
            .params
            .get("prompt")
            .and_then(Value::as_array)
            .and_then(|blocks| blocks.first())
            .and_then(|block| block.get("text"))
            .and_then(Value::as_str)
            .ok_or_else(|| protocol_violation("ACP prompt request requires prompt string"))?;
        let session_id = request
            .params
            .get("sessionId")
            .and_then(Value::as_str)
            .ok_or_else(|| protocol_violation("ACP prompt request requires sessionId"))?;
        if session_id != self.session_id {
            endpoint.send_error(Some(request.id), -32602, "ACP session is not active")?;
            return Ok(());
        }
        let output = self
            .prompt_outputs
            .get(prompt)
            .cloned()
            .unwrap_or_else(|| "scripted ACP response".to_string());
        endpoint.send_notification(
            "session/update",
            json!({
                "sessionId": session_id,
                "update": {
                    "sessionUpdate": "agent_message_chunk",
                    "content": {
                        "type": "text",
                        "text": output
                    }
                }
            }),
        )?;
        endpoint.send_result(request.id, json!({"stopReason": "end_turn"}))?;
        Ok(())
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
