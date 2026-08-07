use std::sync::OnceLock;

const MIGRATION_ASSET_KEY_BYTES: usize = 32;
const MIGRATION_ASSET_KEY_AAD_PREFIX: &str = "lifetrace-vault-asset-key-v1";
const MIGRATION_META_AAD_PREFIX: &str = "lifetrace-vault-migration-meta-v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VaultMigrationState {
    Queued,
    Encrypting,
    Verifying,
    Committing,
    Encrypted,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultAssetView {
    pub id: String,
    pub original_name: String,
    pub mime_type: String,
    pub size: u64,
    pub imported_at: String,
    pub has_thumbnail: bool,
    pub state: VaultAssetState,
    pub album_ids: Vec<String>,
    pub deleted_at: Option<String>,
    pub migration_state: Option<VaultMigrationState>,
}

impl From<VaultAsset> for VaultAssetView {
    fn from(asset: VaultAsset) -> Self {
        Self {
            id: asset.id,
            original_name: asset.original_name,
            mime_type: asset.mime_type,
            size: asset.size,
            imported_at: asset.imported_at,
            has_thumbnail: asset.has_thumbnail,
            state: asset.state,
            album_ids: asset.album_ids,
            deleted_at: asset.deleted_at,
            migration_state: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VaultMigrationTask {
    asset_id: String,
    photo_id: String,
    size: u64,
    state: VaultMigrationState,
    created_at: String,
}

struct MigrationWork {
    asset_id: String,
    photo_id: String,
    asset_key: Zeroizing<Vec<u8>>,
}

struct PhotoSourceInfo {
    source_path: PathBuf,
    thumbnail_path: Option<PathBuf>,
    original_name: String,
    mime_type: String,
}

static MIGRATION_WORKER: OnceLock<Mutex<()>> = OnceLock::new();

fn migration_worker() -> &'static Mutex<()> {
    MIGRATION_WORKER.get_or_init(|| Mutex::new(()))
}

