//! Deterministic test-kit helpers for SDK consumers. Use these fakes and harnesses to
//! exercise public contracts without live providers, real stores, product UI, network
//! telemetry, or wall-clock-dependent infrastructure. They mutate only their
//! in-memory state unless noted. This file contains the event portion of that
//! contract.
//!
use std::collections::VecDeque;

use serde::{Serialize, de::DeserializeOwned};

use crate::{
    domain::AgentError,
    event::{CompiledEventFilter, EventFrame},
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
/// In-memory fake event conformance harness fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct FakeEventConformanceHarness {
    frames: VecDeque<EventFrame>,
}

impl FakeEventConformanceHarness {
    /// Operates on in-memory or journal-derived testing::event state for
    /// diagnostics and repair evidence. It does not create a second run loop
    /// or product workflow owner.
    pub fn push(&mut self, frame: EventFrame) {
        self.frames.push_back(frame);
    }

    /// Returns the matching frames currently held by this value.
    /// This reads deterministic in-memory test state and performs no external I/O.
    pub fn matching_frames(&self, filter: &CompiledEventFilter) -> Vec<EventFrame> {
        self.frames
            .iter()
            .filter(|frame| filter.matches_envelope(&frame.event.envelope))
            .cloned()
            .collect()
    }

    /// Assert fixture round trip.
    /// This reads or mutates deterministic in-memory test state unless the method explicitly
    /// names a fixture file.
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
