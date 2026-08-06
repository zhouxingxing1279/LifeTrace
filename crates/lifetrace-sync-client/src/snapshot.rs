use lifetrace_contracts::sync::v1::{SnapshotRequestV1, SyncClientInfo};
use lifetrace_contracts::{EntityType, RequestId, SnapshotId};

use crate::{LocalProfileId, SyncError, SyncScope, SyncStore, SyncTransport};

pub(crate) async fn run_snapshot<T: SyncTransport, S: SyncStore>(
    transport: &T,
    store: &S,
    profile: &LocalProfileId,
    client: &SyncClientInfo,
    scope: &SyncScope,
    page_size: u32,
) -> Result<(), SyncError> {
    let (mut snapshot_id, mut page_token) = store.snapshot_resume(profile, scope).await?;
    if snapshot_id.is_none() {
        store.begin_snapshot(profile, scope).await?;
    }
    loop {
        let response = transport
            .snapshot(SnapshotRequestV1 {
                request_id: RequestId::new(format!("snapshot-{}", profile.as_str())),
                client: client.clone(),
                snapshot_id: snapshot_id.as_ref().map(SnapshotId::new),
                page_token: page_token.clone(),
                entity_types: scope
                    .entity_types
                    .as_ref()
                    .map(|values| values.iter().map(EntityType::new).collect()),
                page_size,
            })
            .await?;
        store.stage_snapshot_page(profile, scope, &response).await?;
        snapshot_id = Some(response.snapshot_id.as_str().to_owned());
        page_token = response.next_page_token.clone();
        if response.completed {
            store
                .finalize_snapshot(profile, scope, &response.snapshot_cursor)
                .await?;
            break;
        }
    }
    Ok(())
}
