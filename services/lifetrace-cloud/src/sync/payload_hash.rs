//! Stable change hashing for idempotency.
//!
//! The hash covers every semantic field of a change (entityType, entityId,
//! operation, baseServerVersion, entitySchemaVersion, payload,
//! atomicGroupId, dependencies) but never requestId, server time or header
//! order.

use lifetrace_contracts::sync::v1::SyncChangeV1;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use super::canonical_json::canonical_json;

/// SHA-256 hex digest of bytes.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Canonical hash of an entity-type scope (used to bind cursors/tokens).
pub fn scope_hash(entity_types: &Option<Vec<lifetrace_contracts::EntityType>>) -> String {
    let mut names: Vec<String> = entity_types
        .as_ref()
        .map(|types| types.iter().map(|value| value.as_str().to_owned()).collect())
        .unwrap_or_default();
    names.sort();
    sha256_hex(canonical_json(&Value::Array(
        names.into_iter().map(Value::String).collect(),
    ))
    .as_bytes())
}

/// Scope hash for "all entity types" (used by push/snapshot cursors).
pub fn empty_scope() -> String {
    scope_hash(&None)
}

/// Stable semantic hash of one change.
pub fn change_hash(change: &SyncChangeV1) -> String {
    let mut doc = Map::new();
    doc.insert("entityType".to_owned(), json!(change.entity_type.as_str()));
    doc.insert("entityId".to_owned(), json!(change.entity_id.as_str()));
    doc.insert("operation".to_owned(), json!(change.operation.as_str()));
    doc.insert(
        "baseServerVersion".to_owned(),
        json!(change.base_server_version.as_str()),
    );
    doc.insert(
        "entitySchemaVersion".to_owned(),
        json!(change.entity_schema_version),
    );
    doc.insert(
        "payload".to_owned(),
        change
            .payload
            .as_ref()
            .map(|value| value.0.clone())
            .unwrap_or(Value::Null),
    );
    doc.insert(
        "atomicGroupId".to_owned(),
        change
            .atomic_group_id
            .as_ref()
            .map(|value| json!(value.as_str()))
            .unwrap_or(Value::Null),
    );
    doc.insert(
        "dependencies".to_owned(),
        Value::Array(
            change
                .dependencies
                .iter()
                .map(|dependency| {
                    json!({
                        "entityType": dependency.entity_type.as_str(),
                        "entityId": dependency.entity_id.as_str(),
                    })
                })
                .collect(),
        ),
    );
    sha256_hex(canonical_json(&Value::Object(doc)).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_is_hex_and_stable() {
        let a = sha256_hex(b"hello");
        let b = sha256_hex(b"hello");
        assert_eq!(a.len(), 64);
        assert_eq!(a, b);
        assert_ne!(a, sha256_hex(b"world"));
    }
}
