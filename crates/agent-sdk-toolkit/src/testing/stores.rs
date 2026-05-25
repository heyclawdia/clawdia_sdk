//! Toolkit-specific deterministic test helpers. Use these fakes for content stores,
//! argument stores, and scripted protocol harnesses without live editors, MCP
//! servers, or product hosts. Helpers mutate only in-memory state unless noted. This
//! file contains the stores portion of that contract.
//!
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use agent_sdk_core::{AgentError, domain::ContentRef};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

#[derive(Clone, Debug, Default)]
/// In-memory JSON argument store fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct InMemoryJsonArgumentStore {
    entries: Arc<Mutex<BTreeMap<ContentRef, Value>>>,
}

impl InMemoryJsonArgumentStore {
    /// Adds data to this in-memory testing::stores collection. It does not
    /// perform external I/O, execute tools, or append journals.
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

    /// Looks up an entry in this local store without registry or runtime work.
    /// This reads deterministic in-memory test store state and performs no external I/O.
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
/// In-memory toolkit content store fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct InMemoryToolkitContentStore {
    entries: Arc<Mutex<BTreeMap<ContentRef, Value>>>,
}

impl InMemoryToolkitContentStore {
    /// Adds data to this in-memory testing::stores collection. It does not
    /// perform external I/O, execute tools, or append journals.
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

    /// Looks up an entry in this local store without registry or runtime work.
    /// This reads deterministic in-memory test store state and performs no external I/O.
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
