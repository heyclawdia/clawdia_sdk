use serde_json::{Map, Value};

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
