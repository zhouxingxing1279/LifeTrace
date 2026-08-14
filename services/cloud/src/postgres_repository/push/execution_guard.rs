use lifetrace_contracts::sync::v1::{ChangeOperation, PushChangeResultV1, SyncChangeV1};
use lifetrace_contracts::{ErrorCode, UserId};
use sqlx::{Postgres, Transaction};

use crate::error::ApiError;
use crate::postgres_repository::PostgresRepository;

impl PostgresRepository {
    /// Enforce finish-before-start dependencies at the authoritative sync layer.
    ///
    /// This runs immediately before each task change inside the same transaction
    /// (and inside the same nested transaction for an atomic group). Therefore a
    /// group may complete a predecessor first and then start/finish its successor,
    /// while stale or out-of-order clients cannot bypass the dependency graph.
    pub(super) async fn validate_execution_transition(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: &UserId,
        change: &SyncChangeV1,
    ) -> Result<Option<PushChangeResultV1>, ApiError> {
        if change.entity_type.as_str() != "execution.task"
            || change.operation.as_str() != ChangeOperation::UPSERT
        {
            return Ok(None);
        }
        let Some(payload) = change.payload.as_ref() else {
            return Ok(None);
        };
        let status = payload
            .0
            .get("status")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        if !matches!(status, "in_progress" | "done") {
            return Ok(None);
        }

        let user_uuid = Self::user_uuid(user_id);
        let blocker: Option<String> = sqlx::query_scalar(
            r#"
            SELECT dependency.payload->>'dependsOnTaskId'
            FROM sync_entities dependency
            LEFT JOIN sync_entities predecessor
              ON predecessor.user_id = dependency.user_id
             AND predecessor.entity_type = 'execution.task'
             AND predecessor.entity_id = dependency.payload->>'dependsOnTaskId'
             AND predecessor.is_deleted = FALSE
            WHERE dependency.user_id = $1
              AND dependency.entity_type = 'execution.task_dependency'
              AND dependency.is_deleted = FALSE
              AND dependency.payload->>'taskId' = $2
              AND dependency.payload->>'dependencyType' = 'finish_before_start'
              AND (
                    predecessor.entity_id IS NULL
                 OR COALESCE(predecessor.payload->>'status', '') <> 'done'
              )
            ORDER BY dependency.entity_id
            LIMIT 1
            "#,
        )
        .bind(user_uuid)
        .bind(change.entity_id.as_str())
        .fetch_optional(&mut **tx)
        .await
        .map_err(Self::db_error)?;

        Ok(blocker.map(|blocker_id| PushChangeResultV1::Rejected {
            change_id: change.change_id.clone(),
            entity_type: change.entity_type.clone(),
            entity_id: change.entity_id.clone(),
            code: ErrorCode::DependencyMissing,
            message: format!(
                "task transition is blocked until predecessor {blocker_id} is completed"
            ),
            field_errors: vec![],
        }))
    }
}
