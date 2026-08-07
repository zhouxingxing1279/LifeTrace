// Background private-album migration is split by responsibility so the mature
// vault core stays small and each part is independently reviewable.
include!("migration/model.rs");
include!("migration/storage.rs");
include!("migration/queue.rs");
include!("migration/assets.rs");
include!("migration/commands.rs");
include!("migration/tests.rs");
