//! Concrete workspace tool helpers layered over core tool/effect contracts. Use these
//! modules for bounded read, search, edit, write, and format-aware extraction
//! behavior under a host-selected workspace policy. Reads search local files;
//! edit/write helpers may mutate files only through explicit executor calls. This
//! file contains the anchor portion of that contract.
//!
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Workspace hash line anchor request or result value.
/// Creating the value does not touch the filesystem; workspace executors document read, write, edit, or search effects.
pub struct HashLineAnchor {
    /// Line used by this record or request.
    pub line: usize,
    /// Deterministic before hash used for stale checks, package evidence, or
    /// replay comparisons.
    pub before_hash: String,
}
