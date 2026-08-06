use std::{
    fs::{self, File},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};

use aes_gcm::{
    aead::{Aead, Payload},
    Aes256Gcm, KeyInit, Nonce,
};
use anyhow::{anyhow, bail, Context, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Utc;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

const VAULT_VERSION: u32 = 1;
const MASTER_KEY_BYTES: usize = 32;
const SALT_BYTES: usize = 16;
const NONCE_BYTES: usize = 12;
const OBJECT_MAGIC: &[u8; 8] = b"LTVLT001";
const OBJECT_CHUNK_BYTES: usize = 1024 * 1024;
const DEFAULT_AUTO_LOCK_SECONDS: u64 = 300;
const MIN_PASSWORD_CHARS: usize = 12;
const MAX_PREVIEW_BYTES: u64 = 512 * 1024 * 1024;
const DELETE_CONFIRMATION: &str = "永久删除私密相册";
const MASTER_KEY_AAD: &[u8] = b"lifetrace-vault-master-key-v1";
const MANIFEST_AAD: &[u8] = b"lifetrace-vault-manifest-v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KdfConfig {
    algorithm: String,
    memory_cost: u32,
    time_cost: u32,
    parallelism: u32,
    salt_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptedBlob {
    nonce_base64: String,
    ciphertext_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VaultConfig {
    version: u32,
    kdf: KdfConfig,
    wrapped_master_key: EncryptedBlob,
    auto_lock_seconds: u64,
    #[serde(default)]
    lock_on_blur: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum VaultAssetState {
    #[default]
    Active,
    Trash,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VaultAsset {
    pub id: String,
    pub original_name: String,
    pub mime_type: String,
    pub size: u64,
    pub imported_at: String,
    pub has_thumbnail: bool,
    #[serde(default)]
    pub state: VaultAssetState,
    #[serde(default)]
    pub album_ids: Vec<String>,
    #[serde(default)]
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VaultAlbum {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VaultManifest {
    version: u32,
    assets: Vec<VaultAsset>,
    #[serde(default)]
    albums: Vec<VaultAlbum>,
}

impl Default for VaultManifest {
    fn default() -> Self {
        Self {
            version: VAULT_VERSION,
            assets: Vec::new(),
            albums: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    pub configured: bool,
    pub unlocked: bool,
    pub asset_count: Option<usize>,
    pub trash_count: Option<usize>,
    pub album_count: Option<usize>,
    pub auto_lock_seconds: u64,
    pub lock_on_blur: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultIntegrityReport {
    pub checked: usize,
    pub healthy: usize,
    pub corrupted_asset_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultAssetPayload {
    pub asset: VaultAsset,
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultThumbnailPayload {
    pub asset_id: String,
    pub mime_type: String,
    pub data_base64: String,
}

struct VaultSession {
    master_key: Zeroizing<Vec<u8>>,
    last_activity: Instant,
    auto_lock: Duration,
}

#[derive(Default)]
struct AttemptState {
    failures: u32,
    blocked_until: Option<Instant>,
}

pub struct VaultState {
    root: PathBuf,
    session: Mutex<Option<VaultSession>>,
    attempts: Mutex<AttemptState>,
}

impl VaultState {
    pub fn new(root: PathBuf) -> Result<Self> {
        let state = Self {
            root,
            session: Mutex::new(None),
            attempts: Mutex::new(AttemptState::default()),
        };
        state.cleanup_temporary_files()?;
        Ok(state)
    }

    fn cleanup_temporary_files(&self) -> Result<()> {
        let temp = self.temp_dir();
        if temp.exists() {
            fs::remove_dir_all(&temp).context("failed to clean private album temporary files")?;
        }
        if self.root.exists() {
            fs::create_dir_all(&temp)
                .context("failed to recreate private album temporary directory")?;
        }
        Ok(())
    }

    fn config_path(&self) -> PathBuf {
        self.root.join("config.json")
    }

    fn manifest_path(&self) -> PathBuf {
        self.root.join("manifest.vlt")
    }

    fn objects_dir(&self) -> PathBuf {
        self.root.join("objects")
    }

    fn thumbnails_dir(&self) -> PathBuf {
        self.root.join("thumbnails")
    }

    fn temp_dir(&self) -> PathBuf {
        self.root.join("temp")
    }

    fn ensure_directories(&self) -> Result<()> {
        fs::create_dir_all(self.objects_dir())
            .context("failed to create vault objects directory")?;
        fs::create_dir_all(self.thumbnails_dir())
            .context("failed to create vault thumbnail directory")?;
        fs::create_dir_all(self.temp_dir())
            .context("failed to create vault temporary directory")?;
        Ok(())
    }

    fn read_config(&self) -> Result<VaultConfig> {
        let bytes = fs::read(self.config_path()).context("private album is not configured")?;
        let config: VaultConfig =
            serde_json::from_slice(&bytes).context("private album configuration is invalid")?;
        if config.version != VAULT_VERSION {
            bail!("unsupported private album format version");
        }
        Ok(config)
    }

    fn validate_password(password: &str) -> Result<()> {
        if password.chars().count() < MIN_PASSWORD_CHARS {
            bail!("密码至少需要 {MIN_PASSWORD_CHARS} 个字符");
        }
        if password.chars().all(|character| character.is_ascii_digit()) {
            bail!("密码不能只包含数字");
        }
        Ok(())
    }

    fn derive_kek(password: &str, config: &KdfConfig) -> Result<Zeroizing<Vec<u8>>> {
        if config.algorithm != "argon2id" {
            bail!("unsupported password derivation algorithm");
        }
        let salt = BASE64
            .decode(&config.salt_base64)
            .context("private album salt is invalid")?;
        let params = Params::new(
            config.memory_cost,
            config.time_cost,
            config.parallelism,
            Some(MASTER_KEY_BYTES),
        )
        .map_err(|_| anyhow!("private album password parameters are invalid"))?;
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let mut key = Zeroizing::new(vec![0_u8; MASTER_KEY_BYTES]);
        argon2
            .hash_password_into(password.as_bytes(), &salt, key.as_mut_slice())
            .map_err(|_| anyhow!("failed to derive private album key"))?;
        Ok(key)
    }

    fn new_kdf_config() -> KdfConfig {
        let mut salt = [0_u8; SALT_BYTES];
        OsRng.fill_bytes(&mut salt);
        KdfConfig {
            algorithm: "argon2id".to_string(),
            memory_cost: 65_536,
            time_cost: 3,
            parallelism: 1,
            salt_base64: BASE64.encode(salt),
        }
    }

    fn encrypt_blob(key: &[u8], plaintext: &[u8], aad: &[u8]) -> Result<EncryptedBlob> {
        let cipher = Aes256Gcm::new_from_slice(key).context("invalid encryption key")?;
        let mut nonce = [0_u8; NONCE_BYTES];
        OsRng.fill_bytes(&mut nonce);
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: plaintext,
                    aad,
                },
            )
            .map_err(|_| anyhow!("failed to encrypt private album data"))?;
        Ok(EncryptedBlob {
            nonce_base64: BASE64.encode(nonce),
            ciphertext_base64: BASE64.encode(ciphertext),
        })
    }

    fn decrypt_blob(key: &[u8], blob: &EncryptedBlob, aad: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
        let cipher = Aes256Gcm::new_from_slice(key).context("invalid decryption key")?;
        let nonce = BASE64
            .decode(&blob.nonce_base64)
            .context("private album nonce is invalid")?;
        if nonce.len() != NONCE_BYTES {
            bail!("private album nonce length is invalid");
        }
        let ciphertext = BASE64
            .decode(&blob.ciphertext_base64)
            .context("private album ciphertext is invalid")?;
        let plaintext = cipher
            .decrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: &ciphertext,
                    aad,
                },
            )
            .map_err(|_| anyhow!("密码错误或私密相册数据无法验证"))?;
        Ok(Zeroizing::new(plaintext))
    }

    fn write_json_atomic<T: Serialize>(&self, path: &Path, value: &T) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("failed to create private album directory")?;
        }
        let bytes =
            serde_json::to_vec_pretty(value).context("failed to serialize private album data")?;
        let temporary = self.temp_dir().join(format!("{}.partial", Uuid::new_v4()));
        fs::write(&temporary, bytes).context("failed to write private album temporary file")?;
        if path.exists() {
            fs::remove_file(path).context("failed to replace private album file")?;
        }
        fs::rename(&temporary, path).context("failed to commit private album file")?;
        Ok(())
    }

    fn save_manifest(&self, master_key: &[u8], manifest: &VaultManifest) -> Result<()> {
        let bytes = Zeroizing::new(
            serde_json::to_vec(manifest).context("failed to serialize private album manifest")?,
        );
        let encrypted = Self::encrypt_blob(master_key, bytes.as_slice(), MANIFEST_AAD)?;
        self.write_json_atomic(&self.manifest_path(), &encrypted)
    }

    fn load_manifest(&self, master_key: &[u8]) -> Result<VaultManifest> {
        let bytes = fs::read(self.manifest_path()).context("private album manifest is missing")?;
        let encrypted: EncryptedBlob =
            serde_json::from_slice(&bytes).context("private album manifest is invalid")?;
        let plaintext = Self::decrypt_blob(master_key, &encrypted, MANIFEST_AAD)?;
        let manifest: VaultManifest = serde_json::from_slice(plaintext.as_slice())
            .context("private album manifest cannot be decoded")?;
        if manifest.version != VAULT_VERSION {
            bail!("unsupported private album manifest version");
        }
        Ok(manifest)
    }

    fn session_key<T>(&self, operation: impl FnOnce(&[u8]) -> Result<T>) -> Result<T> {
        let mut guard = self
            .session
            .lock()
            .map_err(|_| anyhow!("private album session is unavailable"))?;
        let expired = guard
            .as_ref()
            .map(|session| session.last_activity.elapsed() >= session.auto_lock)
            .unwrap_or(false);
        if expired {
            *guard = None;
            bail!("私密相册已自动锁定");
        }
        let session = guard.as_mut().ok_or_else(|| anyhow!("私密相册尚未解锁"))?;
        session.last_activity = Instant::now();
        operation(session.master_key.as_slice())
    }

    fn install_session(&self, master_key: Vec<u8>, auto_lock_seconds: u64) -> Result<()> {
        if master_key.len() != MASTER_KEY_BYTES {
            bail!("private album master key is invalid");
        }
        let mut guard = self
            .session
            .lock()
            .map_err(|_| anyhow!("private album session is unavailable"))?;
        *guard = Some(VaultSession {
            master_key: Zeroizing::new(master_key),
            last_activity: Instant::now(),
            auto_lock: Duration::from_secs(auto_lock_seconds.max(30)),
        });
        Ok(())
    }

    fn lock_internal(&self) -> Result<()> {
        let mut guard = self
            .session
            .lock()
            .map_err(|_| anyhow!("private album session is unavailable"))?;
        *guard = None;
        Ok(())
    }

    fn is_unlocked(&self) -> bool {
        let Ok(mut guard) = self.session.lock() else {
            return false;
        };
        let expired = guard
            .as_ref()
            .map(|session| session.last_activity.elapsed() >= session.auto_lock)
            .unwrap_or(false);
        if expired {
            *guard = None;
            return false;
        }
        guard.is_some()
    }

    fn initialize(&self, password: &str) -> Result<VaultStatus> {
        Self::validate_password(password)?;
        if self.config_path().exists() {
            bail!("私密相册已经创建");
        }
        self.ensure_directories()?;

        let kdf = Self::new_kdf_config();
        let mut master_key = Zeroizing::new(vec![0_u8; MASTER_KEY_BYTES]);
        OsRng.fill_bytes(master_key.as_mut_slice());
        let kek = Self::derive_kek(password, &kdf)?;
        let wrapped_master_key =
            Self::encrypt_blob(kek.as_slice(), master_key.as_slice(), MASTER_KEY_AAD)?;
        let now = Utc::now().to_rfc3339();
        let config = VaultConfig {
            version: VAULT_VERSION,
            kdf,
            wrapped_master_key,
            auto_lock_seconds: DEFAULT_AUTO_LOCK_SECONDS,
            lock_on_blur: false,
            created_at: now.clone(),
            updated_at: now,
        };

        self.save_manifest(master_key.as_slice(), &VaultManifest::default())?;
        if let Err(error) = self.write_json_atomic(&self.config_path(), &config) {
            let _ = fs::remove_file(self.manifest_path());
            return Err(error);
        }
        self.install_session(master_key.to_vec(), config.auto_lock_seconds)?;
        self.reset_attempts();
        self.status()
    }

    fn unlock(&self, password: &str) -> Result<VaultStatus> {
        let config = self.read_config()?;
        let result = (|| {
            let kek = Self::derive_kek(password, &config.kdf)?;
            let mut master_key =
                Self::decrypt_blob(kek.as_slice(), &config.wrapped_master_key, MASTER_KEY_AAD)?;
            if master_key.len() != MASTER_KEY_BYTES {
                bail!("密码错误或私密相册数据无法验证");
            }
            let key = master_key.to_vec();
            master_key.zeroize();
            self.install_session(key, config.auto_lock_seconds)?;
            self.session_key(|key| self.load_manifest(key).map(|_| ()))?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                self.reset_attempts();
                self.status()
            }
            Err(_) => {
                let _ = self.lock_internal();
                self.record_failure();
                bail!("密码错误或私密相册数据无法验证")
            }
        }
    }

    fn reset_attempts(&self) {
        if let Ok(mut attempts) = self.attempts.lock() {
            *attempts = AttemptState::default();
        }
    }

    fn record_failure(&self) {
        if let Ok(mut attempts) = self.attempts.lock() {
            attempts.failures = attempts.failures.saturating_add(1);
            let seconds = match attempts.failures {
                0..=3 => 0,
                4 => 5,
                5 => 15,
                6 => 30,
                count => ((count - 5) as u64 * 30).min(300),
            };
            attempts.blocked_until = if seconds == 0 {
                None
            } else {
                Some(Instant::now() + Duration::from_secs(seconds))
            };
        }
    }

    fn remaining_attempt_delay(&self) -> Duration {
        let Ok(attempts) = self.attempts.lock() else {
            return Duration::ZERO;
        };
        attempts
            .blocked_until
            .and_then(|deadline| deadline.checked_duration_since(Instant::now()))
            .unwrap_or(Duration::ZERO)
    }

    fn status(&self) -> Result<VaultStatus> {
        let configured = self.config_path().exists();
        if !configured {
            return Ok(VaultStatus {
                configured: false,
                unlocked: false,
                asset_count: None,
                trash_count: None,
                album_count: None,
                auto_lock_seconds: DEFAULT_AUTO_LOCK_SECONDS,
                lock_on_blur: false,
            });
        }
        let config = self.read_config()?;
        let unlocked = self.is_unlocked();
        let (asset_count, trash_count, album_count) = if unlocked {
            self.session_key(|key| {
                let manifest = self.load_manifest(key)?;
                Ok((
                    Some(
                        manifest
                            .assets
                            .iter()
                            .filter(|asset| asset.state == VaultAssetState::Active)
                            .count(),
                    ),
                    Some(
                        manifest
                            .assets
                            .iter()
                            .filter(|asset| asset.state == VaultAssetState::Trash)
                            .count(),
                    ),
                    Some(manifest.albums.len()),
                ))
            })?
        } else {
            (None, None, None)
        };
        Ok(VaultStatus {
            configured,
            unlocked,
            asset_count,
            trash_count,
            album_count,
            auto_lock_seconds: config.auto_lock_seconds,
            lock_on_blur: config.lock_on_blur,
        })
    }

    fn list_assets(&self, trashed: bool, album_id: Option<&str>) -> Result<Vec<VaultAsset>> {
        self.session_key(|key| {
            let expected = if trashed {
                VaultAssetState::Trash
            } else {
                VaultAssetState::Active
            };
            let mut assets: Vec<_> = self
                .load_manifest(key)?
                .assets
                .into_iter()
                .filter(|asset| asset.state == expected)
                .filter(|asset| {
                    album_id
                        .map(|id| asset.album_ids.iter().any(|candidate| candidate == id))
                        .unwrap_or(true)
                })
                .collect();
            assets.sort_by(|left, right| right.imported_at.cmp(&left.imported_at));
            Ok(assets)
        })
    }

    fn list_albums(&self) -> Result<Vec<VaultAlbum>> {
        self.session_key(|key| {
            let mut albums = self.load_manifest(key)?.albums;
            albums.sort_by(|left, right| left.name.cmp(&right.name));
            Ok(albums)
        })
    }

    fn object_path(&self, asset_id: &str) -> PathBuf {
        self.objects_dir().join(format!("{asset_id}.vlt"))
    }

    fn thumbnail_path(&self, asset_id: &str) -> PathBuf {
        self.thumbnails_dir().join(format!("{asset_id}.vlt"))
    }

    fn object_aad(asset_id: &str, index: u32, plaintext_len: usize) -> Vec<u8> {
        format!("lifetrace-vault-object-v1:{asset_id}:{index}:{plaintext_len}").into_bytes()
    }

    fn encrypt_file(
        &self,
        source: &Path,
        target: &Path,
        asset_id: &str,
        master_key: &[u8],
    ) -> Result<()> {
        let cipher =
            Aes256Gcm::new_from_slice(master_key).context("invalid private album master key")?;
        let mut nonce_prefix = [0_u8; 8];
        OsRng.fill_bytes(&mut nonce_prefix);
        let temporary = self.temp_dir().join(format!("{asset_id}.partial"));
        let result = (|| {
            let mut input = File::open(source).context("failed to open selected file")?;
            let mut output =
                File::create(&temporary).context("failed to create encrypted private file")?;
            output.write_all(OBJECT_MAGIC)?;
            output.write_all(&(OBJECT_CHUNK_BYTES as u32).to_be_bytes())?;
            output.write_all(&nonce_prefix)?;
            let mut buffer = Zeroizing::new(vec![0_u8; OBJECT_CHUNK_BYTES]);
            let mut index = 0_u32;
            loop {
                let read = input
                    .read(buffer.as_mut_slice())
                    .context("failed to read selected file")?;
                if read == 0 {
                    break;
                }
                let mut nonce = [0_u8; NONCE_BYTES];
                nonce[..8].copy_from_slice(&nonce_prefix);
                nonce[8..].copy_from_slice(&index.to_be_bytes());
                let aad = Self::object_aad(asset_id, index, read);
                let ciphertext = cipher
                    .encrypt(
                        Nonce::from_slice(&nonce),
                        Payload {
                            msg: &buffer[..read],
                            aad: &aad,
                        },
                    )
                    .map_err(|_| anyhow!("failed to encrypt selected file"))?;
                output.write_all(&(read as u32).to_be_bytes())?;
                output.write_all(&(ciphertext.len() as u32).to_be_bytes())?;
                output.write_all(&ciphertext)?;
                index = index
                    .checked_add(1)
                    .ok_or_else(|| anyhow!("selected file is too large"))?;
            }
            output
                .sync_all()
                .context("failed to flush encrypted private file")?;
            if target.exists() {
                fs::remove_file(target).context("failed to replace encrypted private file")?;
            }
            fs::rename(&temporary, target).context("failed to commit encrypted private file")?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    fn process_encrypted_file(
        &self,
        source: &Path,
        asset_id: &str,
        master_key: &[u8],
        maximum_bytes: Option<u64>,
        mut consume: impl FnMut(&[u8]) -> Result<()>,
    ) -> Result<u64> {
        let cipher =
            Aes256Gcm::new_from_slice(master_key).context("invalid private album master key")?;
        let mut input = File::open(source).context("encrypted private file is missing")?;
        let mut magic = [0_u8; 8];
        input
            .read_exact(&mut magic)
            .context("encrypted private file header is incomplete")?;
        if &magic != OBJECT_MAGIC {
            bail!("encrypted private file format is invalid");
        }
        let mut integer = [0_u8; 4];
        input.read_exact(&mut integer)?;
        let chunk_size = u32::from_be_bytes(integer) as usize;
        if chunk_size == 0 || chunk_size > OBJECT_CHUNK_BYTES {
            bail!("encrypted private file chunk size is invalid");
        }
        let mut nonce_prefix = [0_u8; 8];
        input.read_exact(&mut nonce_prefix)?;
        let mut total = 0_u64;
        let mut index = 0_u32;
        loop {
            let mut first = [0_u8; 1];
            match input.read(&mut first) {
                Ok(0) => break,
                Ok(1) => {}
                Ok(_) => unreachable!(),
                Err(error) => return Err(error).context("failed to read encrypted private file"),
            }
            let mut plain_length_bytes = [0_u8; 4];
            plain_length_bytes[0] = first[0];
            input
                .read_exact(&mut plain_length_bytes[1..])
                .context("encrypted private file is truncated")?;
            let plaintext_len = u32::from_be_bytes(plain_length_bytes) as usize;
            input
                .read_exact(&mut integer)
                .context("encrypted private file is truncated")?;
            let ciphertext_len = u32::from_be_bytes(integer) as usize;
            if plaintext_len == 0
                || plaintext_len > chunk_size
                || ciphertext_len != plaintext_len + 16
            {
                bail!("encrypted private file chunk is invalid");
            }
            total = total
                .checked_add(plaintext_len as u64)
                .ok_or_else(|| anyhow!("private file is too large"))?;
            if maximum_bytes.is_some_and(|maximum| total > maximum) {
                bail!("private file is too large to preview safely");
            }
            let mut ciphertext = vec![0_u8; ciphertext_len];
            input
                .read_exact(&mut ciphertext)
                .context("encrypted private file is truncated")?;
            let mut nonce = [0_u8; NONCE_BYTES];
            nonce[..8].copy_from_slice(&nonce_prefix);
            nonce[8..].copy_from_slice(&index.to_be_bytes());
            let aad = Self::object_aad(asset_id, index, plaintext_len);
            let mut plaintext = cipher
                .decrypt(
                    Nonce::from_slice(&nonce),
                    Payload {
                        msg: &ciphertext,
                        aad: &aad,
                    },
                )
                .map_err(|_| anyhow!("private file integrity verification failed"))?;
            ciphertext.zeroize();
            if plaintext.len() != plaintext_len {
                plaintext.zeroize();
                bail!("private file chunk length is invalid");
            }
            let consume_result = consume(&plaintext);
            plaintext.zeroize();
            consume_result?;
            index = index
                .checked_add(1)
                .ok_or_else(|| anyhow!("private file is too large"))?;
        }
        Ok(total)
    }

    fn decrypt_file(
        &self,
        source: &Path,
        asset_id: &str,
        master_key: &[u8],
        maximum_bytes: u64,
    ) -> Result<Zeroizing<Vec<u8>>> {
        let mut output = Zeroizing::new(Vec::new());
        self.process_encrypted_file(source, asset_id, master_key, Some(maximum_bytes), |chunk| {
            output.extend_from_slice(chunk);
            Ok(())
        })?;
        Ok(output)
    }

    fn verify_encrypted_file(
        &self,
        source: &Path,
        asset_id: &str,
        master_key: &[u8],
        expected_size: u64,
    ) -> Result<()> {
        let actual = self.process_encrypted_file(source, asset_id, master_key, None, |_| Ok(()))?;
        if actual != expected_size {
            bail!("private file size does not match encrypted metadata");
        }
        Ok(())
    }

    fn decrypt_to_file(
        &self,
        source: &Path,
        target: &Path,
        asset_id: &str,
        master_key: &[u8],
        expected_size: u64,
    ) -> Result<()> {
        let mut output = File::create(target).context("failed to create exported private file")?;
        let result = self.process_encrypted_file(source, asset_id, master_key, None, |chunk| {
            output
                .write_all(chunk)
                .context("failed to write exported private file")
        });
        match result {
            Ok(actual) if actual == expected_size => {
                output
                    .sync_all()
                    .context("failed to flush exported private file")?;
                Ok(())
            }
            Ok(_) => bail!("private file size does not match encrypted metadata"),
            Err(error) => Err(error),
        }
    }

    fn create_thumbnail(&self, source: &Path, asset_id: &str, master_key: &[u8]) -> Result<bool> {
        let image = match image::open(source) {
            Ok(image) => image,
            Err(_) => return Ok(false),
        };
        let thumbnail = image.thumbnail(480, 480);
        let mut cursor = Cursor::new(Vec::new());
        thumbnail
            .write_to(&mut cursor, image::ImageFormat::Png)
            .context("failed to encode private thumbnail")?;
        let bytes = Zeroizing::new(cursor.into_inner());
        let aad = format!("lifetrace-vault-thumbnail-v1:{asset_id}");
        let encrypted = Self::encrypt_blob(master_key, bytes.as_slice(), aad.as_bytes())?;
        self.write_json_atomic(&self.thumbnail_path(asset_id), &encrypted)?;
        Ok(true)
    }

    fn import_files(
        &self,
        source_paths: Vec<String>,
        move_source: bool,
        album_id: Option<String>,
    ) -> Result<Vec<VaultAsset>> {
        if source_paths.is_empty() {
            return Ok(Vec::new());
        }
        self.ensure_directories()?;
        self.session_key(|master_key| {
            let mut manifest = self.load_manifest(master_key)?;
            if let Some(id) = album_id.as_deref() {
                if !manifest.albums.iter().any(|album| album.id == id) {
                    bail!("private album was not found");
                }
            }
            let mut imported = Vec::new();
            for source_path in source_paths {
                let source = PathBuf::from(&source_path);
                let metadata = fs::metadata(&source)
                    .with_context(|| format!("无法读取所选文件：{source_path}"))?;
                if !metadata.is_file() {
                    bail!("所选路径不是文件：{source_path}");
                }
                if source
                    .canonicalize()
                    .ok()
                    .is_some_and(|path| path.starts_with(&self.root))
                {
                    bail!("不能从私密相册内部目录重复导入文件");
                }
                let original_name = source
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| anyhow!("所选文件名无效"))?
                    .to_string();
                let asset_id = Uuid::new_v4().to_string();
                let target = self.object_path(&asset_id);
                self.encrypt_file(&source, &target, &asset_id, master_key)?;
                if let Err(error) =
                    self.verify_encrypted_file(&target, &asset_id, master_key, metadata.len())
                {
                    let _ = fs::remove_file(&target);
                    return Err(error);
                }
                let mime_type = mime_guess::from_path(&source)
                    .first_or_octet_stream()
                    .essence_str()
                    .to_string();
                let has_thumbnail = if mime_type.starts_with("image/") {
                    self.create_thumbnail(&source, &asset_id, master_key)
                        .unwrap_or(false)
                } else {
                    false
                };
                let asset = VaultAsset {
                    id: asset_id.clone(),
                    original_name,
                    mime_type,
                    size: metadata.len(),
                    imported_at: Utc::now().to_rfc3339(),
                    has_thumbnail,
                    state: VaultAssetState::Active,
                    album_ids: album_id.iter().cloned().collect(),
                    deleted_at: None,
                };
                let previous_manifest = manifest.clone();
                manifest.assets.push(asset.clone());
                if let Err(error) = self.save_manifest(master_key, &manifest) {
                    let _ = fs::remove_file(&target);
                    let _ = fs::remove_file(self.thumbnail_path(&asset_id));
                    return Err(error);
                }
                if move_source {
                    if let Err(error) = fs::remove_file(&source) {
                        manifest = previous_manifest;
                        let rollback = self.save_manifest(master_key, &manifest);
                        let _ = fs::remove_file(&target);
                        let _ = fs::remove_file(self.thumbnail_path(&asset_id));
                        rollback?;
                        return Err(error).context("加密成功，但无法删除原文件，已回滚此次移入");
                    }
                }
                imported.push(asset);
            }
            Ok(imported)
        })
    }

    fn read_asset(&self, asset_id: &str) -> Result<VaultAssetPayload> {
        self.session_key(|master_key| {
            let manifest = self.load_manifest(master_key)?;
            let asset = manifest
                .assets
                .into_iter()
                .find(|asset| asset.id == asset_id)
                .ok_or_else(|| anyhow!("private asset was not found"))?;
            let bytes = self.decrypt_file(
                &self.object_path(asset_id),
                asset_id,
                master_key,
                MAX_PREVIEW_BYTES,
            )?;
            Ok(VaultAssetPayload {
                asset,
                data_base64: BASE64.encode(bytes.as_slice()),
            })
        })
    }

    fn read_thumbnail(&self, asset_id: &str) -> Result<VaultThumbnailPayload> {
        self.session_key(|master_key| {
            let manifest = self.load_manifest(master_key)?;
            let asset = manifest
                .assets
                .iter()
                .find(|asset| asset.id == asset_id && asset.has_thumbnail)
                .ok_or_else(|| anyhow!("private thumbnail was not found"))?;
            let bytes =
                fs::read(self.thumbnail_path(asset_id)).context("private thumbnail is missing")?;
            let encrypted: EncryptedBlob =
                serde_json::from_slice(&bytes).context("private thumbnail is invalid")?;
            let aad = format!("lifetrace-vault-thumbnail-v1:{asset_id}");
            let plaintext = Self::decrypt_blob(master_key, &encrypted, aad.as_bytes())?;
            Ok(VaultThumbnailPayload {
                asset_id: asset.id.clone(),
                mime_type: "image/png".to_string(),
                data_base64: BASE64.encode(plaintext.as_slice()),
            })
        })
    }

    fn unique_export_path(directory: &Path, original_name: &str) -> Result<PathBuf> {
        let safe_name = Path::new(original_name)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("private asset filename is invalid"))?;
        let direct = directory.join(safe_name);
        if !direct.exists() {
            return Ok(direct);
        }
        let path = Path::new(safe_name);
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("file");
        let extension = path.extension().and_then(|value| value.to_str());
        for index in 1..=9_999 {
            let name = match extension {
                Some(extension) => format!("{stem} ({index}).{extension}"),
                None => format!("{stem} ({index})"),
            };
            let candidate = directory.join(name);
            if !candidate.exists() {
                return Ok(candidate);
            }
        }
        bail!("无法为导出文件生成不冲突的文件名")
    }

    fn export_asset(
        &self,
        asset_id: &str,
        target_directory: &str,
        remove_from_vault: bool,
    ) -> Result<String> {
        let directory = PathBuf::from(target_directory);
        if !directory.is_dir() {
            bail!("请选择有效的本地导出目录");
        }
        self.session_key(|master_key| {
            let mut manifest = self.load_manifest(master_key)?;
            let asset = manifest
                .assets
                .iter()
                .find(|asset| asset.id == asset_id && asset.state == VaultAssetState::Active)
                .cloned()
                .ok_or_else(|| anyhow!("private asset was not found"))?;
            let target = Self::unique_export_path(&directory, &asset.original_name)?;
            let temporary = directory.join(format!(".lifetrace-{}.partial", Uuid::new_v4()));
            let result = self.decrypt_to_file(
                &self.object_path(asset_id),
                &temporary,
                asset_id,
                master_key,
                asset.size,
            );
            if let Err(error) = result {
                let _ = fs::remove_file(&temporary);
                return Err(error);
            }
            if let Err(error) = fs::rename(&temporary, &target) {
                let _ = fs::remove_file(&temporary);
                return Err(error).context("failed to commit exported private file");
            }
            if remove_from_vault {
                manifest.assets.retain(|candidate| candidate.id != asset_id);
                if let Err(error) = self.save_manifest(master_key, &manifest) {
                    let _ = fs::remove_file(&target);
                    return Err(error);
                }
                let _ = fs::remove_file(self.object_path(asset_id));
                let _ = fs::remove_file(self.thumbnail_path(asset_id));
            }
            Ok(target.to_string_lossy().into_owned())
        })
    }

    fn move_to_trash(&self, asset_id: &str) -> Result<()> {
        self.session_key(|master_key| {
            let mut manifest = self.load_manifest(master_key)?;
            let asset = manifest
                .assets
                .iter_mut()
                .find(|asset| asset.id == asset_id)
                .ok_or_else(|| anyhow!("private asset was not found"))?;
            asset.state = VaultAssetState::Trash;
            asset.deleted_at = Some(Utc::now().to_rfc3339());
            self.save_manifest(master_key, &manifest)
        })
    }

    fn restore_asset(&self, asset_id: &str) -> Result<()> {
        self.session_key(|master_key| {
            let mut manifest = self.load_manifest(master_key)?;
            let asset = manifest
                .assets
                .iter_mut()
                .find(|asset| asset.id == asset_id && asset.state == VaultAssetState::Trash)
                .ok_or_else(|| anyhow!("private trash item was not found"))?;
            asset.state = VaultAssetState::Active;
            asset.deleted_at = None;
            self.save_manifest(master_key, &manifest)
        })
    }

    fn delete_asset_permanently(&self, asset_id: &str) -> Result<()> {
        self.session_key(|master_key| {
            let mut manifest = self.load_manifest(master_key)?;
            let index = manifest
                .assets
                .iter()
                .position(|asset| asset.id == asset_id && asset.state == VaultAssetState::Trash)
                .ok_or_else(|| anyhow!("private trash item was not found"))?;
            manifest.assets.remove(index);
            self.save_manifest(master_key, &manifest)?;
            let _ = fs::remove_file(self.object_path(asset_id));
            let _ = fs::remove_file(self.thumbnail_path(asset_id));
            Ok(())
        })
    }

    fn create_album(&self, name: &str) -> Result<VaultAlbum> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 80 {
            bail!("子相册名称需要 1 到 80 个字符");
        }
        self.session_key(|master_key| {
            let mut manifest = self.load_manifest(master_key)?;
            if manifest.albums.iter().any(|album| album.name == name) {
                bail!("已存在同名私密子相册");
            }
            let album = VaultAlbum {
                id: Uuid::new_v4().to_string(),
                name: name.to_string(),
                created_at: Utc::now().to_rfc3339(),
            };
            manifest.albums.push(album.clone());
            self.save_manifest(master_key, &manifest)?;
            Ok(album)
        })
    }

    fn rename_album(&self, album_id: &str, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 80 {
            bail!("子相册名称需要 1 到 80 个字符");
        }
        self.session_key(|master_key| {
            let mut manifest = self.load_manifest(master_key)?;
            if manifest
                .albums
                .iter()
                .any(|album| album.id != album_id && album.name == name)
            {
                bail!("已存在同名私密子相册");
            }
            let album = manifest
                .albums
                .iter_mut()
                .find(|album| album.id == album_id)
                .ok_or_else(|| anyhow!("private album was not found"))?;
            album.name = name.to_string();
            self.save_manifest(master_key, &manifest)
        })
    }

    fn delete_album(&self, album_id: &str) -> Result<()> {
        self.session_key(|master_key| {
            let mut manifest = self.load_manifest(master_key)?;
            let before = manifest.albums.len();
            manifest.albums.retain(|album| album.id != album_id);
            if manifest.albums.len() == before {
                bail!("private album was not found");
            }
            for asset in &mut manifest.assets {
                asset.album_ids.retain(|id| id != album_id);
            }
            self.save_manifest(master_key, &manifest)
        })
    }

    fn set_asset_album(&self, asset_id: &str, album_id: &str, assigned: bool) -> Result<()> {
        self.session_key(|master_key| {
            let mut manifest = self.load_manifest(master_key)?;
            if !manifest.albums.iter().any(|album| album.id == album_id) {
                bail!("private album was not found");
            }
            let asset = manifest
                .assets
                .iter_mut()
                .find(|asset| asset.id == asset_id)
                .ok_or_else(|| anyhow!("private asset was not found"))?;
            if assigned {
                if !asset.album_ids.iter().any(|id| id == album_id) {
                    asset.album_ids.push(album_id.to_string());
                }
            } else {
                asset.album_ids.retain(|id| id != album_id);
            }
            self.save_manifest(master_key, &manifest)
        })
    }

    fn verify_integrity(&self) -> Result<VaultIntegrityReport> {
        self.session_key(|master_key| {
            let manifest = self.load_manifest(master_key)?;
            let mut corrupted_asset_ids = Vec::new();
            for asset in &manifest.assets {
                let object_result = self.verify_encrypted_file(
                    &self.object_path(&asset.id),
                    &asset.id,
                    master_key,
                    asset.size,
                );
                let thumbnail_result = if asset.has_thumbnail {
                    (|| {
                        let bytes = fs::read(self.thumbnail_path(&asset.id))?;
                        let encrypted: EncryptedBlob = serde_json::from_slice(&bytes)?;
                        let aad = format!("lifetrace-vault-thumbnail-v1:{}", asset.id);
                        Self::decrypt_blob(master_key, &encrypted, aad.as_bytes()).map(|_| ())
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

    fn change_password(&self, old_password: &str, new_password: &str) -> Result<VaultStatus> {
        Self::validate_password(new_password)?;
        let mut config = self.read_config()?;
        let old_kek = Self::derive_kek(old_password, &config.kdf)?;
        let master_key = Self::decrypt_blob(
            old_kek.as_slice(),
            &config.wrapped_master_key,
            MASTER_KEY_AAD,
        )
        .map_err(|_| anyhow!("当前密码错误"))?;
        if master_key.len() != MASTER_KEY_BYTES {
            bail!("当前密码错误");
        }
        self.load_manifest(master_key.as_slice())?;
        let new_kdf = Self::new_kdf_config();
        let new_kek = Self::derive_kek(new_password, &new_kdf)?;
        let wrapped_master_key =
            Self::encrypt_blob(new_kek.as_slice(), master_key.as_slice(), MASTER_KEY_AAD)?;
        config.kdf = new_kdf;
        config.wrapped_master_key = wrapped_master_key;
        config.updated_at = Utc::now().to_rfc3339();
        self.write_json_atomic(&self.config_path(), &config)?;
        self.install_session(master_key.to_vec(), config.auto_lock_seconds)?;
        self.reset_attempts();
        self.status()
    }

    fn set_auto_lock(&self, seconds: u64) -> Result<VaultStatus> {
        let seconds = match seconds {
            30 | 60 | 300 | 600 | 1800 => seconds,
            _ => bail!("unsupported auto-lock interval"),
        };
        self.session_key(|_| Ok(()))?;
        let mut config = self.read_config()?;
        config.auto_lock_seconds = seconds;
        config.updated_at = Utc::now().to_rfc3339();
        self.write_json_atomic(&self.config_path(), &config)?;
        if let Ok(mut guard) = self.session.lock() {
            if let Some(session) = guard.as_mut() {
                session.auto_lock = Duration::from_secs(seconds);
                session.last_activity = Instant::now();
            }
        }
        self.status()
    }

    fn set_lock_on_blur(&self, enabled: bool) -> Result<VaultStatus> {
        self.session_key(|_| Ok(()))?;
        let mut config = self.read_config()?;
        config.lock_on_blur = enabled;
        config.updated_at = Utc::now().to_rfc3339();
        self.write_json_atomic(&self.config_path(), &config)?;
        self.status()
    }

    fn delete_all(&self, confirmation: &str) -> Result<()> {
        if confirmation != DELETE_CONFIRMATION {
            bail!("删除确认文本不正确");
        }
        self.lock_internal()?;
        if self.root.exists() {
            fs::remove_dir_all(&self.root).context("failed to delete private album")?;
        }
        self.reset_attempts();
        Ok(())
    }
}

fn command_result<T>(result: Result<T>) -> std::result::Result<T, String> {
    result.map_err(|error| error.to_string())
}

#[tauri::command]
pub fn vault_status(state: State<'_, VaultState>) -> std::result::Result<VaultStatus, String> {
    command_result(state.status())
}

#[tauri::command]
pub fn vault_initialize(
    password: String,
    state: State<'_, VaultState>,
) -> std::result::Result<VaultStatus, String> {
    command_result(state.initialize(&password))
}

#[tauri::command]
pub async fn vault_unlock(
    password: String,
    state: State<'_, VaultState>,
) -> std::result::Result<VaultStatus, String> {
    let delay = state.remaining_attempt_delay();
    if !delay.is_zero() {
        tokio::time::sleep(delay).await;
    }
    command_result(state.unlock(&password))
}

#[tauri::command]
pub fn vault_lock(state: State<'_, VaultState>) -> std::result::Result<VaultStatus, String> {
    command_result(state.lock_internal().and_then(|_| state.status()))
}

#[tauri::command]
pub fn vault_list_assets(
    trashed: bool,
    album_id: Option<String>,
    state: State<'_, VaultState>,
) -> std::result::Result<Vec<VaultAsset>, String> {
    command_result(state.list_assets(trashed, album_id.as_deref()))
}

#[tauri::command]
pub fn vault_list_albums(
    state: State<'_, VaultState>,
) -> std::result::Result<Vec<VaultAlbum>, String> {
    command_result(state.list_albums())
}

#[tauri::command]
pub fn vault_import_files(
    source_paths: Vec<String>,
    move_source: bool,
    album_id: Option<String>,
    state: State<'_, VaultState>,
) -> std::result::Result<Vec<VaultAsset>, String> {
    command_result(state.import_files(source_paths, move_source, album_id))
}

#[tauri::command]
pub fn vault_read_asset(
    asset_id: String,
    state: State<'_, VaultState>,
) -> std::result::Result<VaultAssetPayload, String> {
    command_result(state.read_asset(&asset_id))
}

#[tauri::command]
pub fn vault_read_thumbnail(
    asset_id: String,
    state: State<'_, VaultState>,
) -> std::result::Result<VaultThumbnailPayload, String> {
    command_result(state.read_thumbnail(&asset_id))
}

#[tauri::command]
pub fn vault_export_asset(
    asset_id: String,
    target_directory: String,
    remove_from_vault: bool,
    state: State<'_, VaultState>,
) -> std::result::Result<String, String> {
    command_result(state.export_asset(&asset_id, &target_directory, remove_from_vault))
}

#[tauri::command]
pub fn vault_move_to_trash(
    asset_id: String,
    state: State<'_, VaultState>,
) -> std::result::Result<(), String> {
    command_result(state.move_to_trash(&asset_id))
}

#[tauri::command]
pub fn vault_restore_asset(
    asset_id: String,
    state: State<'_, VaultState>,
) -> std::result::Result<(), String> {
    command_result(state.restore_asset(&asset_id))
}

#[tauri::command]
pub fn vault_delete_asset_permanently(
    asset_id: String,
    state: State<'_, VaultState>,
) -> std::result::Result<(), String> {
    command_result(state.delete_asset_permanently(&asset_id))
}

#[tauri::command]
pub fn vault_create_album(
    name: String,
    state: State<'_, VaultState>,
) -> std::result::Result<VaultAlbum, String> {
    command_result(state.create_album(&name))
}

#[tauri::command]
pub fn vault_rename_album(
    album_id: String,
    name: String,
    state: State<'_, VaultState>,
) -> std::result::Result<(), String> {
    command_result(state.rename_album(&album_id, &name))
}

#[tauri::command]
pub fn vault_delete_album(
    album_id: String,
    state: State<'_, VaultState>,
) -> std::result::Result<(), String> {
    command_result(state.delete_album(&album_id))
}

#[tauri::command]
pub fn vault_set_asset_album(
    asset_id: String,
    album_id: String,
    assigned: bool,
    state: State<'_, VaultState>,
) -> std::result::Result<(), String> {
    command_result(state.set_asset_album(&asset_id, &album_id, assigned))
}

#[tauri::command]
pub fn vault_verify_integrity(
    state: State<'_, VaultState>,
) -> std::result::Result<VaultIntegrityReport, String> {
    command_result(state.verify_integrity())
}

#[tauri::command]
pub fn vault_change_password(
    old_password: String,
    new_password: String,
    state: State<'_, VaultState>,
) -> std::result::Result<VaultStatus, String> {
    command_result(state.change_password(&old_password, &new_password))
}

#[tauri::command]
pub fn vault_set_auto_lock(
    seconds: u64,
    state: State<'_, VaultState>,
) -> std::result::Result<VaultStatus, String> {
    command_result(state.set_auto_lock(seconds))
}

#[tauri::command]
pub fn vault_set_lock_on_blur(
    enabled: bool,
    state: State<'_, VaultState>,
) -> std::result::Result<VaultStatus, String> {
    command_result(state.set_lock_on_blur(enabled))
}

#[tauri::command]
pub fn vault_delete_all(
    confirmation: String,
    state: State<'_, VaultState>,
) -> std::result::Result<(), String> {
    command_result(state.delete_all(&confirmation))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWORD: &str = "Correct-Horse-Battery-Staple";
    const NEW_PASSWORD: &str = "A-New-Private-Album-Password";

    fn test_state() -> (VaultState, PathBuf) {
        let workspace =
            std::env::temp_dir().join(format!("lifetrace-vault-test-{}", Uuid::new_v4()));
        let vault_root = workspace.join("vault");
        (VaultState::new(vault_root).expect("state"), workspace)
    }

    fn sample_file(root: &Path, name: &str, content: &[u8]) -> PathBuf {
        fs::create_dir_all(root).expect("sample directory");
        let path = root.join(name);
        fs::write(&path, content).expect("sample file");
        path
    }

    fn import_one(state: &VaultState, source: &Path) -> VaultAsset {
        state
            .import_files(vec![source.to_string_lossy().into_owned()], false, None)
            .expect("import")
            .remove(0)
    }

    #[test]
    fn initialization_requires_a_strong_password() {
        let (state, root) = test_state();
        assert!(state.initialize("123456").is_err());
        assert!(state.initialize("abcdefghijkl").is_ok());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn wrong_password_never_unlocks_the_vault() {
        let (state, root) = test_state();
        state.initialize(PASSWORD).expect("initialize");
        state.lock_internal().expect("lock");
        assert!(state.unlock("This-Password-Is-Wrong").is_err());
        assert!(!state.is_unlocked());
        assert!(state.unlock(PASSWORD).is_ok());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn imported_content_is_encrypted_and_round_trips() {
        let (state, root) = test_state();
        state.initialize(PASSWORD).expect("initialize");
        let plaintext = b"private image bytes that must not appear on disk";
        let source = sample_file(&root.join("source"), "secret-photo.jpg", plaintext);
        let asset = import_one(&state, &source);
        let encrypted = fs::read(state.object_path(&asset.id)).expect("encrypted object");
        assert!(!encrypted
            .windows(plaintext.len())
            .any(|window| window == plaintext));
        assert!(!encrypted
            .windows(b"secret-photo.jpg".len())
            .any(|window| window == b"secret-photo.jpg"));
        let manifest = fs::read(state.manifest_path()).expect("manifest");
        assert!(!manifest
            .windows(b"secret-photo.jpg".len())
            .any(|window| window == b"secret-photo.jpg"));
        let payload = state.read_asset(&asset.id).expect("read asset");
        assert_eq!(
            BASE64.decode(payload.data_base64).expect("base64"),
            plaintext
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn encrypting_the_same_file_twice_produces_distinct_ciphertext() {
        let (state, root) = test_state();
        state.initialize(PASSWORD).expect("initialize");
        let source = sample_file(&root.join("source"), "same.bin", b"same plaintext");
        let imported = state
            .import_files(
                vec![
                    source.to_string_lossy().into_owned(),
                    source.to_string_lossy().into_owned(),
                ],
                false,
                None,
            )
            .expect("import");
        let first = fs::read(state.object_path(&imported[0].id)).expect("first");
        let second = fs::read(state.object_path(&imported[1].id)).expect("second");
        assert_ne!(first, second);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn tampering_is_detected_before_plaintext_is_returned() {
        let (state, root) = test_state();
        state.initialize(PASSWORD).expect("initialize");
        let source = sample_file(&root.join("source"), "tamper.bin", b"authenticated content");
        let asset = import_one(&state, &source);
        let path = state.object_path(&asset.id);
        let mut bytes = fs::read(&path).expect("read");
        let last = bytes.len() - 1;
        bytes[last] ^= 0x55;
        fs::write(&path, bytes).expect("write");
        assert!(state.read_asset(&asset.id).is_err());
        let report = state.verify_integrity().expect("integrity report");
        assert_eq!(report.corrupted_asset_ids, vec![asset.id]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn lock_removes_access_to_metadata_and_content() {
        let (state, root) = test_state();
        state.initialize(PASSWORD).expect("initialize");
        state.lock_internal().expect("lock");
        let status = state.status().expect("status");
        assert!(!status.unlocked);
        assert_eq!(status.asset_count, None);
        assert!(state.list_assets(false, None).is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn move_import_removes_source_only_after_encrypted_commit() {
        let (state, root) = test_state();
        state.initialize(PASSWORD).expect("initialize");
        let source = sample_file(&root.join("source"), "move.bin", b"move me safely");
        let imported = state
            .import_files(vec![source.to_string_lossy().into_owned()], true, None)
            .expect("move import");
        assert!(!source.exists());
        assert_eq!(
            BASE64
                .decode(
                    state
                        .read_asset(&imported[0].id)
                        .expect("read moved asset")
                        .data_base64,
                )
                .expect("base64"),
            b"move me safely"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn export_round_trips_and_can_remove_from_vault() {
        let (state, root) = test_state();
        state.initialize(PASSWORD).expect("initialize");
        let source = sample_file(&root.join("source"), "export.bin", b"exported content");
        let asset = import_one(&state, &source);
        let export_dir = root.join("export");
        fs::create_dir_all(&export_dir).expect("export directory");
        let exported = state
            .export_asset(&asset.id, &export_dir.to_string_lossy(), true)
            .expect("export");
        assert_eq!(
            fs::read(exported).expect("exported file"),
            b"exported content"
        );
        assert!(state.list_assets(false, None).expect("assets").is_empty());
        assert!(!state.object_path(&asset.id).exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn trash_restore_and_permanent_delete_are_separate() {
        let (state, root) = test_state();
        state.initialize(PASSWORD).expect("initialize");
        let source = sample_file(&root.join("source"), "trash.bin", b"trash content");
        let asset = import_one(&state, &source);
        state.move_to_trash(&asset.id).expect("trash");
        assert!(state.list_assets(false, None).expect("active").is_empty());
        assert_eq!(state.list_assets(true, None).expect("trash").len(), 1);
        state.restore_asset(&asset.id).expect("restore");
        assert_eq!(state.list_assets(false, None).expect("active").len(), 1);
        state.move_to_trash(&asset.id).expect("trash again");
        state
            .delete_asset_permanently(&asset.id)
            .expect("permanent delete");
        assert!(!state.object_path(&asset.id).exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn album_names_and_membership_are_encrypted() {
        let (state, root) = test_state();
        state.initialize(PASSWORD).expect("initialize");
        let album = state.create_album("Only In Ciphertext").expect("album");
        let source = sample_file(&root.join("source"), "album.bin", b"album content");
        let asset = import_one(&state, &source);
        state
            .set_asset_album(&asset.id, &album.id, true)
            .expect("assign album");
        assert_eq!(
            state
                .list_assets(false, Some(&album.id))
                .expect("album assets")
                .len(),
            1
        );
        let manifest = fs::read(state.manifest_path()).expect("manifest");
        assert!(!manifest
            .windows(b"Only In Ciphertext".len())
            .any(|window| window == b"Only In Ciphertext"));
        state.delete_album(&album.id).expect("delete album");
        assert!(state.list_albums().expect("albums").is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn importing_from_the_vault_directory_is_rejected() {
        let (state, root) = test_state();
        state.initialize(PASSWORD).expect("initialize");
        let source = sample_file(
            &root.join("vault").join("nested"),
            "already-private.bin",
            b"must not be imported recursively",
        );
        assert!(state
            .import_files(vec![source.to_string_lossy().into_owned()], false, None)
            .is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn temporary_files_are_removed_on_startup() {
        let root = std::env::temp_dir().join(format!("lifetrace-vault-test-{}", Uuid::new_v4()));
        let temp = root.join("temp");
        fs::create_dir_all(&temp).expect("temp");
        fs::write(temp.join("leaked.partial"), b"temporary plaintext").expect("temporary");
        let _state = VaultState::new(root.clone()).expect("state");
        assert!(!temp.join("leaked.partial").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn password_change_rewraps_the_master_key_without_reencrypting_assets() {
        let (state, root) = test_state();
        state.initialize(PASSWORD).expect("initialize");
        let source = sample_file(&root.join("source"), "preserved.bin", b"preserved content");
        let asset = import_one(&state, &source);
        let before = fs::read(state.object_path(&asset.id)).expect("before");
        state
            .change_password(PASSWORD, NEW_PASSWORD)
            .expect("change password");
        let after = fs::read(state.object_path(&asset.id)).expect("after");
        assert_eq!(before, after);
        state.lock_internal().expect("lock");
        assert!(state.unlock(PASSWORD).is_err());
        assert!(state.unlock(NEW_PASSWORD).is_ok());
        assert_eq!(
            BASE64
                .decode(state.read_asset(&asset.id).expect("read").data_base64)
                .expect("base64"),
            b"preserved content"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn delete_all_requires_the_exact_irreversible_confirmation() {
        let (state, root) = test_state();
        state.initialize(PASSWORD).expect("initialize");
        assert!(state.delete_all("delete").is_err());
        assert!(state.config_path().exists());
        state.delete_all(DELETE_CONFIRMATION).expect("delete vault");
        assert!(!root.join("vault").exists());
    }
}
