use crate::{
    approval_records::{ApprovalDecision, ApprovalRequest},
    domain::AgentError,
};

pub type ApprovalDispatchRequest = ApprovalRequest;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApprovalDispatchResponse {
    Decision(ApprovalDecision),
    TimedOut,
    Cancelled,
    Unavailable { reason_code: String },
}

impl ApprovalDispatchResponse {
    pub fn decision(decision: ApprovalDecision) -> Self {
        Self::Decision(decision)
    }

    pub fn unavailable(reason_code: impl Into<String>) -> Self {
        Self::Unavailable {
            reason_code: reason_code.into(),
        }
    }
}

pub trait ApprovalDispatcher: Send + Sync {
    fn dispatch(
        &self,
        request: ApprovalDispatchRequest,
    ) -> Result<ApprovalDispatchResponse, AgentError>;
}
