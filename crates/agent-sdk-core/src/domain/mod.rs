//! Domain primitives for stable SDK vocabulary. Use these items for IDs, refs,
//! policy, privacy, trust, and errors that cross crate or host boundaries. They are
//! data-only and must not perform provider, filesystem, network, or UI side effects.
//!
pub(crate) mod json;

pub use crate::effect::{EffectIntent, EffectKind, EffectResult, EffectTerminalStatus};
pub use crate::error::{AgentError, AgentErrorKind, RetryClassification};
pub use crate::ids::IdValidationError;
pub use crate::ids::{
    AgentId, AgentPoolId, ApprovalRequestId, ArchiveCursorId, ArtifactId, ArtifactRef, AttemptId,
    ContentId, ContentRef, ContextItemId, ContextProjectionId, CorrelationEntry, CorrelationKey,
    CorrelationValue, DedupeKey, EffectId, EventCursorId, EventId, IdempotencyKey, JournalCursor,
    JournalCursorId, LineageId, MAX_ID_LEN, MessageId, OutputSchemaId, RepairAttemptId, RunId,
    RuntimePackageId, SessionId, SpanId, ToolCallId, TopicId, TraceId, TurnId, ValidatedOutputId,
    ValidationAttemptId, WakeConditionId,
};
pub use crate::privacy::{PrivacyClass, RetentionClass, TrustClass};
pub use crate::refs::{
    AdapterRef, DestinationId, DestinationKind, DestinationRef, EntityId, EntityKind, EntityRef,
    LineageRef, PolicyId, PolicyKind, PolicyRef, SourceId, SourceKind, SourceRef,
};
