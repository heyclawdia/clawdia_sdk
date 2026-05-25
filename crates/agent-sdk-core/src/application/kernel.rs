//! Application-layer coordination over core primitives. Use these services to lower
//! helpers, drive runs, validate output, coordinate tools, approvals, delivery,
//! isolation, telemetry, and feature layers. Methods in this layer may call
//! configured ports, mutate in-memory stores, append journals, or publish events as
//! documented. This file contains the kernel portion of that contract.
//!
/// Readiness profile for the first fake-provider text run. Use it when
/// labeling P0 package or validation evidence; it is data-only.
pub const READINESS_PROFILE_P0: &str = "p0-text-run";
/// Constant value for the application::kernel contract. Use it to keep
/// SDK records and tests aligned on the same stable value.
pub const READINESS_PROFILE_P1: &str = "p1-typed-output";
/// Constant value for the application::kernel contract. Use it to keep
/// SDK records and tests aligned on the same stable value.
pub const READINESS_PROFILE_P2: &str = "p2-side-effects";
