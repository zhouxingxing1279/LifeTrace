use chrono::Utc;
use rusqlite::types::Value as SqlValue;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const PROJECTION_VERSION: i64 = 1;
const FACTS_VERSION: i64 = 1;
const INSIGHT_ALGORITHM_VERSION: &str = "epic14-v1";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionStatus {
    pub dirty: bool,
    pub event_count: i64,
    pub search_document_count: i64,
    pub last_rebuilt_at: Option<String>,
    pub projection_version: i64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub domain: Option<String>,
    pub event_type: Option<String>,
    pub keyword: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEvent {
    pub id: String,
    pub occurred_at: String,
    pub ended_at: Option<String>,
    pub local_date: String,
    pub timezone: Option<String>,
    pub domain: String,
    pub event_type: String,
    pub title: String,
    pub summary: String,
    pub entity_type: String,
    pub entity_id: String,
    pub metrics: Value,
    pub tags: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelinePage {
    pub items: Vec<TimelineEvent>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    pub q: String,
    pub domain: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub id: String,
    pub domain: String,
    pub entity_type: String,
    pub entity_id: String,
    pub title: String,
    pub snippet: String,
    pub occurred_at: Option<String>,
    pub updated_at: String,
    pub score: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportSnapshot {
    pub id: String,
    pub report_type: String,
    pub period_start: String,
    pub period_end: String,
    pub timezone: String,
    pub facts: Value,
    pub coverage: Value,
    pub generated_at: String,
    pub facts_version: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightSnapshot {
    pub id: String,
    pub insight_type: String,
    pub period_start: String,
    pub period_end: String,
    pub title: String,
    pub summary: String,
    pub evidence: Value,
    pub sample_size: i64,
    pub confidence: Value,
    pub algorithm_version: String,
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn json_or(raw: String, fallback: Value) -> Value {
    serde_json::from_str(&raw).unwrap_or(fallback)
}

pub fn projection_status(
    connection: &Connection,
    user_id: &str,
) -> Result<ProjectionStatus, String> {
    let state = connection
        .query_row(
            "SELECT dirty,projection_version,last_rebuilt_at,last_error
               FROM analytics_projection_state WHERE user_id=?1",
            [user_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let event_count = connection
        .query_row(
            "SELECT COUNT(*) FROM analytics_events WHERE user_id=?1",
            [user_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| error.to_string())?;
    let search_document_count = connection
        .query_row(
            "SELECT COUNT(*) FROM analytics_search_documents WHERE user_id=?1",
            [user_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| error.to_string())?;
    Ok(match state {
        Some((dirty, version, last_rebuilt_at, last_error)) => ProjectionStatus {
            dirty: dirty != 0,
            event_count,
            search_document_count,
            last_rebuilt_at,
            projection_version: version,
            last_error,
        },
        None => ProjectionStatus {
            dirty: true,
            event_count,
            search_document_count,
            last_rebuilt_at: None,
            projection_version: PROJECTION_VERSION,
            last_error: None,
        },
    })
}

pub fn ensure_current(
    connection: &mut Connection,
    user_id: &str,
) -> Result<ProjectionStatus, String> {
    let status = projection_status(connection, user_id)?;
    if status.dirty || status.projection_version != PROJECTION_VERSION {
        rebuild(connection, user_id)
    } else {
        Ok(status)
    }
}

pub fn rebuild(connection: &mut Connection, user_id: &str) -> Result<ProjectionStatus, String> {
    let stamp = now();
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    let result = (|| -> Result<(), String> {
        transaction
            .execute("DELETE FROM analytics_events WHERE user_id=?1", [user_id])
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "DELETE FROM analytics_search_documents WHERE user_id=?1",
                [user_id],
            )
            .map_err(|error| error.to_string())?;

        project_events(&transaction, user_id, &stamp)?;
        project_search_documents(&transaction, user_id, &stamp)?;

        transaction
            .execute(
                "INSERT INTO analytics_projection_state(
                   user_id,dirty,projection_version,last_rebuilt_at,last_error
                 ) VALUES(?1,0,?2,?3,NULL)
                 ON CONFLICT(user_id) DO UPDATE SET
                   dirty=0,
                   projection_version=excluded.projection_version,
                   last_rebuilt_at=excluded.last_rebuilt_at,
                   last_error=NULL",
                params![user_id, PROJECTION_VERSION, stamp],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            transaction.commit().map_err(|error| error.to_string())?;
            projection_status(connection, user_id)
        }
        Err(error) => {
            let _ = transaction.rollback();
            let _ = connection.execute(
                "INSERT INTO analytics_projection_state(
                   user_id,dirty,projection_version,last_error
                 ) VALUES(?1,1,?2,?3)
                 ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=excluded.last_error",
                params![user_id, PROJECTION_VERSION, error],
            );
            Err(error)
        }
    }
}

fn project_events(connection: &Connection, user_id: &str, stamp: &str) -> Result<(), String> {
    let statements = [
        r#"INSERT INTO analytics_events(
             id,user_id,occurred_at,ended_at,local_date,timezone,domain,event_type,title,summary,
             entity_type,entity_id,source_updated_at,metrics_json,tags_json,search_text,
             projection_version,projected_at
           )
           SELECT
             'finance:transaction:'||t.id,t.user_id,
             COALESCE(strftime('%Y-%m-%dT%H:%M:%fZ',t.occurred_at),t.occurred_at),NULL,t.local_date,NULL,
             'finance','transaction',
             COALESCE(NULLIF(t.counterparty,''),NULLIF(t.merchant,''),NULLIF(t.item,''),
                      NULLIF(t.legacy_category_name,''),'交易'),
             COALESCE(t.note,''),'transaction',t.id,t.updated_at,
             json_object('transactionType',t.transaction_type,'amountCents',t.amount_cents,
                         'currency',t.currency,'status',t.status),
             '[]',trim(COALESCE(t.counterparty,'')||' '||COALESCE(t.merchant,'')||' '||
                       COALESCE(t.item,'')||' '||COALESCE(t.note,'')),1,?2
             FROM transactions t
            WHERE t.user_id=?1 AND t.deleted_at IS NULL AND t.status<>'ignored'"#,
        r#"INSERT INTO analytics_events(
             id,user_id,occurred_at,ended_at,local_date,timezone,domain,event_type,title,summary,
             entity_type,entity_id,source_updated_at,metrics_json,tags_json,search_text,
             projection_version,projected_at
           )
           SELECT
             'habits:activity_log:'||l.id,l.user_id,
             COALESCE(strftime('%Y-%m-%dT%H:%M:%fZ',l.created_at),l.log_date||'T00:00:00Z'),
             NULL,l.log_date,NULL,'habits','habit_log',COALESCE(a.name,'坚持记录'),COALESCE(l.note,''),
             'activity_log',l.id,l.updated_at,
             json_object('activityId',l.activity_id,'value',l.value,'status',l.status),
             '[]',trim(COALESCE(a.name,'')||' '||COALESCE(l.note,'')),1,?2
             FROM activity_logs l LEFT JOIN activities a ON a.id=l.activity_id
            WHERE l.user_id=?1 AND l.deleted_at IS NULL"#,
        r#"INSERT INTO analytics_events(
             id,user_id,occurred_at,ended_at,local_date,timezone,domain,event_type,title,summary,
             entity_type,entity_id,source_updated_at,metrics_json,tags_json,search_text,
             projection_version,projected_at
           )
           SELECT
             'habits:daily_review:'||r.id,r.user_id,
             COALESCE(strftime('%Y-%m-%dT%H:%M:%fZ',r.created_at),r.review_date||'T00:00:00Z'),
             NULL,r.review_date,NULL,'habits','daily_review','每日复盘',
             trim(COALESCE(r.best_thing,'')||' '||COALESCE(r.problem,'')||' '||COALESCE(r.note,'')),
             'daily_review',r.id,r.updated_at,
             json_object('energy',r.energy,'mood',r.mood,'completionScore',r.completion_score),
             '[]',trim(COALESCE(r.best_thing,'')||' '||COALESCE(r.problem,'')||' '||
                       COALESCE(r.tomorrow_priority,'')||' '||COALESCE(r.note,'')),1,?2
             FROM daily_reviews r
            WHERE r.user_id=?1 AND r.deleted_at IS NULL"#,
        r#"INSERT INTO analytics_events(
             id,user_id,occurred_at,ended_at,local_date,timezone,domain,event_type,title,summary,
             entity_type,entity_id,source_updated_at,metrics_json,tags_json,search_text,
             projection_version,projected_at
           )
           SELECT
             'notes:note:'||n.id,n.user_id,
             COALESCE(strftime('%Y-%m-%dT%H:%M:%fZ',n.created_at),n.created_at),NULL,
             substr(n.created_at,1,10),NULL,'notes','note',COALESCE(NULLIF(n.title,''),'未命名笔记'),
             COALESCE(NULLIF(n.summary,''),substr(n.content_text,1,240),''),'note',n.id,n.updated_at,
             json_object('noteType',n.note_type,'pinned',n.is_pinned,'favorite',n.is_favorite),
             COALESCE(n.ai_tags_json,'[]'),trim(COALESCE(n.title,'')||' '||COALESCE(n.content_text,'')),1,?2
             FROM notes n WHERE n.user_id=?1 AND n.deleted_at IS NULL"#,
        r#"INSERT INTO analytics_events(
             id,user_id,occurred_at,ended_at,local_date,timezone,domain,event_type,title,summary,
             entity_type,entity_id,source_updated_at,metrics_json,tags_json,search_text,
             projection_version,projected_at
           )
           SELECT
             'english:learning_record:'||r.id,r.user_id,
             COALESCE(strftime('%Y-%m-%dT%H:%M:%fZ',COALESCE(r.completed_at,r.started_at,r.created_at)),
                      COALESCE(r.completed_at,r.started_at,r.created_at)),NULL,r.record_date,NULL,
             'english','english_learning',COALESCE(a.title,'英语学习'),COALESCE(r.summary,''),
             'english_learning_record',r.id,r.updated_at,
             json_object('articleId',r.article_id,'readingTimeSeconds',r.reading_time_seconds,
                         'score',r.score,'completionStatus',r.completion_status),
             '[]',trim(COALESCE(a.title,'')||' '||COALESCE(r.summary,'')),1,?2
             FROM english_learning_records r
             LEFT JOIN english_articles a ON a.id=r.article_id
            WHERE r.user_id=?1 AND r.deleted_at IS NULL"#,
        r#"INSERT INTO analytics_events(
             id,user_id,occurred_at,ended_at,local_date,timezone,domain,event_type,title,summary,
             entity_type,entity_id,source_updated_at,metrics_json,tags_json,search_text,
             projection_version,projected_at
           )
           SELECT
             'fitness:workout:'||w.id,w.user_id,
             COALESCE(strftime('%Y-%m-%dT%H:%M:%fZ',w.occurred_at),w.occurred_at),NULL,w.local_date,NULL,
             'fitness','workout',w.name,'','workout',w.id,w.updated_at,
             json_object('durationSeconds',w.duration_seconds,'exerciseCount',w.exercise_count,
                         'setCount',w.set_count,'volumeKg',w.volume_kg,'caloriesKcal',w.calories_kcal,
                         'source',w.source),
             '[]',w.name,1,?2
             FROM workouts w WHERE w.user_id=?1 AND w.deleted_at IS NULL"#,
        r#"INSERT INTO analytics_events(
             id,user_id,occurred_at,ended_at,local_date,timezone,domain,event_type,title,summary,
             entity_type,entity_id,source_updated_at,metrics_json,tags_json,search_text,
             projection_version,projected_at
           )
           SELECT
             'execution:task:'||t.id,t.user_id,
             COALESCE(strftime('%Y-%m-%dT%H:%M:%fZ',
                               COALESCE(t.completed_at,t.scheduled_start_at,t.due_at,t.created_at)),
                      COALESCE(t.completed_at,t.scheduled_start_at,t.due_at,t.created_at)),
             CASE WHEN t.scheduled_end_at IS NULL THEN NULL
                  ELSE COALESCE(strftime('%Y-%m-%dT%H:%M:%fZ',t.scheduled_end_at),t.scheduled_end_at) END,
             substr(COALESCE(t.completed_at,t.scheduled_start_at,t.due_at,t.created_at),1,10),t.timezone,
             'execution',CASE WHEN t.status='done' THEN 'task_completed' ELSE 'task' END,
             t.title,COALESCE(t.description,''),'execution_task',t.id,t.updated_at,
             json_object('status',t.status,'priority',t.priority,'estimatedMinutes',t.estimated_minutes,
                         'actualMinutes',t.actual_minutes),
             '[]',trim(t.title||' '||COALESCE(t.description,'')||' '||COALESCE(t.context,'')),1,?2
             FROM execution_tasks t WHERE t.user_id=?1 AND t.deleted_at IS NULL"#,
        r#"INSERT INTO analytics_events(
             id,user_id,occurred_at,ended_at,local_date,timezone,domain,event_type,title,summary,
             entity_type,entity_id,source_updated_at,metrics_json,tags_json,search_text,
             projection_version,projected_at
           )
           SELECT
             'execution:calendar:'||e.id,e.user_id,
             CASE WHEN e.is_all_day=1 THEN e.start_local_date||'T00:00:00Z'
                  ELSE COALESCE(strftime('%Y-%m-%dT%H:%M:%fZ',e.start_at),e.start_at) END,
             CASE WHEN e.is_all_day=1 THEN e.end_local_date||'T23:59:59Z'
                  ELSE COALESCE(strftime('%Y-%m-%dT%H:%M:%fZ',e.end_at),e.end_at) END,
             CASE WHEN e.is_all_day=1 THEN e.start_local_date ELSE substr(e.start_at,1,10) END,
             e.timezone,'execution','calendar_event',e.title,COALESCE(e.description,''),
             'calendar_event',e.id,e.updated_at,
             json_object('allDay',e.is_all_day,'status',e.status,'sourceTaskId',e.source_task_id),
             '[]',trim(e.title||' '||COALESCE(e.description,'')),1,?2
             FROM execution_calendar_events e
            WHERE e.user_id=?1 AND e.deleted_at IS NULL AND e.status<>'cancelled'"#,
    ];

    for statement in statements {
        connection
            .execute(statement, params![user_id, stamp])
            .map_err(|error| format!("project analytics events: {error}"))?;
    }
    Ok(())
}

fn project_search_documents(
    connection: &Connection,
    user_id: &str,
    stamp: &str,
) -> Result<(), String> {
    let statements = [
        r#"INSERT INTO analytics_search_documents(
             id,user_id,domain,entity_type,entity_id,title,body,keywords,tags_json,occurred_at,
             updated_at,projection_version,projected_at
           )
           SELECT 'finance:transaction:'||t.id,t.user_id,'finance','transaction',t.id,
             COALESCE(NULLIF(t.counterparty,''),NULLIF(t.merchant,''),NULLIF(t.item,''),
                      NULLIF(t.legacy_category_name,''),'交易'),
             trim(COALESCE(t.note,'')||' '||COALESCE(t.item,'')),
             trim(COALESCE(t.counterparty,'')||' '||COALESCE(t.merchant,'')||' '||
                  COALESCE(t.legacy_category_name,'')||' '||COALESCE(t.transaction_type,'')),
             '[]',t.occurred_at,t.updated_at,1,?2
             FROM transactions t
            WHERE t.user_id=?1 AND t.deleted_at IS NULL AND t.status<>'ignored'"#,
        r#"INSERT INTO analytics_search_documents(
             id,user_id,domain,entity_type,entity_id,title,body,keywords,tags_json,occurred_at,
             updated_at,projection_version,projected_at
           )
           SELECT 'habits:activity:'||a.id,a.user_id,'habits','habit',a.id,a.name,
             COALESCE(a.description,''),trim(COALESCE(a.activity_type,'')||' '||COALESCE(a.unit,'')),
             '[]',a.created_at,a.updated_at,1,?2
             FROM activities a WHERE a.user_id=?1 AND a.deleted_at IS NULL"#,
        r#"INSERT INTO analytics_search_documents(
             id,user_id,domain,entity_type,entity_id,title,body,keywords,tags_json,occurred_at,
             updated_at,projection_version,projected_at
           )
           SELECT 'notes:note:'||n.id,n.user_id,'notes','note',n.id,
             COALESCE(NULLIF(n.title,''),'未命名笔记'),n.content_text,
             trim(COALESCE(n.summary,'')||' '||COALESCE(n.content_markdown,'')),
             COALESCE(n.ai_tags_json,'[]'),n.created_at,n.updated_at,1,?2
             FROM notes n WHERE n.user_id=?1 AND n.deleted_at IS NULL"#,
        r#"INSERT INTO analytics_search_documents(
             id,user_id,domain,entity_type,entity_id,title,body,keywords,tags_json,occurred_at,
             updated_at,projection_version,projected_at
           )
           SELECT 'english:article:'||a.id,r.user_id,'english','english_article',a.id,a.title,a.content,
             trim(COALESCE(a.summary,'')||' '||COALESCE(a.category,'')||' '||COALESCE(a.level,'')),
             '[]',COALESCE(a.published_at,a.created_at),a.updated_at,1,?2
             FROM english_articles a
             JOIN (SELECT user_id,article_id FROM english_learning_records
                    WHERE user_id=?1 AND deleted_at IS NULL AND article_id IS NOT NULL
                    GROUP BY user_id,article_id) r ON r.article_id=a.id
            WHERE a.deleted_at IS NULL"#,
        r#"INSERT INTO analytics_search_documents(
             id,user_id,domain,entity_type,entity_id,title,body,keywords,tags_json,occurred_at,
             updated_at,projection_version,projected_at
           )
           SELECT 'english:vocabulary:'||v.id,v.user_id,'english','vocabulary',v.id,v.display_word,
             trim(COALESCE(v.definition,'')||' '||COALESCE(v.notes,'')||' '||
                  COALESCE(v.source_sentence,'')),
             trim(COALESCE(v.lemma,'')||' '||COALESCE(v.part_of_speech,'')||' '||
                  COALESCE(v.source_article_title,'')),COALESCE(v.tags_json,'[]'),v.created_at,
             v.updated_at,1,?2
             FROM english_vocabulary v WHERE v.user_id=?1 AND v.deleted_at IS NULL"#,
        r#"INSERT INTO analytics_search_documents(
             id,user_id,domain,entity_type,entity_id,title,body,keywords,tags_json,occurred_at,
             updated_at,projection_version,projected_at
           )
           SELECT 'fitness:workout:'||w.id,w.user_id,'fitness','workout',w.id,w.name,
             trim('时长 '||w.duration_seconds||' 秒，动作 '||w.exercise_count||' 个，组数 '||w.set_count),
             trim(COALESCE(w.source,'')||' '||COALESCE(w.status,'')),'[]',w.occurred_at,w.updated_at,1,?2
             FROM workouts w WHERE w.user_id=?1 AND w.deleted_at IS NULL"#,
        r#"INSERT INTO analytics_search_documents(
             id,user_id,domain,entity_type,entity_id,title,body,keywords,tags_json,occurred_at,
             updated_at,projection_version,projected_at
           )
           SELECT 'execution:task:'||t.id,t.user_id,'execution','execution_task',t.id,t.title,
             COALESCE(t.description,''),trim(COALESCE(t.context,'')||' '||t.status||' '||t.priority),
             '[]',COALESCE(t.completed_at,t.scheduled_start_at,t.due_at,t.created_at),t.updated_at,1,?2
             FROM execution_tasks t WHERE t.user_id=?1 AND t.deleted_at IS NULL"#,
        r#"INSERT INTO analytics_search_documents(
             id,user_id,domain,entity_type,entity_id,title,body,keywords,tags_json,occurred_at,
             updated_at,projection_version,projected_at
           )
           SELECT 'execution:calendar:'||e.id,e.user_id,'execution','calendar_event',e.id,e.title,
             COALESCE(e.description,''),trim(COALESCE(e.timezone,'')||' '||e.status),'[]',
             CASE WHEN e.is_all_day=1 THEN e.start_local_date ELSE e.start_at END,e.updated_at,1,?2
             FROM execution_calendar_events e WHERE e.user_id=?1 AND e.deleted_at IS NULL"#,
        r#"INSERT INTO analytics_search_documents(
             id,user_id,domain,entity_type,entity_id,title,body,keywords,tags_json,occurred_at,
             updated_at,projection_version,projected_at
           )
           SELECT 'execution:memo:'||m.id,m.user_id,'execution','memo',m.id,
             substr(m.plain_text,1,80),m.plain_text,trim(COALESCE(m.context,'')||' '||m.status),'[]',
             m.created_at,m.updated_at,1,?2
             FROM execution_memos m WHERE m.user_id=?1 AND m.deleted_at IS NULL"#,
    ];

    for statement in statements {
        connection
            .execute(statement, params![user_id, stamp])
            .map_err(|error| format!("project search documents: {error}"))?;
    }
    Ok(())
}

pub fn timeline(
    connection: &Connection,
    user_id: &str,
    query: &TimelineQuery,
) -> Result<TimelinePage, String> {
    let limit = query.limit.unwrap_or(60).clamp(1, 200);
    let mut sql = String::from(
        "SELECT id,occurred_at,ended_at,local_date,timezone,domain,event_type,title,summary,
                entity_type,entity_id,metrics_json,tags_json
           FROM analytics_events WHERE user_id=?",
    );
    let mut values = vec![SqlValue::Text(user_id.to_owned())];
    if let Some(from) = query.from.as_deref().filter(|value| !value.is_empty()) {
        sql.push_str(" AND local_date>=?");
        values.push(SqlValue::Text(from.to_owned()));
    }
    if let Some(to) = query.to.as_deref().filter(|value| !value.is_empty()) {
        sql.push_str(" AND local_date<=?");
        values.push(SqlValue::Text(to.to_owned()));
    }
    if let Some(domain) = query.domain.as_deref().filter(|value| !value.is_empty()) {
        sql.push_str(" AND domain=?");
        values.push(SqlValue::Text(domain.to_owned()));
    }
    if let Some(event_type) = query
        .event_type
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        sql.push_str(" AND event_type=?");
        values.push(SqlValue::Text(event_type.to_owned()));
    }
    if let Some(keyword) = query
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        sql.push_str(" AND (title LIKE ? COLLATE NOCASE OR summary LIKE ? COLLATE NOCASE OR search_text LIKE ? COLLATE NOCASE)");
        let pattern = format!("%{keyword}%");
        values.push(SqlValue::Text(pattern.clone()));
        values.push(SqlValue::Text(pattern.clone()));
        values.push(SqlValue::Text(pattern));
    }
    if let Some((occurred_at, id)) = query
        .cursor
        .as_deref()
        .and_then(|value| value.rsplit_once('|'))
    {
        sql.push_str(" AND (occurred_at<? OR (occurred_at=? AND id<?))");
        values.push(SqlValue::Text(occurred_at.to_owned()));
        values.push(SqlValue::Text(occurred_at.to_owned()));
        values.push(SqlValue::Text(id.to_owned()));
    }
    sql.push_str(" ORDER BY occurred_at DESC,id DESC LIMIT ?");
    values.push(SqlValue::Integer((limit + 1) as i64));

    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params_from_iter(values.iter()), |row| {
            let metrics_raw: String = row.get(11)?;
            let tags_raw: String = row.get(12)?;
            Ok(TimelineEvent {
                id: row.get(0)?,
                occurred_at: row.get(1)?,
                ended_at: row.get(2)?,
                local_date: row.get(3)?,
                timezone: row.get(4)?,
                domain: row.get(5)?,
                event_type: row.get(6)?,
                title: row.get(7)?,
                summary: row.get(8)?,
                entity_type: row.get(9)?,
                entity_id: row.get(10)?,
                metrics: json_or(metrics_raw, json!({})),
                tags: json_or(tags_raw, json!([])),
            })
        })
        .map_err(|error| error.to_string())?;
    let mut items = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let has_more = items.len() > limit;
    if has_more {
        items.truncate(limit);
    }
    let next_cursor = if has_more {
        items
            .last()
            .map(|item| format!("{}|{}", item.occurred_at, item.id))
    } else {
        None
    };
    Ok(TimelinePage { items, next_cursor })
}

pub fn search(
    connection: &Connection,
    user_id: &str,
    query: &SearchQuery,
) -> Result<Vec<SearchHit>, String> {
    let keyword = query.q.trim();
    if keyword.is_empty() {
        return Ok(Vec::new());
    }
    let limit = query.limit.unwrap_or(50).clamp(1, 100) as i64;
    let exact = keyword.to_owned();
    let prefix = format!("{keyword}%");
    let contains = format!("%{keyword}%");
    let mut sql = String::from(
        "SELECT id,domain,entity_type,entity_id,title,
                CASE WHEN length(body)>180 THEN substr(body,1,180)||'…' ELSE body END,
                occurred_at,updated_at,
                CASE
                  WHEN lower(title)=lower(?) THEN 100
                  WHEN title LIKE ? COLLATE NOCASE THEN 80
                  WHEN title LIKE ? COLLATE NOCASE THEN 60
                  WHEN keywords LIKE ? COLLATE NOCASE THEN 40
                  ELSE 20
                END AS score
           FROM analytics_search_documents
          WHERE user_id=?
            AND (title LIKE ? COLLATE NOCASE OR body LIKE ? COLLATE NOCASE OR keywords LIKE ? COLLATE NOCASE)",
    );
    let mut values = vec![
        SqlValue::Text(exact),
        SqlValue::Text(prefix),
        SqlValue::Text(contains.clone()),
        SqlValue::Text(contains.clone()),
        SqlValue::Text(user_id.to_owned()),
        SqlValue::Text(contains.clone()),
        SqlValue::Text(contains.clone()),
        SqlValue::Text(contains),
    ];
    if let Some(domain) = query.domain.as_deref().filter(|value| !value.is_empty()) {
        sql.push_str(" AND domain=?");
        values.push(SqlValue::Text(domain.to_owned()));
    }
    if let Some(from) = query.from.as_deref().filter(|value| !value.is_empty()) {
        sql.push_str(" AND (occurred_at IS NULL OR substr(occurred_at,1,10)>=?)");
        values.push(SqlValue::Text(from.to_owned()));
    }
    if let Some(to) = query.to.as_deref().filter(|value| !value.is_empty()) {
        sql.push_str(" AND (occurred_at IS NULL OR substr(occurred_at,1,10)<=?)");
        values.push(SqlValue::Text(to.to_owned()));
    }
    sql.push_str(" ORDER BY score DESC,updated_at DESC LIMIT ?");
    values.push(SqlValue::Integer(limit));

    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params_from_iter(values.iter()), |row| {
            Ok(SearchHit {
                id: row.get(0)?,
                domain: row.get(1)?,
                entity_type: row.get(2)?,
                entity_id: row.get(3)?,
                title: row.get(4)?,
                snippet: row.get(5)?,
                occurred_at: row.get(6)?,
                updated_at: row.get(7)?,
                score: row.get(8)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn count(
    connection: &Connection,
    sql: &str,
    values: &[&dyn rusqlite::ToSql],
) -> Result<i64, String> {
    connection
        .query_row(sql, values, |row| row.get::<_, i64>(0))
        .map_err(|error| error.to_string())
}

pub fn generate_report(
    connection: &Connection,
    user_id: &str,
    report_type: &str,
    period_start: &str,
    period_end: &str,
    timezone: &str,
) -> Result<ReportSnapshot, String> {
    if !matches!(report_type, "weekly" | "monthly" | "custom") {
        return Err(format!("unsupported report type: {report_type}"));
    }
    if period_start > period_end {
        return Err("periodStart must not be after periodEnd".to_owned());
    }

    let params_range: [&dyn rusqlite::ToSql; 3] = [&user_id, &period_start, &period_end];
    let transaction_count = count(
        connection,
        "SELECT COUNT(*) FROM transactions
          WHERE user_id=?1 AND local_date BETWEEN ?2 AND ?3 AND deleted_at IS NULL AND status<>'ignored'",
        &params_range,
    )?;
    let expense_cents = count(
        connection,
        "SELECT COALESCE(SUM(amount_cents),0) FROM transactions
          WHERE user_id=?1 AND local_date BETWEEN ?2 AND ?3 AND deleted_at IS NULL
            AND status<>'ignored' AND transaction_type IN ('expense','fee')",
        &params_range,
    )?;
    let income_cents = count(
        connection,
        "SELECT COALESCE(SUM(amount_cents),0) FROM transactions
          WHERE user_id=?1 AND local_date BETWEEN ?2 AND ?3 AND deleted_at IS NULL
            AND status<>'ignored' AND transaction_type IN ('income','refund')",
        &params_range,
    )?;

    let habit_logs = count(
        connection,
        "SELECT COUNT(*) FROM activity_logs
          WHERE user_id=?1 AND log_date BETWEEN ?2 AND ?3 AND deleted_at IS NULL",
        &params_range,
    )?;
    let habit_completed = count(
        connection,
        "SELECT COUNT(*) FROM activity_logs
          WHERE user_id=?1 AND log_date BETWEEN ?2 AND ?3 AND deleted_at IS NULL AND status='completed'",
        &params_range,
    )?;
    let habit_completion_rate = if habit_logs == 0 {
        0.0
    } else {
        habit_completed as f64 / habit_logs as f64
    };

    let workout_row = connection
        .query_row(
            "SELECT COUNT(*),COALESCE(SUM(duration_seconds),0),COALESCE(SUM(volume_kg),0.0),
                    COALESCE(SUM(calories_kcal),0.0)
               FROM workouts
              WHERE user_id=?1 AND local_date BETWEEN ?2 AND ?3 AND deleted_at IS NULL",
            params![user_id, period_start, period_end],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, f64>(3)?,
                ))
            },
        )
        .map_err(|error| error.to_string())?;

    let english_row = connection
        .query_row(
            "SELECT COUNT(*),COALESCE(SUM(reading_time_seconds),0),
                    COALESCE(SUM(CASE WHEN completion_status='completed' OR completed_at IS NOT NULL THEN 1 ELSE 0 END),0)
               FROM english_learning_records
              WHERE user_id=?1 AND record_date BETWEEN ?2 AND ?3 AND deleted_at IS NULL",
            params![user_id, period_start, period_end],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .map_err(|error| error.to_string())?;

    let vocabulary_count = count(
        connection,
        "SELECT COUNT(*) FROM english_vocabulary
          WHERE user_id=?1 AND substr(created_at,1,10) BETWEEN ?2 AND ?3 AND deleted_at IS NULL",
        &params_range,
    )?;
    let note_count = count(
        connection,
        "SELECT COUNT(*) FROM notes
          WHERE user_id=?1 AND substr(created_at,1,10) BETWEEN ?2 AND ?3 AND deleted_at IS NULL",
        &params_range,
    )?;
    let task_row = connection
        .query_row(
            "SELECT COUNT(*),COALESCE(SUM(CASE WHEN status='done' THEN 1 ELSE 0 END),0)
               FROM execution_tasks
              WHERE user_id=?1
                AND substr(COALESCE(completed_at,scheduled_start_at,due_at,created_at),1,10) BETWEEN ?2 AND ?3
                AND deleted_at IS NULL",
            params![user_id, period_start, period_end],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(|error| error.to_string())?;
    let calendar_count = count(
        connection,
        "SELECT COUNT(*) FROM execution_calendar_events
          WHERE user_id=?1
            AND (CASE WHEN is_all_day=1 THEN start_local_date ELSE substr(start_at,1,10) END) BETWEEN ?2 AND ?3
            AND deleted_at IS NULL AND status<>'cancelled'",
        &params_range,
    )?;
    let review_row = connection
        .query_row(
            "SELECT COUNT(*),COALESCE(AVG(mood),0.0),COALESCE(AVG(energy),0.0)
               FROM daily_reviews
              WHERE user_id=?1 AND review_date BETWEEN ?2 AND ?3 AND deleted_at IS NULL",
            params![user_id, period_start, period_end],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, f64>(2)?,
                ))
            },
        )
        .map_err(|error| error.to_string())?;

    let coverage = json!({
        "finance": transaction_count > 0,
        "habits": habit_logs > 0,
        "fitness": workout_row.0 > 0,
        "english": english_row.0 > 0 || vocabulary_count > 0,
        "notes": note_count > 0,
        "execution": task_row.0 > 0 || calendar_count > 0,
        "reviews": review_row.0 > 0
    });
    let facts = json!({
        "period": { "start": period_start, "end": period_end, "timezone": timezone },
        "finance": {
            "transactionCount": transaction_count,
            "expenseCents": expense_cents,
            "incomeCents": income_cents,
            "netCents": income_cents - expense_cents
        },
        "habits": {
            "logCount": habit_logs,
            "completedCount": habit_completed,
            "completionRate": habit_completion_rate
        },
        "fitness": {
            "workoutCount": workout_row.0,
            "durationSeconds": workout_row.1,
            "volumeKg": workout_row.2,
            "caloriesKcal": workout_row.3
        },
        "english": {
            "sessionCount": english_row.0,
            "readingTimeSeconds": english_row.1,
            "completedCount": english_row.2,
            "newVocabularyCount": vocabulary_count
        },
        "notes": { "createdCount": note_count },
        "execution": {
            "taskCount": task_row.0,
            "completedTaskCount": task_row.1,
            "calendarEventCount": calendar_count
        },
        "reviews": {
            "count": review_row.0,
            "averageMood": review_row.1,
            "averageEnergy": review_row.2
        }
    });
    let generated_at = now();
    let id = format!(
        "report:{user_id}:{report_type}:{period_start}:{period_end}:{timezone}:v{FACTS_VERSION}"
    );
    connection
        .execute(
            "INSERT INTO analytics_reports(
               id,user_id,report_type,period_start,period_end,timezone,facts_json,narrative_json,
               source_coverage_json,facts_version,generated_at,updated_at
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,NULL,?8,?9,?10,?10)
             ON CONFLICT(user_id,report_type,period_start,period_end,timezone,facts_version)
             DO UPDATE SET facts_json=excluded.facts_json,
                           source_coverage_json=excluded.source_coverage_json,
                           generated_at=excluded.generated_at,
                           updated_at=excluded.updated_at",
            params![
                id,
                user_id,
                report_type,
                period_start,
                period_end,
                timezone,
                facts.to_string(),
                coverage.to_string(),
                FACTS_VERSION,
                generated_at
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(ReportSnapshot {
        id,
        report_type: report_type.to_owned(),
        period_start: period_start.to_owned(),
        period_end: period_end.to_owned(),
        timezone: timezone.to_owned(),
        facts,
        coverage,
        generated_at,
        facts_version: FACTS_VERSION,
    })
}

pub fn generate_insights(
    connection: &Connection,
    user_id: &str,
    period_start: &str,
    period_end: &str,
) -> Result<Vec<InsightSnapshot>, String> {
    if period_start > period_end {
        return Err("periodStart must not be after periodEnd".to_owned());
    }
    let stamp = now();
    connection
        .execute(
            "DELETE FROM analytics_insights
              WHERE user_id=?1 AND period_start=?2 AND period_end=?3
                AND algorithm_version=?4",
            params![user_id, period_start, period_end, INSIGHT_ALGORITHM_VERSION],
        )
        .map_err(|error| error.to_string())?;

    let mut insights = Vec::new();
    let overlap = connection
        .query_row(
            "WITH habit_days AS (
               SELECT DISTINCT log_date AS day FROM activity_logs
                WHERE user_id=?1 AND log_date BETWEEN ?2 AND ?3
                  AND deleted_at IS NULL AND status='completed'
             ), task_days AS (
               SELECT DISTINCT substr(completed_at,1,10) AS day FROM execution_tasks
                WHERE user_id=?1 AND completed_at IS NOT NULL
                  AND substr(completed_at,1,10) BETWEEN ?2 AND ?3
                  AND deleted_at IS NULL AND status='done'
             ), active_days AS (
               SELECT day FROM habit_days UNION SELECT day FROM task_days
             )
             SELECT
               (SELECT COUNT(*) FROM active_days),
               (SELECT COUNT(*) FROM habit_days),
               (SELECT COUNT(*) FROM task_days),
               (SELECT COUNT(*) FROM habit_days h INNER JOIN task_days t ON t.day=h.day)",
            params![user_id, period_start, period_end],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .map_err(|error| error.to_string())?;
    if overlap.0 >= 7 && overlap.1 > 0 && overlap.2 > 0 {
        let ratio = overlap.3 as f64 / overlap.0 as f64;
        let confidence = if overlap.0 >= 21 { "medium" } else { "low" };
        insights.push(InsightSnapshot {
            id: format!("insight:{user_id}:habit-execution:{period_start}:{period_end}"),
            insight_type: "habit_execution_overlap".to_owned(),
            period_start: period_start.to_owned(),
            period_end: period_end.to_owned(),
            title: "坚持记录与任务完成的同期情况".to_owned(),
            summary: format!(
                "本周期有 {} 个活跃记录日，其中 {} 天同时出现了已完成的坚持记录和已完成任务。该结果表示同期共现，不代表因果关系。",
                overlap.0, overlap.3
            ),
            evidence: json!({
                "activeDays": overlap.0,
                "habitCompletedDays": overlap.1,
                "taskCompletedDays": overlap.2,
                "overlapDays": overlap.3,
                "overlapRatio": ratio
            }),
            sample_size: overlap.0,
            confidence: json!({ "level": confidence, "causal": false }),
            algorithm_version: INSIGHT_ALGORITHM_VERSION.to_owned(),
        });
    }

    let english_notes = connection
        .query_row(
            "WITH learned AS (
               SELECT DISTINCT article_id FROM english_learning_records
                WHERE user_id=?1 AND record_date BETWEEN ?2 AND ?3
                  AND deleted_at IS NULL AND article_id IS NOT NULL
             ), noted AS (
               SELECT DISTINCT n.article_id FROM english_notes n
                JOIN learned l ON l.article_id=n.article_id
               WHERE n.user_id=?1 AND n.deleted_at IS NULL
             )
             SELECT (SELECT COUNT(*) FROM learned),(SELECT COUNT(*) FROM noted)",
            params![user_id, period_start, period_end],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(|error| error.to_string())?;
    if english_notes.0 >= 3 {
        let ratio = english_notes.1 as f64 / english_notes.0 as f64;
        insights.push(InsightSnapshot {
            id: format!("insight:{user_id}:english-notes:{period_start}:{period_end}"),
            insight_type: "english_reading_notes".to_owned(),
            period_start: period_start.to_owned(),
            period_end: period_end.to_owned(),
            title: "英语阅读与学习笔记关联".to_owned(),
            summary: format!(
                "本周期学习了 {} 篇文章，其中 {} 篇留下了英语学习笔记。",
                english_notes.0, english_notes.1
            ),
            evidence: json!({
                "learnedArticleCount": english_notes.0,
                "notedArticleCount": english_notes.1,
                "noteCoverage": ratio
            }),
            sample_size: english_notes.0,
            confidence: json!({ "level": "descriptive", "causal": false }),
            algorithm_version: INSIGHT_ALGORITHM_VERSION.to_owned(),
        });
    }

    for insight in &insights {
        connection
            .execute(
                "INSERT INTO analytics_insights(
                   id,user_id,insight_type,period_start,period_end,title,summary,evidence_json,
                   sample_size,confidence_json,algorithm_version,created_at,updated_at
                 ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?12)",
                params![
                    insight.id,
                    user_id,
                    insight.insight_type,
                    insight.period_start,
                    insight.period_end,
                    insight.title,
                    insight.summary,
                    insight.evidence.to_string(),
                    insight.sample_size,
                    insight.confidence.to_string(),
                    insight.algorithm_version,
                    stamp
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(insights)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{
        migration_runner::{run, MigrationContext},
        migrations::all,
    };
    use rusqlite::Connection;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn setup() -> (Connection, String) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("lifetrace-analytics-{unique}"));
        std::fs::create_dir_all(&directory).unwrap();
        let context = MigrationContext::new(directory);
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(&mut connection, &context, &all()).unwrap();
        let user_id: String = connection
            .query_row(
                "SELECT active_profile_id FROM app_profile_state WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        (connection, user_id)
    }

    #[test]
    fn rebuild_is_idempotent_and_dirty_state_tracks_source_changes() {
        let (mut connection, user_id) = setup();
        connection
            .execute(
                "INSERT INTO transactions(
                   id,user_id,transaction_type,amount_cents,currency,counterparty,occurred_at,local_date,
                   status,source_type,created_at,updated_at,version
                 ) VALUES('tx1',?1,'expense',2500,'CNY','咖啡店','2026-08-09T08:00:00Z',
                          '2026-08-09','confirmed','manual','2026-08-09T08:00:00Z',
                          '2026-08-09T08:00:00Z',1)",
                [&user_id],
            )
            .unwrap();
        let first = ensure_current(&mut connection, &user_id).unwrap();
        assert!(!first.dirty);
        assert_eq!(first.event_count, 1);
        assert_eq!(first.search_document_count, 1);

        let second = rebuild(&mut connection, &user_id).unwrap();
        assert_eq!(second.event_count, 1);
        assert_eq!(second.search_document_count, 1);

        connection
            .execute(
                "UPDATE transactions SET note='加班咖啡',updated_at='2026-08-09T09:00:00Z' WHERE id='tx1'",
                [],
            )
            .unwrap();
        assert!(projection_status(&connection, &user_id).unwrap().dirty);
        ensure_current(&mut connection, &user_id).unwrap();
        let hits = search(
            &connection,
            &user_id,
            &SearchQuery {
                q: "加班咖啡".to_owned(),
                ..SearchQuery::default()
            },
        )
        .unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn timeline_and_report_use_deterministic_source_facts() {
        let (mut connection, user_id) = setup();
        connection
            .execute(
                "INSERT INTO workouts(
                   id,user_id,source,name,occurred_at,local_date,duration_seconds,exercise_count,set_count,
                   volume_kg,calories_kcal,created_at,updated_at,version
                 ) VALUES('w1',?1,'manual','胸部训练','2026-08-09T10:00:00Z','2026-08-09',3600,4,16,
                          5200.0,450.0,'2026-08-09T10:00:00Z','2026-08-09T11:00:00Z',1)",
                [&user_id],
            )
            .unwrap();
        ensure_current(&mut connection, &user_id).unwrap();
        let page = timeline(
            &connection,
            &user_id,
            &TimelineQuery {
                domain: Some("fitness".to_owned()),
                ..TimelineQuery::default()
            },
        )
        .unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].title, "胸部训练");

        let report = generate_report(
            &connection,
            &user_id,
            "weekly",
            "2026-08-03",
            "2026-08-09",
            "Asia/Shanghai",
        )
        .unwrap();
        assert_eq!(report.facts["fitness"]["workoutCount"], 1);
        assert_eq!(report.facts["fitness"]["durationSeconds"], 3600);
        assert_eq!(report.facts["fitness"]["volumeKg"], 5200.0);
    }
}
