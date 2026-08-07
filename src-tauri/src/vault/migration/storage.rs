impl VaultState {
    fn migration_keys_dir(&self) -> PathBuf {
        self.root.join("keys")
    }

    fn migrations_dir(&self) -> PathBuf {
        self.root.join("migrations")
    }

    fn migration_asset_key_path(&self, asset_id: &str) -> PathBuf {
        self.migration_keys_dir().join(format!("{asset_id}.key"))
    }

    fn migration_task_path(&self, asset_id: &str) -> PathBuf {
        self.migrations_dir().join(format!("{asset_id}.json"))
    }

    fn migration_meta_path(&self, asset_id: &str) -> PathBuf {
        self.migrations_dir().join(format!("{asset_id}.meta.vlt"))
    }

    fn ensure_migration_directories(&self) -> Result<()> {
        self.ensure_directories()?;
        fs::create_dir_all(self.migration_keys_dir())
            .context("failed to create private asset key directory")?;
        fs::create_dir_all(self.migrations_dir())
            .context("failed to create private migration directory")?;
        Ok(())
    }

    fn migration_asset_key_aad(asset_id: &str) -> Vec<u8> {
        format!("{MIGRATION_ASSET_KEY_AAD_PREFIX}:{asset_id}").into_bytes()
    }

    fn migration_meta_aad(asset_id: &str) -> Vec<u8> {
        format!("{MIGRATION_META_AAD_PREFIX}:{asset_id}").into_bytes()
    }

    fn new_migration_asset_key() -> Zeroizing<Vec<u8>> {
        let mut key = Zeroizing::new(vec![0_u8; MIGRATION_ASSET_KEY_BYTES]);
        OsRng.fill_bytes(key.as_mut_slice());
        key
    }

    fn write_migration_asset_key(
        &self,
        master_key: &[u8],
        asset_id: &str,
        asset_key: &[u8],
    ) -> Result<()> {
        let wrapped = Self::encrypt_blob(
            master_key,
            asset_key,
            &Self::migration_asset_key_aad(asset_id),
        )?;
        self.write_json_atomic(&self.migration_asset_key_path(asset_id), &wrapped)
    }

    fn load_migration_asset_key(
        &self,
        master_key: &[u8],
        asset_id: &str,
    ) -> Result<Zeroizing<Vec<u8>>> {
        let bytes = fs::read(self.migration_asset_key_path(asset_id))
            .context("private asset key is missing")?;
        let wrapped: EncryptedBlob =
            serde_json::from_slice(&bytes).context("private asset key is invalid")?;
        let key = Self::decrypt_blob(
            master_key,
            &wrapped,
            &Self::migration_asset_key_aad(asset_id),
        )?;
        if key.len() != MIGRATION_ASSET_KEY_BYTES {
            bail!("private asset key length is invalid");
        }
        Ok(key)
    }

    fn write_migration_meta(&self, master_key: &[u8], asset: &VaultAsset) -> Result<()> {
        let plaintext = Zeroizing::new(
            serde_json::to_vec(asset)
                .context("failed to serialize private migration metadata")?,
        );
        let encrypted = Self::encrypt_blob(
            master_key,
            plaintext.as_slice(),
            &Self::migration_meta_aad(&asset.id),
        )?;
        self.write_json_atomic(&self.migration_meta_path(&asset.id), &encrypted)
    }

    fn load_migration_meta(&self, master_key: &[u8], asset_id: &str) -> Result<VaultAsset> {
        let bytes = fs::read(self.migration_meta_path(asset_id))
            .context("private migration metadata is missing")?;
        let encrypted: EncryptedBlob =
            serde_json::from_slice(&bytes).context("private migration metadata is invalid")?;
        let plaintext = Self::decrypt_blob(
            master_key,
            &encrypted,
            &Self::migration_meta_aad(asset_id),
        )?;
        serde_json::from_slice(plaintext.as_slice())
            .context("private migration metadata cannot be decoded")
    }

    fn write_migration_task(&self, task: &VaultMigrationTask) -> Result<()> {
        self.write_json_atomic(&self.migration_task_path(&task.asset_id), task)
    }

    fn load_migration_task(&self, asset_id: &str) -> Result<VaultMigrationTask> {
        let bytes = fs::read(self.migration_task_path(asset_id))
            .context("private migration task is missing")?;
        serde_json::from_slice(&bytes).context("private migration task is invalid")
    }

    fn load_migration_tasks(&self) -> Result<Vec<VaultMigrationTask>> {
        let directory = self.migrations_dir();
        if !directory.exists() {
            return Ok(Vec::new());
        }
        let mut tasks = Vec::new();
        for entry in fs::read_dir(&directory).context("failed to read private migration tasks")? {
            let entry = entry.context("failed to read private migration task")?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let bytes = fs::read(&path).context("failed to read private migration task")?;
            let task: VaultMigrationTask =
                serde_json::from_slice(&bytes).context("private migration task is invalid")?;
            tasks.push(task);
        }
        tasks.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        Ok(tasks)
    }

    fn set_migration_state(&self, asset_id: &str, state: VaultMigrationState) -> Result<()> {
        let mut task = self.load_migration_task(asset_id)?;
        task.state = state;
        self.write_migration_task(&task)
    }

    fn remove_migration_sidecars(&self, asset_id: &str, keep_asset_key: bool) {
        let _ = fs::remove_file(self.migration_task_path(asset_id));
        let _ = fs::remove_file(self.migration_meta_path(asset_id));
        if !keep_asset_key {
            let _ = fs::remove_file(self.migration_asset_key_path(asset_id));
        }
    }

    fn finalize_completed_migrations(&self, master_key: &[u8]) -> Result<usize> {
        let completed: Vec<_> = self
            .load_migration_tasks()?
            .into_iter()
            .filter(|task| task.state == VaultMigrationState::Encrypted)
            .collect();
        if completed.is_empty() {
            return Ok(0);
        }
        let mut manifest = self.load_manifest(master_key)?;
        let mut committed = Vec::new();
        for task in completed {
            if !manifest.assets.iter().any(|asset| asset.id == task.asset_id) {
                manifest
                    .assets
                    .push(self.load_migration_meta(master_key, &task.asset_id)?);
            }
            committed.push(task.asset_id);
        }
        self.save_manifest(master_key, &manifest)?;
        for asset_id in &committed {
            self.remove_migration_sidecars(asset_id, true);
        }
        Ok(committed.len())
    }

    fn pending_asset_views(
        &self,
        master_key: &[u8],
        album_id: Option<&str>,
    ) -> Result<Vec<VaultAssetView>> {
        let mut assets = Vec::new();
        for task in self.load_migration_tasks()? {
            if task.state == VaultMigrationState::Encrypted {
                continue;
            }
            let asset = self.load_migration_meta(master_key, &task.asset_id)?;
            if album_id
                .map(|id| asset.album_ids.iter().any(|candidate| candidate == id))
                .unwrap_or(true)
            {
                let mut view = VaultAssetView::from(asset);
                view.migration_state = Some(task.state);
                assets.push(view);
            }
        }
        Ok(assets)
    }

    fn migration_status(&self) -> Result<VaultStatus> {
        if !self.config_path().exists() || !self.is_unlocked() {
            return self.status();
        }
        self.session_key(|master_key| {
            self.finalize_completed_migrations(master_key)?;
            Ok(())
        })?;
        let mut status = self.status()?;
        if status.unlocked {
            let pending = self
                .load_migration_tasks()?
                .into_iter()
                .filter(|task| task.state != VaultMigrationState::Encrypted)
                .count();
            if let Some(count) = status.asset_count.as_mut() {
                *count += pending;
            }
        }
        Ok(status)
    }

    fn list_assets_with_migrations(
        &self,
        trashed: bool,
        album_id: Option<&str>,
    ) -> Result<Vec<VaultAssetView>> {
        self.session_key(|master_key| {
            self.finalize_completed_migrations(master_key)?;
            let expected = if trashed {
                VaultAssetState::Trash
            } else {
                VaultAssetState::Active
            };
            let mut assets: Vec<VaultAssetView> = self
                .load_manifest(master_key)?
                .assets
                .into_iter()
                .filter(|asset| asset.state == expected)
                .filter(|asset| {
                    album_id
                        .map(|id| asset.album_ids.iter().any(|candidate| candidate == id))
                        .unwrap_or(true)
                })
                .map(VaultAssetView::from)
                .collect();
            if !trashed {
                assets.extend(self.pending_asset_views(master_key, album_id)?);
            }
            assets.sort_by(|left, right| right.imported_at.cmp(&left.imported_at));
            Ok(assets)
        })
    }

}
