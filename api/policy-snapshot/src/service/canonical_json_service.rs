use serde_json::{Map, Value};

pub fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            let mut ordered = Map::new();
            for key in keys {
                if let Some(v) = map.get(&key) {
                    ordered.insert(key, canonicalize(v));
                }
            }
            Value::Object(ordered)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize).collect()),
        _ => value.clone(),
    }
}

pub fn canonical_string(value: &Value) -> Result<String, String> {
    serde_json::to_string(&canonicalize(value)).map_err(|e| format!("canonical serialization failed: {e}"))
}
