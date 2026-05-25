//! Downstream-facing test kit for SDK consumers.
//!
//! Fakes and scripted adapters in this namespace exercise the same public
//! ports, records, and package contracts as production implementations.

/// Compatibility facade for deterministic fake adapters. Prefer the
/// concrete modules below when a test needs a narrower fake surface.
pub mod fakes {
    pub use crate::fakes::*;
}

/// Public approval namespace. Use it for the documented approval API
/// surface; prefer crate-root re-exports for common imports. Module
/// items must preserve the core ownership and side-effect boundaries
/// described in this file.
pub mod approval {
    pub use crate::approval_testing::*;
}

/// Public content namespace. Use it for the documented content API
/// surface; prefer crate-root re-exports for common imports. Module
/// items must preserve the core ownership and side-effect boundaries
/// described in this file.
pub mod content {
    pub use crate::content_testing::*;
}

/// Public event namespace. Use it for the documented event API surface;
/// prefer crate-root re-exports for common imports. Module items must
/// preserve the core ownership and side-effect boundaries described in
/// this file.
pub mod event {
    pub use crate::event_testing::*;
}

/// Public extension namespace. Use it for the documented extension API
/// surface; prefer crate-root re-exports for common imports. Module
/// items must preserve the core ownership and side-effect boundaries
/// described in this file.
pub mod extension {
    pub use crate::extension_testing::*;
}

/// Public hooks namespace. Use it for the documented hooks API surface;
/// prefer crate-root re-exports for common imports. Module items must
/// preserve the core ownership and side-effect boundaries described in
/// this file.
pub mod hooks {
    pub use crate::hooks_testing::*;
}

/// Public isolation namespace. Use it for the documented isolation API
/// surface; prefer crate-root re-exports for common imports. Module
/// items must preserve the core ownership and side-effect boundaries
/// described in this file.
pub mod isolation {
    pub use crate::isolation_testing::*;
}

/// Public output delivery namespace. Use it for the documented output
/// delivery API surface; prefer crate-root re-exports for common
/// imports. Module items must preserve the core ownership and
/// side-effect boundaries described in this file.
pub mod output_delivery {
    pub use crate::output_delivery_testing::*;
}

/// Public realtime namespace. Use it for the documented realtime API
/// surface; prefer crate-root re-exports for common imports. Module
/// items must preserve the core ownership and side-effect boundaries
/// described in this file.
pub mod realtime {
    pub use crate::realtime_testing::*;
}

/// Public telemetry namespace. Use it for the documented telemetry API
/// surface; prefer crate-root re-exports for common imports. Module
/// items must preserve the core ownership and side-effect boundaries
/// described in this file.
pub mod telemetry {
    pub use crate::telemetry_testing::*;
}

/// Public tool namespace. Use it for the documented tool API surface;
/// prefer crate-root re-exports for common imports. Module items must
/// preserve the core ownership and side-effect boundaries described in
/// this file.
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
