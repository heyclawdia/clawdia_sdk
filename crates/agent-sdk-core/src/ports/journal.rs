//! Journal append port helpers. Use this module to persist intent-before-effect and
//! terminal result records around fallible work. Implementations mutate durable
//! storage and should be idempotent where replay requires it.
//!
use crate::{
    domain::{AgentError, AgentErrorKind, EntityKind, EntityRef, RetryClassification, RunId},
    journal::{JournalCursor, JournalRecord},
};

/// Port or behavior contract for run journal. Implementors should
/// preserve policy, redaction, idempotency, and replay expectations
/// from the surrounding module. Implementations may perform side
/// effects only as described by the trait methods.
pub trait RunJournal: Send + Sync {
    /// Appends one durable journal record and returns its cursor.
    /// Implementations append one durable journal record and return its cursor; they must not
    /// execute the effect described by the record.
    fn append(&self, record: JournalRecord) -> Result<JournalCursor, AgentError>;
}

/// Explicit read-side contract for durable run journals.
///
/// This is intentionally separate from `RunJournal` so append-only behavior
/// remains clear at execution seams while reports, resume helpers, and store
/// adapters can consume durable evidence.
pub trait RunJournalReader: Send + Sync {
    /// Returns records for one run ordered by journal sequence.
    fn records_for_run(&self, run_id: &RunId) -> Result<Vec<JournalRecord>, AgentError>;

    /// Returns records for one run after a journal sequence.
    fn records_after(
        &self,
        run_id: &RunId,
        after_journal_seq: u64,
    ) -> Result<Vec<JournalRecord>, AgentError> {
        Ok(self
            .records_for_run(run_id)?
            .into_iter()
            .filter(|record| record.journal_seq > after_journal_seq)
            .collect())
    }
}

/// Convenience trait for stores that support both append and read contracts.
pub trait RunJournalStore: RunJournal + RunJournalReader {}

impl<T> RunJournalStore for T where T: RunJournal + RunJournalReader {}

/// Appends an intent record before executing the supplied closure.
/// If the append fails, the closure is not called; if it succeeds, the
/// returned cursor proves the effect was journal-gated.
pub fn append_before_effect<J, F, T>(
    journal: &J,
    intent_record: JournalRecord,
    execute: F,
) -> Result<(JournalCursor, T), AgentError>
where
    J: RunJournal,
    F: FnOnce() -> T,
{
    let cursor = journal.append(intent_record).map_err(journal_failure)?;
    Ok((cursor, execute()))
}

/// Appends a terminal result record, falling back to a recovery record on
/// result-append failure.
/// This only writes through the journal port and never re-executes the
/// side effect whose result is being recorded.
pub fn append_result_or_recovery<J>(
    journal: &J,
    result_record: JournalRecord,
    recovery_record: JournalRecord,
) -> Result<JournalCursor, AgentError>
where
    J: RunJournal,
{
    match journal.append(result_record) {
        Ok(cursor) => Ok(cursor),
        Err(result_error) => match journal.append(recovery_record) {
            Ok(cursor) => Ok(cursor),
            Err(recovery_error) => Err(journal_failure(recovery_error).with_subject(
                EntityRef::new(EntityKind::EffectResult, "effect.result.append"),
            )),
        }
        .map_err(|error| {
            let mut context = error.context();
            context.redacted_summary = Some(format!(
                "terminal result append failed first: {}",
                result_error.context().message
            ));
            AgentError::new(
                AgentErrorKind::RecoveryRepairNeeded,
                RetryClassification::RepairNeeded,
                context.message,
            )
        }),
    }
}

fn journal_failure(error: AgentError) -> AgentError {
    AgentError::new(
        AgentErrorKind::JournalFailure,
        RetryClassification::RepairNeeded,
        error.context().message,
    )
}
