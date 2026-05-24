//! Downstream-facing test kit for SDK consumers.
//!
//! Fakes and scripted adapters in this namespace exercise the same public
//! ports, records, and package contracts as production implementations.

pub mod fakes {
    pub use crate::fakes::*;
}

pub mod approval {
    pub use crate::approval_testing::*;
}

pub mod content {
    pub use crate::content_testing::*;
}

pub mod event {
    pub use crate::event_testing::*;
}

pub mod extension {
    pub use crate::extension_testing::*;
}

pub mod hooks {
    pub use crate::hooks_testing::*;
}

pub mod isolation {
    pub use crate::isolation_testing::*;
}

pub mod output_delivery {
    pub use crate::output_delivery_testing::*;
}

pub mod realtime {
    pub use crate::realtime_testing::*;
}

pub mod telemetry {
    pub use crate::telemetry_testing::*;
}

pub mod tool {
    pub use crate::tool_testing::*;
}

pub use crate::approval_testing::ScriptedApprovalDispatcher;
pub use crate::content_testing::FakeContentResolver;
pub use crate::event_testing::FakeEventConformanceHarness;
pub use crate::extension_testing::ScriptedExtensionActionExecutor;
pub use crate::fakes::{
    DeterministicClock, DeterministicIdGenerator, FIXTURE_SCHEMA_VERSION, FakeContentStore,
    FakeEventSink, FakeFixtureHarness, FakeJournalStore, FakeProvider, FixtureManifest,
    FixtureManifestEntry, StoredContent, StoredContentManifestEntry, normalize_json_value,
    read_fixture, write_fixture,
};
pub use crate::hooks_testing::ScriptedHookExecutor;
pub use crate::isolation_testing::FakeIsolationRuntime;
pub use crate::output_delivery_testing::ScriptedOutputSink;
pub use crate::realtime_testing::ScriptedRealtimeAdapter;
pub use crate::telemetry_testing::ScriptedTelemetrySink;
pub use crate::tool_testing::ScriptedToolExecutor;
