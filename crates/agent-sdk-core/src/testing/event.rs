use std::collections::VecDeque;

use serde::{Serialize, de::DeserializeOwned};

use crate::{
    domain::AgentError,
    event::{CompiledEventFilter, EventFrame},
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FakeEventConformanceHarness {
    frames: VecDeque<EventFrame>,
}

impl FakeEventConformanceHarness {
    pub fn push(&mut self, frame: EventFrame) {
        self.frames.push_back(frame);
    }

    pub fn matching_frames(&self, filter: &CompiledEventFilter) -> Vec<EventFrame> {
        self.frames
            .iter()
            .filter(|frame| filter.matches_envelope(&frame.event.envelope))
            .cloned()
            .collect()
    }

    pub fn assert_fixture_round_trip<T>(value: &T, fixture: &str) -> Result<(), AgentError>
    where
        T: Serialize + DeserializeOwned + PartialEq + core::fmt::Debug,
    {
        let expected: T = serde_json::from_str(fixture)
            .map_err(|error| AgentError::contract_violation(error.to_string()))?;
        let actual = serde_json::to_string_pretty(value)
            .and_then(|encoded| serde_json::from_str::<T>(&encoded))
            .map_err(|error| AgentError::contract_violation(error.to_string()))?;
        if actual != expected {
            return Err(AgentError::contract_violation(
                "event fixture round trip mismatch",
            ));
        }
        Ok(())
    }
}
