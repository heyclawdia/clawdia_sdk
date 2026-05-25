//! Domain primitives for stable SDK vocabulary. Use these items for IDs, refs,
//! policy, privacy, trust, and errors that cross crate or host boundaries. They are
//! data-only and must not perform provider, filesystem, network, or UI side effects.
//! This file contains the json portion of that contract.
//!
use serde_json::{Map, Value};

/// Returns normalize json value for the current value.
/// This is a read-only or data-construction helper unless the method body explicitly calls a
/// port or store.
pub(crate) fn normalize_json_value(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(normalize_json_value).collect()),
        Value::Object(entries) => {
            let mut normalized = Map::new();
            let mut sorted = entries.into_iter().collect::<Vec<_>>();
            sorted.sort_by(|(left, _), (right, _)| left.cmp(right));
            for (key, value) in sorted {
                normalized.insert(key, normalize_json_value(value));
            }
            Value::Object(normalized)
        }
        other => other,
    }
}
