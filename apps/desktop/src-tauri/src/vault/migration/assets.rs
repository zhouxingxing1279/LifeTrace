impl VaultState {
    fn read_asset_with_migration_key(&self, asset_id: &str) -> Result<VaultAssetPayload> {
        self.session_key(|master_key| {
            self.finalize_completed_migrations(master_key)?;
            let asset = self
                .load_manifest(master_key)?
                .assets
                .into_iter()
                .find(|asset| asset.id == asset_id)
                .ok_or_else(|| anyhow!("照片仍在后台安全处理中，请稍后再预览"))?;
            let asset_key = self.load_migration_asset_key(master_key, asset_id)?;
            let bytes = self.decrypt_file(
                &self.object_path(asset_id),
                asset_id,
                asset_key.as_slice(),
                MAX_PREVIEW_BYTES,
            )?;
            Ok(VaultAssetPayload {
                asset,
                data_base64: BASE64.encode(bytes.as_slice()),
            })
        })
    }

    fn read_thumbnail_with_migration_key(
        &self,
        asset_id: &str,
    ) -> Result<VaultThumbnailPayload> {
        self.session_key(|master_key| {
            self.finalize_completed_migrations(master_key)?;
            if let Some(asset) = self
                .load_manifest(master_key)?
                .assets
                .iter()
                .find(|asset| asset.id == asset_id && asset.has_thumbnail)
                .cloned()
            {
                let asset_key = self.load_migration_asset_key(master_key, asset_id)?;
                let bytes = fs::read(self.thumbnail_path(asset_id))
                    .context("private thumbnail is missing")?;
                let encrypted: EncryptedBlob =
                    serde_json::from_slice(&bytes).context("private thumbnail is invalid")?;
                let aad = format!("lifetrace-vault-thumbnail-v1:{asset_id}");
                let plaintext = Self::decrypt_blob(
                    asset_key.as_slice(),
                    &encrypted,
                    aad.as_bytes(),
                )?;
                return Ok(VaultThumbnailPayload {
                    asset_id: asset.id,
                    mime_type: "image/png".to_string(),
                    data_base64: BASE64.encode(plaintext.as_slice()),
                });
            }

            let task = self.load_migration_task(asset_id)?;
            let asset = self.load_migration_meta(master_key, asset_id)?;
            if !asset.has_thumbnail {
                bail!("private thumbnail was not found");
            }
            if self.thumbnail_path(asset_id).exists() {
                let asset_key = self.load_migration_asset_key(master_key, asset_id)?;
                let bytes = fs::read(self.thumbnail_path(asset_id))
                    .context("private thumbnail is missing")?;
                let encrypted: EncryptedBlob =
                    serde_json::from_slice(&bytes).context("private thumbnail is invalid")?;
                let aad = format!("lifetrace-vault-thumbnail-v1:{asset_id}");
                let plaintext = Self::decrypt_blob(
                    asset_key.as_slice(),
                    &encrypted,
                    aad.as_bytes(),
                )?;
                return Ok(VaultThumbnailPayload {
                    asset_id: asset_id.to_string(),
                    mime_type: "image/png".to_string(),
                    data_base64: BASE64.encode(plaintext.as_slice()),
                });
            }
            let info = self.photo_source_info_for_migration(&task.photo_id)?;
            let source = info
                .thumbnail_path
                .filter(|path| path.is_file())
                .ok_or_else(|| anyhow!("待迁移缩略图已经不存在"))?;
            let bytes = fs::read(&source).context("无法读取待迁移缩略图")?;
            Ok(VaultThumbnailPayload {
                asset_id: asset_id.to_string(),
                mime_type: mime_guess::from_path(&source)
                    .first_or_octet_stream()
                    .essence_str()
                    .to_string(),
                data_base64: BASE64.encode(bytes),
            })
        })
    }

    fn restore_to_sync_album_with_migration_key(&self, asset_id: &str) -> Result<VaultAsset> {
        let master_key = self.copy_session_key()?;
        self.finalize_completed_migrations(&master_key)?;
        let manifest = self.load_manifest(&master_key)?;
        let asset = manifest
            .assets
            .iter()
            .find(|asset| asset.id == asset_id && asset.state == VaultAssetState::Active)
            .cloned()
            .ok_or_else(|| anyhow!("私密相册中不存在该照片"))?;
        let asset_key = self.load_migration_asset_key(&master_key, asset_id)?;
        let object_path = self.object_path(&asset.id);
        let file_len = fs::metadata(&object_path)
            .map(|metadata| metadata.len())
            .unwrap_or(MAX_PREVIEW_BYTES);
        let plaintext = self.decrypt_file(
            &object_path,
            &asset.id,
            asset_key.as_slice(),
            file_len,
        )?;
        let content_hash = format!("{:x}", Sha256::digest(plaintext.as_slice()));
        let database_path = self.data_dir().join("lifetrace.db");
        let connection = crate::database::connection::open(&database_path)
            .context("无法打开本地照片库数据库")?;
        let existing: Option<(String, Option<String>)> = connection
            .query_row(
                "SELECT id, deleted_at FROM photos WHERE content_hash=?1",
                [&content_hash],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| anyhow!(error.to_string()))?;
        match existing {
            Some((photo_id, deleted_at)) => {
                if deleted_at.is_some() {
                    connection.execute(
                        "UPDATE photos SET deleted_at=NULL, processing_status='completed' WHERE id=?1",
                        [&photo_id],
                    )?;
                }
                self.ensure_photo_files(&connection, &photo_id, &plaintext, &asset.mime_type)?;
            }
            None => {
                self.write_restored_photo(&connection, &asset, &plaintext, &content_hash)?;
            }
        }
        drop(connection);
        let mut manifest = self.load_manifest(&master_key)?;
        manifest.assets.retain(|item| item.id != asset_id);
        self.save_manifest(&master_key, &manifest)?;
        let _ = fs::remove_file(self.object_path(asset_id));
        let _ = fs::remove_file(self.thumbnail_path(asset_id));
        let _ = fs::remove_file(self.migration_asset_key_path(asset_id));
        Ok(asset)
    }

    fn delete_asset_permanently_with_key(&self, asset_id: &str) -> Result<()> {
        self.delete_asset_permanently(asset_id)?;
        let _ = fs::remove_file(self.migration_asset_key_path(asset_id));
        Ok(())
    }

    fn verify_integrity_with_migration_keys(&self) -> Result<VaultIntegrityReport> {
        self.session_key(|master_key| {
            self.finalize_completed_migrations(master_key)?;
            let manifest = self.load_manifest(master_key)?;
            let mut corrupted_asset_ids = Vec::new();
            for asset in &manifest.assets {
                let asset_key = match self.load_migration_asset_key(master_key, &asset.id) {
                    Ok(key) => key,
                    Err(_) => {
                        corrupted_asset_ids.push(asset.id.clone());
                        continue;
                    }
                };
                let object_result = self.verify_encrypted_file(
                    &self.object_path(&asset.id),
                    &asset.id,
                    asset_key.as_slice(),
                    asset.size,
                );
                let thumbnail_result = if asset.has_thumbnail {
                    (|| {
                        let bytes = fs::read(self.thumbnail_path(&asset.id))?;
                        let encrypted: EncryptedBlob = serde_json::from_slice(&bytes)?;
                        let aad = format!("lifetrace-vault-thumbnail-v1:{}", asset.id);
                        Self::decrypt_blob(
                            asset_key.as_slice(),
                            &encrypted,
                            aad.as_bytes(),
                        )
                        .map(|_| ())
                    })()
                } else {
                    Ok(())
                };
                if object_result.is_err() || thumbnail_result.is_err() {
                    corrupted_asset_ids.push(asset.id.clone());
                }
            }
            let checked = manifest.assets.len();
            Ok(VaultIntegrityReport {
                checked,
                healthy: checked.saturating_sub(corrupted_asset_ids.len()),
                corrupted_asset_ids,
            })
        })
    }

    fn delete_all_with_migrations(&self, confirmation: &str) -> Result<()> {
        if confirmation != DELETE_CONFIRMATION {
            bail!("删除确认文本不正确");
        }
        let _worker = migration_worker()
            .lock()
            .map_err(|_| anyhow!("private migration worker is unavailable"))?;
        for task in self.load_migration_tasks().unwrap_or_default() {
            let _ = self.set_photo_hiding(&[task.photo_id], false);
        }
        self.delete_all(confirmation)
    }
}
