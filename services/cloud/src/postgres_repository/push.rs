mod execution_guard;
#[allow(unused_imports)]
mod process;

use std::collections::HashMap;

use axum::http::StatusCode;
use lifetrace_contracts::sync::v1::*;
use lifetrace_contracts::{ErrorCode, UserId};
use sqlx::{Acquire, Postgres, Transaction};
use uuid::Uuid;

use super::PostgresRepository;
use crate::error::ApiError;
use crate::sync::payload_hash::empty_scope;

impl PostgresRepository {
    async fn prune_change_log(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_uuid: Uuid,
    ) -> Result<(), ApiError> {
        if self.config.retention_entries == 0 {
            return Ok(());
        }
        sqlx::query(
            r#"
            DELETE FROM sync_change_log
            WHERE user_id = $1
              AND cursor < COALESCE((
                  SELECT cursor
                  FROM sync_change_log
                  WHERE user_id = $1
                  ORDER BY cursor DESC
                  OFFSET $2 LIMIT 1
              ), 0)
            "#,
        )
        .bind(user_uuid)
        .bind(self.config.retention_entries.saturating_sub(1) as i64)
        .execute(&mut **tx)
        .await
        .map_err(Self::db_error)?;
        Ok(())
    }

    fn group_indices(changes: &[SyncChangeV1]) -> Vec<Vec<usize>> {
        let mut grouped: HashMap<String, Vec<usize>> = HashMap::new();
        let mut order: Vec<GroupKey> = Vec::new();
        for (index, change) in changes.iter().enumerate() {
            match &change.atomic_group_id {
                Some(group_id) => {
                    let key = group_id.as_str().to_owned();
                    if !grouped.contains_key(&key) {
                        order.push(GroupKey::Atomic(key.clone()));
                    }
                    grouped.entry(key).or_default().push(index);
                }
                None => order.push(GroupKey::Single(index)),
            }
        }
        order
            .into_iter()
            .map(|key| match key {
                GroupKey::Atomic(group_id) => grouped.remove(&group_id).unwrap_or_default(),
                GroupKey::Single(index) => vec![index],
            })
            .collect()
    }

    pub(super) async fn push_impl(
        &self,
        user_id: &UserId,
        request: &PushRequestV1,
    ) -> Result<PushResponseV1, ApiError> {
        self.validate_client(&request.client)?;
        if request.changes.len() > self.config.push_max_changes {
            return Err(ApiError::new(
                ErrorCode::BatchTooLarge,
                format!(
                    "push batch of {} exceeds maximum {}",
                    request.changes.len(),
                    self.config.push_max_changes
                ),
                StatusCode::BAD_REQUEST,
            ));
        }
        let wire_size = serde_json::to_vec(request)
            .map_err(Self::internal_error)?
            .len();
        if wire_size > self.config.request_body_limit_bytes {
            return Err(ApiError::new(
                ErrorCode::PayloadTooLarge,
                format!("request body of {wire_size} bytes exceeds maximum"),
                StatusCode::PAYLOAD_TOO_LARGE,
            ));
        }

        for group in Self::group_indices(&request.changes) {
            if group.len() > self.config.maximum_atomic_group_size {
                return Err(ApiError::new(
                    ErrorCode::BatchTooLarge,
                    "atomic group exceeds maximum size",
                    StatusCode::BAD_REQUEST,
                ));
            }
        }

        let mut tx = self.pool.begin().await.map_err(Self::db_error)?;
        self.ensure_identity(&mut tx, user_id, &request.client)
            .await?;
        let mut results = Vec::with_capacity(request.changes.len());

        for indices in Self::group_indices(&request.changes) {
            if indices.len() == 1 {
                let change = &request.changes[indices[0]];
                if let Some(rejection) = self
                    .validate_execution_transition(&mut tx, user_id, change)
                    .await?
                {
                    results.push(rejection);
                    continue;
                }
                results.push(
                    self.process_change(&mut tx, user_id, &request.client, change)
                        .await?,
                );
                continue;
            }

            let mut nested = tx.begin().await.map_err(Self::db_error)?;
            let mut group_results = Vec::with_capacity(indices.len());
            let mut failed = false;
            for index in &indices {
                let change = &request.changes[*index];
                let result = if let Some(rejection) = self
                    .validate_execution_transition(&mut nested, user_id, change)
                    .await?
                {
                    rejection
                } else {
                    self.process_change(&mut nested, user_id, &request.client, change)
                        .await?
                };
                if !matches!(
                    result,
                    PushChangeResultV1::Accepted { .. } | PushChangeResultV1::Duplicate { .. }
                ) {
                    failed = true;
                }
                group_results.push(result);
            }
            if failed {
                nested.rollback().await.map_err(Self::db_error)?;
                for index in indices {
                    let change = &request.changes[index];
                    results.push(PushChangeResultV1::Rejected {
                        change_id: change.change_id.clone(),
                        entity_type: change.entity_type.clone(),
                        entity_id: change.entity_id.clone(),
                        code: ErrorCode::AtomicGroupFailed,
                        message: "atomic group failed".to_owned(),
                        field_errors: vec![],
                    });
                }
            } else {
                nested.commit().await.map_err(Self::db_error)?;
                results.extend(group_results);
            }
        }

        let user_uuid = Self::user_uuid(user_id);
        self.prune_change_log(&mut tx, user_uuid).await?;
        let latest = self.latest_cursor_raw(&mut *tx, user_uuid).await?;
        tx.commit().await.map_err(Self::db_error)?;
        Ok(PushResponseV1 {
            request_id: request.request_id.clone(),
            server_time: Self::now(),
            results,
            latest_cursor: self.cursor_codec.encode(user_id, &empty_scope(), latest),
        })
    }
}

enum GroupKey {
    Atomic(String),
    Single(usize),
}
