use crate::{
    domain::{AgentId, RunId, SourceRef},
    output::OutputContract,
    typed_output_ports::{TypedOutputDeserializer, TypedOutputModel},
    validated_output::{
        StructuredOutputResult, TypedOutputError, TypedResultPublicationRecord, ValidatedOutput,
        ValidationReportRecord,
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunRequest {
    pub run_id: RunId,
    pub agent_id: AgentId,
    pub source: SourceRef,
    pub input: String,
    pub output_contract: Option<OutputContract>,
}

impl RunRequest {
    pub fn text(
        run_id: RunId,
        agent_id: AgentId,
        source: SourceRef,
        input: impl Into<String>,
    ) -> Self {
        Self {
            run_id,
            agent_id,
            source,
            input: input.into(),
            output_contract: None,
        }
    }

    pub fn with_output_contract(mut self, output_contract: OutputContract) -> Self {
        self.output_contract = Some(output_contract);
        self
    }

    pub fn typed_text<T: TypedOutputModel>(
        run_id: RunId,
        agent_id: AgentId,
        source: SourceRef,
        input: impl Into<String>,
    ) -> Self {
        Self::text(run_id, agent_id, source, input)
            .with_output_contract(OutputContract::for_type::<T>())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResult {
    pub run_id: RunId,
    pub status: RunStatus,
    pub output: String,
    pub structured_output: Option<StructuredOutputArtifacts>,
}

impl RunResult {
    pub fn new(run_id: RunId, status: RunStatus, output: impl Into<String>) -> Self {
        Self {
            run_id,
            status,
            output: output.into(),
            structured_output: None,
        }
    }

    pub fn with_structured_output(mut self, structured_output: StructuredOutputArtifacts) -> Self {
        self.structured_output = Some(structured_output);
        self
    }

    pub fn with_structured_output_if_present(
        mut self,
        structured_output: Option<StructuredOutputArtifacts>,
    ) -> Self {
        self.structured_output = structured_output;
        self
    }

    pub fn structured_output<T, D>(
        &self,
        deserializer: &D,
    ) -> Result<StructuredOutputResult<T>, TypedOutputError>
    where
        D: TypedOutputDeserializer<T>,
    {
        let artifacts = self.structured_output.as_ref().ok_or_else(|| {
            TypedOutputError::MissingValidatedOutput {
                run_id: self.run_id.clone(),
            }
        })?;
        StructuredOutputResult::from_publication(
            &artifacts.validated_output,
            &artifacts.typed_result_publication,
            deserializer,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuredOutputArtifacts {
    pub validation_reports: Vec<ValidationReportRecord>,
    pub validated_output: ValidatedOutput,
    pub typed_result_publication: TypedResultPublicationRecord,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunStatus {
    Pending,
    Running,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
    RepairNeeded,
}

impl RunStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    pub fn as_terminal_str(&self) -> Option<&'static str> {
        match self {
            Self::Completed => Some("completed"),
            Self::Failed => Some("failed"),
            Self::Cancelled => Some("cancelled"),
            _ => None,
        }
    }

    pub fn from_terminal_str(value: &str) -> Option<Self> {
        match value {
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}
