use std::{
    fmt, fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use agent_sdk_core::{AgentError, RetryClassification};
use serde_json::Value;

use crate::error::{
    bounded_body_summary, host_configuration_needed, http_status_failure, provider_failure,
};

#[derive(Clone, PartialEq)]
/// JSON HTTP request emitted by a live provider adapter.
///
/// The request body may contain provider-visible prompt material, so callers
/// should treat captured values as test-only or privacy-governed diagnostics.
pub struct JsonHttpRequest {
    /// Absolute provider endpoint URL.
    pub url: String,
    /// HTTP headers for the request. Secret-bearing headers should not be
    /// persisted in fixtures or public diagnostics.
    pub headers: Vec<(String, String)>,
    /// JSON request body sent to the provider.
    pub body: Value,
}

impl JsonHttpRequest {
    /// Creates a new JSON HTTP request.
    pub fn new(url: impl Into<String>, body: Value) -> Self {
        Self {
            url: url.into(),
            headers: Vec::new(),
            body,
        }
    }

    /// Adds a header to this request.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

impl fmt::Debug for JsonHttpRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("JsonHttpRequest")
            .field("url", &self.url)
            .field(
                "headers",
                &self
                    .headers
                    .iter()
                    .map(|(name, _)| format!("{name}: <redacted>"))
                    .collect::<Vec<_>>(),
            )
            .field("body", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, PartialEq)]
/// JSON HTTP response returned by a provider transport.
pub struct JsonHttpResponse {
    /// HTTP status code observed from the provider.
    pub status: u16,
    /// Decoded JSON response body.
    pub body: Value,
}

impl fmt::Debug for JsonHttpResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("JsonHttpResponse")
            .field("status", &self.status)
            .field("body", &"<redacted>")
            .finish()
    }
}

/// Transport boundary used by live provider adapters.
///
/// Production hosts can use `CurlJsonHttpTransport`; tests should inject a
/// deterministic implementation that captures requests and returns fixtures.
pub trait JsonHttpTransport: Send + Sync {
    /// Sends one JSON POST request.
    fn post_json(&self, request: JsonHttpRequest) -> Result<JsonHttpResponse, AgentError>;
}

#[derive(Clone, Debug, Default)]
/// Blocking JSON HTTP transport backed by the system `curl` executable.
///
/// This keeps the provider crate usable without adding an async runtime or
/// heavyweight HTTP dependency. Hosts that need a different HTTP stack can
/// inject their own `JsonHttpTransport`.
pub struct CurlJsonHttpTransport;

impl CurlJsonHttpTransport {
    /// Creates a blocking JSON HTTP transport.
    pub fn new() -> Self {
        Self
    }
}

impl JsonHttpTransport for CurlJsonHttpTransport {
    fn post_json(&self, request: JsonHttpRequest) -> Result<JsonHttpResponse, AgentError> {
        let mut command = Command::new("curl");
        command
            .arg("--silent")
            .arg("--show-error")
            .arg("--request")
            .arg("POST")
            .arg("--data-binary")
            .arg("@-")
            .arg("--write-out")
            .arg("\n%{http_code}")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let header_pipe = HeaderPipe::new(&request.headers)?;
        if let Some(pipe) = &header_pipe {
            command
                .arg("--header")
                .arg(format!("@{}", pipe.path.display()));
        }
        command.arg(&request.url);

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                if let Some(pipe) = header_pipe {
                    pipe.cleanup_best_effort();
                }
                return Err(host_configuration_needed(format!(
                    "provider HTTP transport requires curl on PATH: {error}"
                )));
            }
        };
        let header_writer = header_pipe.map(HeaderPipe::spawn_writer);
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(request.body.to_string().as_bytes())
                .map_err(|error| {
                    provider_failure(
                        RetryClassification::Retryable,
                        format!("provider HTTP request body could not be written: {error}"),
                    )
                })?;
        }
        let output = child.wait_with_output().map_err(|error| {
            provider_failure(
                RetryClassification::Retryable,
                format!("provider HTTP transport failed: {error}"),
            )
        })?;
        if let Some(writer) = header_writer {
            writer.join().map_err(|_| {
                provider_failure(
                    RetryClassification::RepairNeeded,
                    "provider HTTP header writer panicked",
                )
            })??;
        }
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if !output.status.success() && stdout.trim().is_empty() {
            return Err(provider_failure(
                RetryClassification::Retryable,
                format!(
                    "provider HTTP transport failed: {}",
                    bounded_body_summary(&stderr)
                ),
            ));
        }
        let (body, status) = split_curl_status(&stdout)?;
        if !(200..=299).contains(&status) {
            return Err(http_status_failure("HTTP", status, body));
        }
        decode_response(status, body)
    }
}

fn split_curl_status(stdout: &str) -> Result<(&str, u16), AgentError> {
    let Some((body, status_text)) = stdout.rsplit_once('\n') else {
        return Err(provider_failure(
            RetryClassification::RepairNeeded,
            "provider HTTP response did not include a curl status trailer",
        ));
    };
    let status = status_text.trim().parse::<u16>().map_err(|error| {
        provider_failure(
            RetryClassification::RepairNeeded,
            format!("provider HTTP status trailer was invalid: {error}"),
        )
    })?;
    if status == 0 {
        return Err(provider_failure(
            RetryClassification::Retryable,
            format!(
                "provider HTTP transport returned status 000: {}",
                bounded_body_summary(body)
            ),
        ));
    }
    Ok((body, status))
}

fn decode_response(status: u16, body_text: &str) -> Result<JsonHttpResponse, AgentError> {
    let body = if body_text.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(body_text).map_err(|error| {
            provider_failure(
                RetryClassification::RepairNeeded,
                format!(
                    "provider HTTP response was not valid JSON: {error}; body: {}",
                    bounded_body_summary(body_text)
                ),
            )
        })?
    };
    Ok(JsonHttpResponse { status, body })
}

struct HeaderPipe {
    dir: PathBuf,
    path: PathBuf,
    contents: String,
}

impl HeaderPipe {
    fn new(headers: &[(String, String)]) -> Result<Option<Self>, AgentError> {
        if headers.is_empty() {
            return Ok(None);
        }

        let mut contents = String::new();
        for (name, value) in headers {
            validate_header(name, value)?;
            contents.push_str(name);
            contents.push_str(": ");
            contents.push_str(value);
            contents.push('\n');
        }

        let dir = std::env::temp_dir().join(format!(
            "agent-sdk-provider-curl-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir(&dir).map_err(|error| {
            provider_failure(
                RetryClassification::RepairNeeded,
                format!("provider HTTP header pipe directory could not be created: {error}"),
            )
        })?;
        lock_down_private_dir(&dir)?;
        let path = dir.join("headers");
        let status = Command::new("mkfifo").arg(&path).status().map_err(|error| {
            let _ = fs::remove_dir(&dir);
            host_configuration_needed(format!(
                "provider HTTP transport requires mkfifo on PATH to avoid secret-bearing curl argv: {error}"
            ))
        })?;
        if !status.success() {
            let _ = fs::remove_dir(&dir);
            return Err(host_configuration_needed(
                "provider HTTP transport could not create a private header pipe with mkfifo",
            ));
        }

        Ok(Some(Self {
            dir,
            path,
            contents,
        }))
    }

    fn spawn_writer(self) -> thread::JoinHandle<Result<(), AgentError>> {
        thread::spawn(move || {
            let result = fs::OpenOptions::new()
                .write(true)
                .open(&self.path)
                .and_then(|mut file| file.write_all(self.contents.as_bytes()));
            self.cleanup_best_effort();
            result.map_err(|error| {
                provider_failure(
                    RetryClassification::Retryable,
                    format!("provider HTTP header pipe write failed: {error}"),
                )
            })
        })
    }

    fn cleanup_best_effort(self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_dir(&self.dir);
    }
}

fn validate_header(name: &str, value: &str) -> Result<(), AgentError> {
    if name.is_empty()
        || name
            .bytes()
            .any(|byte| matches!(byte, b':' | b'\r' | b'\n'))
    {
        return Err(provider_failure(
            RetryClassification::RepairNeeded,
            "provider HTTP header name is invalid",
        ));
    }
    if value.bytes().any(|byte| matches!(byte, b'\r' | b'\n')) {
        return Err(provider_failure(
            RetryClassification::RepairNeeded,
            "provider HTTP header value contains a newline",
        ));
    }
    Ok(())
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

#[cfg(unix)]
fn lock_down_private_dir(dir: &std::path::Path) -> Result<(), AgentError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).map_err(|error| {
        provider_failure(
            RetryClassification::RepairNeeded,
            format!("provider HTTP header pipe directory permissions could not be set: {error}"),
        )
    })
}

#[cfg(not(unix))]
fn lock_down_private_dir(_dir: &std::path::Path) -> Result<(), AgentError> {
    Ok(())
}
