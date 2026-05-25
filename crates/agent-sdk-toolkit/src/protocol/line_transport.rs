//! Newline-delimited JSON-RPC transport helpers. Use this module for deterministic
//! stdio-style tests and lightweight protocol harnesses. Endpoint helpers mutate
//! in-memory transcripts and codec helpers read or write caller-provided streams.
//!
use std::{
    collections::VecDeque,
    io::{BufRead, Cursor, Write},
    sync::{Arc, Mutex},
};

use agent_sdk_core::AgentError;

use super::JsonRpcId;
use super::json_rpc::{
    JsonRpcFrame, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, expect_notification,
    expect_response, protocol_violation, stdio_error, validate_json_rpc_line,
};

#[derive(Clone, Debug, Default)]
/// Protocol json rpc line codec value used by toolkit JSON-RPC adapters.
/// Constructing the value prepares protocol data; endpoint and transport methods own transcript or I/O effects.
pub struct JsonRpcLineCodec;

impl JsonRpcLineCodec {
    /// Writes one newline-delimited JSON-RPC frame or raw line to the
    /// caller-provided writer. It does not launch a process, open sockets, or
    /// persist transport state.
    pub fn write_frame(writer: &mut impl Write, frame: &JsonRpcFrame) -> Result<(), AgentError> {
        let line = frame.to_line()?;
        writer.write_all(line.as_bytes()).map_err(stdio_error)?;
        writer.write_all(b"\n").map_err(stdio_error)?;
        writer.flush().map_err(stdio_error)
    }

    /// Reads one newline-delimited JSON-RPC frame or raw line from the
    /// caller-provided reader. It does not launch a process, open sockets, or
    /// persist transport state.
    pub fn read_frame(reader: &mut impl BufRead) -> Result<Option<JsonRpcFrame>, AgentError> {
        let Some(line) = Self::read_line(reader)? else {
            return Ok(None);
        };
        JsonRpcFrame::from_line(&line).map(Some)
    }

    /// Writes one newline-delimited JSON-RPC frame or raw line to the
    /// caller-provided writer. It does not launch a process, open sockets, or
    /// persist transport state.
    pub fn write_raw_line(writer: &mut impl Write, line: &str) -> Result<(), AgentError> {
        validate_json_rpc_line(line)?;
        writer.write_all(line.as_bytes()).map_err(stdio_error)?;
        writer.write_all(b"\n").map_err(stdio_error)?;
        writer.flush().map_err(stdio_error)
    }

    /// Reads one newline-delimited JSON-RPC frame or raw line from the
    /// caller-provided reader. It does not launch a process, open sockets, or
    /// persist transport state.
    pub fn read_line(reader: &mut impl BufRead) -> Result<Option<String>, AgentError> {
        let mut line = String::new();
        let read = reader.read_line(&mut line).map_err(stdio_error)?;
        if read == 0 {
            return Ok(None);
        }
        if !line.ends_with('\n') {
            return Err(protocol_violation(
                "json-rpc stdio frame must be newline-delimited",
            ));
        }
        let line = line.trim_end_matches('\n').trim_end_matches('\r');
        validate_json_rpc_line(line)?;
        Ok(Some(line.to_string()))
    }
}

#[derive(Clone, Debug)]
/// Protocol json rpc line endpoint value used by toolkit JSON-RPC adapters.
/// Constructing the value prepares protocol data; endpoint and transport methods own transcript or I/O effects.
pub struct JsonRpcLineEndpoint {
    name: String,
    incoming: Arc<Mutex<VecDeque<Vec<u8>>>>,
    outgoing: Arc<Mutex<VecDeque<Vec<u8>>>>,
    sent_lines: Arc<Mutex<Vec<String>>>,
    received_lines: Arc<Mutex<Vec<String>>>,
}

impl JsonRpcLineEndpoint {
    /// Builds the pair value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn pair(left_name: impl Into<String>, right_name: impl Into<String>) -> (Self, Self) {
        let left_to_right = Arc::new(Mutex::new(VecDeque::new()));
        let right_to_left = Arc::new(Mutex::new(VecDeque::new()));
        let left = Self {
            name: left_name.into(),
            incoming: right_to_left.clone(),
            outgoing: left_to_right.clone(),
            sent_lines: Arc::new(Mutex::new(Vec::new())),
            received_lines: Arc::new(Mutex::new(Vec::new())),
        };
        let right = Self {
            name: right_name.into(),
            incoming: left_to_right,
            outgoing: right_to_left,
            sent_lines: Arc::new(Mutex::new(Vec::new())),
            received_lines: Arc::new(Mutex::new(Vec::new())),
        };
        (left, right)
    }

    /// Returns the name currently held by this value.
    /// This reads endpoint metadata or a queued response from the in-memory transport.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Queues a JSON-RPC frame on the paired in-memory endpoint and records
    /// transcript state for tests; it performs no OS-level I/O.
    pub fn send_frame(&self, frame: JsonRpcFrame) -> Result<String, AgentError> {
        let line = frame.to_line()?;
        let mut bytes = Vec::new();
        JsonRpcLineCodec::write_frame(&mut bytes, &frame)?;
        self.queue_line(line.clone(), bytes)?;
        Ok(line)
    }

    /// Queues a JSON-RPC frame on the paired in-memory endpoint and records
    /// transcript state for tests; it performs no OS-level I/O.
    pub fn send_raw_line(&self, line: impl Into<String>) -> Result<(), AgentError> {
        let line = line.into();
        let mut bytes = Vec::new();
        JsonRpcLineCodec::write_raw_line(&mut bytes, &line)?;
        self.queue_line(line, bytes)
    }

    /// Reads a JSON-RPC line or frame from the in-memory endpoint queue. It
    /// does not perform OS-level I/O.
    pub fn try_receive_raw_line(&self) -> Result<Option<String>, AgentError> {
        let Some(bytes) = self
            .incoming
            .lock()
            .map_err(|_| protocol_violation("json-rpc incoming lock poisoned"))?
            .pop_front()
        else {
            return Ok(None);
        };
        let mut reader = Cursor::new(bytes);
        let line = JsonRpcLineCodec::read_line(&mut reader)?;
        if let Some(line) = &line {
            self.received_lines
                .lock()
                .map_err(|_| protocol_violation("json-rpc received transcript lock poisoned"))?
                .push(line.clone());
        }
        Ok(line)
    }

    /// Reads a JSON-RPC line or frame from the in-memory endpoint queue. It
    /// does not perform OS-level I/O.
    pub fn try_receive_frame(&self) -> Result<Option<JsonRpcFrame>, AgentError> {
        self.try_receive_raw_line()?
            .map(|line| JsonRpcFrame::from_line(&line))
            .transpose()
    }

    /// Reads a JSON-RPC line or frame from the in-memory endpoint queue. It
    /// does not perform OS-level I/O.
    pub fn receive_frame(&self) -> Result<JsonRpcFrame, AgentError> {
        self.try_receive_frame()?.ok_or_else(|| {
            protocol_violation(format!("{} has no queued json-rpc frame", self.name))
        })
    }

    /// Queues a JSON-RPC frame on the paired in-memory endpoint and records
    /// transcript state for tests; it performs no OS-level I/O.
    pub fn send_request(
        &self,
        id: impl Into<JsonRpcId>,
        method: impl Into<String>,
        params: serde_json::Value,
    ) -> Result<String, AgentError> {
        self.send_frame(JsonRpcFrame::Request(JsonRpcRequest::new(
            id, method, params,
        )))
    }

    /// Queues a JSON-RPC frame on the paired in-memory endpoint and records
    /// transcript state for tests; it performs no OS-level I/O.
    pub fn send_notification(
        &self,
        method: impl Into<String>,
        params: serde_json::Value,
    ) -> Result<String, AgentError> {
        self.send_frame(JsonRpcFrame::Notification(JsonRpcNotification::new(
            method, params,
        )))
    }

    /// Queues a JSON-RPC frame on the paired in-memory endpoint and records
    /// transcript state for tests; it performs no OS-level I/O.
    pub fn send_result(
        &self,
        id: JsonRpcId,
        result: serde_json::Value,
    ) -> Result<String, AgentError> {
        self.send_frame(JsonRpcFrame::Response(JsonRpcResponse::result(id, result)))
    }

    /// Queues a JSON-RPC frame on the paired in-memory endpoint and records
    /// transcript state for tests; it performs no OS-level I/O.
    pub fn send_error(
        &self,
        id: Option<JsonRpcId>,
        code: i64,
        message: impl Into<String>,
    ) -> Result<String, AgentError> {
        self.send_frame(JsonRpcFrame::Response(JsonRpcResponse::error(
            id, code, message,
        )))
    }

    /// Returns the response currently held by this value.
    /// This reads endpoint metadata or a queued response from the in-memory transport.
    pub fn response(&self) -> Result<JsonRpcResponse, AgentError> {
        expect_response(self.receive_frame()?)
    }

    /// Returns notification for this protocol::line_transport value without
    /// performing external I/O.
    pub fn notification(&self) -> Result<JsonRpcNotification, AgentError> {
        expect_notification(self.receive_frame()?)
    }

    /// Returns sent lines for this protocol::line_transport value without
    /// performing external I/O. Panics only if the in-memory test transcript
    /// lock is poisoned.
    pub fn sent_lines(&self) -> Vec<String> {
        self.sent_lines
            .lock()
            .expect("json-rpc sent transcript")
            .clone()
    }

    /// Returns received lines for this protocol::line_transport value without
    /// performing external I/O. Panics only if the in-memory test transcript
    /// lock is poisoned.
    pub fn received_lines(&self) -> Vec<String> {
        self.received_lines
            .lock()
            .expect("json-rpc received transcript")
            .clone()
    }

    fn queue_line(&self, line: String, bytes: Vec<u8>) -> Result<(), AgentError> {
        self.sent_lines
            .lock()
            .map_err(|_| protocol_violation("json-rpc sent transcript lock poisoned"))?
            .push(line);
        self.outgoing
            .lock()
            .map_err(|_| protocol_violation("json-rpc outgoing lock poisoned"))?
            .push_back(bytes);
        Ok(())
    }
}
