//! Deterministic test-kit helpers for SDK consumers. Use these fakes and harnesses to
//! exercise public contracts without live providers, real stores, product UI, network
//! telemetry, or wall-clock-dependent infrastructure. They mutate only their
//! in-memory state unless noted. This file contains the telemetry portion of that
//! contract.
//!
use std::sync::{Arc, Mutex};

use crate::{
    telemetry_ports::{TelemetrySink, TelemetrySinkAck, TelemetrySinkError, TelemetrySinkSpec},
    telemetry_records::{TelemetryExportCursor, TelemetryProjection},
};

#[derive(Clone)]
/// In-memory scripted telemetry sink fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct ScriptedTelemetrySink {
    spec: TelemetrySinkSpec,
    fail_next: Arc<Mutex<Option<TelemetrySinkError>>>,
    exports: Arc<Mutex<Vec<TelemetryProjection>>>,
}

impl ScriptedTelemetrySink {
    /// Creates a new testing::telemetry value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(spec: TelemetrySinkSpec) -> Self {
        Self {
            spec,
            fail_next: Arc::new(Mutex::new(None)),
            exports: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Returns the sink spec currently held by this value.
    /// This configures deterministic in-memory test state only.
    pub fn sink_spec(&self) -> &TelemetrySinkSpec {
        &self.spec
    }

    /// Builds the fail next value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn fail_next(&self, summary: impl Into<String>) {
        *self
            .fail_next
            .lock()
            .expect("telemetry scripted sink fail lock") =
            Some(TelemetrySinkError::unavailable(summary));
    }

    /// Returns the exports currently held by this value.
    /// This configures deterministic in-memory test state only.
    pub fn exports(&self) -> Vec<TelemetryProjection> {
        self.exports
            .lock()
            .expect("telemetry scripted sink exports lock")
            .clone()
    }
}

impl TelemetrySink for ScriptedTelemetrySink {
    fn spec(&self) -> &TelemetrySinkSpec {
        &self.spec
    }

    fn export(
        &self,
        projection: &TelemetryProjection,
        cursor: &TelemetryExportCursor,
    ) -> Result<TelemetrySinkAck, TelemetrySinkError> {
        if let Some(error) = self
            .fail_next
            .lock()
            .expect("telemetry scripted sink fail lock")
            .take()
        {
            return Err(error);
        }
        self.exports
            .lock()
            .expect("telemetry scripted sink exports lock")
            .push(projection.clone());
        Ok(TelemetrySinkAck::accepted(cursor.clone().acknowledged(
            projection.source_record.source_cursor.clone(),
        )))
    }
}
