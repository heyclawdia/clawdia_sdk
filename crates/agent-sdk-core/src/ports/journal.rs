use crate::{
    domain::{AgentError, AgentErrorKind, EntityKind, EntityRef, RetryClassification},
    journal::{JournalCursor, JournalRecord},
};

pub trait RunJournal: Send + Sync {
    fn append(&self, record: JournalRecord) -> Result<JournalCursor, AgentError>;
}

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
