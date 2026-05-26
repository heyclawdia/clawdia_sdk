use agent_sdk_core::{AgentError, AgentErrorKind, RetryClassification};

pub(crate) fn provider_failure(
    retry: RetryClassification,
    message: impl Into<String>,
) -> AgentError {
    AgentError::new(AgentErrorKind::ProviderFailure, retry, message)
}

pub(crate) fn host_configuration_needed(message: impl Into<String>) -> AgentError {
    AgentError::host_configuration_needed(message)
}

pub(crate) fn http_status_failure(provider: &str, status: u16, body: &str) -> AgentError {
    let retry = match status {
        408 | 409 | 429 | 500..=599 => RetryClassification::Retryable,
        401 | 403 => RetryClassification::HostConfigurationNeeded,
        _ => RetryClassification::RepairNeeded,
    };
    provider_failure(
        retry,
        format!(
            "{provider} provider request failed with HTTP status {status}: {}",
            bounded_body_summary(body)
        ),
    )
}

pub(crate) fn unsupported_response(provider: &str, message: impl Into<String>) -> AgentError {
    provider_failure(
        RetryClassification::RepairNeeded,
        format!(
            "{provider} provider returned an unsupported response shape: {}",
            message.into()
        ),
    )
}

pub(crate) fn bounded_body_summary(body: &str) -> String {
    const MAX_SUMMARY_CHARS: usize = 512;
    let summary = body.trim();
    if summary.is_empty() {
        return "<empty body>".to_string();
    }
    let mut chars = summary.chars();
    let bounded = chars.by_ref().take(MAX_SUMMARY_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{bounded}...")
    } else {
        bounded
    }
}
