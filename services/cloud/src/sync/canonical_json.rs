//! Stable JSON canonicalization.
//!
//! Object keys are recursively sorted so semantically equal documents always
//! produce the same hash. Numbers and strings are serialized by serde_json
//! (stable escaping); arrays and nulls are preserved in order.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

/// Returns a canonical (key-sorted) copy of the value.
pub fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted: BTreeMap<String, Value> = map
                .iter()
                .map(|(key, value)| (key.clone(), canonicalize(value)))
                .collect();
            let mut out = Map::new();
            for (key, value) in sorted {
                out.insert(key, value);
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize).collect()),
        other => other.clone(),
    }
}

/// Canonical serialization of a value (stable across key order).
pub fn canonical_json(value: &Value) -> String {
    serde_json::to_string(&canonicalize(value)).expect("canonical JSON is serializable")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn object_key_order_does_not_change_hash_input() {
        let a = json!({ "a": 1, "b": 2, "c": { "x": [1, 2], "y": null } });
        let b = json!({ "c": { "y": null, "x": [1, 2] }, "b": 2, "a": 1 });
        assert_eq!(canonical_json(&a), canonical_json(&b));
    }
}
