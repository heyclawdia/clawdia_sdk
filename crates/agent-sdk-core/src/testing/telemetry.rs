use std::sync::{Arc, Mutex};

use crate::{
    telemetry_ports::{TelemetrySink, TelemetrySinkAck, TelemetrySinkError, TelemetrySinkSpec},
    telemetry_records::{TelemetryExportCursor, TelemetryProjection},
};

#[derive(Clone)]
pub struct ScriptedTelemetrySink {
    spec: TelemetrySinkSpec,
    fail_next: Arc<Mutex<Option<TelemetrySinkError>>>,
    exports: Arc<Mutex<Vec<TelemetryProjection>>>,
}

impl ScriptedTelemetrySink {
    pub fn new(spec: TelemetrySinkSpec) -> Self {
        Self {
            spec,
            fail_next: Arc::new(Mutex::new(None)),
            exports: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn sink_spec(&self) -> &TelemetrySinkSpec {
        &self.spec
    }

    pub fn fail_next(&self, summary: impl Into<String>) {
        *self
            .fail_next
            .lock()
            .expect("telemetry scripted sink fail lock") =
            Some(TelemetrySinkError::unavailable(summary));
    }

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
