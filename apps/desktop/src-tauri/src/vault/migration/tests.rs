#[cfg(test)]
mod migration_tests {
    use super::*;

    const PASSWORD: &str = "Correct-Horse-Battery-Staple";

    fn test_state() -> (VaultState, PathBuf) {
        let workspace =
            std::env::temp_dir().join(format!("lifetrace-vault-migration-test-{}", Uuid::new_v4()));
        let vault_root = workspace.join("vault");
        (VaultState::new(vault_root).expect("state"), workspace)
    }

    fn sample_sync_photo(root: &Path, photo_id: &str) -> (PathBuf, PathBuf) {
        let photos_root = root.join("photos");
        let originals = photos_root.join("originals");
        let thumbnails = photos_root.join("thumbnails");
        fs::create_dir_all(&originals).expect("originals");
        fs::create_dir_all(&thumbnails).expect("thumbnails");
        let source = originals.join(format!("{photo_id}.png"));
        let thumbnail = thumbnails.join(format!("{photo_id}.jpg"));
        let image = image::RgbImage::from_pixel(32, 24, image::Rgb([18, 120, 88]));
        image
            .save_with_format(&source, image::ImageFormat::Png)
            .expect("source image");
        image
            .save_with_format(&thumbnail, image::ImageFormat::Jpeg)
            .expect("thumbnail image");
        let database_path = root.join("lifetrace.db");
        let connection = crate::database::connection::open(&database_path).expect("database");
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS photos(
                    id TEXT PRIMARY KEY,
                    original_path TEXT NOT NULL,
                    thumbnail_path TEXT,
                    original_file_name TEXT NOT NULL,
                    mime_type TEXT,
                    file_size INTEGER NOT NULL,
                    deleted_at TEXT,
                    processing_status TEXT NOT NULL DEFAULT 'completed'
                );
                CREATE TABLE IF NOT EXISTS photo_device_assets(photo_id TEXT);
                CREATE TABLE IF NOT EXISTS photo_upload_tasks(photo_id TEXT);",
            )
            .expect("photos table");
        let size = fs::metadata(&source).expect("metadata").len() as i64;
        connection
            .execute(
                "INSERT INTO photos(
                    id, original_path, thumbnail_path, original_file_name,
                    mime_type, file_size, deleted_at, processing_status
                 ) VALUES (?1,?2,?3,?4,'image/png',?5,NULL,'completed')",
                params![
                    photo_id,
                    format!("originals/{photo_id}.png"),
                    format!("thumbnails/{photo_id}.jpg"),
                    format!("{photo_id}.png"),
                    size,
                ],
            )
            .expect("insert photo");
        (source, thumbnail)
    }

    #[test]
    fn locking_does_not_stop_an_already_submitted_photo_migration() {
        let (state, root) = test_state();
        state.initialize(PASSWORD).expect("initialize");
        let (source, thumbnail) = sample_sync_photo(&root, "photo-lock");
        let works = state
            .prepare_photo_migrations(vec!["photo-lock".to_string()], None)
            .expect("prepare migration");
        let pending = state
            .list_assets_with_migrations(false, None)
            .expect("pending assets");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].migration_state, Some(VaultMigrationState::Queued));
        let pending_thumbnail = state
            .read_thumbnail_with_migration_key(&pending[0].id)
            .expect("pending thumbnail");
        assert!(!pending_thumbnail.data_base64.is_empty());
        state.lock_internal().expect("lock");
        state.process_migration_batch(works);
        assert!(!state.is_unlocked());
        assert!(!source.exists());
        assert!(!thumbnail.exists());
        assert!(state.list_assets_with_migrations(false, None).is_err());
        state.unlock(PASSWORD).expect("unlock");
        let ready = state
            .list_assets_with_migrations(false, None)
            .expect("ready assets");
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].migration_state, None);
        assert!(!state.migration_task_path(&ready[0].id).exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn interrupted_migration_can_resume_after_unlocking_again() {
        let (state, root) = test_state();
        state.initialize(PASSWORD).expect("initialize");
        let (source, _) = sample_sync_photo(&root, "photo-resume");
        let works = state
            .prepare_photo_migrations(vec!["photo-resume".to_string()], None)
            .expect("prepare migration");
        assert_eq!(works.len(), 1);
        drop(works);
        state.lock_internal().expect("lock");
        drop(state);

        let state = VaultState::new(root.join("vault")).expect("restart state");
        state.unlock(PASSWORD).expect("unlock after restart");
        let resumed = state.resume_migration_works().expect("resume works");
        assert_eq!(resumed.len(), 1);
        state.process_migration_batch(resumed);
        let ready = state
            .list_assets_with_migrations(false, None)
            .expect("ready assets");
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].migration_state, None);
        assert!(!source.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn migration_ciphertext_cannot_be_opened_with_the_master_key() {
        let (state, root) = test_state();
        state.initialize(PASSWORD).expect("initialize");
        sample_sync_photo(&root, "photo-dek");
        let works = state
            .prepare_photo_migrations(vec!["photo-dek".to_string()], None)
            .expect("prepare migration");
        let asset_id = works[0].asset_id.clone();
        state.process_migration_batch(works);
        let master_key = state.copy_session_key().expect("master key");
        assert!(state
            .decrypt_file(
                &state.object_path(&asset_id),
                &asset_id,
                master_key.as_slice(),
                MAX_PREVIEW_BYTES,
            )
            .is_err());
        let payload = state
            .read_asset_with_migration_key(&asset_id)
            .expect("read with wrapped DEK");
        assert!(!payload.data_base64.is_empty());
        let _ = fs::remove_dir_all(root);
    }
}
