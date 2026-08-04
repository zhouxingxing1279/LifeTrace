//! 英语 Repository：真实列与前端 DTO 的转换与读写。
//!
//! `english_sources` / `english_sync_tasks` / `english_preferences` 属于运行配置，
//! 仍保持 JSON/KV 结构，由 `server/english.rs` 直接读写。
use chrono::Utc;
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use uuid::Uuid;
use crate::database::legacy::json_parser;
fn now() -> String {
    Utc::now().to_rfc3339()
}
fn text(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    object.get(key).and_then(Value::as_str).filter(|value| !value.is_empty()).map(str::to_owned)
}
fn int_value(object: &serde_json::Map<String, Value>, key: &str) -> Option<i64> {
    object.get(key).and_then(Value::as_i64)
}
fn real_value(object: &serde_json::Map<String, Value>, key: &str) -> Option<f64> {
    object.get(key).and_then(Value::as_f64)
}
fn bool_value(object: &serde_json::Map<String, Value>, key: &str) -> bool {
    object.get(key).and_then(Value::as_bool).unwrap_or(false)
}
fn user_id(object: &serde_json::Map<String, Value>) -> String {
    json_parser::string_field(object, "userId")
        .filter(|value| !value.is_empty())
        .unwrap_or("local")
        .to_owned()
}
/// 写入英语实体（DTO → 真实列）。
pub fn put(connection: &Connection, key: &str, value: &Value) -> Result<(), String> {
    match key {
        "articles" => put_article(connection, value),
        "records" => put_record(connection, value),
        "vocabulary" => put_vocabulary(connection, value),
        "highlights" => put_highlight(connection, value),
        "notes" => put_english_note(connection, value),
        "analysis" => put_analysis(connection, value),
        _ => Err(format!("未知英语数据表: {key}")),
    }
}
fn put_article(connection: &Connection, value: &Value) -> Result<(), String> {
    let object = json_parser::as_object(value, "英语文章")?;
    let id = json_parser::string_field(object, "id")
        .or_else(|| value.get("taskId").and_then(Value::as_str))
        .filter(|id| !id.is_empty())
        .ok_or_else(|| "数据缺少 id".to_owned())?;
    let stamp = text(object, "updatedAt").unwrap_or_else(now);
    connection.execute(
        "INSERT OR REPLACE INTO english_articles(
           id, title, level, category, content, word_count, difficulty, estimated_minutes,
           source, source_key, source_name, source_category, source_url,
           normalized_source_url, external_id, published_at, source_updated_at, image_url,
           audio_url, author, summary, fetched_at, rights_note, content_hash, language,
           quality_score, has_audio, license_type, attribution, processing_status,
           fetch_status, retry_count, last_error, created_time, questions_json,
           vocabulary_json, raw_json, created_at, updated_at, deleted_at, version,
           modified_by_device
         ) VALUES(
           ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,
           ?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,?35,?36,?37,?38,?39,?40,?41,NULL
         )",
        params![
            id,
            json_parser::string_field(object, "title").unwrap_or_default(),
            json_parser::string_field(object, "level").unwrap_or("B1"),
            json_parser::string_field(object, "category").unwrap_or("Life"),
            text(object, "content").unwrap_or_default(),
            int_value(object, "wordCount").unwrap_or(0),
            int_value(object, "difficulty"),
            int_value(object, "estimatedMinutes"),
            text(object, "source"),
            text(object, "sourceKey"),
            text(object, "sourceName"),
            text(object, "sourceCategory"),
            text(object, "sourceUrl"),
            text(object, "normalizedSourceUrl"),
            text(object, "externalId"),
            text(object, "publishedAt"),
            text(object, "sourceUpdatedAt"),
            text(object, "imageUrl"),
            text(object, "audioUrl"),
            text(object, "author"),
            text(object, "summary"),
            text(object, "fetchedAt"),
            text(object, "rightsNote"),
            text(object, "contentHash"),
            text(object, "language"),
            real_value(object, "qualityScore"),
            bool_value(object, "hasAudio"),
            text(object, "licenseType"),
            text(object, "attribution"),
            text(object, "processingStatus"),
            text(object, "fetchStatus"),
            int_value(object, "retryCount").unwrap_or(0),
            text(object, "lastError"),
            text(object, "createdTime"),
            object.get("questions").map(Value::to_string),
            object.get("vocabulary").map(Value::to_string),
            value.to_string(),
            text(object, "createdAt").unwrap_or_else(now),
            stamp,
            object.get("deletedAt").filter(|value| !value.is_null()).and_then(Value::as_str),
            int_value(object, "version").unwrap_or(1).max(1)
        ],
    ).map(|_| ()).map_err(|error| error.to_string())
}
fn put_record(connection: &Connection, value: &Value) -> Result<(), String> {
    let object = json_parser::as_object(value, "学习记录")?;
    let id = json_parser::string_field(object, "id")
        .filter(|id| !id.is_empty())
        .ok_or_else(|| "数据缺少 id".to_owned())?;
    let stamp = text(object, "updatedAt").unwrap_or_else(now);
    connection.execute(
        "INSERT OR REPLACE INTO english_learning_records(
           id, user_id, article_id, record_date, reading_time_seconds, summary, score,
           analysis_id, new_words_json, completion_status, reading_status, started_at,
           completed_at, created_at, updated_at, deleted_at, version, modified_by_device
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,NULL)",
        params![
            id,
            user_id(object),
            text(object, "articleId"),
            json_parser::string_field(object, "date").unwrap_or_default(),
            int_value(object, "readingTimeSeconds").unwrap_or(0),
            text(object, "summary").unwrap_or_default(),
            real_value(object, "score"),
            text(object, "analysisId"),
            object.get("newWords").map(Value::to_string),
            json_parser::string_field(object, "completionStatus").unwrap_or("reading"),
            text(object, "readingStatus"),
            text(object, "startedAt"),
            text(object, "completedAt"),
            text(object, "createdAt").unwrap_or_else(now),
            stamp,
            object.get("deletedAt").filter(|value| !value.is_null()).and_then(Value::as_str),
            int_value(object, "version").unwrap_or(1).max(1)
        ],
    ).map(|_| ()).map_err(|error| error.to_string())
}
fn put_highlight(connection: &Connection, value: &Value) -> Result<(), String> {
    let object = json_parser::as_object(value, "英语高亮")?;
    let id = json_parser::string_field(object, "id")
        .filter(|id| !id.is_empty())
        .ok_or_else(|| "数据缺少 id".to_owned())?;
    let stamp = text(object, "updatedAt").unwrap_or_else(now);
    connection.execute(
        "INSERT OR REPLACE INTO english_highlights(
           id, user_id, article_id, selected_text, block_id, start_offset, end_offset,
           color, prefix, suffix, note, created_at, updated_at, deleted_at, version,
           modified_by_device
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,NULL)",
        params![
            id,
            user_id(object),
            text(object, "articleId"),
            text(object, "text").unwrap_or_default(),
            text(object, "blockId"),
            int_value(object, "startOffset"),
            int_value(object, "endOffset"),
            json_parser::string_field(object, "color").unwrap_or("yellow"),
            text(object, "prefix"),
            text(object, "suffix"),
            text(object, "note"),
            text(object, "createdAt").unwrap_or_else(now),
            stamp,
            object.get("deletedAt").filter(|value| !value.is_null()).and_then(Value::as_str),
            int_value(object, "version").unwrap_or(1).max(1)
        ],
    ).map(|_| ()).map_err(|error| error.to_string())
}
fn put_english_note(connection: &Connection, value: &Value) -> Result<(), String> {
    let object = json_parser::as_object(value, "英语笔记")?;
    let id = json_parser::string_field(object, "id")
        .filter(|id| !id.is_empty())
        .ok_or_else(|| "数据缺少 id".to_owned())?;
    let stamp = text(object, "updatedAt").unwrap_or_else(now);
    connection.execute(
        "INSERT OR REPLACE INTO english_notes(
           id, user_id, article_id, quote, content, block_id, start_offset, end_offset,
           selected_text, prefix, suffix, highlight_id, created_at, updated_at, deleted_at,
           version, modified_by_device
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,NULL)",
        params![
            id,
            user_id(object),
            text(object, "articleId"),
            text(object, "quote"),
            text(object, "content").unwrap_or_default(),
            text(object, "blockId"),
            int_value(object, "startOffset"),
            int_value(object, "endOffset"),
            text(object, "selectedText"),
            text(object, "prefix"),
            text(object, "suffix"),
            text(object, "highlightId"),
            text(object, "createdAt").unwrap_or_else(now),
            stamp,
            object.get("deletedAt").filter(|value| !value.is_null()).and_then(Value::as_str),
            int_value(object, "version").unwrap_or(1).max(1)
        ],
    ).map(|_| ()).map_err(|error| error.to_string())
}
fn put_analysis(connection: &Connection, value: &Value) -> Result<(), String> {
    let object = json_parser::as_object(value, "英语分析")?;
    let id = json_parser::string_field(object, "id")
        .filter(|id| !id.is_empty())
        .ok_or_else(|| "数据缺少 id".to_owned())?;
    let stamp = text(object, "updatedAt").unwrap_or_else(now);
    connection.execute(
        "INSERT OR REPLACE INTO english_ai_analysis(
           id, user_id, record_id, article_id, provider, score, content_score,
           grammar_score, vocabulary_score, structure_score, mistakes_json,
           suggestions_json, improved_summary, weak_points_json, created_at, updated_at,
           deleted_at, version, modified_by_device
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,NULL)",
        params![
            id,
            user_id(object),
            text(object, "recordId"),
            text(object, "articleId"),
            json_parser::string_field(object, "provider").unwrap_or("mock"),
            real_value(object, "score").unwrap_or(0.0),
            real_value(object, "contentScore"),
            real_value(object, "grammarScore"),
            real_value(object, "vocabularyScore"),
            real_value(object, "structureScore"),
            object.get("mistakes").map(Value::to_string),
            object.get("suggestions").map(Value::to_string),
            text(object, "improvedSummary").unwrap_or_default(),
            object.get("weakPoints").map(Value::to_string),
            text(object, "createdAt").unwrap_or_else(now),
            stamp,
            object.get("deletedAt").filter(|value| !value.is_null()).and_then(Value::as_str),
            int_value(object, "version").unwrap_or(1).max(1)
        ],
    ).map(|_| ()).map_err(|error| error.to_string())
}
fn put_vocabulary(connection: &Connection, value: &Value) -> Result<(), String> {
    let object = json_parser::as_object(value, "英语生词")?;
    let id = json_parser::string_field(object, "id")
        .filter(|id| !id.is_empty())
        .ok_or_else(|| "数据缺少 id".to_owned())?;
    let normalized = json_parser::string_field(object, "normalizedWord")
        .or_else(|| json_parser::string_field(object, "word"))
        .map(str::to_lowercase)
        .ok_or_else(|| "生词缺少 word".to_owned())?;
    let selected_meanings = object.get("selectedMeanings").and_then(Value::as_array).cloned().unwrap_or_default();
    let definition = selected_meanings.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("; ");
    let review_logs = object.get("reviewLogs").cloned().unwrap_or_else(|| json!([]));
    let mut metadata = serde_json::Map::new();
    metadata.insert("reviewLogs".to_owned(), review_logs);
    if let Some(value) = int_value(object, "dictionaryWordId") {
        metadata.insert("dictionaryWordId".to_owned(), json!(value));
    }
    let stamp = text(object, "updatedAt").unwrap_or_else(now);
    connection.execute(
        "INSERT OR REPLACE INTO english_vocabulary(
           id, user_id, normalized_word, display_word, definition, phonetic,
           part_of_speech, selected_meanings_json, lemma, source_article_id,
           source_article_title, source_sentence, notes, mastery_level, review_stage,
           review_count, correct_count, incorrect_count, encounter_count,
           last_reviewed_at, next_review_at, status, frequency_rank, tags_json,
           metadata_json, created_at, updated_at, deleted_at, version,
           modified_by_device
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,NULL)",
        params![
            id,
            user_id(object),
            normalized,
            json_parser::string_field(object, "word").unwrap_or_default(),
            definition,
            text(object, "phonetic").unwrap_or_default(),
            text(object, "partOfSpeech").unwrap_or_default(),
            object.get("selectedMeanings").map(Value::to_string),
            text(object, "lemma").unwrap_or_default(),
            text(object, "sourceArticleId"),
            text(object, "sourceArticleTitle"),
            text(object, "sourceSentence"),
            text(object, "notes").unwrap_or_default(),
            int_value(object, "masteryLevel").unwrap_or(0),
            int_value(object, "reviewStage").unwrap_or(0),
            int_value(object, "reviewCount").unwrap_or(0),
            int_value(object, "correctCount").unwrap_or(0),
            int_value(object, "incorrectCount").unwrap_or(0),
            int_value(object, "encounterCount").unwrap_or(0),
            text(object, "lastReviewedAt"),
            text(object, "nextReviewAt"),
            json_parser::string_field(object, "status").unwrap_or("LEARNING"),
            int_value(object, "frequencyRank"),
            object.get("tags").map(Value::to_string),
            Value::Object(metadata).to_string(),
            text(object, "createdAt").unwrap_or_else(now),
            stamp,
            object.get("deletedAt").filter(|value| !value.is_null()).and_then(Value::as_str),
            int_value(object, "version").unwrap_or(1).max(1)
        ],
    ).map_err(|error| error.to_string())?;
    connection.execute("DELETE FROM vocabulary_occurrences WHERE vocabulary_id = ?1", [id]).map_err(|error| error.to_string())?;
    if let Some(occurrences) = object.get("occurrences").and_then(Value::as_array) {
        for occurrence in occurrences {
            let occurrence_object = occurrence.as_object().cloned().unwrap_or_default();
            connection.execute(
                "INSERT OR IGNORE INTO vocabulary_occurrences(
                   id, vocabulary_id, article_id, article_title, source_sentence, created_at
                 ) VALUES(?1,?2,?3,?4,?5,?6)",
                params![
                    text(&occurrence_object, "id").unwrap_or_else(|| Uuid::new_v4().to_string()),
                    id,
                    text(&occurrence_object, "articleId"),
                    text(&occurrence_object, "articleTitle"),
                    text(&occurrence_object, "sourceSentence").unwrap_or_default(),
                    text(&occurrence_object, "createdAt").unwrap_or_else(now)
                ],
            ).map_err(|error| error.to_string())?;
        }
    }
    connection.execute(
        "INSERT OR REPLACE INTO vocabulary_review_state(
           vocabulary_id, due_at, difficulty, stability, retrievability, review_count,
           lapse_count, scheduler_version, updated_at
         ) VALUES(?1,?2,NULL,NULL,NULL,?3,?4,NULL,?5)",
        params![
            id,
            text(object, "nextReviewAt"),
            int_value(object, "reviewCount").unwrap_or(0),
            int_value(object, "incorrectCount").unwrap_or(0),
            stamp
        ],
    ).map_err(|error| error.to_string())?;
    Ok(())
}
/// 读取英语实体列表（DTO）。
pub fn list(connection: &Connection, key: &str) -> Result<Vec<Value>, String> {
    match key {
        "articles" => list_articles(connection),
        "records" => list_records(connection),
        "vocabulary" => list_vocabulary(connection),
        "highlights" => list_highlights(connection),
        "notes" => list_english_notes(connection),
        "analysis" => list_analysis(connection),
        _ => Err(format!("未知英语数据表: {key}")),
    }
}
fn list_articles(connection: &Connection) -> Result<Vec<Value>, String> {
    let mut statement = connection.prepare(
        "SELECT id, title, level, category, content, word_count, difficulty,
                estimated_minutes, source, source_key, source_name, source_category,
                source_url, normalized_source_url, external_id, published_at,
                source_updated_at, image_url, audio_url, author, summary, fetched_at,
                rights_note, content_hash, language, quality_score, has_audio,
                license_type, attribution, processing_status, fetch_status, retry_count,
                last_error, created_time, questions_json, vocabulary_json, created_at,
                updated_at
         FROM english_articles WHERE deleted_at IS NULL
         ORDER BY updated_at DESC",
    ).map_err(|error| error.to_string())?;
    let rows = statement.query_map([], article_from_row).map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())
}
fn article_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    let questions: Option<String> = row.get(34)?;
    let vocabulary: Option<String> = row.get(35)?;
    Ok(json!({
        "id": row.get::<_, String>(0)?,
        "title": row.get::<_, String>(1)?,
        "level": row.get::<_, String>(2)?,
        "category": row.get::<_, String>(3)?,
        "content": row.get::<_, String>(4)?,
        "wordCount": row.get::<_, i64>(5)?,
        "difficulty": row.get::<_, Option<i64>>(6)?,
        "estimatedMinutes": row.get::<_, Option<i64>>(7)?,
        "source": row.get::<_, Option<String>>(8)?,
        "sourceKey": row.get::<_, Option<String>>(9)?,
        "sourceName": row.get::<_, Option<String>>(10)?,
        "sourceCategory": row.get::<_, Option<String>>(11)?,
        "sourceUrl": row.get::<_, Option<String>>(12)?,
        "normalizedSourceUrl": row.get::<_, Option<String>>(13)?,
        "externalId": row.get::<_, Option<String>>(14)?,
        "publishedAt": row.get::<_, Option<String>>(15)?,
        "sourceUpdatedAt": row.get::<_, Option<String>>(16)?,
        "imageUrl": row.get::<_, Option<String>>(17)?,
        "audioUrl": row.get::<_, Option<String>>(18)?,
        "author": row.get::<_, Option<String>>(19)?,
        "summary": row.get::<_, Option<String>>(20)?,
        "fetchedAt": row.get::<_, Option<String>>(21)?,
        "rightsNote": row.get::<_, Option<String>>(22)?,
        "contentHash": row.get::<_, Option<String>>(23)?,
        "language": row.get::<_, Option<String>>(24)?,
        "qualityScore": row.get::<_, Option<f64>>(25)?,
        "hasAudio": row.get::<_, bool>(26)?,
        "licenseType": row.get::<_, Option<String>>(27)?,
        "attribution": row.get::<_, Option<String>>(28)?,
        "processingStatus": row.get::<_, Option<String>>(29)?,
        "fetchStatus": row.get::<_, Option<String>>(30)?,
        "retryCount": row.get::<_, i64>(31)?,
        "lastError": row.get::<_, Option<String>>(32)?,
        "createdTime": row.get::<_, Option<String>>(33)?,
        "questions": questions.as_deref().and_then(|value| serde_json::from_str::<Value>(value).ok()).unwrap_or_else(|| json!([])),
        "vocabulary": vocabulary.as_deref().and_then(|value| serde_json::from_str::<Value>(value).ok()).unwrap_or_else(|| json!([])),
        "createdAt": row.get::<_, String>(36)?,
        "updatedAt": row.get::<_, String>(37)?
    }))
}
fn list_records(connection: &Connection) -> Result<Vec<Value>, String> {
    let mut statement = connection.prepare(
        "SELECT id, user_id, article_id, record_date, reading_time_seconds, summary,
                score, analysis_id, new_words_json, completion_status, reading_status,
                started_at, completed_at, created_at, updated_at
         FROM english_learning_records WHERE deleted_at IS NULL
         ORDER BY updated_at DESC",
    ).map_err(|error| error.to_string())?;
    let rows = statement.query_map([], |row| {
        let new_words: Option<String> = row.get(8)?;
        Ok(json!({
            "id": row.get::<_, String>(0)?,
            "userId": row.get::<_, String>(1)?,
            "articleId": row.get::<_, Option<String>>(2)?,
            "date": row.get::<_, String>(3)?,
            "readingTimeSeconds": row.get::<_, i64>(4)?,
            "summary": row.get::<_, String>(5)?,
            "score": row.get::<_, Option<f64>>(6)?,
            "analysisId": row.get::<_, Option<String>>(7)?,
            "newWords": new_words.as_deref().and_then(|value| serde_json::from_str::<Value>(value).ok()).unwrap_or_else(|| json!([])),
            "completionStatus": row.get::<_, String>(9)?,
            "readingStatus": row.get::<_, Option<String>>(10)?,
            "startedAt": row.get::<_, Option<String>>(11)?,
            "completedAt": row.get::<_, Option<String>>(12)?,
            "createdAt": row.get::<_, String>(13)?,
            "updatedAt": row.get::<_, String>(14)?
        }))
    }).map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())
}
fn list_vocabulary(connection: &Connection) -> Result<Vec<Value>, String> {
    let mut statement = connection.prepare(
        "SELECT v.id, v.user_id, v.normalized_word, v.display_word, v.definition,
                v.phonetic, v.part_of_speech, v.selected_meanings_json, v.lemma,
                v.source_article_id, v.source_article_title, v.source_sentence, v.notes,
                v.mastery_level, v.review_stage, v.review_count, v.correct_count,
                v.incorrect_count, v.encounter_count, v.last_reviewed_at, v.next_review_at,
                v.status, v.frequency_rank, v.tags_json, v.metadata_json, v.created_at,
                v.updated_at
         FROM english_vocabulary v WHERE v.deleted_at IS NULL
         ORDER BY v.updated_at DESC",
    ).map_err(|error| error.to_string())?;
    let mut rows = statement.query([]).map_err(|error| error.to_string())?;
    let mut items = Vec::new();
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        items.push(vocabulary_from_row(connection, &row)?);
    }
    Ok(items)
}
fn vocabulary_from_row(connection: &Connection, row: &rusqlite::Row<'_>) -> Result<Value, String> {
    let vocabulary_id: String = row.get(0).map_err(|error| error.to_string())?;
    let selected_meanings: Option<String> = row.get(7).map_err(|error| error.to_string())?;
    let tags: Option<String> = row.get(23).map_err(|error| error.to_string())?;
    let metadata_json: Option<String> = row.get(24).map_err(|error| error.to_string())?;
    let metadata = metadata_json.as_deref().and_then(|value| serde_json::from_str::<Value>(value).ok()).unwrap_or_else(|| json!({}));
    let review_logs = metadata.get("reviewLogs").cloned().unwrap_or_else(|| json!([]));
    let occurrences = {
        let mut statement = connection.prepare(
            "SELECT id, vocabulary_id, article_id, article_title, source_sentence, created_at
             FROM vocabulary_occurrences WHERE vocabulary_id = ?1 ORDER BY created_at DESC",
        ).map_err(|error| error.to_string())?;
        let rows = statement.query_map([vocabulary_id.clone()], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "vocabularyId": row.get::<_, String>(1)?,
                "articleId": row.get::<_, Option<String>>(2)?,
                "articleTitle": row.get::<_, Option<String>>(3)?,
                "sourceSentence": row.get::<_, String>(4)?,
                "createdAt": row.get::<_, String>(5)?
            }))
        }).map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
            .unwrap_or_default()
    };
    Ok(json!({
        "id": vocabulary_id,
        "userId": row.get::<_, String>(1).map_err(|error| error.to_string())?,
        "normalizedWord": row.get::<_, String>(2).map_err(|error| error.to_string())?,
        "word": row.get::<_, String>(3).map_err(|error| error.to_string())?,
        "definition": row.get::<_, String>(4).map_err(|error| error.to_string())?,
        "phonetic": row.get::<_, String>(5).map_err(|error| error.to_string())?,
        "partOfSpeech": row.get::<_, String>(6).map_err(|error| error.to_string())?,
        "selectedMeanings": selected_meanings.as_deref().and_then(|value| serde_json::from_str::<Value>(value).ok()).unwrap_or_else(|| json!([])),
        "lemma": row.get::<_, String>(8).map_err(|error| error.to_string())?,
        "sourceArticleId": row.get::<_, Option<String>>(9).map_err(|error| error.to_string())?,
        "sourceArticleTitle": row.get::<_, Option<String>>(10).map_err(|error| error.to_string())?,
        "sourceSentence": row.get::<_, Option<String>>(11).map_err(|error| error.to_string())?,
        "notes": row.get::<_, String>(12).map_err(|error| error.to_string())?,
        "masteryLevel": row.get::<_, i64>(13).map_err(|error| error.to_string())?,
        "reviewStage": row.get::<_, i64>(14).map_err(|error| error.to_string())?,
        "reviewCount": row.get::<_, i64>(15).map_err(|error| error.to_string())?,
        "correctCount": row.get::<_, i64>(16).map_err(|error| error.to_string())?,
        "incorrectCount": row.get::<_, i64>(17).map_err(|error| error.to_string())?,
        "encounterCount": row.get::<_, i64>(18).map_err(|error| error.to_string())?,
        "lastReviewedAt": row.get::<_, Option<String>>(19).map_err(|error| error.to_string())?,
        "nextReviewAt": row.get::<_, Option<String>>(20).map_err(|error| error.to_string())?,
        "status": row.get::<_, String>(21).map_err(|error| error.to_string())?,
        "frequencyRank": row.get::<_, Option<i64>>(22).map_err(|error| error.to_string())?,
        "tags": tags.as_deref().and_then(|value| serde_json::from_str::<Value>(value).ok()).unwrap_or_else(|| json!([])),
        "reviewLogs": review_logs,
        "occurrences": occurrences,
        "dictionaryWordId": metadata.get("dictionaryWordId").cloned().unwrap_or(Value::Null),
        "createdAt": row.get::<_, String>(25).map_err(|error| error.to_string())?,
        "updatedAt": row.get::<_, String>(26).map_err(|error| error.to_string())?
    }))
}
fn list_highlights(connection: &Connection) -> Result<Vec<Value>, String> {
    let mut statement = connection.prepare(
        "SELECT id, user_id, article_id, selected_text, block_id, start_offset,
                end_offset, color, prefix, suffix, note, created_at, updated_at
         FROM english_highlights WHERE deleted_at IS NULL
         ORDER BY updated_at DESC",
    ).map_err(|error| error.to_string())?;
    let rows = statement.query_map([], |row| {
        Ok(json!({
            "id": row.get::<_, String>(0)?,
            "userId": row.get::<_, String>(1)?,
            "articleId": row.get::<_, Option<String>>(2)?,
            "text": row.get::<_, String>(3)?,
            "blockId": row.get::<_, Option<String>>(4)?,
            "startOffset": row.get::<_, Option<i64>>(5)?,
            "endOffset": row.get::<_, Option<i64>>(6)?,
            "color": row.get::<_, String>(7)?,
            "prefix": row.get::<_, Option<String>>(8)?,
            "suffix": row.get::<_, Option<String>>(9)?,
            "note": row.get::<_, Option<String>>(10)?,
            "createdAt": row.get::<_, String>(11)?,
            "updatedAt": row.get::<_, String>(12)?
        }))
    }).map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())
}
fn list_english_notes(connection: &Connection) -> Result<Vec<Value>, String> {
    let mut statement = connection.prepare(
        "SELECT id, user_id, article_id, quote, content, block_id, start_offset,
                end_offset, selected_text, prefix, suffix, highlight_id, created_at, updated_at
         FROM english_notes WHERE deleted_at IS NULL
         ORDER BY updated_at DESC",
    ).map_err(|error| error.to_string())?;
    let rows = statement.query_map([], |row| {
        Ok(json!({
            "id": row.get::<_, String>(0)?,
            "userId": row.get::<_, String>(1)?,
            "articleId": row.get::<_, Option<String>>(2)?,
            "quote": row.get::<_, Option<String>>(3)?,
            "content": row.get::<_, String>(4)?,
            "blockId": row.get::<_, Option<String>>(5)?,
            "startOffset": row.get::<_, Option<i64>>(6)?,
            "endOffset": row.get::<_, Option<i64>>(7)?,
            "selectedText": row.get::<_, Option<String>>(8)?,
            "prefix": row.get::<_, Option<String>>(9)?,
            "suffix": row.get::<_, Option<String>>(10)?,
            "highlightId": row.get::<_, Option<String>>(11)?,
            "createdAt": row.get::<_, String>(12)?,
            "updatedAt": row.get::<_, String>(13)?
        }))
    }).map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())
}
fn list_analysis(connection: &Connection) -> Result<Vec<Value>, String> {
    let mut statement = connection.prepare(
        "SELECT id, user_id, record_id, article_id, provider, score, content_score,
                grammar_score, vocabulary_score, structure_score, mistakes_json,
                suggestions_json, improved_summary, weak_points_json, created_at, updated_at
         FROM english_ai_analysis WHERE deleted_at IS NULL
         ORDER BY updated_at DESC",
    ).map_err(|error| error.to_string())?;
    let rows = statement.query_map([], |row| {
        let mistakes: Option<String> = row.get(10)?;
        let suggestions: Option<String> = row.get(11)?;
        let weak_points: Option<String> = row.get(13)?;
        Ok(json!({
            "id": row.get::<_, String>(0)?,
            "userId": row.get::<_, String>(1)?,
            "recordId": row.get::<_, Option<String>>(2)?,
            "articleId": row.get::<_, Option<String>>(3)?,
            "provider": row.get::<_, String>(4)?,
            "score": row.get::<_, f64>(5)?,
            "contentScore": row.get::<_, Option<f64>>(6)?,
            "grammarScore": row.get::<_, Option<f64>>(7)?,
            "vocabularyScore": row.get::<_, Option<f64>>(8)?,
            "structureScore": row.get::<_, Option<f64>>(9)?,
            "mistakes": mistakes.as_deref().and_then(|value| serde_json::from_str::<Value>(value).ok()).unwrap_or_else(|| json!([])),
            "suggestions": suggestions.as_deref().and_then(|value| serde_json::from_str::<Value>(value).ok()).unwrap_or_else(|| json!([])),
            "improvedSummary": row.get::<_, String>(12)?,
            "weakPoints": weak_points.as_deref().and_then(|value| serde_json::from_str::<Value>(value).ok()).unwrap_or_else(|| json!([])),
            "createdAt": row.get::<_, String>(14)?,
            "updatedAt": row.get::<_, String>(15)?
        }))
    }).map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())
}
/// 读取单个英语实体。
pub fn get(connection: &Connection, key: &str, entity_id: &str) -> Result<Option<Value>, String> {
    let items = list(connection, key)?;
    Ok(items.into_iter().find(|item| {
        item.get("id").and_then(Value::as_str) == Some(entity_id)
            || item.get("taskId").and_then(Value::as_str) == Some(entity_id)
    }))
}
/// 删除英语实体（物理删除，保持旧语义）。
pub fn remove(connection: &Connection, key: &str, entity_id: &str) -> Result<bool, String> {
    let table = match key {
        "articles" => "english_articles",
        "records" => "english_learning_records",
        "vocabulary" => "english_vocabulary",
        "highlights" => "english_highlights",
        "notes" => "english_notes",
        "analysis" => "english_ai_analysis",
        _ => return Err(format!("未知英语数据表: {key}")),
    };
    connection.execute(&format!("DELETE FROM {table} WHERE id=?1"), [entity_id])
        .map(|count| count > 0)
        .map_err(|error| error.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn schema(connection: &Connection) {
        connection.execute_batch(
            "CREATE TABLE english_articles(
               id TEXT PRIMARY KEY, title TEXT, level TEXT, category TEXT, content TEXT,
               word_count INTEGER, difficulty INTEGER, estimated_minutes INTEGER,
               source TEXT, source_key TEXT, source_name TEXT, source_category TEXT,
               source_url TEXT, normalized_source_url TEXT, external_id TEXT,
               published_at TEXT, source_updated_at TEXT, image_url TEXT, audio_url TEXT,
               author TEXT, summary TEXT, fetched_at TEXT, rights_note TEXT, content_hash TEXT,
               language TEXT, quality_score REAL, has_audio INTEGER, license_type TEXT,
               attribution TEXT, processing_status TEXT, fetch_status TEXT, retry_count INTEGER,
               last_error TEXT, created_time TEXT, questions_json TEXT, vocabulary_json TEXT,
               raw_json TEXT, created_at TEXT, updated_at TEXT, deleted_at TEXT,
               version INTEGER, modified_by_device TEXT
             );
             CREATE TABLE english_learning_records(
               id TEXT PRIMARY KEY, user_id TEXT, article_id TEXT, record_date TEXT,
               reading_time_seconds INTEGER, summary TEXT, score REAL, analysis_id TEXT,
               new_words_json TEXT, completion_status TEXT, reading_status TEXT,
               started_at TEXT, completed_at TEXT, created_at TEXT, updated_at TEXT,
               deleted_at TEXT, version INTEGER, modified_by_device TEXT
             );
             CREATE TABLE english_highlights(
               id TEXT PRIMARY KEY, user_id TEXT, article_id TEXT, selected_text TEXT,
               block_id TEXT, start_offset INTEGER, end_offset INTEGER, color TEXT,
               prefix TEXT, suffix TEXT, note TEXT, created_at TEXT, updated_at TEXT,
               deleted_at TEXT, version INTEGER, modified_by_device TEXT
             );
             CREATE TABLE english_notes(
               id TEXT PRIMARY KEY, user_id TEXT, article_id TEXT, quote TEXT, content TEXT,
               block_id TEXT, start_offset INTEGER, end_offset INTEGER, selected_text TEXT,
               prefix TEXT, suffix TEXT, highlight_id TEXT, created_at TEXT, updated_at TEXT,
               deleted_at TEXT, version INTEGER, modified_by_device TEXT
             );
             CREATE TABLE english_ai_analysis(
               id TEXT PRIMARY KEY, user_id TEXT, record_id TEXT, article_id TEXT,
               provider TEXT, score REAL, content_score REAL, grammar_score REAL,
               vocabulary_score REAL, structure_score REAL, mistakes_json TEXT,
               suggestions_json TEXT, improved_summary TEXT, weak_points_json TEXT,
               created_at TEXT, updated_at TEXT, deleted_at TEXT, version INTEGER,
               modified_by_device TEXT
             );
             CREATE TABLE english_vocabulary(
               id TEXT PRIMARY KEY, user_id TEXT, normalized_word TEXT, display_word TEXT,
               definition TEXT, phonetic TEXT, part_of_speech TEXT, selected_meanings_json TEXT,
               lemma TEXT, source_article_id TEXT, source_article_title TEXT,
               source_sentence TEXT, notes TEXT, mastery_level INTEGER, review_stage INTEGER,
               review_count INTEGER, correct_count INTEGER, incorrect_count INTEGER,
               encounter_count INTEGER, last_reviewed_at TEXT, next_review_at TEXT, status TEXT,
               frequency_rank INTEGER, tags_json TEXT, metadata_json TEXT, created_at TEXT,
               updated_at TEXT, deleted_at TEXT, version INTEGER, modified_by_device TEXT
             );
             CREATE TABLE vocabulary_occurrences(
               id TEXT PRIMARY KEY, vocabulary_id TEXT, article_id TEXT, article_title TEXT,
               source_sentence TEXT, created_at TEXT
             );
             CREATE TABLE vocabulary_review_state(
               vocabulary_id TEXT PRIMARY KEY, due_at TEXT, difficulty REAL, stability REAL,
               retrievability REAL, review_count INTEGER, lapse_count INTEGER,
               scheduler_version TEXT, updated_at TEXT
             );",
        ).unwrap();
    }
    #[test]
    fn vocabulary_roundtrip_preserves_occurrences_and_review_logs() {
        let connection = Connection::open_in_memory().unwrap();
        schema(&connection);
        let stamp = "2026-07-01T00:00:00Z";
        put(&connection, "vocabulary", &json!({
            "id": "v1", "userId": "local-user", "word": "Hello", "normalizedWord": "hello",
            "selectedMeanings": ["你好"], "status": "LEARNING", "reviewCount": 2,
            "incorrectCount": 1, "nextReviewAt": stamp, "createdAt": stamp, "updatedAt": stamp,
            "occurrences": [{
                "id": "o1", "vocabularyId": "v1", "articleId": "a1",
                "articleTitle": "T", "sourceSentence": "Hello", "createdAt": stamp
            }],
            "reviewLogs": [{"id": "l1", "result": "GOOD", "reviewedAt": stamp}]
        })).unwrap();
        let items = list(&connection, "vocabulary").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["occurrences"].as_array().map(Vec::len), Some(1));
        assert_eq!(items[0]["reviewLogs"].as_array().map(Vec::len), Some(1));
        let state: i64 = connection.query_row(
            "SELECT review_count FROM vocabulary_review_state WHERE vocabulary_id='v1'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(state, 2);
    }
}
