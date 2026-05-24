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
pub struct JsonRpcLineCodec;

impl JsonRpcLineCodec {
    pub fn write_frame(writer: &mut impl Write, frame: &JsonRpcFrame) -> Result<(), AgentError> {
        let line = frame.to_line()?;
        writer.write_all(line.as_bytes()).map_err(stdio_error)?;
        writer.write_all(b"\n").map_err(stdio_error)?;
        writer.flush().map_err(stdio_error)
    }

    pub fn read_frame(reader: &mut impl BufRead) -> Result<Option<JsonRpcFrame>, AgentError> {
        let Some(line) = Self::read_line(reader)? else {
            return Ok(None);
        };
        JsonRpcFrame::from_line(&line).map(Some)
    }

    pub fn write_raw_line(writer: &mut impl Write, line: &str) -> Result<(), AgentError> {
        validate_json_rpc_line(line)?;
        writer.write_all(line.as_bytes()).map_err(stdio_error)?;
        writer.write_all(b"\n").map_err(stdio_error)?;
        writer.flush().map_err(stdio_error)
    }

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
pub struct JsonRpcLineEndpoint {
    name: String,
    incoming: Arc<Mutex<VecDeque<Vec<u8>>>>,
    outgoing: Arc<Mutex<VecDeque<Vec<u8>>>>,
    sent_lines: Arc<Mutex<Vec<String>>>,
    received_lines: Arc<Mutex<Vec<String>>>,
}

impl JsonRpcLineEndpoint {
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

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn send_frame(&self, frame: JsonRpcFrame) -> Result<String, AgentError> {
        let line = frame.to_line()?;
        let mut bytes = Vec::new();
        JsonRpcLineCodec::write_frame(&mut bytes, &frame)?;
        self.queue_line(line.clone(), bytes)?;
        Ok(line)
    }

    pub fn send_raw_line(&self, line: impl Into<String>) -> Result<(), AgentError> {
        let line = line.into();
        let mut bytes = Vec::new();
        JsonRpcLineCodec::write_raw_line(&mut bytes, &line)?;
        self.queue_line(line, bytes)
    }

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

    pub fn try_receive_frame(&self) -> Result<Option<JsonRpcFrame>, AgentError> {
        self.try_receive_raw_line()?
            .map(|line| JsonRpcFrame::from_line(&line))
            .transpose()
    }

    pub fn receive_frame(&self) -> Result<JsonRpcFrame, AgentError> {
        self.try_receive_frame()?.ok_or_else(|| {
            protocol_violation(format!("{} has no queued json-rpc frame", self.name))
        })
    }

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

    pub fn send_notification(
        &self,
        method: impl Into<String>,
        params: serde_json::Value,
    ) -> Result<String, AgentError> {
        self.send_frame(JsonRpcFrame::Notification(JsonRpcNotification::new(
            method, params,
        )))
    }

    pub fn send_result(
        &self,
        id: JsonRpcId,
        result: serde_json::Value,
    ) -> Result<String, AgentError> {
        self.send_frame(JsonRpcFrame::Response(JsonRpcResponse::result(id, result)))
    }

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

    pub fn response(&self) -> Result<JsonRpcResponse, AgentError> {
        expect_response(self.receive_frame()?)
    }

    pub fn notification(&self) -> Result<JsonRpcNotification, AgentError> {
        expect_notification(self.receive_frame()?)
    }

    pub fn sent_lines(&self) -> Vec<String> {
        self.sent_lines
            .lock()
            .expect("json-rpc sent transcript")
            .clone()
    }

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
