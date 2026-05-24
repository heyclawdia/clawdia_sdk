use std::{
    fs,
    path::{Path, PathBuf},
};

use agent_sdk_core::AgentError;
use serde_json::Value;

pub fn temp_fixture_dir(test_name: &str) -> Result<PathBuf, AgentError> {
    let path = std::env::temp_dir()
        .join("agent-sdk-core-fixtures")
        .join(format!("{test_name}-{}", std::process::id()));
    if path.exists() {
        fs::remove_dir_all(&path).map_err(|error| {
            AgentError::contract_violation(format!("failed to reset temp fixture dir: {error}"))
        })?;
    }
    fs::create_dir_all(&path).map_err(|error| {
        AgentError::contract_violation(format!("failed to create temp fixture dir: {error}"))
    })?;
    Ok(path)
}

pub fn assert_fixture_round_trip(path: &Path, expected: &Value) -> Result<(), AgentError> {
    let actual = agent_sdk_core::testing::read_fixture(path)?;
    assert_eq!(&actual, expected);
    Ok(())
}
