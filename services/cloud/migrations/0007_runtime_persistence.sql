-- Runtime metadata required to preserve external device identifiers while
-- retaining UUID primary keys internally.

ALTER TABLE cloud_devices
    ADD COLUMN external_device_id TEXT;

UPDATE cloud_devices
SET external_device_id = id::TEXT
WHERE external_device_id IS NULL;

ALTER TABLE cloud_devices
    ALTER COLUMN external_device_id SET NOT NULL;

CREATE UNIQUE INDEX idx_cloud_devices_external_identity
ON cloud_devices(user_id, app_id, external_device_id);

ALTER TABLE sync_entities
    ADD COLUMN origin_device_external_id TEXT;

ALTER TABLE sync_change_log
    ADD COLUMN origin_device_external_id TEXT;
