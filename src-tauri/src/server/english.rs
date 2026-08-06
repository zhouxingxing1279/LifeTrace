use std::collections::{BTreeMap, HashMap};

use axum::{
    body::{to_bytes, Body},
    extract::{Path, State},
    http::{Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Duration, Utc};
use regex::Regex;
use rusqlite::{params, Connection, OptionalExtension};
use scraper::{Html, Selector};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use super::AppState;

// 运行配置类表仍使用 JSON 结构；业务实体表已规范化，由 Repository 读写。
const JSON_ENTITY_TABLES: [(&str, &str); 2] = [
    ("sources", "english_sources"),
    ("tasks", "english_sync_tasks"),
];

const DEFAULT_SETTINGS: &str = r#"{
  "preferredAccent":"en-US","wordSpeechRate":0.85,"sentenceSpeechRate":0.95,
  "autoPronounce":true,"defaultFirstMeaning":true,"dailyReviewLimit":20,
  "showSourceSentence":true,"includeMasteredInRecommendations":false
}"#;

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn id() -> String {
    Uuid::new_v4().to_string()
}

fn error(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(json!({ "error": message.into() }))).into_response()
}

fn table(key: &str) -> Option<&'static str> {
    JSON_ENTITY_TABLES
        .iter()
        .find_map(|(name, table)| (*name == key).then_some(*table))
}

fn put(connection: &Connection, key: &str, value: &Value) -> Result<(), String> {
    let Some(table) = table(key) else {
        return crate::database::repositories::english::put(connection, key, value);
    };
    let entity_id = value
        .get("id")
        .or_else(|| value.get("taskId"))
        .and_then(Value::as_str)
        .ok_or_else(|| "数据缺少 id".to_owned())?;
    let stamp = value
        .get("updatedAt")
        .and_then(Value::as_str)
        .unwrap_or_else(|| "");
    let stamp = if stamp.is_empty() {
        now()
    } else {
        stamp.to_owned()
    };
    connection
        .execute(
            &format!(
                "INSERT INTO {table}(id,data_json,updated_at) VALUES(?1,?2,?3)
                 ON CONFLICT(id) DO UPDATE SET data_json=excluded.data_json,updated_at=excluded.updated_at"
            ),
            params![entity_id, value.to_string(), stamp],
        )
        .map_err(|value| value.to_string())?;
    Ok(())
}

fn list(connection: &Connection, key: &str) -> Result<Vec<Value>, String> {
    let Some(table) = table(key) else {
        return crate::database::repositories::english::list(connection, key);
    };
    let mut statement = connection
        .prepare(&format!(
            "SELECT data_json FROM {table} ORDER BY updated_at DESC"
        ))
        .map_err(|value| value.to_string())?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|value| value.to_string())?;
    rows.map(|row| {
        let raw = row.map_err(|value| value.to_string())?;
        serde_json::from_str(&raw).map_err(|value| value.to_string())
    })
    .collect()
}

fn get(connection: &Connection, key: &str, entity_id: &str) -> Result<Option<Value>, String> {
    let Some(table) = table(key) else {
        return crate::database::repositories::english::get(connection, key, entity_id);
    };
    let raw = connection
        .query_row(
            &format!("SELECT data_json FROM {table} WHERE id=?1"),
            [entity_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|value| value.to_string())?;
    raw.map(|value| serde_json::from_str(&value).map_err(|value| value.to_string()))
        .transpose()
}

fn remove(connection: &Connection, key: &str, entity_id: &str) -> Result<bool, String> {
    let Some(table) = table(key) else {
        return crate::database::repositories::english::remove(connection, key, entity_id);
    };
    connection
        .execute(&format!("DELETE FROM {table} WHERE id=?1"), [entity_id])
        .map(|count| count > 0)
        .map_err(|value| value.to_string())
}

fn query(uri: &Uri) -> HashMap<String, String> {
    url::form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes())
        .into_owned()
        .collect()
}

fn body_json(body: Body) -> impl std::future::Future<Output = Result<Value, String>> {
    async move {
        let bytes = to_bytes(body, 2 * 1024 * 1024)
            .await
            .map_err(|value| value.to_string())?;
        if bytes.is_empty() {
            Ok(json!({}))
        } else {
            serde_json::from_slice(&bytes).map_err(|value| value.to_string())
        }
    }
}

pub fn ensure_schema(connection: &Connection) -> rusqlite::Result<()> {
    // 业务实体表已由版本化 Migration 创建，这里只保留配置类表与种子数据。
    connection.execute(
        "CREATE TABLE IF NOT EXISTS english_sources(
           id TEXT PRIMARY KEY,data_json TEXT NOT NULL,updated_at TEXT NOT NULL
         )",
        [],
    )?;
    connection.execute(
        "CREATE TABLE IF NOT EXISTS english_sync_tasks(
           id TEXT PRIMARY KEY,data_json TEXT NOT NULL,updated_at TEXT NOT NULL
         )",
        [],
    )?;
    connection.execute(
        "CREATE TABLE IF NOT EXISTS english_preferences(
           key TEXT PRIMARY KEY,value_json TEXT NOT NULL,updated_at TEXT NOT NULL
         )",
        [],
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO english_preferences(key,value_json,updated_at) VALUES('vocabulary',?1,?2)",
        params![DEFAULT_SETTINGS, now()],
    )?;
    seed_articles(connection)?;
    seed_sources(connection)?;
    Ok(())
}

fn seed_articles(connection: &Connection) -> rusqlite::Result<()> {
    let count: i64 = connection.query_row("SELECT COUNT(*) FROM english_articles", [], |row| {
        row.get(0)
    })?;
    if count > 0 {
        return Ok(());
    }
    let stamp = now();
    let seeds = [
        (
            "local-healthy-habits",
            "Small Habits Can Create Lasting Change",
            "Life",
            "B1",
            "Researchers say lasting change often begins with a very small action. A person who wants to exercise more might start with a ten-minute walk after dinner. The action is easy to repeat, and repetition helps the brain connect the behavior with a regular time and place.\n\nExperts also recommend making progress visible. A simple calendar or journal can show how often the habit is completed. Missing one day is not a failure. The important step is to return to the routine quickly and continue.",
        ),
        (
            "local-urban-trees",
            "Why Cities Need More Trees",
            "Science",
            "B1",
            "Trees make city streets cooler by providing shade and releasing water into the air. Studies also show that green spaces can support mental health and give birds and insects places to live.\n\nPlanting trees is only the first step. Cities must choose species that can survive local weather, provide enough soil, and care for young trees during dry periods. Good planning helps urban forests remain healthy for many years.",
        ),
        (
            "local-ai-work",
            "How Artificial Intelligence Is Changing Daily Work",
            "Technology",
            "B2",
            "Artificial intelligence tools can summarize documents, organize information, and help people explore possible solutions. Their greatest value often comes from supporting human judgment rather than replacing it.\n\nWorkers still need to check facts, protect private information, and understand the limits of automated systems. Organizations that introduce the technology gradually can learn where it saves time and where careful human review remains essential.",
        ),
    ];
    for (article_id, title, category, level, content) in seeds {
        let word_count = content.split_whitespace().count();
        let article = json!({
            "id": article_id, "title": title, "level": level, "category": category,
            "content": content, "vocabulary": [], "questions": [],
            "difficulty": if level == "B2" { 4 } else { 3 },
            "estimatedMinutes": ((word_count as f64 / 120.0).ceil() as usize).max(2),
            "createdTime": stamp, "updatedAt": stamp, "source": "local",
            "wordCount": word_count, "language": "en", "qualityScore": 80,
            "hasAudio": false, "processingStatus": "READY", "fetchStatus": "SUCCESS"
        });
        put(connection, "articles", &article)
            .map_err(|message| rusqlite::Error::ToSqlConversionFailure(message.into()))?;
    }
    Ok(())
}

fn seed_sources(connection: &Connection) -> rusqlite::Result<()> {
    let count: i64 =
        connection.query_row("SELECT COUNT(*) FROM english_sources", [], |row| row.get(0))?;
    if count > 0 {
        return Ok(());
    }
    let stamp = now();
    let sources = [
        (
            "voa-science",
            "VOA Science & Technology",
            "science",
            "https://learningenglish.voanews.com/api/zmg_pl-vomx-tpeymtm",
        ),
        (
            "voa-health",
            "VOA Health & Lifestyle",
            "health",
            "https://learningenglish.voanews.com/api/zmmpql-vomx-tpey-_q",
        ),
        (
            "voa-words",
            "VOA Words and Their Stories",
            "words",
            "https://learningenglish.voanews.com/api/zmypyl-vomx-tpeyry_",
        ),
    ];
    for (source_key, source_name, category, source_url) in sources {
        let source = json!({
            "id": source_key, "sourceKey": source_key, "sourceName": source_name,
            "sourceType": "rss", "sourceUrl": source_url, "category": category,
            "enabled": true, "syncInterval": 86400, "initialFetchLimit": 12,
            "recentScanLimit": 8, "overlapDays": 7, "requestIntervalMs": 350,
            "consecutiveFailures": 0, "status": "active", "articleCount": 0,
            "createdAt": stamp, "updatedAt": stamp
        });
        put(connection, "sources", &source)
            .map_err(|message| rusqlite::Error::ToSqlConversionFailure(message.into()))?;
    }
    Ok(())
}

fn reading_status(record: Option<&Value>) -> &'static str {
    match record
        .and_then(|value| value.get("readingStatus"))
        .and_then(Value::as_str)
    {
        Some("completed") => "completed",
        Some("reading") => "reading",
        _ => "unread",
    }
}

fn record_for<'a>(records: &'a [Value], article_id: &str) -> Option<&'a Value> {
    records
        .iter()
        .find(|value| value.get("articleId").and_then(Value::as_str) == Some(article_id))
}

fn date_key() -> String {
    (Utc::now() + Duration::hours(8))
        .format("%Y-%m-%d")
        .to_string()
}

fn annotations(connection: &Connection, article_id: &str) -> Result<Value, String> {
    let highlights = list(connection, "highlights")?
        .into_iter()
        .filter(|value| value.get("articleId").and_then(Value::as_str) == Some(article_id))
        .collect::<Vec<_>>();
    let notes = list(connection, "notes")?
        .into_iter()
        .filter(|value| value.get("articleId").and_then(Value::as_str) == Some(article_id))
        .collect::<Vec<_>>();
    Ok(json!({ "highlights": highlights, "notes": notes }))
}

fn article_page(
    connection: &Connection,
    params: &HashMap<String, String>,
) -> Result<Value, String> {
    let records = list(connection, "records")?;
    let mut articles = list(connection, "articles")?;
    articles.retain(|article| {
        let level_matches = params
            .get("level")
            .is_none_or(|level| article.get("level").and_then(Value::as_str) == Some(level));
        let category_matches = params.get("category").is_none_or(|category| {
            article.get("category").and_then(Value::as_str) == Some(category)
        });
        let query_matches = params.get("q").is_none_or(|term| {
            article
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_lowercase()
                .contains(&term.to_lowercase())
        });
        level_matches && category_matches && query_matches
    });
    for article in &mut articles {
        if let Some(object) = article.as_object_mut() {
            let article_id = object.get("id").and_then(Value::as_str).unwrap_or_default();
            let record = record_for(&records, article_id);
            object.insert("readingStatus".to_owned(), json!(reading_status(record)));
            if params.get("summary").is_some_and(|value| value == "1") {
                if let Some(content) = object.get("content").and_then(Value::as_str) {
                    object.insert(
                        "content".to_owned(),
                        json!(content.chars().take(360).collect::<String>()),
                    );
                }
            }
        }
    }
    if !params.contains_key("page") {
        return Ok(json!({ "articles": articles }));
    }
    let page = params
        .get("page")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1)
        .max(1);
    let page_size = params
        .get("pageSize")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(18)
        .clamp(6, 48);
    let total = articles.len();
    let offset = (page - 1) * page_size;
    let items = articles
        .into_iter()
        .skip(offset)
        .take(page_size)
        .collect::<Vec<_>>();
    Ok(json!({
        "articles": items, "total": total, "page": page, "pageSize": page_size,
        "hasMore": offset + page_size < total
    }))
}

fn vocabulary_stats(connection: &Connection) -> Result<Value, String> {
    let items = list(connection, "vocabulary")?;
    let now = now();
    let week = (Utc::now() - Duration::days(7)).to_rfc3339();
    Ok(json!({
        "dueToday": items.iter().filter(|item| {
            item.get("status").and_then(Value::as_str) != Some("MASTERED")
                && item.get("nextReviewAt").and_then(Value::as_str).is_none_or(|due| due <= now.as_str())
        }).count(),
        "addedWeek": items.iter().filter(|item| item.get("createdAt").and_then(Value::as_str).is_some_and(|stamp| stamp >= week.as_str())).count(),
        "mastered": items.iter().filter(|item| item.get("status").and_then(Value::as_str) == Some("MASTERED")).count(),
        "total": items.len()
    }))
}

fn vocabulary_list(
    connection: &Connection,
    params: &HashMap<String, String>,
) -> Result<Value, String> {
    let mut items = list(connection, "vocabulary")?;
    items.retain(|item| {
        let query_matches = params.get("query").is_none_or(|query| {
            let query = query.to_lowercase();
            ["word", "lemma", "notes"].iter().any(|key| {
                item.get(key)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains(&query)
            })
        });
        let status_matches = params
            .get("status")
            .is_none_or(|status| item.get("status").and_then(Value::as_str) == Some(status));
        let article_matches = params.get("articleId").is_none_or(|article| {
            item.get("sourceArticleId").and_then(Value::as_str) == Some(article)
        });
        let due_matches = params.get("due").is_none_or(|due| {
            due != "true"
                || item
                    .get("nextReviewAt")
                    .and_then(Value::as_str)
                    .is_none_or(|stamp| stamp <= now().as_str())
        });
        query_matches && status_matches && article_matches && due_matches
    });
    if params.get("sort").is_some_and(|value| value == "word") {
        items.sort_by_key(|item| {
            item.get("word")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_lowercase()
        });
    } else if params.get("sort").is_some_and(|value| value == "review") {
        items.sort_by_key(|item| {
            item.get("nextReviewAt")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        });
    }
    let page = params
        .get("page")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1)
        .max(1);
    let page_size = params
        .get("pageSize")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(50)
        .clamp(1, 200);
    let total = items.len();
    let offset = (page - 1) * page_size;
    Ok(
        json!({ "items": items.into_iter().skip(offset).take(page_size).collect::<Vec<_>>(), "total": total, "page": page, "pageSize": page_size }),
    )
}

fn add_vocabulary(connection: &Connection, mut body: Value) -> Result<Value, String> {
    let stamp = now();
    let object = body
        .as_object_mut()
        .ok_or_else(|| "生词数据无效".to_owned())?;
    let word = object
        .get("word")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned();
    if word.is_empty()
        || object
            .get("selectedMeanings")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    {
        return Err("请选择至少一条需要记忆的释义".to_owned());
    }
    let normalized = object
        .get("normalizedWord")
        .and_then(Value::as_str)
        .unwrap_or(&word)
        .to_lowercase();
    if let Some(existing) = list(connection, "vocabulary")?.into_iter().find(|value| {
        value.get("normalizedWord").and_then(Value::as_str) == Some(normalized.as_str())
    }) {
        return Ok(existing);
    }
    object.insert("id".to_owned(), json!(id()));
    object.insert("normalizedWord".to_owned(), json!(normalized));
    object
        .entry("lemma".to_owned())
        .or_insert_with(|| json!(word.to_lowercase()));
    object
        .entry("phonetic".to_owned())
        .or_insert_with(|| json!(""));
    object
        .entry("partOfSpeech".to_owned())
        .or_insert_with(|| json!(""));
    object
        .entry("notes".to_owned())
        .or_insert_with(|| json!(""));
    object
        .entry("masteryLevel".to_owned())
        .or_insert_with(|| json!(0));
    object
        .entry("reviewStage".to_owned())
        .or_insert_with(|| json!(0));
    object
        .entry("reviewCount".to_owned())
        .or_insert_with(|| json!(0));
    object
        .entry("correctCount".to_owned())
        .or_insert_with(|| json!(0));
    object
        .entry("incorrectCount".to_owned())
        .or_insert_with(|| json!(0));
    object
        .entry("encounterCount".to_owned())
        .or_insert_with(|| json!(1));
    object
        .entry("status".to_owned())
        .or_insert_with(|| json!("LEARNING"));
    object.entry("tags".to_owned()).or_insert_with(|| json!([]));
    object.insert("nextReviewAt".to_owned(), json!(stamp));
    object.insert("createdAt".to_owned(), json!(stamp));
    object.insert("updatedAt".to_owned(), json!(stamp));
    put(connection, "vocabulary", &body)?;
    Ok(body)
}

fn update_vocabulary(
    connection: &Connection,
    entity_id: &str,
    patch: Value,
) -> Result<Value, String> {
    let mut item =
        get(connection, "vocabulary", entity_id)?.ok_or_else(|| "生词不存在".to_owned())?;
    let patch = patch.as_object().ok_or_else(|| "更新内容无效".to_owned())?;
    let object = item
        .as_object_mut()
        .ok_or_else(|| "生词数据损坏".to_owned())?;
    for (key, value) in patch {
        if key != "id" && key != "createdAt" {
            object.insert(key.clone(), value.clone());
        }
    }
    object.insert("updatedAt".to_owned(), json!(now()));
    put(connection, "vocabulary", &item)?;
    Ok(item)
}

fn review_vocabulary(
    connection: &Connection,
    entity_id: &str,
    body: &Value,
) -> Result<Value, String> {
    let result = body.get("result").and_then(Value::as_str).unwrap_or("GOOD");
    let mut item =
        get(connection, "vocabulary", entity_id)?.ok_or_else(|| "生词不存在".to_owned())?;
    let object = item
        .as_object_mut()
        .ok_or_else(|| "生词数据损坏".to_owned())?;
    let before = object
        .get("reviewStage")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let after = match result {
        "FORGOT" => 0,
        "HARD" => before.max(1),
        "EASY" => (before + 2).min(8),
        _ => (before + 1).min(8),
    };
    let intervals = [0, 1, 2, 4, 7, 14, 30, 60, 120];
    let reviewed_at = now();
    let next = (Utc::now() + Duration::days(intervals[after as usize])).to_rfc3339();
    object.insert("reviewStage".to_owned(), json!(after));
    object.insert("masteryLevel".to_owned(), json!((after * 12).min(100)));
    object.insert(
        "reviewCount".to_owned(),
        json!(
            object
                .get("reviewCount")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                + 1
        ),
    );
    let correct_key = if result == "FORGOT" {
        "incorrectCount"
    } else {
        "correctCount"
    };
    object.insert(
        correct_key.to_owned(),
        json!(object.get(correct_key).and_then(Value::as_i64).unwrap_or(0) + 1),
    );
    object.insert("lastReviewedAt".to_owned(), json!(reviewed_at));
    object.insert("nextReviewAt".to_owned(), json!(next));
    object.insert(
        "status".to_owned(),
        json!(if after >= 7 {
            "MASTERED"
        } else if after > 1 {
            "REVIEWING"
        } else {
            "LEARNING"
        }),
    );
    object.insert("updatedAt".to_owned(), json!(now()));
    let log = json!({
        "id": id(), "vocabularyId": entity_id, "result": result, "stageBefore": before,
        "stageAfter": after, "reviewedAt": reviewed_at, "nextReviewAt": next,
        "responseTimeMs": body.get("responseTimeMs").cloned().unwrap_or(Value::Null)
    });
    object
        .entry("reviewLogs".to_owned())
        .or_insert_with(|| json!([]));
    if let Some(logs) = object.get_mut("reviewLogs").and_then(Value::as_array_mut) {
        logs.insert(0, log);
    }
    put(connection, "vocabulary", &item)?;
    Ok(item)
}

fn upsert_annotation(connection: &Connection, key: &str, mut body: Value) -> Result<Value, String> {
    let stamp = now();
    let object = body
        .as_object_mut()
        .ok_or_else(|| "标注内容无效".to_owned())?;
    if let Some(entity_id) = object.get("id").and_then(Value::as_str).map(str::to_owned) {
        if let Some(existing) = get(connection, key, &entity_id)? {
            let mut merged = existing.as_object().cloned().unwrap_or_default();
            for (name, value) in object.iter() {
                merged.insert(name.clone(), value.clone());
            }
            merged.insert("updatedAt".to_owned(), json!(stamp));
            body = Value::Object(merged);
        }
    } else {
        object.insert("id".to_owned(), json!(id()));
        object.insert("userId".to_owned(), json!("local-user"));
        object.insert("createdAt".to_owned(), json!(stamp));
        object.insert("updatedAt".to_owned(), json!(stamp));
    }
    put(connection, key, &body)?;
    Ok(body)
}

fn article_category(source_category: &str) -> &'static str {
    match source_category {
        "science" => "Science",
        "health" => "Life",
        "words" => "Culture",
        _ => "Life",
    }
}

fn text_between(block: &str, tag: &str) -> Option<String> {
    let pattern = format!(r"(?is)<{tag}(?:\s[^>]*)?>(.*?)</{tag}>");
    Regex::new(&pattern)
        .ok()?
        .captures(block)?
        .get(1)
        .map(|value| {
            let fragment = Html::parse_fragment(value.as_str());
            fragment
                .root_element()
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_owned()
        })
}

fn feed_entries(xml: &str) -> Vec<(String, String, Option<String>)> {
    let Ok(item_pattern) = Regex::new(r"(?is)<item(?:\s[^>]*)?>(.*?)</item>") else {
        return Vec::new();
    };
    item_pattern
        .captures_iter(xml)
        .filter_map(|item| {
            let block = item.get(1)?.as_str();
            let title = text_between(block, "title")?;
            let link = text_between(block, "link")?;
            let published = text_between(block, "pubDate");
            Some((title, link, published))
        })
        .collect()
}

fn select_meta(document: &Html, key: &str) -> Option<String> {
    let selector =
        Selector::parse(&format!(r#"meta[property="{key}"],meta[name="{key}"]"#)).ok()?;
    document
        .select(&selector)
        .find_map(|node| node.value().attr("content"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn parse_article_html(
    source: &Value,
    fallback_title: &str,
    article_url: &str,
    published: Option<&str>,
    html: &str,
) -> Option<Value> {
    let document = Html::parse_document(html);
    let title = select_meta(&document, "og:title").unwrap_or_else(|| fallback_title.to_owned());
    let selectors = [
        r#"[data-qa="article-body"]"#,
        ".article-body",
        ".story-body",
        ".wsw",
        ".content-floated-wrap",
        "main article",
        "article",
    ];
    let mut paragraphs = Vec::new();
    for raw_selector in selectors {
        let Ok(selector) = Selector::parse(raw_selector) else {
            continue;
        };
        let Some(container) = document.select(&selector).next() else {
            continue;
        };
        let paragraph_selector = Selector::parse("p").ok()?;
        paragraphs = container
            .select(&paragraph_selector)
            .map(|paragraph| paragraph.text().collect::<Vec<_>>().join(" "))
            .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|value| value.len() >= 35)
            .filter(|value| {
                let lower = value.to_lowercase();
                ![
                    "embed",
                    "share",
                    "subscribe",
                    "direct link",
                    "no media source",
                ]
                .iter()
                .any(|blocked| lower.contains(blocked))
            })
            .collect();
        if paragraphs.len() >= 2 {
            break;
        }
    }
    let content = paragraphs.join("\n\n");
    let word_count = content.split_whitespace().count();
    if word_count < 80 {
        return None;
    }
    let source_key = source
        .get("sourceKey")
        .and_then(Value::as_str)
        .unwrap_or("voa");
    let category = source
        .get("category")
        .and_then(Value::as_str)
        .unwrap_or("science");
    let hash = format!("{:x}", md5::compute(article_url.as_bytes()));
    let stamp = now();
    let audio_selector = Selector::parse(r#"audio source[src],audio[src]"#).ok()?;
    let audio_url = document
        .select(&audio_selector)
        .find_map(|node| node.value().attr("src"))
        .map(str::to_owned);
    Some(json!({
        "id": format!("voa-{}", &hash[..16]), "title": title,
        "level": if word_count > 750 { "B2" } else { "B1" },
        "category": article_category(category), "content": content,
        "vocabulary": [], "questions": [], "difficulty": if word_count > 750 { 4 } else { 3 },
        "estimatedMinutes": ((word_count as f64 / 130.0).ceil() as usize).max(2),
        "createdTime": stamp, "updatedAt": stamp, "source": "voa",
        "sourceKey": source_key,
        "sourceName": source.get("sourceName").cloned().unwrap_or_else(|| json!("VOA Learning English")),
        "sourceCategory": category, "sourceUrl": article_url, "normalizedSourceUrl": article_url,
        "externalId": hash, "publishedAt": published, "imageUrl": select_meta(&document, "og:image"),
        "audioUrl": audio_url.clone(), "author": select_meta(&document, "author"),
        "summary": select_meta(&document, "og:description"), "wordCount": word_count,
        "fetchedAt": stamp, "rightsNote": "VOA Learning English source; attribution retained.",
        "language": "en", "qualityScore": 80, "hasAudio": audio_url.is_some(),
        "licenseType": "source-attributed", "attribution": "VOA Learning English",
        "processingStatus": "READY", "fetchStatus": "SUCCESS", "retryCount": 0
    }))
}

async fn sync_voa(
    state: &AppState,
    source_key: Option<&str>,
    limit: usize,
    task_type: &str,
) -> Result<Value, String> {
    let task_id = id();
    let created_at = now();
    let initial_task = json!({
        "taskId": task_id, "taskType": task_type, "sourceKey": source_key,
        "requestedLimit": limit, "status": "RUNNING", "startedAt": created_at,
        "totalCount": 0, "successCount": 0, "insertedCount": 0, "updatedCount": 0,
        "skippedCount": 0, "failedCount": 0, "progress": 0,
        "createdAt": created_at, "updatedAt": created_at
    });
    {
        let connection = state
            .database
            .lock()
            .map_err(|_| "SQLite 锁已损坏".to_owned())?;
        put(&connection, "tasks", &initial_task)?;
    }
    let sources = {
        let connection = state
            .database
            .lock()
            .map_err(|_| "SQLite 锁已损坏".to_owned())?;
        list(&connection, "sources")?
    };
    let selected = sources
        .into_iter()
        .filter(|source| {
            source
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(true)
        })
        .filter(|source| {
            source_key
                .is_none_or(|key| source.get("sourceKey").and_then(Value::as_str) == Some(key))
        })
        .collect::<Vec<_>>();
    let client = reqwest::Client::builder()
        .user_agent("LifeTrace/2.0 personal educational reader")
        .timeout(std::time::Duration::from_secs(25))
        .build()
        .map_err(|value| value.to_string())?;
    let mut inserted = 0usize;
    let mut updated = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;
    let mut total = 0usize;
    for mut source in selected {
        let feed_url = source
            .get("sourceUrl")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let feed_result = client.get(&feed_url).send().await;
        let xml = match feed_result {
            Ok(response) if response.status().is_success() => {
                response.text().await.map_err(|value| value.to_string())?
            }
            Ok(response) => {
                failed += 1;
                if let Some(object) = source.as_object_mut() {
                    object.insert("status".to_owned(), json!("error"));
                    object.insert(
                        "lastError".to_owned(),
                        json!(format!("RSS HTTP {}", response.status())),
                    );
                    object.insert("updatedAt".to_owned(), json!(now()));
                }
                let connection = state
                    .database
                    .lock()
                    .map_err(|_| "SQLite 锁已损坏".to_owned())?;
                put(&connection, "sources", &source)?;
                continue;
            }
            Err(fetch_error) => {
                failed += 1;
                if let Some(object) = source.as_object_mut() {
                    object.insert("status".to_owned(), json!("error"));
                    object.insert("lastError".to_owned(), json!(fetch_error.to_string()));
                    object.insert("updatedAt".to_owned(), json!(now()));
                }
                let connection = state
                    .database
                    .lock()
                    .map_err(|_| "SQLite 锁已损坏".to_owned())?;
                put(&connection, "sources", &source)?;
                continue;
            }
        };
        let entries = feed_entries(&xml)
            .into_iter()
            .take(limit)
            .collect::<Vec<_>>();
        total += entries.len();
        for (title, url, published) in entries {
            let article_id = format!(
                "voa-{}",
                &format!("{:x}", md5::compute(url.as_bytes()))[..16]
            );
            let exists = {
                let connection = state
                    .database
                    .lock()
                    .map_err(|_| "SQLite 锁已损坏".to_owned())?;
                get(&connection, "articles", &article_id)?.is_some()
            };
            if exists && task_type == "incremental" {
                skipped += 1;
                continue;
            }
            match client.get(&url).send().await {
                Ok(response) if response.status().is_success() => match response.text().await {
                    Ok(html) => {
                        if let Some(article) =
                            parse_article_html(&source, &title, &url, published.as_deref(), &html)
                        {
                            let connection = state
                                .database
                                .lock()
                                .map_err(|_| "SQLite 锁已损坏".to_owned())?;
                            put(&connection, "articles", &article)?;
                            if exists {
                                updated += 1
                            } else {
                                inserted += 1
                            }
                        } else {
                            skipped += 1;
                        }
                    }
                    Err(_) => failed += 1,
                },
                _ => failed += 1,
            }
        }
        if let Some(object) = source.as_object_mut() {
            object.insert("status".to_owned(), json!("active"));
            object.insert("lastSyncAt".to_owned(), json!(now()));
            object.insert("lastSuccessAt".to_owned(), json!(now()));
            object.insert("consecutiveFailures".to_owned(), json!(0));
            object.remove("lastError");
            object.insert("updatedAt".to_owned(), json!(now()));
            let connection = state
                .database
                .lock()
                .map_err(|_| "SQLite 锁已损坏".to_owned())?;
            let article_count = list(&connection, "articles")?
                .iter()
                .filter(|article| article.get("sourceKey") == object.get("sourceKey"))
                .count();
            object.insert("articleCount".to_owned(), json!(article_count));
            put(&connection, "sources", &source)?;
        }
    }
    let finished = now();
    let status = if failed == 0 {
        "COMPLETED"
    } else if inserted + updated > 0 {
        "PARTIAL_SUCCESS"
    } else {
        "FAILED"
    };
    let task = json!({
        "taskId": task_id, "taskType": task_type, "sourceKey": source_key,
        "requestedLimit": limit, "status": status, "startedAt": created_at, "finishedAt": finished,
        "totalCount": total, "successCount": inserted + updated, "insertedCount": inserted,
        "updatedCount": updated, "skippedCount": skipped, "failedCount": failed, "progress": 100,
        "createdAt": created_at, "updatedAt": finished
    });
    let connection = state
        .database
        .lock()
        .map_err(|_| "SQLite 锁已损坏".to_owned())?;
    put(&connection, "tasks", &task)?;
    Ok(json!({ "created": true, "cached": false, "taskId": task_id, "task": task }))
}

fn today(connection: &Connection, params: &HashMap<String, String>) -> Result<Value, String> {
    let articles = list(connection, "articles")?;
    let records = list(connection, "records")?;
    let level = params.get("level").map(String::as_str).unwrap_or("B1");
    let chosen = params
        .get("articleId")
        .and_then(|article_id| {
            articles
                .iter()
                .find(|article| article.get("id").and_then(Value::as_str) == Some(article_id))
        })
        .or_else(|| {
            articles
                .iter()
                .find(|article| article.get("level").and_then(Value::as_str) == Some(level))
        })
        .or_else(|| articles.first())
        .cloned()
        .ok_or_else(|| "英语文章库为空".to_owned())?;
    let article_id = chosen.get("id").and_then(Value::as_str).unwrap_or_default();
    let record = record_for(&records, article_id).cloned();
    let completed_dates = records
        .iter()
        .filter(|record| reading_status(Some(record)) == "completed")
        .filter_map(|record| {
            record
                .get("date")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect::<Vec<_>>();
    let recent = records
        .iter()
        .take(5)
        .map(|record| {
            let mut value = record.clone();
            if let Some(object) = value.as_object_mut() {
                if let Some(article_id) = object.get("articleId").and_then(Value::as_str) {
                    object.insert(
                        "article".to_owned(),
                        articles
                            .iter()
                            .find(|article| {
                                article.get("id").and_then(Value::as_str) == Some(article_id)
                            })
                            .cloned()
                            .unwrap_or(Value::Null),
                    );
                }
            }
            value
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "article": chosen, "record": record, "currentLevel": level,
        "streak": calculate_streak(&completed_dates), "weekCompleted": completed_dates,
        "recentRecords": recent
    }))
}

fn calculate_streak(completed: &[String]) -> usize {
    let mut cursor = Utc::now() + Duration::hours(8);
    if !completed.contains(&cursor.format("%Y-%m-%d").to_string()) {
        cursor -= Duration::days(1);
    }
    let mut result = 0;
    while completed.contains(&cursor.format("%Y-%m-%d").to_string()) {
        result += 1;
        cursor -= Duration::days(1);
    }
    result
}

fn reading(
    connection: &Connection,
    article_id: &str,
    action: Option<&str>,
    seconds: i64,
) -> Result<Value, String> {
    if get(connection, "articles", article_id)?.is_none() {
        return Err("文章不存在".to_owned());
    }
    let records = list(connection, "records")?;
    let stamp = now();
    let mut record = record_for(&records, article_id)
        .cloned()
        .unwrap_or_else(|| {
            json!({
                "id": format!("reading-{article_id}"), "userId": "local-user", "date": date_key(),
                "articleId": article_id, "readingTimeSeconds": 0, "summary": "", "newWords": [],
                "completionStatus": "reading", "readingStatus": "reading", "startedAt": stamp,
                "createdAt": stamp, "updatedAt": stamp
            })
        });
    if let Some(object) = record.as_object_mut() {
        if action == Some("complete") {
            object.insert("readingStatus".to_owned(), json!("completed"));
            object.insert("completedAt".to_owned(), json!(stamp));
            if object
                .get("summary")
                .and_then(Value::as_str)
                .is_some_and(|value| !value.is_empty())
            {
                object.insert("completionStatus".to_owned(), json!("completed"));
            }
        } else if action == Some("start") {
            object.insert("readingStatus".to_owned(), json!("reading"));
        }
        let previous = object
            .get("readingTimeSeconds")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        object.insert(
            "readingTimeSeconds".to_owned(),
            json!(previous.max(seconds)),
        );
        object.insert("updatedAt".to_owned(), json!(stamp));
    }
    put(connection, "records", &record)?;
    Ok(record)
}

fn history(connection: &Connection) -> Result<Value, String> {
    let articles = list(connection, "articles")?;
    let analyses = list(connection, "analysis")?;
    let records = list(connection, "records")?;
    let dated = (Utc::now() - Duration::days(30))
        .format("%Y-%m-%d")
        .to_string();
    let mut scores = Vec::new();
    let mut decorated = Vec::new();
    for record in &records {
        if let Some(score) = record.get("score").and_then(Value::as_f64) {
            if record
                .get("date")
                .and_then(Value::as_str)
                .is_some_and(|date| date >= dated.as_str())
            {
                scores.push(score);
            }
        }
        let mut item = record.clone();
        if let Some(object) = item.as_object_mut() {
            let article_id = object
                .get("articleId")
                .and_then(Value::as_str)
                .unwrap_or_default();
            object.insert(
                "article".to_owned(),
                articles
                    .iter()
                    .find(|article| article.get("id").and_then(Value::as_str) == Some(article_id))
                    .cloned()
                    .unwrap_or(Value::Null),
            );
            object.insert(
                "analysis".to_owned(),
                analyses
                    .iter()
                    .find(|analysis| analysis.get("recordId") == object.get("id"))
                    .cloned()
                    .unwrap_or(Value::Null),
            );
        }
        decorated.push(item);
    }
    let completed = records
        .iter()
        .filter(|record| reading_status(Some(record)) == "completed")
        .filter_map(|record| {
            record
                .get("date")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "records": decorated,
        "stats": {
            "readingCount30": records.iter().filter(|record| record.get("date").and_then(Value::as_str).is_some_and(|date| date >= dated.as_str())).count(),
            "averageScore30": if scores.is_empty() { 0.0 } else { scores.iter().sum::<f64>() / scores.len() as f64 },
            "vocabularyGrowth30": list(connection, "vocabulary")?.iter().filter(|item| item.get("createdAt").and_then(Value::as_str).is_some_and(|date| date >= dated.as_str())).count(),
            "streak": calculate_streak(&completed)
        }
    }))
}

fn library_stats(connection: &Connection) -> Result<Value, String> {
    let articles = list(connection, "articles")?;
    let sources = list(connection, "sources")?;
    let mut by_cefr = BTreeMap::<String, usize>::new();
    let mut by_category = BTreeMap::<String, usize>::new();
    for article in &articles {
        *by_cefr
            .entry(
                article
                    .get("level")
                    .and_then(Value::as_str)
                    .unwrap_or("B1")
                    .to_owned(),
            )
            .or_default() += 1;
        *by_category
            .entry(
                article
                    .get("category")
                    .and_then(Value::as_str)
                    .unwrap_or("Life")
                    .to_owned(),
            )
            .or_default() += 1;
    }
    let last_sync = sources
        .iter()
        .filter_map(|source| source.get("lastSyncAt").and_then(Value::as_str))
        .max();
    Ok(json!({
        "total": articles.len(), "ready": articles.iter().filter(|item| item.get("processingStatus").and_then(Value::as_str).unwrap_or("READY") == "READY").count(),
        "pending": 0, "failed": 0, "rejected": 0,
        "withAudio": articles.iter().filter(|item| item.get("hasAudio").and_then(Value::as_bool).unwrap_or(false)).count(),
        "byCefr": by_cefr, "byCategory": by_category, "lastSyncAt": last_sync,
        "lastNewArticleAt": articles.iter().filter_map(|article| article.get("createdTime").and_then(Value::as_str)).max(),
        "initialization": { "status": "completed", "initializedAt": now(), "initialArticleCount": articles.len(), "targetArticleCount": 30 }
    }))
}

fn analyze(connection: &Connection, body: &Value) -> Result<Value, String> {
    let record_id = body
        .get("recordId")
        .and_then(Value::as_str)
        .ok_or_else(|| "缺少学习记录".to_owned())?;
    let mut record =
        get(connection, "records", record_id)?.ok_or_else(|| "学习记录不存在".to_owned())?;
    let summary = record
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let word_count = summary.split_whitespace().count();
    let score = (58 + word_count.min(35) as i64).min(92);
    let stamp = now();
    let analysis_id = id();
    let analysis = json!({
        "id": analysis_id, "userId": "local-user", "recordId": record_id,
        "articleId": record.get("articleId").cloned().unwrap_or(Value::Null),
        "provider": "mock", "score": score, "contentScore": score,
        "grammarScore": (score - 3).max(0), "vocabularyScore": (score + 2).min(100),
        "structureScore": score, "mistakes": [],
        "suggestions": ["Use one clear topic sentence.", "Support the main idea with a specific detail."],
        "improvedSummary": summary, "weakPoints": if word_count < 30 { json!(["summary length"]) } else { json!([]) },
        "createdAt": stamp, "updatedAt": stamp
    });
    put(connection, "analysis", &analysis)?;
    if let Some(object) = record.as_object_mut() {
        object.insert("analysisId".to_owned(), json!(analysis_id));
        object.insert("score".to_owned(), json!(score));
        object.insert("completionStatus".to_owned(), json!("analyzed"));
        object.insert("updatedAt".to_owned(), json!(stamp));
    }
    put(connection, "records", &record)?;
    Ok(json!({ "analysis": analysis, "record": record }))
}

fn save_summary(connection: &Connection, body: &Value) -> Result<Value, String> {
    let article_id = body
        .get("articleId")
        .and_then(Value::as_str)
        .ok_or_else(|| "缺少文章编号".to_owned())?;
    let summary = body
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if summary.is_empty() {
        return Err("英文总结不能为空".to_owned());
    }
    let records = list(connection, "records")?;
    let mut record = body
        .get("recordId")
        .and_then(Value::as_str)
        .and_then(|record_id| {
            records
                .iter()
                .find(|record| record.get("id").and_then(Value::as_str) == Some(record_id))
        })
        .or_else(|| record_for(&records, article_id))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let stamp = now();
    if let Some(object) = record.as_object_mut() {
        object.entry("id".to_owned()).or_insert_with(|| json!(id()));
        object
            .entry("userId".to_owned())
            .or_insert_with(|| json!("local-user"));
        object
            .entry("date".to_owned())
            .or_insert_with(|| json!(date_key()));
        object.insert("articleId".to_owned(), json!(article_id));
        object.insert("summary".to_owned(), json!(summary));
        object.insert(
            "readingTimeSeconds".to_owned(),
            body.get("readingTimeSeconds")
                .cloned()
                .unwrap_or_else(|| json!(0)),
        );
        object
            .entry("newWords".to_owned())
            .or_insert_with(|| json!([]));
        object.insert("completionStatus".to_owned(), json!("summarized"));
        object
            .entry("readingStatus".to_owned())
            .or_insert_with(|| json!("reading"));
        object
            .entry("startedAt".to_owned())
            .or_insert_with(|| json!(stamp));
        object
            .entry("createdAt".to_owned())
            .or_insert_with(|| json!(stamp));
        object.insert("updatedAt".to_owned(), json!(stamp));
    }
    put(connection, "records", &record)?;
    Ok(record)
}

pub async fn dispatch(
    State(state): State<AppState>,
    Path(path): Path<String>,
    method: Method,
    uri: Uri,
    body: Body,
) -> Response {
    let params = query(&uri);
    let path = path.trim_matches('/');

    if method == Method::POST
        && (path == "sync"
            || path == "sync/backfill"
            || path == "sync/retry-failed"
            || path == "sync/repair"
            || path.ends_with("/sync"))
    {
        let payload = match body_json(body).await {
            Ok(value) => value,
            Err(message) => return error(StatusCode::BAD_REQUEST, message),
        };
        let source_key = if let Some(value) = path
            .strip_prefix("sources/")
            .and_then(|value| value.strip_suffix("/sync"))
        {
            Some(value)
        } else {
            payload.get("sourceKey").and_then(Value::as_str)
        };
        let task_type = match path {
            "sync/backfill" => "backfill",
            "sync/retry-failed" => "retry_failed",
            "sync/repair" => {
                if payload
                    .get("deep")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    "monthly_health"
                } else {
                    "weekly_repair"
                }
            }
            _ => "incremental",
        };
        let limit = payload
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(if task_type == "backfill" { 30 } else { 8 }) as usize;
        return match sync_voa(&state, source_key, limit.clamp(1, 50), task_type).await {
            Ok(value) => (StatusCode::OK, Json(value)).into_response(),
            Err(message) => error(StatusCode::BAD_GATEWAY, message),
        };
    }

    let payload = if method == Method::GET || method == Method::DELETE {
        json!({})
    } else {
        match body_json(body).await {
            Ok(value) => value,
            Err(message) => return error(StatusCode::BAD_REQUEST, message),
        }
    };
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return error(StatusCode::INTERNAL_SERVER_ERROR, "SQLite 锁已损坏"),
    };
    let result: Result<Value, String> = (|| match (method.as_str(), path) {
        ("GET", "today") => today(&connection, &params),
        ("GET", "history") => history(&connection),
        ("GET", "assistant") => {
            let analyses = list(&connection, "analysis")?;
            let weak_points = analyses
                .iter()
                .flat_map(|item| {
                    item.get("weakPoints")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                })
                .cloned()
                .collect::<Vec<_>>();
            Ok(json!({
                "sampleSize": analyses.len(), "weakPoints": weak_points,
                "message": if analyses.is_empty() { "完成一次英文总结后，这里会给出针对性建议。" } else { "继续保持稳定阅读，并在总结中加入具体细节。" },
                "nextStage": "完成阅读、总结和复习的闭环"
            }))
        }
        ("GET", "articles") => {
            if let Some(article_id) = params.get("id") {
                get(&connection, "articles", article_id)?.ok_or_else(|| "文章不存在".to_owned())
            } else {
                article_page(&connection, &params)
            }
        }
        ("GET", "articles/stats") => library_stats(&connection),
        ("GET", "highlights") | ("GET", "notes") => {
            let article_id = params
                .get("articleId")
                .ok_or_else(|| "缺少文章编号".to_owned())?;
            annotations(&connection, article_id)
        }
        ("POST", "highlights") => upsert_annotation(&connection, "highlights", payload),
        ("POST", "notes") | ("PATCH", "notes") => upsert_annotation(&connection, "notes", payload),
        ("DELETE", "highlights") => {
            let entity_id = params.get("id").ok_or_else(|| "缺少高亮编号".to_owned())?;
            Ok(json!({ "ok": remove(&connection, "highlights", entity_id)? }))
        }
        ("DELETE", "notes") => {
            let entity_id = params.get("id").ok_or_else(|| "缺少笔记编号".to_owned())?;
            Ok(json!({ "ok": remove(&connection, "notes", entity_id)? }))
        }
        ("GET", "reading") => {
            let article_id = params
                .get("articleId")
                .ok_or_else(|| "缺少文章编号".to_owned())?;
            let record = reading(&connection, article_id, None, 0)?;
            Ok(json!({
                "status": reading_status(Some(&record)),
                "record": record,
                "transitioned": false
            }))
        }
        ("POST", "reading") => {
            let article_id = payload
                .get("articleId")
                .and_then(Value::as_str)
                .ok_or_else(|| "缺少文章编号".to_owned())?;
            let previous_status = list(&connection, "records")?
                .iter()
                .find(|record| record.get("articleId").and_then(Value::as_str) == Some(article_id))
                .map(|record| reading_status(Some(record)))
                .unwrap_or("unread");
            let record = reading(
                &connection,
                article_id,
                payload.get("action").and_then(Value::as_str),
                payload
                    .get("readingTimeSeconds")
                    .and_then(Value::as_i64)
                    .unwrap_or(0),
            )?;
            let status = reading_status(Some(&record));
            Ok(json!({
                "status": status,
                "record": record,
                "transitioned": previous_status != "completed" && status == "completed"
            }))
        }
        ("POST", "summary") => save_summary(&connection, &payload),
        ("POST", "analyze") => analyze(&connection, &payload),
        ("GET", "vocabulary/stats") => vocabulary_stats(&connection),
        ("GET", "vocabulary/settings") => {
            let raw: String = connection
                .query_row(
                    "SELECT value_json FROM english_preferences WHERE key='vocabulary'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|value| value.to_string())?;
            serde_json::from_str(&raw).map_err(|value| value.to_string())
        }
        ("PATCH", "vocabulary/settings") => {
            let raw: String = connection
                .query_row(
                    "SELECT value_json FROM english_preferences WHERE key='vocabulary'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|value| value.to_string())?;
            let mut settings: Map<String, Value> = serde_json::from_str::<Value>(&raw)
                .map_err(|value| value.to_string())?
                .as_object()
                .cloned()
                .unwrap_or_default();
            for (key, value) in payload.as_object().cloned().unwrap_or_default() {
                settings.insert(key, value);
            }
            let value = Value::Object(settings);
            connection.execute("UPDATE english_preferences SET value_json=?1,updated_at=?2 WHERE key='vocabulary'", params![value.to_string(), now()]).map_err(|value| value.to_string())?;
            Ok(value)
        }
        ("GET", "vocabulary") => vocabulary_list(&connection, &params),
        ("POST", "vocabulary") => add_vocabulary(&connection, payload),
        ("GET", "vocabulary/review/today") => {
            let mut due_params = params.clone();
            due_params.insert("due".to_owned(), "true".to_owned());
            due_params.insert("sort".to_owned(), "review".to_owned());
            vocabulary_list(&connection, &due_params)
        }
        ("GET", "sources") => Ok(json!({ "sources": list(&connection, "sources")? })),
        ("GET", "sync/status") => {
            let tasks = list(&connection, "tasks")?;
            let active = tasks
                .iter()
                .find(|task| {
                    matches!(
                        task.get("status").and_then(Value::as_str),
                        Some("PENDING" | "RUNNING")
                    )
                })
                .cloned();
            Ok(
                json!({ "activeTask": active, "tasks": tasks.into_iter().take(20).collect::<Vec<_>>() }),
            )
        }
        ("GET", "sync/logs") => Ok(json!({ "logs": [] })),
        _ => {
            if let Some(entity_id) = path
                .strip_prefix("vocabulary/")
                .and_then(|value| value.strip_suffix("/review"))
            {
                if method == Method::POST {
                    review_vocabulary(&connection, entity_id, &payload)
                } else {
                    Err("不支持的操作".to_owned())
                }
            } else if let Some(entity_id) = path
                .strip_prefix("vocabulary/")
                .and_then(|value| value.strip_suffix("/occurrences"))
            {
                if method == Method::POST {
                    let mut item = get(&connection, "vocabulary", entity_id)?
                        .ok_or_else(|| "生词不存在".to_owned())?;
                    let occurrence = json!({
                        "id": id(), "vocabularyId": entity_id,
                        "articleId": payload.get("articleId").cloned().unwrap_or(Value::Null),
                        "articleTitle": payload.get("articleTitle").cloned().unwrap_or(Value::Null),
                        "sourceSentence": payload.get("sourceSentence").cloned().unwrap_or_else(|| json!("")),
                        "createdAt": now()
                    });
                    if let Some(object) = item.as_object_mut() {
                        object
                            .entry("occurrences".to_owned())
                            .or_insert_with(|| json!([]));
                        if let Some(items) =
                            object.get_mut("occurrences").and_then(Value::as_array_mut)
                        {
                            items.insert(0, occurrence.clone());
                        }
                        object.insert(
                            "encounterCount".to_owned(),
                            json!(
                                object
                                    .get("encounterCount")
                                    .and_then(Value::as_i64)
                                    .unwrap_or(0)
                                    + 1
                            ),
                        );
                        object.insert("updatedAt".to_owned(), json!(now()));
                    }
                    put(&connection, "vocabulary", &item)?;
                    Ok(occurrence)
                } else {
                    Err("不支持的操作".to_owned())
                }
            } else if let Some(entity_id) = path.strip_prefix("vocabulary/") {
                match method {
                    Method::GET => get(&connection, "vocabulary", entity_id)?
                        .ok_or_else(|| "生词不存在".to_owned()),
                    Method::PATCH => update_vocabulary(&connection, entity_id, payload),
                    Method::DELETE => {
                        Ok(json!({ "ok": remove(&connection, "vocabulary", entity_id)? }))
                    }
                    _ => Err("不支持的操作".to_owned()),
                }
            } else if let Some(source_key) = path.strip_prefix("sources/") {
                if method == Method::PATCH {
                    let mut source = get(&connection, "sources", source_key)?
                        .ok_or_else(|| "数据源不存在".to_owned())?;
                    if let Some(object) = source.as_object_mut() {
                        if let Some(enabled) = payload.get("enabled").and_then(Value::as_bool) {
                            object.insert("enabled".to_owned(), json!(enabled));
                            object.insert(
                                "status".to_owned(),
                                json!(if enabled { "active" } else { "disabled" }),
                            );
                        }
                        object.insert("updatedAt".to_owned(), json!(now()));
                    }
                    put(&connection, "sources", &source)?;
                    Ok(json!({ "source": source }))
                } else {
                    Err("不支持的操作".to_owned())
                }
            } else {
                Err(format!("未实现的英语接口: {method} {path}"))
            }
        }
    })();
    match result {
        Ok(value) => Json(value).into_response(),
        Err(message) if message.contains("不存在") => error(StatusCode::NOT_FOUND, message),
        Err(message) => error(StatusCode::BAD_REQUEST, message),
    }
}
