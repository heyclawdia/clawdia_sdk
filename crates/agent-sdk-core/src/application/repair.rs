//! Application-layer coordination over core primitives. Use these services to lower
//! helpers, drive runs, validate output, coordinate tools, approvals, delivery,
//! isolation, telemetry, and feature layers. Methods in this layer may call
//! configured ports, mutate in-memory stores, append journals, or publish events as
//! documented. This file contains the repair portion of that contract.
//!
use crate as sdk;

use serde::{Deserialize, Serialize};

use crate::validation::{
    JsonSchemaSubsetValidator, OutputCandidate, StructuredOutputValidator,
    TerminalValidationFailure, ValidationErrorReport, ValidationSuccess,
};
use sdk::{
    AgentError, CandidateContentRepairPolicy, OutputContract, RepairAttemptId, RetryBudget,
    ValidationAttemptId,
    structured_output::{
        REPAIR_RECORD_SCHEMA_VERSION, RepairExhaustionRecord, RepairPrompt,
        RepairPromptCandidateContent, RepairRecord, RepairRecordKind, ValidationRecord,
    },
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
/// Holds repair policy controller application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct RepairPolicyController;

impl RepairPolicyController {
    /// Builds the next attempt value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn next_attempt(
        &self,
        contract: &OutputContract,
        report: &ValidationErrorReport,
        accounting: &RepairAccounting,
    ) -> RepairDecision {
        if report.schema_rejected {
            let failure = TerminalValidationFailure::from_reports(
                core::slice::from_ref(report),
                accounting.repair_attempts.clone(),
                false,
            );
            return RepairDecision::Exhausted {
                failure: failure.clone(),
                record: repair_exhaustion_record_from_failure(
                    &failure,
                    "schema rejected before repair",
                ),
            };
        }

        let max_attempts = effective_repair_attempt_limit(contract);
        if accounting.repair_attempts.len() >= usize::from(max_attempts) {
            let failure = TerminalValidationFailure::from_reports(
                core::slice::from_ref(report),
                accounting.repair_attempts.clone(),
                true,
            );
            return RepairDecision::Exhausted {
                failure: failure.clone(),
                record: repair_exhaustion_record_from_failure(
                    &failure,
                    "repair attempt budget exhausted",
                ),
            };
        }

        let attempt_index = accounting.repair_attempts.len() as u8 + 1;
        let repair_attempt_id = RepairAttemptId::new(format!(
            "repair.{}.{}",
            report.validation_attempt_id.as_str(),
            attempt_index
        ));
        let prompt = repair_prompt(
            contract,
            report,
            repair_attempt_id.clone(),
            attempt_index,
            max_attempts,
        );
        let record = repair_record_requested(contract, report, &prompt);

        RepairDecision::Attempt { prompt, record }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
/// Holds repair accounting application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct RepairAccounting {
    /// Attempt identifier or attempt history for bounded retry/repair.
    /// Use it to preserve ordering and avoid retry loops that cannot be audited.
    pub repair_attempts: Vec<RepairAttemptId>,
}

impl RepairAccounting {
    /// Sets record attempt on the value and returns it.
    /// This mutates the in-memory repair tracker by recording the attempt id.
    pub fn record_attempt(&mut self, repair_attempt_id: RepairAttemptId) {
        self.repair_attempts.push(repair_attempt_id);
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Enumerates the finite repair decision cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RepairDecision {
    /// Use this variant when the contract needs to represent attempt; selecting it has no side effect by itself.
    Attempt {
        /// Prompt used by this record or request.
        prompt: RepairPrompt,
        /// Record used by this record or request.
        record: RepairRecord,
    },
    /// Use this variant when the contract needs to represent exhausted; selecting it has no side effect by itself.
    Exhausted {
        /// Failure used by this record or request.
        failure: TerminalValidationFailure,
        /// Record used by this record or request.
        record: RepairExhaustionRecord,
    },
}

fn repair_prompt(
    contract: &OutputContract,
    report: &ValidationErrorReport,
    repair_attempt_id: RepairAttemptId,
    attempt_index: u8,
    max_repair_attempts: u8,
) -> RepairPrompt {
    RepairPrompt {
        record_schema_version: REPAIR_RECORD_SCHEMA_VERSION,
        repair_attempt_id,
        validation_attempt_id: report.validation_attempt_id.clone(),
        source_attempt_id: report.source_attempt_id.clone(),
        schema_id: contract.schema_id.clone(),
        output_schema_version: contract.schema_version,
        schema_fingerprint: contract.schema_fingerprint(),
        repair_adapter_ref: contract.repair.repair_adapter_ref.clone(),
        attempt_index,
        max_repair_attempts,
        include_schema_in_prompt: contract.repair.include_schema_in_prompt,
        redacted_errors: if contract.repair.include_redacted_errors {
            report.errors.clone()
        } else {
            Vec::new()
        },
        candidate_content: repair_prompt_candidate_content_from_policy(contract, report),
        prompt_summary: format!(
            "repair structured output for schema {} using redacted validation errors",
            contract.schema_id.as_str()
        ),
    }
}

fn repair_prompt_candidate_content_from_policy(
    contract: &OutputContract,
    report: &ValidationErrorReport,
) -> RepairPromptCandidateContent {
    match contract.repair.include_candidate_content {
        CandidateContentRepairPolicy::ContentRefOnly => {
            RepairPromptCandidateContent::ContentRefOnly {
                candidate_content_ref: report.candidate_content_ref.clone(),
            }
        }
        CandidateContentRepairPolicy::RedactedCandidate => {
            RepairPromptCandidateContent::RedactedCandidate {
                redacted_summary: "candidate content redacted by repair policy".to_string(),
            }
        }
        CandidateContentRepairPolicy::OmitCandidate => RepairPromptCandidateContent::Omitted,
    }
}

fn repair_record_requested(
    contract: &OutputContract,
    report: &ValidationErrorReport,
    prompt: &RepairPrompt,
) -> RepairRecord {
    RepairRecord {
        record_schema_version: REPAIR_RECORD_SCHEMA_VERSION,
        record_kind: RepairRecordKind::RepairRequested,
        repair_attempt_id: prompt.repair_attempt_id.clone(),
        validation_attempt_id: report.validation_attempt_id.clone(),
        source_attempt_id: report.source_attempt_id.clone(),
        schema_id: contract.schema_id.clone(),
        output_schema_version: contract.schema_version,
        schema_fingerprint: contract.schema_fingerprint(),
        repair_adapter_ref: contract.repair.repair_adapter_ref.clone(),
        attempt_index: prompt.attempt_index,
        max_repair_attempts: prompt.max_repair_attempts,
        prompt: prompt.clone(),
        redacted_summary: format!(
            "repair attempt {} requested after validation failure",
            prompt.attempt_index
        ),
        privacy: report.privacy,
    }
}

/// Builds the repair exhaustion record from failure record for this contract.
/// This derives recovery or repair data from the supplied failure state and does not perform
/// the repair by itself.
pub(crate) fn repair_exhaustion_record_from_failure(
    failure: &TerminalValidationFailure,
    reason: impl Into<String>,
) -> RepairExhaustionRecord {
    RepairExhaustionRecord {
        record_schema_version: REPAIR_RECORD_SCHEMA_VERSION,
        record_kind: RepairRecordKind::RepairExhausted,
        schema_id: failure.schema_id.clone(),
        output_schema_version: failure.schema_version,
        validation_attempts: failure.validation_attempts.clone(),
        repair_attempts: failure.repair_attempts.clone(),
        source_attempt_ids: failure.source_attempt_ids.clone(),
        candidate_content_ref: failure.candidate_content_ref.clone(),
        retry_exhausted: failure.retry_exhausted,
        redacted_summary: failure.redacted_error_summary.clone(),
        reason: reason.into(),
        privacy: failure.privacy,
    }
}

#[derive(Clone, Debug)]
/// Holds local validation repair service application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct LocalValidationRepairService<V = JsonSchemaSubsetValidator> {
    validator: V,
    repair_controller: RepairPolicyController,
}

impl LocalValidationRepairService<JsonSchemaSubsetValidator> {
    /// Builds the default json schema subset value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn default_json_schema_subset() -> Self {
        Self::new(JsonSchemaSubsetValidator::default())
    }
}

impl<V> LocalValidationRepairService<V>
where
    V: StructuredOutputValidator,
{
    /// Creates a new application::repair value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(validator: V) -> Self {
        Self {
            validator,
            repair_controller: RepairPolicyController,
        }
    }

    /// Validates the application::repair invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
    pub fn validate_candidates(
        &self,
        contract: &OutputContract,
        candidates: impl IntoIterator<Item = OutputCandidate>,
    ) -> Result<ValidationRepairOutcome, AgentError> {
        let mut candidates = candidates.into_iter().peekable();
        let mut validation_records = Vec::new();
        let mut repair_records = Vec::new();
        let mut reports = Vec::new();
        let mut accounting = RepairAccounting::default();
        while let Some(candidate) = candidates.next() {
            let validation_attempt_id = ValidationAttemptId::new(format!(
                "validation.{}",
                candidate.source_attempt_id.as_str()
            ));
            match self
                .validator
                .validate_candidate(contract, validation_attempt_id, &candidate)
            {
                Ok(success) => {
                    validation_records.push(success.record.clone());
                    return Ok(ValidationRepairOutcome::Validated {
                        success,
                        validation_records,
                        repair_records,
                        repair_attempts: accounting.repair_attempts,
                    });
                }
                Err(report) => {
                    validation_records.push(report.record.clone());
                    reports.push(report.clone());

                    match self
                        .repair_controller
                        .next_attempt(contract, &report, &accounting)
                    {
                        RepairDecision::Attempt { prompt, record } => {
                            accounting.record_attempt(prompt.repair_attempt_id.clone());
                            repair_records.push(record.clone());
                            if candidates.peek().is_none() {
                                return Ok(ValidationRepairOutcome::RepairRequested {
                                    latest_report: report,
                                    prompt,
                                    validation_records,
                                    repair_records,
                                });
                            }
                        }
                        RepairDecision::Exhausted { .. } => {
                            let retry_exhausted = !report.schema_rejected;
                            let failure = TerminalValidationFailure::from_reports(
                                &reports,
                                accounting.repair_attempts.clone(),
                                retry_exhausted,
                            );
                            let reason = if report.schema_rejected {
                                "schema rejected before repair"
                            } else {
                                "repair attempt budget exhausted"
                            };
                            let record = repair_exhaustion_record_from_failure(&failure, reason);
                            validation_records.push(failure.record.clone());
                            return Ok(ValidationRepairOutcome::Failed {
                                failure,
                                validation_records,
                                repair_records,
                                exhaustion_record: record,
                            });
                        }
                    }
                }
            }
        }

        Err(AgentError::missing_required_field(
            "structured_output.candidates",
        ))
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Enumerates the finite validation repair outcome cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ValidationRepairOutcome {
    /// Use this variant when the contract needs to represent validated; selecting it has no side effect by itself.
    Validated {
        /// Success used by this record or request.
        success: ValidationSuccess,
        /// Validation policy applied before output is accepted as typed data.
        /// It controls validator selection, bounds, failure visibility, and local validation
        /// behavior.
        validation_records: Vec<ValidationRecord>,
        /// Repair policy used after structured output validation fails.
        /// It controls whether repair is attempted and which policy gates must approve it.
        repair_records: Vec<RepairRecord>,
        /// Attempt identifier or attempt history for bounded retry/repair.
        /// Use it to preserve ordering and avoid retry loops that cannot be audited.
        repair_attempts: Vec<RepairAttemptId>,
    },
    /// Use this variant when the contract needs to represent repair requested; selecting it has no side effect by itself.
    RepairRequested {
        /// Latest report used by this record or request.
        latest_report: ValidationErrorReport,
        /// Prompt used by this record or request.
        prompt: RepairPrompt,
        /// Validation policy applied before output is accepted as typed data.
        /// It controls validator selection, bounds, failure visibility, and local validation
        /// behavior.
        validation_records: Vec<ValidationRecord>,
        /// Repair policy used after structured output validation fails.
        /// It controls whether repair is attempted and which policy gates must approve it.
        repair_records: Vec<RepairRecord>,
    },
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed {
        /// Failure used by this record or request.
        failure: TerminalValidationFailure,
        /// Validation policy applied before output is accepted as typed data.
        /// It controls validator selection, bounds, failure visibility, and local validation
        /// behavior.
        validation_records: Vec<ValidationRecord>,
        /// Repair policy used after structured output validation fails.
        /// It controls whether repair is attempted and which policy gates must approve it.
        repair_records: Vec<RepairRecord>,
        /// Exhaustion record used by this record or request.
        exhaustion_record: RepairExhaustionRecord,
    },
}

fn effective_repair_attempt_limit(contract: &OutputContract) -> u8 {
    let RetryBudget { max_attempts } = contract.retry_budget;
    contract.repair.max_repair_attempts.min(max_attempts)
}
