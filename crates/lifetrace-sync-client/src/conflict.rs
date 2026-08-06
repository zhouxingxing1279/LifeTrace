use lifetrace_contracts::json_value::JsonValue;
use lifetrace_contracts::registry::EntityType;
use lifetrace_contracts::{ChangeId, ConflictId, EntityId, ServerVersion};

#[derive(Debug, Clone, PartialEq)]
pub struct PersistedConflict {
    pub conflict_id: ConflictId,
    pub change_id: Option<ChangeId>,
    pub entity_type: EntityType,
    pub entity_id: EntityId,
    pub base_version: ServerVersion,
    pub server_version: ServerVersion,
    pub local_payload: Option<JsonValue>,
    pub remote_payload: Option<JsonValue>,
    pub server_deleted: bool,
    pub kind: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    AcceptRemote,
    KeepLocal,
    Discard,
}
