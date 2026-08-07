impl VaultState {
    fn photo_source_info_for_migration(&self, photo_id: &str) -> Result<PhotoSourceInfo> {
        let database_path = self.data_dir().join("lifetrace.db");
        let connection = crate::database::connection::open(&database_path)
            .context("无法打开本地照片库数据库")?;
        let row: Option<(String, Option<String>, String, Option<String>)> = connection
            .query_row(
                "SELECT original_path, thumbnail_path, original_file_name, mime_type
                 FROM photos WHERE id=?1 AND deleted_at IS NULL",
                [photo_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(|error| anyhow!(error.to_string()))?;
        let (original, thumbnail, original_name, mime_type) =
            row.ok_or_else(|| anyhow!("同步相册中不存在该照片：{photo_id}"))?;
        let source_path = self.data_dir().join("photos").join(original);
        let thumbnail_path = thumbnail.map(|relative| self.data_dir().join("photos").join(relative));
        let mime_type = mime_type.unwrap_or_else(|| {
            mime_guess::from_path(&source_path)
                .first_or_octet_stream()
                .essence_str()
                .to_string()
        });
        Ok(PhotoSourceInfo {
            source_path,
            thumbnail_path,
            original_name,
            mime_type,
        })
    }

    fn prepare_photo_migrations(
        &self,
        photo_ids: Vec<String>,
        album_id: Option<String>,
    ) -> Result<Vec<MigrationWork>> {
        if photo_ids.is_empty() {
            return Ok(Vec::new());
        }
        self.ensure_migration_directories()?;
        let existing = self.load_migration_tasks()?;
        if photo_ids
            .iter()
            .any(|photo_id| existing.iter().any(|task| task.photo_id == *photo_id))
        {
            bail!("所选照片中存在已经排队的私密迁移任务");
        }

        let works = self.session_key(|master_key| {
            let manifest = self.load_manifest(master_key)?;
            if let Some(id) = album_id.as_deref() {
                if !manifest.albums.iter().any(|album| album.id == id) {
                    bail!("private album was not found");
                }
            }
            let mut works = Vec::with_capacity(photo_ids.len());
            let mut prepared_ids = Vec::with_capacity(photo_ids.len());
            for photo_id in &photo_ids {
                let result = (|| {
                    let info = self.photo_source_info_for_migration(photo_id)?;
                    let metadata = fs::metadata(&info.source_path)
                        .with_context(|| format!("无法读取待隐藏照片：{photo_id}"))?;
                    if !metadata.is_file() {
                        bail!("待隐藏照片原文件不存在：{photo_id}");
                    }
                    let asset_id = Uuid::new_v4().to_string();
                    let asset_key = Self::new_migration_asset_key();
                    self.write_migration_asset_key(
                        master_key,
                        &asset_id,
                        asset_key.as_slice(),
                    )?;
                    let asset = VaultAsset {
                        id: asset_id.clone(),
                        original_name: info.original_name,
                        mime_type: info.mime_type.clone(),
                        size: metadata.len(),
                        imported_at: Utc::now().to_rfc3339(),
                        has_thumbnail: info.mime_type.starts_with("image/"),
                        state: VaultAssetState::Active,
                        album_ids: album_id.iter().cloned().collect(),
                        deleted_at: None,
                    };
                    if let Err(error) = self.write_migration_meta(master_key, &asset) {
                        let _ = fs::remove_file(self.migration_asset_key_path(&asset_id));
                        return Err(error);
                    }
                    let task = VaultMigrationTask {
                        asset_id: asset_id.clone(),
                        photo_id: photo_id.clone(),
                        size: asset.size,
                        state: VaultMigrationState::Queued,
                        created_at: asset.imported_at.clone(),
                    };
                    if let Err(error) = self.write_migration_task(&task) {
                        self.remove_migration_sidecars(&asset_id, false);
                        return Err(error);
                    }
                    prepared_ids.push(asset_id.clone());
                    works.push(MigrationWork {
                        asset_id,
                        photo_id: photo_id.clone(),
                        asset_key,
                    });
                    Ok(())
                })();
                if let Err(error) = result {
                    for asset_id in prepared_ids {
                        self.remove_migration_sidecars(&asset_id, false);
                    }
                    return Err(error);
                }
            }
            Ok(works)
        })?;

        if let Err(error) = self.set_photo_hiding(&photo_ids, true) {
            for work in &works {
                self.remove_migration_sidecars(&work.asset_id, false);
            }
            return Err(error);
        }
        Ok(works)
    }

    fn process_migration_batch(&self, works: Vec<MigrationWork>) {
        let _worker = match migration_worker().lock() {
            Ok(worker) => worker,
            Err(_) => {
                for work in works {
                    let _ = self.set_photo_hiding(&[work.photo_id], false);
                }
                return;
            }
        };
        if !self.config_path().exists() {
            for work in works {
                let _ = self.set_photo_hiding(&[work.photo_id], false);
            }
            return;
        }
        for work in works {
            if let Err(error) = self.process_one_migration(work) {
                eprintln!("LifeTrace private migration failed: {error}");
            }
        }
    }

    fn process_one_migration(&self, work: MigrationWork) -> Result<()> {
        let result = (|| {
            let task = self.load_migration_task(&work.asset_id)?;
            let target = self.object_path(&work.asset_id);
            if task.state == VaultMigrationState::Committing && target.exists() {
                self.verify_encrypted_file(
                    &target,
                    &work.asset_id,
                    work.asset_key.as_slice(),
                    task.size,
                )?;
                self.finalize_photo_hide_for_migration(&[work.photo_id.clone()])?;
                self.set_migration_state(&work.asset_id, VaultMigrationState::Encrypted)?;
                return Ok(());
            }

            let info = self.photo_source_info_for_migration(&work.photo_id)?;
            self.set_migration_state(&work.asset_id, VaultMigrationState::Encrypting)?;
            self.encrypt_file(
                &info.source_path,
                &target,
                &work.asset_id,
                work.asset_key.as_slice(),
            )?;
            self.set_migration_state(&work.asset_id, VaultMigrationState::Verifying)?;
            self.verify_encrypted_file(
                &target,
                &work.asset_id,
                work.asset_key.as_slice(),
                task.size,
            )?;
            if info.mime_type.starts_with("image/") {
                let thumbnail_source = info
                    .thumbnail_path
                    .as_deref()
                    .filter(|path| path.is_file())
                    .unwrap_or(&info.source_path);
                if !self.create_thumbnail(
                    thumbnail_source,
                    &work.asset_id,
                    work.asset_key.as_slice(),
                )? {
                    bail!("无法加密待迁移缩略图");
                }
            }
            self.set_migration_state(&work.asset_id, VaultMigrationState::Committing)?;
            self.finalize_photo_hide_for_migration(&[work.photo_id.clone()])?;
            self.set_migration_state(&work.asset_id, VaultMigrationState::Encrypted)?;
            Ok(())
        })();

        if let Err(error) = result {
            let state = self
                .load_migration_task(&work.asset_id)
                .map(|task| task.state)
                .unwrap_or(VaultMigrationState::Queued);
            if state != VaultMigrationState::Committing {
                let _ = fs::remove_file(self.object_path(&work.asset_id));
                let _ = fs::remove_file(self.thumbnail_path(&work.asset_id));
                self.remove_migration_sidecars(&work.asset_id, false);
                let _ = self.set_photo_hiding(&[work.photo_id], false);
            }
            return Err(error);
        }
        Ok(())
    }

    fn remove_file_if_present(path: &Path) -> Result<()> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error).with_context(|| format!("无法删除照片文件：{}", path.display())),
        }
    }

    fn finalize_photo_hide_for_migration(&self, photo_ids: &[String]) -> Result<()> {
        if photo_ids.is_empty() {
            return Ok(());
        }
        let database_path = self.data_dir().join("lifetrace.db");
        let mut connection = crate::database::connection::open(&database_path)
            .context("无法打开本地照片库数据库")?;
        for photo_id in photo_ids {
            let paths: Option<(String, Option<String>)> = connection
                .query_row(
                    "SELECT original_path, thumbnail_path FROM photos WHERE id=?1",
                    [photo_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|error| anyhow!(error.to_string()))?;
            if let Some((original, thumbnail)) = paths {
                Self::remove_file_if_present(&self.data_dir().join("photos").join(original))?;
                if let Some(thumbnail) = thumbnail {
                    Self::remove_file_if_present(
                        &self.data_dir().join("photos").join(thumbnail),
                    )?;
                }
            }
            let transaction = connection.transaction()?;
            let _ = transaction.execute(
                "DELETE FROM photo_device_assets WHERE photo_id=?1",
                [photo_id],
            );
            let _ = transaction.execute(
                "DELETE FROM photo_upload_tasks WHERE photo_id=?1",
                [photo_id],
            );
            transaction.execute("DELETE FROM photos WHERE id=?1", [photo_id])?;
            transaction.commit()?;
        }
        Ok(())
    }

    fn resume_migration_works(&self) -> Result<Vec<MigrationWork>> {
        self.session_key(|master_key| {
            self.finalize_completed_migrations(master_key)?;
            let mut works = Vec::new();
            for task in self.load_migration_tasks()? {
                works.push(MigrationWork {
                    asset_id: task.asset_id.clone(),
                    photo_id: task.photo_id,
                    asset_key: self.load_migration_asset_key(master_key, &task.asset_id)?,
                });
            }
            Ok(works)
        })
    }

}
