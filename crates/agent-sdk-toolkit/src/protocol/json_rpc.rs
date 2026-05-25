//! JSON-RPC 2.0 frame DTOs for toolkit protocol conformance. Use this module for
//! encoded request, response, notification, and error frames. Serialization is
//! data-only and does not own process transport.
//!
use agent_sdk_core::AgentError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
/// Identifier carried by JSON-RPC requests and responses.
/// Use numbers or strings for ordinary correlated calls; `Null` is reserved for parse errors or
/// protocol failures where JSON-RPC requires an id but no valid request id exists.
pub enum JsonRpcId {
    /// Numeric request id.
    Number(i64),
    /// String request id.
    String(String),
    /// Null id used for JSON-RPC error responses that cannot be correlated to a valid request.
    Null,
}

impl JsonRpcId {
    /// Returns this value as key. The accessor is side-effect free and
    /// keeps ownership with the caller.
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
/// JSON-RPC request frame sent when a client expects a response.
/// Constructing the value only prepares serialized protocol data; transport effects occur when a
/// line endpoint sends the frame.
pub struct JsonRpcRequest {
    /// Protocol version marker; toolkit constructors always set this to `"2.0"`.
    pub jsonrpc: String,
    /// Correlation id that must be echoed by the matching response.
    pub id: JsonRpcId,
    /// Remote method name, such as `initialize`, `tools/list`, or a host-specific extension
    /// method.
    pub method: String,
    #[serde(default)]
    /// Method parameters serialized as JSON; absent params deserialize as an empty/default value.
    pub params: Value,
}

impl JsonRpcRequest {
    /// Creates a new protocol::json_rpc value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
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
/// JSON-RPC notification frame sent for one-way protocol messages.
/// Notifications have no id and do not receive responses; sending still mutates the chosen line
/// endpoint transcript or transport.
pub struct JsonRpcNotification {
    /// Protocol version marker; toolkit constructors always set this to `"2.0"`.
    pub jsonrpc: String,
    /// Notification method name.
    pub method: String,
    #[serde(default)]
    /// Notification parameters serialized as JSON; absent params deserialize as an empty/default
    /// value.
    pub params: Value,
}

impl JsonRpcNotification {
    /// Creates a new protocol::json_rpc value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
/// JSON-RPC error object embedded in an error response.
/// It carries protocol failure details only; constructing it does not log, publish, or send the
/// error.
pub struct JsonRpcErrorObject {
    /// JSON-RPC error code, using standard protocol codes or an adapter-defined extension code.
    pub code: i64,
    /// Human-readable protocol error message.
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional data value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub data: Option<Value>,
}

impl JsonRpcErrorObject {
    /// Creates a new protocol::json_rpc value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
/// JSON-RPC response frame for a completed request.
/// A response contains exactly one of `result` or `error`; sending it is handled by the endpoint
/// or transport layer.
pub struct JsonRpcResponse {
    /// Protocol version marker; toolkit constructors always set this to `"2.0"`.
    pub jsonrpc: String,
    /// Request id this response is correlated with.
    pub id: JsonRpcId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Result payload produced by a validator, executor, sink, or adapter.
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed error payload or redacted error detail for failed operations.
    pub error: Option<JsonRpcErrorObject>,
}

impl JsonRpcResponse {
    /// Builds the result value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn result(id: JsonRpcId, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Builds the error value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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
/// Parsed JSON-RPC frame used by line transports and scripted protocol fakes.
/// Matching on the frame is side-effect free; endpoint methods own any transcript or transport
/// mutation.
pub enum JsonRpcFrame {
    /// Client request that expects a response.
    Request(JsonRpcRequest),
    /// Response to a previously sent request.
    Response(JsonRpcResponse),
    /// One-way notification with no response id.
    Notification(JsonRpcNotification),
}

impl JsonRpcFrame {
    /// Converts this value into line data.
    /// This serializes the frame into one JSON-RPC line and performs no transport I/O.
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

    /// Constructs this value from line. Use it when adapting canonical
    /// SDK records without introducing a second behavior path.
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

/// Extracts a response frame or returns a protocol violation.
/// This is a pure frame check used by scripted tests; it does not read or write a transport.
pub(crate) fn expect_response(frame: JsonRpcFrame) -> Result<JsonRpcResponse, AgentError> {
    match frame {
        JsonRpcFrame::Response(response) => Ok(response),
        _ => Err(protocol_violation("expected json-rpc response frame")),
    }
}

/// Extracts a notification frame or returns a protocol violation.
/// This is a pure frame check used by scripted tests; it does not read or write a transport.
pub(crate) fn expect_notification(frame: JsonRpcFrame) -> Result<JsonRpcNotification, AgentError> {
    match frame {
        JsonRpcFrame::Notification(notification) => Ok(notification),
        _ => Err(protocol_violation("expected json-rpc notification frame")),
    }
}

/// Converts a serde JSON failure into the toolkit's protocol-violation error.
/// The conversion only allocates an error value; callers decide whether to return, send, or log it.
pub(crate) fn json_error(error: serde_json::Error) -> AgentError {
    protocol_violation(format!("json-rpc serialization failed: {error}"))
}

/// Converts a stdio transport failure into the toolkit's protocol-violation error.
/// The conversion only allocates an error value; it does not retry or touch the transport.
pub(crate) fn stdio_error(error: std::io::Error) -> AgentError {
    protocol_violation(format!("json-rpc stdio transport failed: {error}"))
}

/// Validates the protocol::json_rpc invariants and returns a typed
/// error on failure. Validation is pure and does not perform I/O,
/// dispatch, journal appends, or adapter calls.
pub(crate) fn validate_json_rpc_line(line: &str) -> Result<(), AgentError> {
    if line.contains('\n') || line.contains('\r') {
        return Err(protocol_violation(
            "json-rpc line transport frames must not contain embedded newlines",
        ));
    }
    Ok(())
}

/// Creates a typed contract-violation error for malformed protocol frames.
/// This helper is side-effect free; endpoint code decides whether the error becomes a JSON-RPC
/// error response.
pub(crate) fn protocol_violation(message: impl Into<String>) -> AgentError {
    AgentError::contract_violation(message)
}
