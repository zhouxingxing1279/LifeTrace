//! Transactional BeeCount sync facade over LifeTrace's PostgreSQL entity log.

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use lifetrace_contracts::json_value::JsonValue;
use lifetrace_contracts::{ErrorCode, UserId};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::beecount_collaboration::{
    ensure_owner_registry_tx, resolve_ledger_access_tx, ROLE_OWNER,
};
use crate::beecount_compat::{
    beecount_payload, beecount_wire_id, canonical_payload, clamp_client_updated_at,
    incoming_clock_wins, lifetrace_entity_id, BeeCountBoundaryError, BeeCountConflictSample,
    BeeCountEntityKind, BeeCountReadLedgerOut, BeeCountScope, BeeCountSyncChangeOut,
    BeeCountSyncFullResponse, BeeCountSyncLedgerOut, BeeCountSyncPullResponse,
    BeeCountSyncPushRequest, BeeCountSyncPushResponse, USER_GLOBAL_LEDGER_SENTINEL,
};
use crate::error::ApiError;

const MAX_CONFLICT_SAMPLES: usize = 20;
const MAX_PULL_LIMIT: i64 = 5000;

#[derive(Clone)]
pub struct BeeCountSyncService {
    pool: PgPool,
}

impl BeeCountSyncService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn push(
        &self,
        user_id: &UserId,
        session_device_id: &str,
        request: BeeCountSyncPushRequest,
    ) -> Result<BeeCountSyncPushResponse, ApiError> {
        if request.changes.len() > 5000 || request.device_id.is_empty() {
            return Err(invalid("invalid BeeCount sync batch"));
        }
        let actor_uuid = user_uuid(user_id)?;
        let actor_wire_user_id =
            crate::beecount_collaboration::beecount_user_id(&self.pool, actor_uuid).await?;
        let device_uuid = Uuid::parse_str(session_device_id)
            .map_err(|_| unauthorized("invalid BeeCount session device"))?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        verify_device(&mut tx, actor_uuid, device_uuid, &request.device_id).await?;

        let server_now = Utc::now();
        let mut accepted = 0usize;
        let mut rejected = 0usize;
        let mut conflict_count = 0usize;
        let mut conflict_samples = Vec::new();

        for mut incoming in request.changes {
            let Some(kind) = BeeCountEntityKind::parse(&incoming.entity_type) else {
                rejected += 1;
                continue;
            };
            if !valid_external_id(&incoming.entity_sync_id)
                || !matches!(incoming.action.as_str(), "upsert" | "delete")
            {
                rejected += 1;
                continue;
            }
            let scope = kind.scope();
            let ledger_id = match scope {
                BeeCountScope::User => None,
                BeeCountScope::Ledger => {
                    let value = incoming.ledger_id.as_deref().unwrap_or_else(|| {
                        if kind == BeeCountEntityKind::Ledger {
                            incoming.entity_sync_id.as_str()
                        } else {
                            ""
                        }
                    });
                    if !valid_external_id(value) {
                        rejected += 1;
                        continue;
                    }
                    Some(value.to_owned())
                }
            };
            let entity_type = kind.lifetrace_entity_type();
            let entity_id = lifetrace_entity_id(&incoming.entity_sync_id);
            let updated_at = clamp_client_updated_at(incoming.updated_at, server_now);
            let storage_uuid = match ledger_id.as_deref() {
                None => actor_uuid,
                Some(ledger_id) => {
                    match resolve_ledger_access_tx(&mut tx, actor_uuid, ledger_id, true).await {
                        Ok(access) => access.storage_user_id,
                        Err(_)
                            if kind == BeeCountEntityKind::Ledger
                                && incoming.action == "upsert" =>
                        {
                            actor_uuid
                        }
                        Err(_) => {
                            rejected += 1;
                            continue;
                        }
                    }
                }
            };

            let current = sqlx::query(
                "SELECT server_version,payload,is_deleted,last_cursor,server_modified_at, \
                        origin_device_external_id,created_at \
                 FROM sync_entities \
                 WHERE user_id=$1 AND entity_type=$2 AND entity_id=$3 FOR UPDATE",
            )
            .bind(storage_uuid)
            .bind(entity_type)
            .bind(&entity_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_error)?;
            let clock = sqlx::query(
                "SELECT updated_at,updated_by_device_id,lifetrace_cursor,source_change_id \
                 FROM beecount_entity_clocks \
                 WHERE user_id=$1 AND entity_type=$2 AND entity_sync_id=$3 FOR UPDATE",
            )
            .bind(storage_uuid)
            .bind(kind.as_beecount_type())
            .bind(&incoming.entity_sync_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_error)?;

            let current_version = current
                .as_ref()
                .and_then(|row| row.try_get::<i64, _>("server_version").ok())
                .unwrap_or(0)
                .max(0);
            let current_cursor = current
                .as_ref()
                .and_then(|row| row.try_get::<i64, _>("last_cursor").ok())
                .unwrap_or(0)
                .max(0);
            let current_payload = current
                .as_ref()
                .and_then(|row| row.try_get::<Option<Value>, _>("payload").ok())
                .flatten();
            if kind == BeeCountEntityKind::Transaction && incoming.action == "upsert" {
                if let Some(object) = incoming.payload.as_object_mut() {
                    object
                        .entry("updatedByUserId")
                        .or_insert_with(|| Value::String(actor_wire_user_id.clone()));
                    let existing_creator = current_payload
                        .as_ref()
                        .and_then(|payload| payload.get("createdByUserId"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| actor_wire_user_id.clone());
                    object
                        .entry("createdByUserId")
                        .or_insert_with(|| Value::String(existing_creator));
                }
            }
            let existing_clock = current.as_ref().map(|row| {
                let server_time = row
                    .try_get::<DateTime<Utc>, _>("server_modified_at")
                    .unwrap_or(server_now);
                let server_device = row
                    .try_get::<Option<String>, _>("origin_device_external_id")
                    .ok()
                    .flatten()
                    .unwrap_or_default();
                if let Some(clock) = clock.as_ref() {
                    let clock_cursor = clock
                        .try_get::<i64, _>("lifetrace_cursor")
                        .unwrap_or_default();
                    if clock_cursor == current_cursor {
                        return (
                            clock
                                .try_get::<DateTime<Utc>, _>("updated_at")
                                .unwrap_or(server_time),
                            clock
                                .try_get::<String, _>("updated_by_device_id")
                                .unwrap_or(server_device),
                        );
                    }
                }
                (server_time, server_device)
            });

            if let Some((existing_time, existing_device)) = existing_clock {
                if existing_time == updated_at && existing_device == request.device_id {
                    accepted += 1;
                    continue;
                }
                if !incoming_clock_wins(
                    updated_at,
                    &request.device_id,
                    existing_time,
                    &existing_device,
                ) {
                    rejected += 1;
                    conflict_count += 1;
                    if conflict_samples.len() < MAX_CONFLICT_SAMPLES {
                        conflict_samples.push(BeeCountConflictSample {
                            reason: "lww_rejected_older_change".to_owned(),
                            ledger_id: ledger_id.clone(),
                            entity_type: incoming.entity_type.clone(),
                            entity_sync_id: incoming.entity_sync_id.clone(),
                            existing_change_id: clock.as_ref().and_then(|row| {
                                row.try_get::<Option<i64>, _>("source_change_id")
                                    .ok()
                                    .flatten()
                            }),
                        });
                    }
                    continue;
                }
            }

            let next_version = current_version.saturating_add(1).max(1);
            let payload = if incoming.action == "upsert" {
                match canonical_payload(
                    kind,
                    &incoming.entity_sync_id,
                    ledger_id.as_deref(),
                    &incoming.payload,
                    user_id,
                    &request.device_id,
                    updated_at,
                    current_payload.as_ref(),
                    next_version as u64,
                ) {
                    Ok(value) => Some(value),
                    Err(_) => {
                        rejected += 1;
                        continue;
                    }
                }
            } else {
                None
            };
            let cursor = append_change(
                &mut tx,
                storage_uuid,
                device_uuid,
                &request.device_id,
                entity_type,
                &entity_id,
                &incoming.action,
                next_version,
                payload.as_ref(),
                updated_at,
                server_now,
            )
            .await?;
            persist_entity(
                &mut tx,
                storage_uuid,
                device_uuid,
                &request.device_id,
                entity_type,
                &entity_id,
                next_version,
                payload.as_ref(),
                current
                    .as_ref()
                    .and_then(|row| row.try_get::<DateTime<Utc>, _>("created_at").ok()),
                updated_at,
                server_now,
                cursor,
            )
            .await?;
            sqlx::query(
                "INSERT INTO beecount_entity_clocks ( \
                    user_id,entity_type,entity_sync_id,ledger_id,scope,updated_at, \
                    updated_by_device_id,lifetrace_entity_type,lifetrace_entity_id, \
                    lifetrace_server_version,lifetrace_cursor,source_change_id,is_deleted \
                 ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$11,$12) \
                 ON CONFLICT (user_id,entity_type,entity_sync_id) DO UPDATE SET \
                    ledger_id=EXCLUDED.ledger_id,scope=EXCLUDED.scope, \
                    updated_at=EXCLUDED.updated_at, \
                    updated_by_device_id=EXCLUDED.updated_by_device_id, \
                    lifetrace_entity_type=EXCLUDED.lifetrace_entity_type, \
                    lifetrace_entity_id=EXCLUDED.lifetrace_entity_id, \
                    lifetrace_server_version=EXCLUDED.lifetrace_server_version, \
                    lifetrace_cursor=EXCLUDED.lifetrace_cursor, \
                    source_change_id=EXCLUDED.source_change_id,is_deleted=EXCLUDED.is_deleted",
            )
            .bind(storage_uuid)
            .bind(kind.as_beecount_type())
            .bind(&incoming.entity_sync_id)
            .bind(ledger_id.as_deref())
            .bind(scope.as_str())
            .bind(updated_at)
            .bind(&request.device_id)
            .bind(entity_type)
            .bind(&entity_id)
            .bind(next_version)
            .bind(cursor)
            .bind(incoming.action == "delete")
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
            if kind == BeeCountEntityKind::Ledger && incoming.action == "upsert" {
                ensure_owner_registry_tx(
                    &mut tx,
                    storage_uuid,
                    ledger_id.as_deref().unwrap_or(&incoming.entity_sync_id),
                )
                .await?;
            }
            accepted += 1;
        }

        sqlx::query(
            "UPDATE cloud_devices SET last_seen_at=now(),last_sync_at=now() \
             WHERE id=$1 AND user_id=$2",
        )
        .bind(device_uuid)
        .bind(actor_uuid)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
        let server_cursor: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(cursor),0)::BIGINT FROM sync_change_log")
                .fetch_one(&mut *tx)
                .await
                .map_err(db_error)?;
        tx.commit().await.map_err(db_error)?;
        Ok(BeeCountSyncPushResponse {
            accepted,
            rejected,
            conflict_count,
            conflict_samples,
            server_cursor,
            server_timestamp: server_now,
        })
    }

    pub async fn pull(
        &self,
        user_id: &UserId,
        since: i64,
        device_id: Option<&str>,
        limit: i64,
    ) -> Result<BeeCountSyncPullResponse, ApiError> {
        if since < 0 || !(1..=MAX_PULL_LIMIT).contains(&limit) {
            return Err(invalid("invalid BeeCount pull cursor or limit"));
        }
        let user_uuid = user_uuid(user_id)?;
        let supported_entity_types = vec![
            "finance.ledger".to_owned(),
            "finance.account".to_owned(),
            "finance.category".to_owned(),
            "finance.transaction".to_owned(),
            "finance.tag".to_owned(),
            "finance.budget".to_owned(),
        ];
        let rows = sqlx::query(
            "SELECT l.cursor,l.entity_type,l.entity_id,l.operation,l.payload, \
                    l.server_modified_at,l.origin_device_external_id, \
                    c.entity_type AS beecount_entity_type,c.entity_sync_id, \
                    c.ledger_id,c.scope,c.updated_at AS clock_updated_at, \
                    c.updated_by_device_id,c.lifetrace_cursor \
             FROM sync_change_log l \
             LEFT JOIN beecount_entity_clocks c \
               ON c.user_id=l.user_id AND c.lifetrace_entity_type=l.entity_type \
              AND c.lifetrace_entity_id=l.entity_id \
             WHERE l.cursor>$2 \
               AND (l.user_id=$1 OR EXISTS ( \
                    SELECT 1 FROM beecount_ledger_members m \
                    JOIN beecount_shared_ledgers s ON s.ledger_id=m.ledger_id \
                    WHERE m.user_id=$1 AND s.storage_user_id=l.user_id \
                      AND c.scope='ledger' AND c.ledger_id=m.ledger_id)) \
               AND (l.entity_type = ANY($3) \
                    OR (l.entity_type='user.preference' AND l.entity_id LIKE 'beecount:%')) \
               AND ($4::text IS NULL OR l.origin_device_external_id IS DISTINCT FROM $4) \
             ORDER BY l.cursor ASC LIMIT $5",
        )
        .bind(user_uuid)
        .bind(since)
        .bind(&supported_entity_types)
        .bind(device_id)
        .bind(limit + 1)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        let has_more = rows.len() as i64 > limit;
        let mut changes = Vec::with_capacity(rows.len().min(limit as usize));
        for row in rows.into_iter().take(limit as usize) {
            if let Some(change) = row_to_change(row)? {
                changes.push(change);
            }
        }
        let server_cursor = changes.last().map(|row| row.change_id).unwrap_or(since);
        Ok(BeeCountSyncPullResponse {
            changes,
            server_cursor,
            has_more,
        })
    }

    pub async fn ledgers(&self, user_id: &UserId) -> Result<Vec<BeeCountSyncLedgerOut>, ApiError> {
        let actor_uuid = user_uuid(user_id)?;
        let rows = sqlx::query(
            "SELECT e.entity_id,e.payload,e.server_modified_at,e.last_cursor, \
                    COALESCE(m.role,'owner') AS role, \
                    (SELECT COUNT(*)::BIGINT FROM sync_entities t \
                     WHERE t.user_id=e.user_id AND t.entity_type='finance.transaction' \
                       AND t.is_deleted=FALSE \
                       AND COALESCE(t.payload->>'beecountLedgerId','') = \
                           COALESCE(e.payload->>'beecountLedgerId',substring(e.entity_id from 10))) \
                    AS tx_count \
             FROM sync_entities e \
             LEFT JOIN beecount_shared_ledgers s ON s.storage_user_id=e.user_id \
               AND s.ledger_id=COALESCE(e.payload->>'beecountLedgerId', \
                 CASE WHEN e.entity_id LIKE 'beecount:%' THEN substring(e.entity_id from 10) \
                      ELSE 'lifetrace:' || e.entity_id END) \
             LEFT JOIN beecount_ledger_members m ON m.ledger_id=s.ledger_id AND m.user_id=$1 \
             WHERE e.entity_type='finance.ledger' AND e.is_deleted=FALSE \
               AND ((s.ledger_id IS NULL AND e.user_id=$1) OR m.user_id IS NOT NULL) \
             ORDER BY e.created_at,e.entity_id",
        )
        .bind(actor_uuid)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        rows.into_iter()
            .map(|row| {
                let entity_id: String = row.try_get("entity_id").map_err(internal)?;
                let payload: Value = row.try_get("payload").map_err(internal)?;
                let ledger_id = payload
                    .get("beecountLedgerId")
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
                    .unwrap_or_else(|| beecount_wire_id(&entity_id));
                let tx_count = row.try_get::<i64, _>("tx_count").unwrap_or(0).max(0);
                Ok(BeeCountSyncLedgerOut {
                    path: ledger_id.clone(),
                    ledger_id,
                    updated_at: row.try_get("server_modified_at").ok(),
                    size: 512 + tx_count * 300,
                    metadata: json!({
                        "source": "lifetrace-postgresql",
                        "ledgerName": payload.get("name"),
                        "currency": payload.get("currency"),
                        "monthStartDay": payload.get("monthStartDay"),
                    }),
                    role: row
                        .try_get::<Option<String>, _>("role")
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| ROLE_OWNER.to_owned()),
                })
            })
            .collect()
    }

    /// `GET /api/v1/read/ledgers` facade: same ledger access as the sync
    /// listing, enriched with the transaction stats the stock BeeCount client
    /// expects on the read namespace.
    pub async fn read_ledgers(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<BeeCountReadLedgerOut>, ApiError> {
        let base = self.ledgers(user_id).await?;
        let mut out = Vec::with_capacity(base.len());
        for ledger in base {
            let ledger_id = ledger.ledger_id.clone();
            let stats = sqlx::query(
                "SELECT \
                   COALESCE(SUM(CASE WHEN payload->>'transactionType' = 'income' \
                                     THEN (payload->>'amountCents')::bigint ELSE 0 END), 0)::bigint \
                     AS income_cents, \
                   COALESCE(SUM(CASE WHEN payload->>'transactionType' = 'expense' \
                                     THEN (payload->>'amountCents')::bigint ELSE 0 END), 0)::bigint \
                     AS expense_cents, \
                   COUNT(*)::bigint AS tx_count \
                 FROM sync_entities \
                 WHERE entity_type='finance.transaction' AND is_deleted=FALSE \
                   AND payload->>'beecountLedgerId' = $1",
            )
            .bind(&ledger_id)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
            let income_cents: i64 = stats.try_get("income_cents").unwrap_or(0).max(0);
            let expense_cents: i64 = stats.try_get("expense_cents").unwrap_or(0).max(0);
            let transaction_count: i64 = stats.try_get("tx_count").unwrap_or(0).max(0);

            let member_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*)::BIGINT FROM beecount_ledger_members WHERE ledger_id=$1",
            )
            .bind(&ledger_id)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;

            let metadata = &ledger.metadata;
            let ledger_name = metadata
                .get("ledgerName")
                .and_then(Value::as_str)
                .unwrap_or(&ledger_id)
                .to_owned();
            let currency = metadata
                .get("currency")
                .and_then(Value::as_str)
                .unwrap_or("CNY")
                .to_owned();
            let month_start_day = metadata
                .get("monthStartDay")
                .and_then(Value::as_i64)
                .unwrap_or(1);
            let income_total = income_cents as f64 / 100.0;
            let expense_total = expense_cents as f64 / 100.0;

            out.push(BeeCountReadLedgerOut {
                ledger_id,
                ledger_name,
                currency,
                month_start_day,
                transaction_count,
                income_total,
                expense_total,
                balance: income_total - expense_total,
                exported_at: None,
                updated_at: ledger.updated_at.unwrap_or_else(Utc::now),
                role: ledger.role,
                is_shared: member_count > 1,
                member_count,
            });
        }
        Ok(out)
    }

    pub async fn full(
        &self,
        user_id: &UserId,
        ledger_id: &str,
    ) -> Result<BeeCountSyncFullResponse, ApiError> {
        let actor_uuid = user_uuid(user_id)?;
        let access = crate::beecount_collaboration::resolve_ledger_access(
            &self.pool, actor_uuid, ledger_id, false,
        )
        .await?;
        let storage_uuid = access.storage_user_id;
        let resource_owner_uuid: Uuid = sqlx::query_scalar(
            "SELECT user_id FROM beecount_ledger_members WHERE ledger_id=$1 AND role='owner'",
        )
        .bind(ledger_id)
        .fetch_one(&self.pool)
        .await
        .map_err(db_error)?;
        let latest_cursor: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(cursor),0)::BIGINT FROM sync_change_log")
                .fetch_one(&self.pool)
                .await
                .map_err(db_error)?;
        let ledger_entity_id = lifetrace_entity_id(ledger_id);
        let ledger = sqlx::query(
            "SELECT payload,last_cursor,server_modified_at FROM sync_entities \
             WHERE user_id=$1 AND entity_type='finance.ledger' \
               AND (entity_id=$2 OR payload->>'beecountLedgerId'=$3) \
               AND is_deleted=FALSE",
        )
        .bind(storage_uuid)
        .bind(&ledger_entity_id)
        .bind(ledger_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;
        let Some(ledger) = ledger else {
            return Ok(BeeCountSyncFullResponse {
                ledger_id: ledger_id.to_owned(),
                snapshot: None,
                latest_cursor,
            });
        };
        let ledger_payload: Value = ledger.try_get("payload").map_err(internal)?;
        let ledger_raw = beecount_payload(BeeCountEntityKind::Ledger, ledger_id, &ledger_payload)
            .map_err(boundary_error)?;
        let rows = sqlx::query(
            "SELECT entity_type,entity_id,payload,last_cursor,server_modified_at \
             FROM sync_entities \
             WHERE is_deleted=FALSE AND ( \
               (user_id=$1 AND entity_type = ANY($2)) OR \
               (user_id=$3 AND entity_type = ANY($4))) \
             ORDER BY entity_type,entity_id",
        )
        .bind(storage_uuid)
        .bind(vec![
            "finance.transaction".to_owned(),
            "finance.budget".to_owned(),
        ])
        .bind(resource_owner_uuid)
        .bind(vec![
            "finance.account".to_owned(),
            "finance.category".to_owned(),
            "finance.tag".to_owned(),
        ])
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        let mut accounts = Vec::new();
        let mut categories = Vec::new();
        let mut items = Vec::new();
        let mut tags = Vec::new();
        let mut budgets = Vec::new();
        let mut snapshot_cursor = ledger.try_get::<i64, _>("last_cursor").unwrap_or(0);
        let mut snapshot_updated = ledger
            .try_get::<DateTime<Utc>, _>("server_modified_at")
            .unwrap_or_else(|_| Utc::now());
        for row in rows {
            let entity_type: String = row.try_get("entity_type").map_err(internal)?;
            let Some(kind) = BeeCountEntityKind::from_lifetrace(&entity_type) else {
                continue;
            };
            let entity_id: String = row.try_get("entity_id").map_err(internal)?;
            let payload: Value = row.try_get("payload").map_err(internal)?;
            if matches!(
                kind,
                BeeCountEntityKind::Transaction | BeeCountEntityKind::Budget
            ) && canonical_ledger_id(&payload).as_deref() != Some(ledger_id)
            {
                continue;
            }
            let wire_id = beecount_wire_id(&entity_id);
            let raw = beecount_payload(kind, &wire_id, &payload).map_err(boundary_error)?;
            match kind {
                BeeCountEntityKind::Account => accounts.push(raw),
                BeeCountEntityKind::Category => categories.push(raw),
                BeeCountEntityKind::Transaction => items.push(raw),
                BeeCountEntityKind::Tag => tags.push(raw),
                BeeCountEntityKind::Budget => budgets.push(raw),
                _ => {}
            }
            snapshot_cursor = snapshot_cursor.max(row.try_get("last_cursor").unwrap_or(0));
            if let Ok(updated) = row.try_get::<DateTime<Utc>, _>("server_modified_at") {
                snapshot_updated = snapshot_updated.max(updated);
            }
        }
        let content = json!({
            "version": 6,
            "exportedAt": Utc::now(),
            "ledgerSyncId": ledger_id,
            "ledgerName": ledger_raw.get("ledgerName").and_then(Value::as_str).unwrap_or(ledger_id),
            "currency": ledger_raw.get("currency").and_then(Value::as_str).unwrap_or("CNY"),
            "monthStartDay": ledger_raw.get("monthStartDay").and_then(Value::as_i64).unwrap_or(1),
            "count": items.len(),
            "accounts": accounts,
            "categories": categories,
            "tags": tags,
            "budgets": budgets,
            "items": items,
        });
        Ok(BeeCountSyncFullResponse {
            ledger_id: ledger_id.to_owned(),
            snapshot: Some(BeeCountSyncChangeOut {
                change_id: snapshot_cursor,
                ledger_id: ledger_id.to_owned(),
                entity_type: "ledger_snapshot".to_owned(),
                entity_sync_id: ledger_id.to_owned(),
                action: "upsert".to_owned(),
                payload: json!({
                    "content": serde_json::to_string(&content).map_err(internal)?,
                    "metadata": {"source": "lifetrace-postgresql"},
                }),
                updated_at: snapshot_updated,
                updated_by_device_id: None,
                scope: "ledger".to_owned(),
            }),
            latest_cursor,
        })
    }
}

async fn verify_device(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    device_id: Uuid,
    external_device_id: &str,
) -> Result<(), ApiError> {
    let valid: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM cloud_devices \
         WHERE id=$1 AND user_id=$2 AND app_id='beecount-mobile' \
           AND external_device_id=$3 AND status='active' AND revoked_at IS NULL)",
    )
    .bind(device_id)
    .bind(user_id)
    .bind(external_device_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(db_error)?;
    if !valid {
        return Err(unauthorized("invalid BeeCount device"));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn append_change(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    device_id: Uuid,
    external_device_id: &str,
    entity_type: &str,
    entity_id: &str,
    operation: &str,
    server_version: i64,
    payload: Option<&JsonValue>,
    client_modified_at: DateTime<Utc>,
    server_modified_at: DateTime<Utc>,
) -> Result<i64, ApiError> {
    let payload_value = payload.map(|value| value.0.clone());
    let payload_hash = payload_value
        .as_ref()
        .map(|value| serde_json::to_vec(value).map(|bytes| Sha256::digest(bytes).to_vec()))
        .transpose()
        .map_err(internal)?;
    let tombstone = (operation == "delete").then(|| {
        json!({
            "entityType": entity_type,
            "entityId": entity_id,
            "deletedAt": server_modified_at,
            "serverVersion": server_version.to_string(),
            "deletedByDevice": external_device_id,
        })
    });
    sqlx::query_scalar(
        "INSERT INTO sync_change_log ( \
            user_id,entity_type,entity_id,operation,entity_schema_version,server_version, \
            payload,payload_hash,tombstone,origin_device_id,origin_device_external_id, \
            client_modified_at,server_modified_at \
         ) VALUES ($1,$2,$3,$4,1,$5,$6,$7,$8,$9,$10,$11,$12) RETURNING cursor",
    )
    .bind(user_id)
    .bind(entity_type)
    .bind(entity_id)
    .bind(operation)
    .bind(server_version)
    .bind(payload_value)
    .bind(payload_hash)
    .bind(tombstone)
    .bind(device_id)
    .bind(external_device_id)
    .bind(client_modified_at)
    .bind(server_modified_at)
    .fetch_one(&mut **tx)
    .await
    .map_err(db_error)
}

#[allow(clippy::too_many_arguments)]
async fn persist_entity(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    device_id: Uuid,
    external_device_id: &str,
    entity_type: &str,
    entity_id: &str,
    server_version: i64,
    payload: Option<&JsonValue>,
    created_at: Option<DateTime<Utc>>,
    client_modified_at: DateTime<Utc>,
    server_modified_at: DateTime<Utc>,
    cursor: i64,
) -> Result<(), ApiError> {
    let payload_value = payload.map(|value| value.0.clone());
    let payload_hash = payload_value
        .as_ref()
        .map(|value| serde_json::to_vec(value).map(|bytes| Sha256::digest(bytes).to_vec()))
        .transpose()
        .map_err(internal)?;
    let is_deleted = payload.is_none();
    sqlx::query(
        "INSERT INTO sync_entities ( \
            user_id,entity_type,entity_id,entity_schema_version,server_version,payload, \
            payload_hash,is_deleted,deleted_at,origin_device_id,origin_device_external_id, \
            created_at,server_modified_at,client_modified_at,last_cursor \
         ) VALUES ($1,$2,$3,1,$4,$5,$6,$7,CASE WHEN $7 THEN $8 ELSE NULL END, \
                   $9,$10,$11,$8,$12,$13) \
         ON CONFLICT (user_id,entity_type,entity_id) DO UPDATE SET \
            entity_schema_version=1,server_version=EXCLUDED.server_version, \
            payload=EXCLUDED.payload,payload_hash=EXCLUDED.payload_hash, \
            is_deleted=EXCLUDED.is_deleted,deleted_at=EXCLUDED.deleted_at, \
            origin_device_id=EXCLUDED.origin_device_id, \
            origin_device_external_id=EXCLUDED.origin_device_external_id, \
            server_modified_at=EXCLUDED.server_modified_at, \
            client_modified_at=EXCLUDED.client_modified_at,last_cursor=EXCLUDED.last_cursor",
    )
    .bind(user_id)
    .bind(entity_type)
    .bind(entity_id)
    .bind(server_version)
    .bind(payload_value)
    .bind(payload_hash)
    .bind(is_deleted)
    .bind(server_modified_at)
    .bind(device_id)
    .bind(external_device_id)
    .bind(created_at.unwrap_or(server_modified_at))
    .bind(client_modified_at)
    .bind(cursor)
    .execute(&mut **tx)
    .await
    .map_err(db_error)?;
    Ok(())
}

fn row_to_change(row: sqlx::postgres::PgRow) -> Result<Option<BeeCountSyncChangeOut>, ApiError> {
    let entity_type: String = row.try_get("entity_type").map_err(internal)?;
    let kind = row
        .try_get::<Option<String>, _>("beecount_entity_type")
        .ok()
        .flatten()
        .as_deref()
        .and_then(BeeCountEntityKind::parse)
        .or_else(|| BeeCountEntityKind::from_lifetrace(&entity_type));
    let Some(kind) = kind else { return Ok(None) };
    let entity_id: String = row.try_get("entity_id").map_err(internal)?;
    let sync_id = row
        .try_get::<Option<String>, _>("entity_sync_id")
        .ok()
        .flatten()
        .unwrap_or_else(|| beecount_wire_id(&entity_id));
    let payload: Option<Value> = row.try_get("payload").map_err(internal)?;
    let operation: String = row.try_get("operation").map_err(internal)?;
    let scope = kind.scope();
    let ledger_id = match scope {
        BeeCountScope::User => USER_GLOBAL_LEDGER_SENTINEL.to_owned(),
        BeeCountScope::Ledger => row
            .try_get::<Option<String>, _>("ledger_id")
            .ok()
            .flatten()
            .or_else(|| payload.as_ref().and_then(canonical_ledger_id))
            .unwrap_or_else(|| sync_id.clone()),
    };
    let cursor: i64 = row.try_get("cursor").map_err(internal)?;
    let clock_matches = row
        .try_get::<Option<i64>, _>("lifetrace_cursor")
        .ok()
        .flatten()
        == Some(cursor);
    let updated_at = if clock_matches {
        row.try_get::<Option<DateTime<Utc>>, _>("clock_updated_at")
            .ok()
            .flatten()
            .unwrap_or(row.try_get("server_modified_at").map_err(internal)?)
    } else {
        row.try_get("server_modified_at").map_err(internal)?
    };
    let updated_by_device_id = if clock_matches {
        row.try_get::<Option<String>, _>("updated_by_device_id")
            .ok()
            .flatten()
            .or_else(|| {
                row.try_get::<Option<String>, _>("origin_device_external_id")
                    .ok()
                    .flatten()
            })
    } else {
        row.try_get::<Option<String>, _>("origin_device_external_id")
            .ok()
            .flatten()
    };
    let output_payload = if operation == "delete" {
        json!({})
    } else {
        beecount_payload(
            kind,
            &sync_id,
            payload
                .as_ref()
                .ok_or_else(|| internal("upsert payload missing"))?,
        )
        .map_err(boundary_error)?
    };
    Ok(Some(BeeCountSyncChangeOut {
        change_id: cursor,
        ledger_id,
        entity_type: kind.as_beecount_type().to_owned(),
        entity_sync_id: sync_id,
        action: operation,
        payload: output_payload,
        updated_at,
        updated_by_device_id,
        scope: scope.as_str().to_owned(),
    }))
}

fn canonical_ledger_id(payload: &Value) -> Option<String> {
    payload
        .get("beecountLedgerId")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            payload
                .get("ledgerId")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(beecount_wire_id)
        })
}

fn user_uuid(user_id: &UserId) -> Result<Uuid, ApiError> {
    Uuid::parse_str(user_id.as_str()).map_err(|_| unauthorized("invalid BeeCount user"))
}

fn valid_external_id(value: &str) -> bool {
    let trimmed = value.trim();
    let native_suffix_is_valid = value
        .strip_prefix(crate::beecount_compat::NATIVE_WIRE_ID_PREFIX)
        .map_or(true, |suffix| !suffix.is_empty());
    trimmed == value
        && !trimmed.is_empty()
        && trimmed.len() <= 256
        && value.chars().all(|character| !character.is_control())
        && !trimmed.starts_with(crate::beecount_compat::ENTITY_ID_PREFIX)
        && native_suffix_is_valid
}

fn db_error(_error: sqlx::Error) -> ApiError {
    ApiError::new(
        ErrorCode::TemporarilyUnavailable,
        "BeeCount storage temporarily unavailable",
        StatusCode::SERVICE_UNAVAILABLE,
    )
}

fn internal(error: impl std::fmt::Display) -> ApiError {
    ApiError::new(
        ErrorCode::InternalError,
        error.to_string(),
        StatusCode::INTERNAL_SERVER_ERROR,
    )
}

fn invalid(message: &str) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message, StatusCode::BAD_REQUEST)
}

fn unauthorized(message: &str) -> ApiError {
    ApiError::new(ErrorCode::AuthInvalid, message, StatusCode::UNAUTHORIZED)
}

fn boundary_error(error: BeeCountBoundaryError) -> ApiError {
    ApiError::new(
        ErrorCode::InvalidEntityPayload,
        error.to_string(),
        StatusCode::BAD_REQUEST,
    )
}
