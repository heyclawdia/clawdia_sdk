use std::{
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use agent_sdk_core::{AgentError, AgentErrorKind, RetryClassification};

use super::types::{ShellRequest, ShellResult};

pub(super) fn run_command(request: &ShellRequest) -> Result<ShellResult, AgentError> {
    let mut command = Command::new(&request.argv[0]);
    command.args(&request.argv[1..]);
    if let Some(cwd) = &request.cwd {
        command.current_dir(cwd);
    }
    for (key, value) in &request.env {
        command.env(key, value);
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(tool_failure)?;
    let deadline = Instant::now() + Duration::from_millis(request.timeout_ms);
    loop {
        if child.try_wait().map_err(tool_failure)?.is_some() {
            let output = child.wait_with_output().map_err(tool_failure)?;
            return Ok(ShellResult {
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                timed_out: false,
                agent_owned: true,
            });
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let output = child.wait_with_output().map_err(tool_failure)?;
            return Ok(ShellResult {
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                timed_out: true,
                agent_owned: true,
            });
        }
        thread::sleep(Duration::from_millis(5));
    }
}

fn tool_failure(error: std::io::Error) -> AgentError {
    AgentError::new(
        AgentErrorKind::ToolFailure,
        RetryClassification::UserActionNeeded,
        error.to_string(),
    )
}
