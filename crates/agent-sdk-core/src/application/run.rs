//! Run request and result records used by both simple helpers and explicit advanced
//! callers. Use these DTOs at host boundaries when constructing input and reading
//! terminal output. Constructors are data-only and do not contact providers.
//!
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
/// Holds run request application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct RunRequest {
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Input used by this record or request.
    pub input: String,
    /// Optional output contract value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub output_contract: Option<OutputContract>,
}

impl RunRequest {
    /// Builds the text value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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

    /// Returns this value with its output contract setting replaced.
    /// The method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_output_contract(mut self, output_contract: OutputContract) -> Self {
        self.output_contract = Some(output_contract);
        self
    }

    /// Builds the typed text value with the documented defaults.
    /// This uses only local coordinator state and performs no hidden host work.
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
/// Holds run result application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct RunResult {
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Finite status for this record or lifecycle stage.
    pub status: RunStatus,
    /// Output used by this record or request.
    pub output: String,
    /// Optional structured output value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub structured_output: Option<StructuredOutputArtifacts>,
}

impl RunResult {
    /// Creates a new application::run value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(run_id: RunId, status: RunStatus, output: impl Into<String>) -> Self {
        Self {
            run_id,
            status,
            output: output.into(),
            structured_output: None,
        }
    }

    /// Returns this value with its structured output setting replaced.
    /// The method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_structured_output(mut self, structured_output: StructuredOutputArtifacts) -> Self {
        self.structured_output = Some(structured_output);
        self
    }

    /// Returns this value with its structured output if present setting
    /// replaced. The method follows builder-style data construction and
    /// does not execute external work.
    pub fn with_structured_output_if_present(
        mut self,
        structured_output: Option<StructuredOutputArtifacts>,
    ) -> Self {
        self.structured_output = structured_output;
        self
    }

    /// Structured output.
    /// This decodes structured output from the completed run result and does not rerun
    /// validation or call a provider.
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
/// Holds structured output artifacts application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct StructuredOutputArtifacts {
    /// Validation policy applied before output is accepted as typed data.
    /// It controls validator selection, bounds, failure visibility, and local validation
    /// behavior.
    pub validation_reports: Vec<ValidationReportRecord>,
    /// Validated output used by this record or request.
    pub validated_output: ValidatedOutput,
    /// Typed result publication used by this record or request.
    pub typed_result_publication: TypedResultPublicationRecord,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Enumerates the finite run status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RunStatus {
    /// Use this variant when the contract needs to represent pending; selecting it has no side effect by itself.
    Pending,
    /// Use this variant when the contract needs to represent running; selecting it has no side effect by itself.
    Running,
    /// Use this variant when the contract needs to represent cancelling; selecting it has no side effect by itself.
    Cancelling,
    /// Use this variant when the contract needs to represent completed; selecting it has no side effect by itself.
    Completed,
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed,
    /// Use this variant when the contract needs to represent cancelled; selecting it has no side effect by itself.
    Cancelled,
    /// Use this variant when the contract needs to represent repair needed; selecting it has no side effect by itself.
    RepairNeeded,
}

impl RunStatus {
    /// Reports whether this value is terminal. The check is pure and
    /// does not mutate SDK or host state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// Returns this value as terminal str. The accessor is side-effect
    /// free and keeps ownership with the caller.
    pub fn as_terminal_str(&self) -> Option<&'static str> {
        match self {
            Self::Completed => Some("completed"),
            Self::Failed => Some("failed"),
            Self::Cancelled => Some("cancelled"),
            _ => None,
        }
    }

    /// Constructs this value from terminal str. Use it when adapting
    /// canonical SDK records without introducing a second behavior
    /// path.
    pub fn from_terminal_str(value: &str) -> Option<Self> {
        match value {
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}
