use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use agent_sdk_core::{AgentError, domain::ContentRef};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

pub mod protocol;

pub use protocol::{
    IsolatedJsonRpcProcess, McpHostProxy, ScriptedAcpAgent, ScriptedAcpClient, ScriptedMcpServer,
};

#[derive(Clone, Debug, Default)]
pub struct InMemoryJsonArgumentStore {
    entries: Arc<Mutex<BTreeMap<ContentRef, Value>>>,
}

impl InMemoryJsonArgumentStore {
    pub fn insert<T: Serialize>(
        &self,
        content_ref: ContentRef,
        value: &T,
    ) -> Result<(), AgentError> {
        let value = serde_json::to_value(value).map_err(|error| {
            AgentError::contract_violation(format!("argument serialization failed: {error}"))
        })?;
        self.entries
            .lock()
            .map_err(|_| AgentError::contract_violation("argument store lock poisoned"))?
            .insert(content_ref, value);
        Ok(())
    }

    pub fn get<T: DeserializeOwned>(&self, content_ref: &ContentRef) -> Result<T, AgentError> {
        let value = self
            .entries
            .lock()
            .map_err(|_| AgentError::contract_violation("argument store lock poisoned"))?
            .get(content_ref)
            .cloned()
            .ok_or_else(|| AgentError::missing_required_field("tool_argument.content_ref"))?;
        serde_json::from_value(value).map_err(|error| {
            AgentError::contract_violation(format!("argument deserialization failed: {error}"))
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryToolkitContentStore {
    entries: Arc<Mutex<BTreeMap<ContentRef, Value>>>,
}

impl InMemoryToolkitContentStore {
    pub fn put<T: Serialize>(&self, content_ref: ContentRef, value: &T) -> Result<(), AgentError> {
        let value = serde_json::to_value(value).map_err(|error| {
            AgentError::contract_violation(format!("content serialization failed: {error}"))
        })?;
        self.entries
            .lock()
            .map_err(|_| AgentError::contract_violation("content store lock poisoned"))?
            .insert(content_ref, value);
        Ok(())
    }

    pub fn get<T: DeserializeOwned>(&self, content_ref: &ContentRef) -> Result<T, AgentError> {
        let value = self
            .entries
            .lock()
            .map_err(|_| AgentError::contract_violation("content store lock poisoned"))?
            .get(content_ref)
            .cloned()
            .ok_or_else(|| AgentError::missing_required_field("tool_content.content_ref"))?;
        serde_json::from_value(value).map_err(|error| {
            AgentError::contract_violation(format!("content deserialization failed: {error}"))
        })
    }
}
