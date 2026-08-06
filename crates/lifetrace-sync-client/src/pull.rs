use lifetrace_contracts::sync::v1::{PullRequestV1, SyncClientInfo};
use lifetrace_contracts::{EntityType, RequestId};

use crate::{ApplyPageResult, LocalProfileId, SyncError, SyncScope, SyncStore, SyncTransport};

pub(crate) async fn run_pull<T: SyncTransport, S: SyncStore>(
    transport: &T,
    store: &S,
    profile: &LocalProfileId,
    client: &SyncClientInfo,
    scope: &SyncScope,
    page_limit: u32,
) -> Result<ApplyPageResult, SyncError> {
    let mut total = ApplyPageResult::default();
    let mut after = store.cursor(profile, scope).await?;
    loop {
        let response = transport
            .pull(PullRequestV1 {
                request_id: RequestId::new(format!("pull-{}", profile.as_str())),
                client: client.clone(),
                after_cursor: after.clone(),
                limit: page_limit,
                entity_types: scope
                    .entity_types
                    .as_ref()
                    .map(|values| values.iter().map(EntityType::new).collect()),
            })
            .await?;
        let page = store.apply_pull_page(profile, scope, &response).await?;
        total.applied += page.applied;
        total.confirmed_local += page.confirmed_local;
        total.conflicts += page.conflicts;
        after = Some(response.next_cursor.clone());
        if !response.has_more {
            break;
        }
    }
    Ok(total)
}
