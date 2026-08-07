-- 关键查询索引。

CREATE INDEX idx_sync_entities_user_type
ON sync_entities(user_id, entity_type);

CREATE INDEX idx_sync_entities_user_active
ON sync_entities(user_id, is_deleted, entity_type);

CREATE INDEX idx_sync_entities_last_cursor
ON sync_entities(user_id, last_cursor);

CREATE INDEX idx_change_log_user_cursor
ON sync_change_log(user_id, cursor);

CREATE INDEX idx_change_log_user_type_cursor
ON sync_change_log(user_id, entity_type, cursor);

CREATE INDEX idx_snapshot_items_keyset
ON sync_snapshot_items(snapshot_id, entity_type, entity_id);
