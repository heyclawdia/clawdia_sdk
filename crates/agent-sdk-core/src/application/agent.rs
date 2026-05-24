use crate::{
    domain::{AgentError, AgentId, RunId, RuntimePackageId, SourceRef},
    package::AgentSnapshot,
    run::{RunRequest, RunResult},
    runtime::AgentRuntime,
    typed_output_ports::TypedOutputModel,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Agent {
    id: AgentId,
    name: String,
    instructions: Option<String>,
    default_package_id: Option<RuntimePackageId>,
}

impl Agent {
    pub fn builder() -> AgentBuilder {
        AgentBuilder::default()
    }

    pub fn id(&self) -> &AgentId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn instructions(&self) -> Option<&str> {
        self.instructions.as_deref()
    }

    pub fn default_package_id(&self) -> Option<&RuntimePackageId> {
        self.default_package_id.as_ref()
    }

    pub fn snapshot(&self) -> AgentSnapshot {
        AgentSnapshot {
            agent_id: self.id.clone(),
            name: self.name.clone(),
            default_behavior_refs: Vec::new(),
        }
    }

    pub fn run_text(
        &self,
        runtime: &AgentRuntime,
        run_id: RunId,
        source: SourceRef,
        input: impl Into<String>,
    ) -> Result<RunResult, AgentError> {
        runtime.run_text(RunRequest::text(run_id, self.id.clone(), source, input))
    }

    pub fn run_typed<T: TypedOutputModel>(
        &self,
        runtime: &AgentRuntime,
        run_id: RunId,
        source: SourceRef,
        input: impl Into<String>,
    ) -> Result<RunResult, AgentError> {
        runtime.run_typed::<T>(self.typed_text_request::<T>(run_id, source, input))
    }

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
pub struct AgentBuilder {
    id: Option<AgentId>,
    name: Option<String>,
    instructions: Option<String>,
    default_package_id: Option<RuntimePackageId>,
}

impl AgentBuilder {
    pub fn id(mut self, id: AgentId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    pub fn default_package_id(mut self, package_id: RuntimePackageId) -> Self {
        self.default_package_id = Some(package_id);
        self
    }

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
