use agent_sdk_core::AgentError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum JsonRpcId {
    Number(i64),
    String(String),
    Null,
}

impl JsonRpcId {
    pub fn as_key(&self) -> String {
        match self {
            Self::Number(value) => value.to_string(),
            Self::String(value) => value.clone(),
            Self::Null => "null".to_string(),
        }
    }
}

impl From<&str> for JsonRpcId {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<String> for JsonRpcId {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<i64> for JsonRpcId {
    fn from(value: i64) -> Self {
        Self::Number(value)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

impl JsonRpcRequest {
    pub fn new(id: impl Into<JsonRpcId>, method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            method: method.into(),
            params,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

impl JsonRpcNotification {
    pub fn new(method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct JsonRpcErrorObject {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcErrorObject {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcErrorObject>,
}

impl JsonRpcResponse {
    pub fn result(id: JsonRpcId, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<JsonRpcId>, code: i64, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.unwrap_or(JsonRpcId::Null),
            result: None,
            error: Some(JsonRpcErrorObject::new(code, message)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum JsonRpcFrame {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
}

impl JsonRpcFrame {
    pub fn to_line(&self) -> Result<String, AgentError> {
        let value = match self {
            Self::Request(frame) => serde_json::to_value(frame),
            Self::Response(frame) => serde_json::to_value(frame),
            Self::Notification(frame) => serde_json::to_value(frame),
        }
        .map_err(json_error)?;
        let line = serde_json::to_string(&value).map_err(json_error)?;
        validate_json_rpc_line(&line)?;
        Ok(line)
    }

    pub fn from_line(line: &str) -> Result<Self, AgentError> {
        validate_json_rpc_line(line)?;
        let value: Value = serde_json::from_str(line).map_err(json_error)?;
        let object = value
            .as_object()
            .ok_or_else(|| protocol_violation("json-rpc frame must be an object"))?;
        match object.get("jsonrpc").and_then(Value::as_str) {
            Some("2.0") => {}
            _ => {
                return Err(protocol_violation(
                    "json-rpc frame must declare version 2.0",
                ));
            }
        }
        if object.contains_key("method") {
            if object.contains_key("id") {
                if object.get("id").is_some_and(Value::is_null) {
                    return Err(protocol_violation("json-rpc request id must not be null"));
                }
                return serde_json::from_value(value)
                    .map(Self::Request)
                    .map_err(json_error);
            }
            return serde_json::from_value(value)
                .map(Self::Notification)
                .map_err(json_error);
        }
        if object.contains_key("result") || object.contains_key("error") {
            if object.contains_key("result") == object.contains_key("error") {
                return Err(protocol_violation(
                    "json-rpc response must contain exactly one of result or error",
                ));
            }
            if !object.contains_key("id") {
                return Err(protocol_violation("json-rpc response must include id"));
            }
            return serde_json::from_value(value)
                .map(Self::Response)
                .map_err(json_error);
        }
        Err(protocol_violation(
            "json-rpc frame is neither request nor response",
        ))
    }
}

pub(crate) fn expect_response(frame: JsonRpcFrame) -> Result<JsonRpcResponse, AgentError> {
    match frame {
        JsonRpcFrame::Response(response) => Ok(response),
        _ => Err(protocol_violation("expected json-rpc response frame")),
    }
}

pub(crate) fn expect_notification(frame: JsonRpcFrame) -> Result<JsonRpcNotification, AgentError> {
    match frame {
        JsonRpcFrame::Notification(notification) => Ok(notification),
        _ => Err(protocol_violation("expected json-rpc notification frame")),
    }
}

pub(crate) fn json_error(error: serde_json::Error) -> AgentError {
    protocol_violation(format!("json-rpc serialization failed: {error}"))
}

pub(crate) fn stdio_error(error: std::io::Error) -> AgentError {
    protocol_violation(format!("json-rpc stdio transport failed: {error}"))
}

pub(crate) fn validate_json_rpc_line(line: &str) -> Result<(), AgentError> {
    if line.contains('\n') || line.contains('\r') {
        return Err(protocol_violation(
            "json-rpc line transport frames must not contain embedded newlines",
        ));
    }
    Ok(())
}

pub(crate) fn protocol_violation(message: impl Into<String>) -> AgentError {
    AgentError::contract_violation(message)
}
