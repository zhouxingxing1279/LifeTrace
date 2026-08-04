use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};

use crate::database::legacy::json_parser;
use crate::database::migration_runner::{Migration, MigrationContext, MigrationError, MigrationReport};

const LEGACY_ARTICLES_TABLE: &str = "legacy_english_articles_json_v1";
const LEGACY_RECORDS_TABLE: &str = "legacy_english_learning_records_json_v1";
const LEGACY_HIGHLIGHTS_TABLE: &str = "legacy_english_highlights_json_v1";
const LEGACY_ENGLISH_NOTES_TABLE: &str = "legacy_english_notes_json_v1";
const LEGACY_ANALYSIS_TABLE: &str = "legacy_english_ai_analysis_json_v1";
const LEGACY_VOCABULARY_TABLE: &str = "legacy_english_user_vocabulary_json_v1";

fn table_exists(connection: &Connection, table: &str) -> bool {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
            [table],
            |_| Ok(()),
        )
        .optional()
        .ok()
        .flatten()
        .is_some()
}

fn text(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
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

/// m0005：英语模块 schema 规范化。
pub struct M0005English;

impl Migration for M0005English {
    fn version(&self) -> i64 {
        5
    }

    fn name(&self) -> &'static str {
        "english-normalization"
    }

    fn checksum(&self) -> &'static str {
        "m0005-english-v1"
    }

    fn up(
        &self,
        transaction: &Transaction,
        context: &MigrationContext,
    ) -> Result<MigrationReport, MigrationError> {
        rename_legacy_tables(transaction)?;
        create_normalized_tables(transaction)?;

        let legacy_articles = read_legacy(transaction, LEGACY_ARTICLES_TABLE)?;
        let legacy_records = read_legacy(transaction, LEGACY_RECORDS_TABLE)?;
        let legacy_highlights = read_legacy(transaction, LEGACY_HIGHLIGHTS_TABLE)?;
        let legacy_english_notes = read_legacy(transaction, LEGACY_ENGLISH_NOTES_TABLE)?;
        let legacy_analysis = read_legacy(transaction, LEGACY_ANALYSIS_TABLE)?;
        let legacy_vocabulary = read_legacy(transaction, LEGACY_VOCABULARY_TABLE)?;

        let mut article_count = 0usize;
        for value in &legacy_articles {
            insert_article(transaction, value)?;
            article_count += 1;
        }

        let mut record_count = 0usize;
        for value in &legacy_records {
            insert_record(transaction, value, context)?;
            record_count += 1;
        }

        let mut highlight_count = 0usize;
        for value in &legacy_highlights {
            insert_highlight(transaction, value)?;
            highlight_count += 1;
        }

        let mut english_note_count = 0usize;
        for value in &legacy_english_notes {
            insert_english_note(transaction, value)?;
            english_note_count += 1;
        }

        let mut analysis_count = 0usize;
        for value in &legacy_analysis {
            insert_analysis(transaction, value, context)?;
            analysis_count += 1;
        }

        let mut vocabulary_count = 0usize;
        let mut occurrence_count = 0usize;
        for value in &legacy_vocabulary {
            occurrence_count += insert_vocabulary(transaction, value, context)?;
            vocabulary_count += 1;
        }

        validate_english(
            transaction,
            &legacy_articles,
            &legacy_records,
            &legacy_highlights,
            &legacy_english_notes,
            &legacy_analysis,
            &legacy_vocabulary,
        )?;

        let mut report = MigrationReport::default();
        report.migrated = article_count + record_count + highlight_count + english_note_count
            + analysis_count
            + vocabulary_count;
        report.metrics.insert("english_articles".to_owned(), article_count as i64);
        report
            .metrics
            .insert("english_learning_records".to_owned(), record_count as i64);
        report
            .metrics
            .insert("english_highlights".to_owned(), highlight_count as i64);
        report
            .metrics
            .insert("english_notes".to_owned(), english_note_count as i64);
        report
            .metrics
            .insert("english_ai_analysis".to_owned(), analysis_count as i64);
        report
            .metrics
            .insert("english_vocabulary".to_owned(), vocabulary_count as i64);
        report
            .metrics
            .insert("vocabulary_occurrences".to_owned(), occurrence_count as i64);
        report
            .metrics
            .insert("vocabulary_review_state".to_owned(), vocabulary_count as i64);
        Ok(report)
    }
}

fn read_legacy(connection: &Connection, table: &str) -> Result<Vec<Value>, MigrationError> {
    if table_exists(connection, table) {
        json_parser::read_json_rows(connection, table).map_err(|message| MigrationError {
            version: 5,
            message,
        })
    } else {
        Ok(Vec::new())
    }
}

fn rename_legacy_tables(connection: &Connection) -> Result<(), MigrationError> {
    for (source, legacy) in [
        ("english_articles", LEGACY_ARTICLES_TABLE),
        ("english_learning_records", LEGACY_RECORDS_TABLE),
        ("english_highlights", LEGACY_HIGHLIGHTS_TABLE),
        ("english_notes", LEGACY_ENGLISH_NOTES_TABLE),
        ("english_ai_analysis", LEGACY_ANALYSIS_TABLE),
        ("english_user_vocabulary", LEGACY_VOCABULARY_TABLE),
    ] {
        if table_exists(connection, source) && !table_exists(connection, legacy) {
            connection
                .execute(&format!("ALTER TABLE {source} RENAME TO {legacy}"), [])
                .map_err(|error| MigrationError {
                    version: 5,
                    message: format!("重命名 {source} 失败: {error}"),
                })?;
        }
    }
    Ok(())
}

fn create_normalized_tables(connection: &Connection) -> Result<(), MigrationError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS english_articles (
               id TEXT PRIMARY KEY,
               title TEXT NOT NULL,
               level TEXT NOT NULL DEFAULT 'B1',
               category TEXT NOT NULL DEFAULT 'Life',
               content TEXT NOT NULL DEFAULT '',
               word_count INTEGER NOT NULL DEFAULT 0,
               difficulty INTEGER,
               estimated_minutes INTEGER,
               source TEXT,
               source_key TEXT,
               source_name TEXT,
               source_category TEXT,
               source_url TEXT,
               normalized_source_url TEXT,
               external_id TEXT,
               published_at TEXT,
               source_updated_at TEXT,
               image_url TEXT,
               audio_url TEXT,
               author TEXT,
               summary TEXT,
               fetched_at TEXT,
               rights_note TEXT,
               content_hash TEXT,
               language TEXT,
               quality_score REAL,
               has_audio INTEGER NOT NULL DEFAULT 0,
               license_type TEXT,
               attribution TEXT,
               processing_status TEXT,
               fetch_status TEXT,
               retry_count INTEGER NOT NULL DEFAULT 0,
               last_error TEXT,
               created_time TEXT,
               questions_json TEXT,
               vocabulary_json TEXT,
               raw_json TEXT,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               deleted_at TEXT,
               version INTEGER NOT NULL DEFAULT 1,
               modified_by_device TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_english_articles_level
               ON english_articles(level, category);
             CREATE INDEX IF NOT EXISTS idx_english_articles_source
               ON english_articles(source_key, published_at);
             CREATE TABLE IF NOT EXISTS english_learning_records (
               id TEXT PRIMARY KEY,
               user_id TEXT NOT NULL DEFAULT 'local',
               article_id TEXT REFERENCES english_articles(id),
               record_date TEXT NOT NULL,
               reading_time_seconds INTEGER NOT NULL DEFAULT 0,
               summary TEXT NOT NULL DEFAULT '',
               score REAL,
               analysis_id TEXT,
               new_words_json TEXT,
               completion_status TEXT NOT NULL DEFAULT 'reading',
               reading_status TEXT,
               started_at TEXT,
               completed_at TEXT,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               deleted_at TEXT,
               version INTEGER NOT NULL DEFAULT 1,
               modified_by_device TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_english_records_article
               ON english_learning_records(article_id, record_date);
             CREATE TABLE IF NOT EXISTS english_highlights (
               id TEXT PRIMARY KEY,
               user_id TEXT NOT NULL DEFAULT 'local',
               article_id TEXT REFERENCES english_articles(id),
               selected_text TEXT NOT NULL DEFAULT '',
               block_id TEXT,
               start_offset INTEGER,
               end_offset INTEGER,
               color TEXT NOT NULL DEFAULT 'yellow',
               prefix TEXT,
               suffix TEXT,
               note TEXT,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               deleted_at TEXT,
               version INTEGER NOT NULL DEFAULT 1,
               modified_by_device TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_english_highlights_article
               ON english_highlights(article_id);
             CREATE TABLE IF NOT EXISTS english_notes (
               id TEXT PRIMARY KEY,
               user_id TEXT NOT NULL DEFAULT 'local',
               article_id TEXT REFERENCES english_articles(id),
               quote TEXT,
               content TEXT NOT NULL DEFAULT '',
               block_id TEXT,
               start_offset INTEGER,
               end_offset INTEGER,
               selected_text TEXT,
               prefix TEXT,
               suffix TEXT,
               highlight_id TEXT,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               deleted_at TEXT,
               version INTEGER NOT NULL DEFAULT 1,
               modified_by_device TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_english_notes_article
               ON english_notes(article_id);
             CREATE TABLE IF NOT EXISTS english_vocabulary (
               id TEXT PRIMARY KEY,
               user_id TEXT NOT NULL DEFAULT 'local',
               normalized_word TEXT NOT NULL,
               display_word TEXT NOT NULL,
               definition TEXT NOT NULL DEFAULT '',
               phonetic TEXT NOT NULL DEFAULT '',
               part_of_speech TEXT NOT NULL DEFAULT '',
               selected_meanings_json TEXT,
               lemma TEXT NOT NULL DEFAULT '',
               source_article_id TEXT,
               source_article_title TEXT,
               source_sentence TEXT,
               notes TEXT NOT NULL DEFAULT '',
               mastery_level INTEGER NOT NULL DEFAULT 0,
               review_stage INTEGER NOT NULL DEFAULT 0,
               review_count INTEGER NOT NULL DEFAULT 0,
               correct_count INTEGER NOT NULL DEFAULT 0,
               incorrect_count INTEGER NOT NULL DEFAULT 0,
               encounter_count INTEGER NOT NULL DEFAULT 0,
               last_reviewed_at TEXT,
               next_review_at TEXT,
               status TEXT NOT NULL DEFAULT 'LEARNING',
               frequency_rank INTEGER,
               tags_json TEXT,
               metadata_json TEXT,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               deleted_at TEXT,
               version INTEGER NOT NULL DEFAULT 1,
               modified_by_device TEXT
             );
             CREATE UNIQUE INDEX IF NOT EXISTS uq_english_vocabulary_word
               ON english_vocabulary(user_id, normalized_word)
               WHERE deleted_at IS NULL;
             CREATE TABLE IF NOT EXISTS vocabulary_occurrences (
               id TEXT PRIMARY KEY,
               vocabulary_id TEXT NOT NULL REFERENCES english_vocabulary(id) ON DELETE CASCADE,
               article_id TEXT,
               article_title TEXT,
               source_sentence TEXT NOT NULL DEFAULT '',
               created_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_vocabulary_occurrences_word
               ON vocabulary_occurrences(vocabulary_id);
             CREATE TABLE IF NOT EXISTS vocabulary_review_state (
               vocabulary_id TEXT PRIMARY KEY REFERENCES english_vocabulary(id) ON DELETE CASCADE,
               due_at TEXT,
               difficulty REAL,
               stability REAL,
               retrievability REAL,
               review_count INTEGER NOT NULL DEFAULT 0,
               lapse_count INTEGER NOT NULL DEFAULT 0,
               scheduler_version TEXT,
               updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS english_ai_analysis (
               id TEXT PRIMARY KEY,
               user_id TEXT NOT NULL DEFAULT 'local',
               record_id TEXT REFERENCES english_learning_records(id),
               article_id TEXT,
               provider TEXT NOT NULL DEFAULT 'mock',
               score REAL NOT NULL DEFAULT 0,
               content_score REAL,
               grammar_score REAL,
               vocabulary_score REAL,
               structure_score REAL,
               mistakes_json TEXT,
               suggestions_json TEXT,
               improved_summary TEXT NOT NULL DEFAULT '',
               weak_points_json TEXT,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               deleted_at TEXT,
               version INTEGER NOT NULL DEFAULT 1,
               modified_by_device TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_english_analysis_record
               ON english_ai_analysis(record_id);",
        )
        .map_err(|error| MigrationError {
            version: 5,
            message: format!("创建英语规范化表失败: {error}"),
        })
}

fn insert_article(connection: &Connection, value: &Value) -> Result<(), MigrationError> {
    let object = json_parser::as_object(value, "英语文章").map_err(|message| MigrationError {
        version: 5,
        message,
    })?;
    let id = json_parser::string_field(object, "id")
        .ok_or_else(|| MigrationError { version: 5, message: format!("文章缺少 id: {}", value) })?;
    let title = json_parser::string_field(object, "title")
        .ok_or_else(|| MigrationError { version: 5, message: format!("文章 {id} 缺少 title") })?;
    let stamp = now();
    connection
        .execute(
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
            rusqlite::params![
                id,
                title,
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
                text(object, "createdAt").unwrap_or_else(|| stamp.clone()),
                text(object, "updatedAt").unwrap_or(stamp.clone()),
                object
                    .get("deletedAt")
                    .filter(|value| !value.is_null())
                    .and_then(Value::as_str),
                int_value(object, "version").unwrap_or(1).max(1)
            ],
        )
        .map(|_| ())
        .map_err(|error| MigrationError { version: 5, message: error.to_string() })
}

fn article_exists(connection: &Connection, id: &str) -> bool {
    connection
        .query_row(
            "SELECT 1 FROM english_articles WHERE id=?1",
            [id],
            |_| Ok(()),
        )
        .optional()
        .ok()
        .flatten()
        .is_some()
}

fn insert_record(
    connection: &Transaction,
    value: &Value,
    context: &MigrationContext,
) -> Result<(), MigrationError> {
    let object = json_parser::as_object(value, "学习记录").map_err(|message| MigrationError {
        version: 5,
        message,
    })?;
    let id = json_parser::string_field(object, "id")
        .ok_or_else(|| MigrationError { version: 5, message: format!("学习记录缺少 id: {}", value) })?;
    let article_id = json_parser::string_field(object, "articleId");
    if let Some(article_id) = article_id {
        if !article_exists(connection, article_id) {
            let _ = crate::database::migration_runner::record_issue(
                connection,
                context,
                "english_learning_records",
                Some(id),
                "warning",
                &format!("学习记录 {id} 引用的文章 {article_id} 不存在，article_id 置空"),
                Some(&value.to_string()),
            );
        }
    }
    let stamp = now();
    connection
        .execute(
            "INSERT OR REPLACE INTO english_learning_records(
               id, user_id, article_id, record_date, reading_time_seconds, summary, score,
               analysis_id, new_words_json, completion_status, reading_status, started_at,
               completed_at, created_at, updated_at, deleted_at, version, modified_by_device
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,NULL)",
            rusqlite::params![
                id,
                json_parser::string_field(object, "userId")
                    .filter(|value| !value.is_empty())
                    .unwrap_or("local"),
                article_id.filter(|article_id| article_exists(connection, article_id)),
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
                text(object, "createdAt").unwrap_or_else(|| stamp.clone()),
                text(object, "updatedAt").unwrap_or(stamp),
                object
                    .get("deletedAt")
                    .filter(|value| !value.is_null())
                    .and_then(Value::as_str),
                int_value(object, "version").unwrap_or(1).max(1)
            ],
        )
        .map(|_| ())
        .map_err(|error| MigrationError { version: 5, message: error.to_string() })
}

fn insert_highlight(connection: &Connection, value: &Value) -> Result<(), MigrationError> {
    let object = json_parser::as_object(value, "英语高亮").map_err(|message| MigrationError {
        version: 5,
        message,
    })?;
    let id = json_parser::string_field(object, "id")
        .ok_or_else(|| MigrationError { version: 5, message: format!("高亮缺少 id: {}", value) })?;
    let stamp = now();
    connection
        .execute(
            "INSERT OR REPLACE INTO english_highlights(
               id, user_id, article_id, selected_text, block_id, start_offset, end_offset,
               color, prefix, suffix, note, created_at, updated_at, deleted_at, version,
               modified_by_device
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,NULL)",
            rusqlite::params![
                id,
                json_parser::string_field(object, "userId")
                    .filter(|value| !value.is_empty())
                    .unwrap_or("local"),
                json_parser::string_field(object, "articleId"),
                text(object, "text").unwrap_or_default(),
                text(object, "blockId"),
                int_value(object, "startOffset"),
                int_value(object, "endOffset"),
                json_parser::string_field(object, "color").unwrap_or("yellow"),
                text(object, "prefix"),
                text(object, "suffix"),
                text(object, "note"),
                text(object, "createdAt").unwrap_or_else(|| stamp.clone()),
                text(object, "updatedAt").unwrap_or(stamp),
                object
                    .get("deletedAt")
                    .filter(|value| !value.is_null())
                    .and_then(Value::as_str),
                int_value(object, "version").unwrap_or(1).max(1)
            ],
        )
        .map(|_| ())
        .map_err(|error| MigrationError { version: 5, message: error.to_string() })
}

fn insert_english_note(connection: &Connection, value: &Value) -> Result<(), MigrationError> {
    let object = json_parser::as_object(value, "英语笔记").map_err(|message| MigrationError {
        version: 5,
        message,
    })?;
    let id = json_parser::string_field(object, "id")
        .ok_or_else(|| MigrationError { version: 5, message: format!("英语笔记缺少 id: {}", value) })?;
    let stamp = now();
    connection
        .execute(
            "INSERT OR REPLACE INTO english_notes(
               id, user_id, article_id, quote, content, block_id, start_offset, end_offset,
               selected_text, prefix, suffix, highlight_id, created_at, updated_at, deleted_at,
               version, modified_by_device
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,NULL)",
            rusqlite::params![
                id,
                json_parser::string_field(object, "userId")
                    .filter(|value| !value.is_empty())
                    .unwrap_or("local"),
                json_parser::string_field(object, "articleId"),
                text(object, "quote"),
                text(object, "content").unwrap_or_default(),
                text(object, "blockId"),
                int_value(object, "startOffset"),
                int_value(object, "endOffset"),
                text(object, "selectedText"),
                text(object, "prefix"),
                text(object, "suffix"),
                text(object, "highlightId"),
                text(object, "createdAt").unwrap_or_else(|| stamp.clone()),
                text(object, "updatedAt").unwrap_or(stamp),
                object
                    .get("deletedAt")
                    .filter(|value| !value.is_null())
                    .and_then(Value::as_str),
                int_value(object, "version").unwrap_or(1).max(1)
            ],
        )
        .map(|_| ())
        .map_err(|error| MigrationError { version: 5, message: error.to_string() })
}

fn insert_analysis(
    connection: &Transaction,
    value: &Value,
    context: &MigrationContext,
) -> Result<(), MigrationError> {
    let object = json_parser::as_object(value, "英语分析").map_err(|message| MigrationError {
        version: 5,
        message,
    })?;
    let id = json_parser::string_field(object, "id")
        .ok_or_else(|| MigrationError { version: 5, message: format!("分析缺少 id: {}", value) })?;
    let record_id = json_parser::string_field(object, "recordId");
    if let Some(record_id) = record_id {
        let record_exists: bool = connection
            .query_row(
                "SELECT 1 FROM english_learning_records WHERE id=?1",
                [record_id],
                |_| Ok(()),
            )
            .optional()
            .ok()
            .flatten()
            .is_some();
        if !record_exists {
            let _ = crate::database::migration_runner::record_issue(
                connection,
                context,
                "english_ai_analysis",
                Some(id),
                "warning",
                &format!("分析 {id} 引用的学习记录 {record_id} 不存在，record_id 置空"),
                Some(&value.to_string()),
            );
        }
    }
    let stamp = now();
    connection
        .execute(
            "INSERT OR REPLACE INTO english_ai_analysis(
               id, user_id, record_id, article_id, provider, score, content_score,
               grammar_score, vocabulary_score, structure_score, mistakes_json,
               suggestions_json, improved_summary, weak_points_json, created_at, updated_at,
               deleted_at, version, modified_by_device
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,NULL)",
            rusqlite::params![
                id,
                json_parser::string_field(object, "userId")
                    .filter(|value| !value.is_empty())
                    .unwrap_or("local"),
                record_id.filter(|record_id| {
                    connection
                        .query_row(
                            "SELECT 1 FROM english_learning_records WHERE id=?1",
                            [record_id],
                            |_| Ok(()),
                        )
                        .optional()
                        .ok()
                        .flatten()
                        .is_some()
                }),
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
                text(object, "createdAt").unwrap_or_else(|| stamp.clone()),
                text(object, "updatedAt").unwrap_or(stamp),
                object
                    .get("deletedAt")
                    .filter(|value| !value.is_null())
                    .and_then(Value::as_str),
                int_value(object, "version").unwrap_or(1).max(1)
            ],
        )
        .map(|_| ())
        .map_err(|error| MigrationError { version: 5, message: error.to_string() })
}

fn insert_vocabulary(
    connection: &Transaction,
    value: &Value,
    context: &MigrationContext,
) -> Result<usize, MigrationError> {
    let object = json_parser::as_object(value, "英语生词").map_err(|message| MigrationError {
        version: 5,
        message,
    })?;
    let id = json_parser::string_field(object, "id")
        .ok_or_else(|| MigrationError { version: 5, message: format!("生词缺少 id: {}", value) })?;
    let normalized = json_parser::string_field(object, "normalizedWord")
        .or_else(|| json_parser::string_field(object, "word"))
        .map(str::to_lowercase)
        .ok_or_else(|| MigrationError { version: 5, message: format!("生词 {id} 缺少 word") })?;
    let stamp = now();
    let selected_meanings = object
        .get("selectedMeanings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let definition = selected_meanings
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join("; ");
    let review_logs = object
        .get("reviewLogs")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let mut metadata = serde_json::Map::new();
    metadata.insert("reviewLogs".to_owned(), review_logs);
    if let Some(value) = int_value(object, "dictionaryWordId") {
        metadata.insert("dictionaryWordId".to_owned(), json!(value));
    }
    let metadata_json = Value::Object(metadata);
    let status = json_parser::string_field(object, "status").unwrap_or("LEARNING");
    connection
        .execute(
            "INSERT OR REPLACE INTO english_vocabulary(
               id, user_id, normalized_word, display_word, definition, phonetic,
               part_of_speech, selected_meanings_json, lemma, source_article_id,
               source_article_title, source_sentence, notes, mastery_level, review_stage,
               review_count, correct_count, incorrect_count, encounter_count,
               last_reviewed_at, next_review_at, status, frequency_rank, tags_json,
               metadata_json, created_at, updated_at, deleted_at, version,
               modified_by_device
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,NULL)",
            rusqlite::params![
                id,
                json_parser::string_field(object, "userId")
                    .filter(|value| !value.is_empty())
                    .unwrap_or("local"),
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
                status,
                int_value(object, "frequencyRank"),
                object.get("tags").map(Value::to_string),
                metadata_json.to_string(),
                text(object, "createdAt").unwrap_or_else(|| stamp.clone()),
                text(object, "updatedAt").unwrap_or(stamp.clone()),
                object
                    .get("deletedAt")
                    .filter(|value| !value.is_null())
                    .and_then(Value::as_str),
                int_value(object, "version").unwrap_or(1).max(1)
            ],
        )
        .map_err(|error| MigrationError { version: 5, message: error.to_string() })?;

    // 出现记录。
    let mut occurrence_count = 0usize;
    let occurrences = object
        .get("occurrences")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for occurrence in &occurrences {
        let occurrence_id = occurrence
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        connection
            .execute(
                "INSERT OR IGNORE INTO vocabulary_occurrences(
                   id, vocabulary_id, article_id, article_title, source_sentence, created_at
                 ) VALUES(?1,?2,?3,?4,?5,?6)",
                rusqlite::params![
                    occurrence_id,
                    id,
                    text(occurrence.as_object().unwrap_or(&serde_json::Map::new()), "articleId"),
                    text(occurrence.as_object().unwrap_or(&serde_json::Map::new()), "articleTitle"),
                    text(occurrence.as_object().unwrap_or(&serde_json::Map::new()), "sourceSentence")
                        .unwrap_or_default(),
                    text(occurrence.as_object().unwrap_or(&serde_json::Map::new()), "createdAt")
                        .unwrap_or_else(now)
                ],
            )
            .map_err(|error| MigrationError { version: 5, message: error.to_string() })?;
        occurrence_count += 1;
    }

    // 复习状态（FSRS 字段预留）。
    connection
        .execute(
            "INSERT OR REPLACE INTO vocabulary_review_state(
               vocabulary_id, due_at, difficulty, stability, retrievability, review_count,
               lapse_count, scheduler_version, updated_at
             ) VALUES(?1,?2,NULL,NULL,NULL,?3,?4,NULL,?5)",
            rusqlite::params![
                id,
                text(object, "nextReviewAt"),
                int_value(object, "reviewCount").unwrap_or(0),
                int_value(object, "incorrectCount").unwrap_or(0),
                stamp
            ],
        )
        .map_err(|error| MigrationError { version: 5, message: error.to_string() })?;

    // 去重检查：同名不同 id 时记录 issue。
    let duplicate: bool = connection
        .query_row(
            "SELECT 1 FROM english_vocabulary
             WHERE user_id=?1 AND normalized_word=?2 AND id<>?3 AND deleted_at IS NULL",
            rusqlite::params![
                json_parser::string_field(object, "userId")
                    .filter(|value| !value.is_empty())
                    .unwrap_or("local"),
                normalized,
                id
            ],
            |_| Ok(()),
        )
        .optional()
        .ok()
        .flatten()
        .is_some();
    if duplicate {
        let _ = crate::database::migration_runner::record_issue(
            connection,
            context,
            "english_vocabulary",
            Some(id),
            "warning",
            &format!("生词 {normalized} 存在重复记录（{id}），统一保留一条活跃记录"),
            Some(&value.to_string()),
        );
    }
    Ok(occurrence_count)
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn validate_english(
    connection: &Connection,
    legacy_articles: &[Value],
    legacy_records: &[Value],
    legacy_highlights: &[Value],
    legacy_english_notes: &[Value],
    legacy_analysis: &[Value],
    legacy_vocabulary: &[Value],
) -> Result<(), MigrationError> {
    let counts: (i64, i64, i64, i64, i64, i64) = connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM english_articles),
               (SELECT COUNT(*) FROM english_learning_records),
               (SELECT COUNT(*) FROM english_highlights),
               (SELECT COUNT(*) FROM english_notes),
               (SELECT COUNT(*) FROM english_ai_analysis),
               (SELECT COUNT(*) FROM english_vocabulary)",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .map_err(|error| MigrationError { version: 5, message: error.to_string() })?;
    if counts.0 != legacy_articles.len() as i64 {
        return Err(MigrationError { version: 5, message: format!("文章数量不一致: 旧 {}，新 {}", legacy_articles.len(), counts.0) });
    }
    if counts.1 != legacy_records.len() as i64 {
        return Err(MigrationError { version: 5, message: format!("学习记录数量不一致: 旧 {}，新 {}", legacy_records.len(), counts.1) });
    }
    if counts.2 != legacy_highlights.len() as i64 {
        return Err(MigrationError { version: 5, message: format!("高亮数量不一致: 旧 {}，新 {}", legacy_highlights.len(), counts.2) });
    }
    if counts.3 != legacy_english_notes.len() as i64 {
        return Err(MigrationError { version: 5, message: format!("英语笔记数量不一致: 旧 {}，新 {}", legacy_english_notes.len(), counts.3) });
    }
    if counts.4 != legacy_analysis.len() as i64 {
        return Err(MigrationError { version: 5, message: format!("分析数量不一致: 旧 {}，新 {}", legacy_analysis.len(), counts.4) });
    }
    if counts.5 != legacy_vocabulary.len() as i64 {
        return Err(MigrationError { version: 5, message: format!("生词数量不一致: 旧 {}，新 {}", legacy_vocabulary.len(), counts.5) });
    }
    // 生词唯一约束生效检查。
    let unique_violations: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM (
               SELECT user_id, normalized_word FROM english_vocabulary
               WHERE deleted_at IS NULL
               GROUP BY user_id, normalized_word HAVING COUNT(*) > 1
             )",
            [],
            |row| row.get(0),
        )
        .map_err(|error| MigrationError { version: 5, message: error.to_string() })?;
    if unique_violations > 0 {
        return Err(MigrationError {
            version: 5,
            message: format!("生词唯一约束冲突 {unique_violations} 组"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migrations::{M0001Framework, M0002Finance, M0003HabitsReviews, M0004Notes};
    use crate::database::migration_runner::run;
    use rusqlite::Connection;
    use serde_json::json;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("lifetrace-english-{label}-{unique}"));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn seed_legacy_json(connection: &Connection) {
        connection
            .execute_batch(
                "CREATE TABLE english_articles(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );
                 CREATE TABLE english_learning_records(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );
                 CREATE TABLE english_highlights(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );
                 CREATE TABLE english_notes(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );
                 CREATE TABLE english_ai_analysis(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );
                 CREATE TABLE english_user_vocabulary(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );",
            )
            .unwrap();
        let stamp = "2026-07-01T00:00:00Z";
        connection
            .execute(
                "INSERT INTO english_articles VALUES('a1', ?1, ?2)",
                rusqlite::params![
                    json!({
                        "id": "a1", "title": "Test Article", "level": "B1", "category": "Science",
                        "content": "Hello world content.", "vocabulary": [], "questions": [],
                        "difficulty": 3, "estimatedMinutes": 2, "createdTime": stamp,
                        "updatedAt": stamp, "source": "local", "wordCount": 3,
                        "processingStatus": "READY", "fetchStatus": "SUCCESS"
                    })
                    .to_string(),
                    stamp
                ],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO english_learning_records VALUES('r1', ?1, ?2)",
                rusqlite::params![
                    json!({
                        "id": "r1", "userId": "local-user", "date": "2026-07-01",
                        "articleId": "a1", "readingTimeSeconds": 60, "summary": "ok",
                        "newWords": [], "completionStatus": "completed",
                        "readingStatus": "completed", "startedAt": stamp, "completedAt": stamp,
                        "createdAt": stamp, "updatedAt": stamp
                    })
                    .to_string(),
                    stamp
                ],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO english_highlights VALUES('h1', ?1, ?2)",
                rusqlite::params![
                    json!({
                        "id": "h1", "userId": "local-user", "articleId": "a1", "text": "Hello",
                        "color": "yellow", "createdAt": stamp, "updatedAt": stamp
                    })
                    .to_string(),
                    stamp
                ],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO english_user_vocabulary VALUES('v1', ?1, ?2)",
                rusqlite::params![
                    json!({
                        "id": "v1", "userId": "local-user", "word": "Hello", "normalizedWord": "hello",
                        "lemma": "hello", "phonetic": "/həˈloʊ/", "selectedMeanings": ["你好"],
                        "partOfSpeech": "interj", "notes": "", "masteryLevel": 0, "reviewStage": 0,
                        "reviewCount": 0, "correctCount": 0, "incorrectCount": 0,
                        "encounterCount": 1, "nextReviewAt": stamp, "status": "LEARNING",
                        "createdAt": stamp, "updatedAt": stamp,
                        "occurrences": [{
                            "id": "o1", "vocabularyId": "v1", "articleId": "a1",
                            "articleTitle": "Test Article", "sourceSentence": "Hello world",
                            "createdAt": stamp
                        }]
                    })
                    .to_string(),
                    stamp
                ],
            )
            .unwrap();
    }

    #[test]
    fn migrates_english_with_vocabulary_and_occurrences() {
        let directory = temp_dir("migrate");
        let mut connection = Connection::open(directory.join("test.db")).unwrap();
        seed_legacy_json(&connection);
        let context = crate::database::migration_runner::MigrationContext::new(directory.clone());
        let migrations: Vec<Box<dyn Migration>> = vec![
            Box::new(M0001Framework),
            Box::new(M0002Finance),
            Box::new(M0003HabitsReviews),
            Box::new(M0004Notes),
            Box::new(M0005English),
        ];
        run(&mut connection, &context, &migrations).unwrap();
        let articles: i64 = connection
            .query_row("SELECT COUNT(*) FROM english_articles", [], |row| row.get(0))
            .unwrap();
        let records: i64 = connection
            .query_row("SELECT COUNT(*) FROM english_learning_records", [], |row| row.get(0))
            .unwrap();
        let vocabulary: i64 = connection
            .query_row("SELECT COUNT(*) FROM english_vocabulary", [], |row| row.get(0))
            .unwrap();
        let occurrences: i64 = connection
            .query_row("SELECT COUNT(*) FROM vocabulary_occurrences", [], |row| row.get(0))
            .unwrap();
        let review_state: i64 = connection
            .query_row("SELECT COUNT(*) FROM vocabulary_review_state", [], |row| row.get(0))
            .unwrap();
        assert_eq!(articles, 1);
        assert_eq!(records, 1);
        assert_eq!(vocabulary, 1);
        assert_eq!(occurrences, 1);
        assert_eq!(review_state, 1);
        let word: String = connection
            .query_row(
                "SELECT normalized_word FROM english_vocabulary WHERE id='v1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(word, "hello");
        fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn duplicate_vocabulary_is_detected() {
        let directory = temp_dir("duplicate");
        let mut connection = Connection::open(directory.join("test.db")).unwrap();
        seed_legacy_json(&connection);
        connection
            .execute(
                "INSERT INTO english_user_vocabulary VALUES('v2', ?1, '2026-07-01T00:00:00Z')",
                rusqlite::params![json!({
                    "id": "v2", "userId": "local-user", "word": "hello", "normalizedWord": "hello",
                    "selectedMeanings": ["你好"], "status": "LEARNING",
                    "createdAt": "2026-07-01T00:00:00Z", "updatedAt": "2026-07-01T00:00:00Z"
                })
                .to_string()],
            )
            .unwrap();
        let context = crate::database::migration_runner::MigrationContext::new(directory.clone());
        let migrations: Vec<Box<dyn Migration>> = vec![
            Box::new(M0001Framework),
            Box::new(M0002Finance),
            Box::new(M0003HabitsReviews),
            Box::new(M0004Notes),
            Box::new(M0005English),
        ];
        let result = run(&mut connection, &context, &migrations);
        assert!(result.is_err(), "重复生词应导致校验失败并回滚");
        fs::remove_dir_all(&directory).ok();
    }
}
