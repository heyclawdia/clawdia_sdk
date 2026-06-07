use agent_sdk_core::{AgentError, AgentErrorKind, RetryClassification};

#[derive(Clone, Debug, Eq, PartialEq)]
/// HTTP request emitted by the Supabase store client.
pub struct SupabaseHttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

impl SupabaseHttpRequest {
    /// Returns a header value by case-insensitive name.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// HTTP response consumed by the Supabase store client.
pub struct SupabaseHttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

impl SupabaseHttpResponse {
    /// Creates an empty HTTP response with the provided status.
    pub fn empty(status: u16) -> Self {
        Self {
            status,
            body: Vec::new(),
        }
    }

    /// Creates a JSON HTTP response.
    pub fn json(status: u16, body: serde_json::Value) -> Self {
        Self {
            status,
            body: serde_json::to_vec(&body).expect("json response serializes"),
        }
    }
}

/// Injectable HTTP transport for Supabase REST calls.
pub trait SupabaseHttpTransport: Send + Sync {
    fn send(&self, request: SupabaseHttpRequest) -> Result<SupabaseHttpResponse, AgentError>;
}

pub(crate) fn supabase_error(message: impl Into<String>) -> AgentError {
    AgentError::new(
        AgentErrorKind::RecoveryRepairNeeded,
        RetryClassification::Retryable,
        message,
    )
}
