// Keep the mature vault core isolated from the background migration extension.
// The base file is the pre-migration implementation; migration.rs adds the
// per-photo DEK queue and alternate Tauri commands used by the desktop bridge.
include!("vault/base.rs");
include!("vault/migration.rs");
