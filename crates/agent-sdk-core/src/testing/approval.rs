use std::sync::{Arc, Mutex};

use crate::{
    approval_ports::{ApprovalDispatchRequest, ApprovalDispatchResponse, ApprovalDispatcher},
    domain::AgentError,
};

#[derive(Clone)]
pub struct ScriptedApprovalDispatcher {
    response: ApprovalDispatchResponse,
    requests: Arc<Mutex<Vec<ApprovalDispatchRequest>>>,
}

impl ScriptedApprovalDispatcher {
    pub fn new(response: ApprovalDispatchResponse) -> Self {
        Self {
            response,
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn requests(&self) -> Vec<ApprovalDispatchRequest> {
        self.requests
            .lock()
            .expect("approval dispatcher requests lock")
            .clone()
    }
}

impl ApprovalDispatcher for ScriptedApprovalDispatcher {
    fn dispatch(
        &self,
        request: ApprovalDispatchRequest,
    ) -> Result<ApprovalDispatchResponse, AgentError> {
        self.requests
            .lock()
            .expect("approval dispatcher requests lock")
            .push(request);
        Ok(self.response.clone())
    }
}
