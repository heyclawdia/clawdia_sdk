//! Convenience facade over the split Agent SDK crates.
//!
//! This crate exists for imports and examples. It does not define a runtime,
//! package registry, policy path, journal path, event stream, tool executor,
//! telemetry truth store, persistence backend, approval UI, or workflow engine.
//! Runtime behavior remains owned by the split crates and their canonical
//! contracts.
//!
//! ```
//! use clawdia_sdk::prelude::*;
//!
//! let agent = Agent::builder()
//!     .id(AgentId::new("agent.docs.facade"))
//!     .name("facade docs")
//!     .build()?;
//! let request = RunRequest::text(
//!     RunId::new("run.docs.facade"),
//!     agent.id().clone(),
//!     SourceRef::with_kind(SourceKind::Host, "source.docs.facade"),
//!     "hello",
//! );
//!
//! assert_eq!(request.agent_id, agent.id().clone());
//! # Ok::<(), clawdia_sdk::prelude::AgentError>(())
//! ```

/// Advanced core namespace.
///
/// Use this when a facade consumer wants explicit access to the primitive
/// kernel while still depending on the convenience crate.
pub mod core {
    pub use agent_sdk_core::*;
}

/// Common app-building imports from `agent-sdk-core`.
///
/// This prelude is a re-export only. It does not add helper behavior or a
/// second execution path.
pub mod prelude {
    pub use agent_sdk_core::prelude::*;
}

/// Provider adapter namespace.
///
/// Enabled by the `providers` feature. Provider adapters still implement core
/// provider ports and do not own runtime policy, journals, events, approval, or
/// tool execution.
#[cfg(feature = "providers")]
pub mod providers {
    pub use agent_sdk_provider::*;
}

/// Workspace and tool helper namespace.
///
/// Enabled by the `workspace-tools` feature. Toolkit helpers lower into core
/// runtime-package, tool, policy, content-ref, journal, event, and effect
/// contracts.
#[cfg(feature = "workspace-tools")]
pub mod tools {
    pub use agent_sdk_toolkit::*;
}

/// Evaluation namespace.
///
/// Enabled by the `evals` feature. Evaluation helpers are post-hoc projections
/// over supplied core traces and evidence; they do not run agents or append
/// journals.
#[cfg(feature = "evals")]
pub mod eval {
    pub use agent_sdk_eval::*;
}

/// Deterministic test-support namespace.
///
/// Enabled by the `test-support` feature so production imports do not
/// accidentally communicate that test helpers are runtime behavior.
#[cfg(feature = "test-support")]
pub mod testing {
    pub use agent_sdk_core::testing::*;

    #[cfg(feature = "evals")]
    pub use agent_sdk_eval::testing::*;

    #[cfg(feature = "workspace-tools")]
    pub use agent_sdk_toolkit::testing::*;
}
