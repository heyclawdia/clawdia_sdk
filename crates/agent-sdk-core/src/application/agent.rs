//! Agent construction and common run-request helpers. Use this module to define
//! immutable agent identity and lower simple text or typed-output calls into
//! canonical run requests. Side effects are limited to methods that explicitly call a
//! runtime; builders and request helpers are data-only.
//!
use crate::{
    domain::{AgentError, AgentId, RunId, RuntimePackageId, SourceRef},
    package::AgentSnapshot,
    run::{RunRequest, RunResult},
    runtime::AgentRuntime,
    typed_output_ports::TypedOutputModel,
};

#[derive(Clone, Debug, Eq, PartialEq)]
/// Holds agent application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct Agent {
    id: AgentId,
    name: String,
    instructions: Option<String>,
    default_package_id: Option<RuntimePackageId>,
}

impl Agent {
    /// Starts a builder for this application::agent value. Building is
    /// data-only; runtime side effects occur only when a later
    /// coordinator or host port executes the built configuration.
    pub fn builder() -> AgentBuilder {
        AgentBuilder::default()
    }

    /// Returns the configured agent id stored on this `Agent`.
    /// This is a data-only accessor and performs no runtime work.
    pub fn id(&self) -> &AgentId {
        &self.id
    }

    /// Returns the display name stored on this `Agent`.
    /// This is a data-only accessor and performs no runtime work.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the optional instruction text attached to this agent.
    /// This is a read-only accessor and does not register or run the agent.
    pub fn instructions(&self) -> Option<&str> {
        self.instructions.as_deref()
    }

    /// Returns the default runtime package id for this agent, when one is set.
    /// This is a read-only accessor and performs no runtime work.
    pub fn default_package_id(&self) -> Option<&RuntimePackageId> {
        self.default_package_id.as_ref()
    }

    /// Builds a snapshot of this agent's identity metadata.
    /// This clones agent identity metadata into an AgentSnapshot without registering or running
    /// the agent.
    pub fn snapshot(&self) -> AgentSnapshot {
        AgentSnapshot {
            agent_id: self.id.clone(),
            name: self.name.clone(),
            default_behavior_refs: Vec::new(),
        }
    }

    /// Runs a text request for this agent through the supplied runtime.
    /// This helper builds the `RunRequest` with this agent id; all provider,
    /// journal, event, and validation side effects are owned by
    /// `AgentRuntime::run_text`.
    pub fn run_text(
        &self,
        runtime: &AgentRuntime,
        run_id: RunId,
        source: SourceRef,
        input: impl Into<String>,
    ) -> Result<RunResult, AgentError> {
        runtime.run_text(RunRequest::text(run_id, self.id.clone(), source, input))
    }

    /// Runs a typed request for this agent through the supplied runtime.
    /// This helper builds the typed `RunRequest`; all provider, journal,
    /// event, validation, and repair side effects are owned by
    /// `AgentRuntime::run_typed`.
    pub fn run_typed<T: TypedOutputModel>(
        &self,
        runtime: &AgentRuntime,
        run_id: RunId,
        source: SourceRef,
        input: impl Into<String>,
    ) -> Result<RunResult, AgentError> {
        runtime.run_typed::<T>(self.typed_text_request::<T>(run_id, source, input))
    }

    /// Builds a typed text run request for this agent.
    /// This constructs data only; provider calls, validation, repair, journals,
    /// and events occur later through `AgentRuntime::run_typed`.
    pub fn typed_text_request<T: TypedOutputModel>(
        &self,
        run_id: RunId,
        source: SourceRef,
        input: impl Into<String>,
    ) -> RunRequest {
        RunRequest::typed_text::<T>(run_id, self.id.clone(), source, input)
    }
}

#[derive(Clone, Debug)]
/// Holds agent builder application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct AgentBuilder {
    id: Option<AgentId>,
    name: Option<String>,
    instructions: Option<String>,
    default_package_id: Option<RuntimePackageId>,
}

impl AgentBuilder {
    /// Sets the agent id on the builder.
    /// This only updates builder state; it does not register the agent or start a run.
    pub fn id(mut self, id: AgentId) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the display name on the builder.
    /// This only updates builder state; it does not register the agent or start a run.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets optional instruction text on the builder.
    /// This only updates builder state; it does not register the agent or start a run.
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Sets the default runtime package id for the agent.
    /// This only updates builder state; it does not register the agent or start a run.
    pub fn default_package_id(mut self, package_id: RuntimePackageId) -> Self {
        self.default_package_id = Some(package_id);
        self
    }

    /// Finishes builder validation and returns the configured value.
    /// This is data-only unless the surrounding builder explicitly
    /// documents adapter or store access.
    pub fn build(self) -> Result<Agent, AgentError> {
        let name = self.name.unwrap_or_else(|| "agent".to_string());
        if name.trim().is_empty() {
            return Err(AgentError::missing_required_field("agent.name"));
        }

        Ok(Agent {
            id: self.id.unwrap_or_else(|| AgentId::new("agent.default")),
            name,
            instructions: self.instructions,
            default_package_id: self.default_package_id,
        })
    }
}

impl Default for AgentBuilder {
    fn default() -> Self {
        Self {
            id: None,
            name: None,
            instructions: None,
            default_package_id: None,
        }
    }
}
