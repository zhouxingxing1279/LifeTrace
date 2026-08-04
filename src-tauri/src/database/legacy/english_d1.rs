//! 旧 D1 英语数据导入规范化表。

use rusqlite::{Connection, OptionalExtension};
use serde_json::json;

use crate::database::repositories::english as english_repo;

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

/// 从旧 JSON 表导入英语业务实体。
pub fn import_json_table(
    source: &Connection,
    destination: &mut Connection,
    source_table: &str,
    destination_table: &str,
) -> Result<usize, String> {
    if source_table == "english_user_vocabulary" {
        return import_d1_vocabulary(source, destination);
    }
    let rows = super::json_parser::read_json_rows(source, source_table)?;
    let key = match destination_table {
        "english_articles" => "articles",
        "english_learning_records" => "records",
        "english_highlights" => "highlights",
        "english_notes" => "notes",
        "english_ai_analysis" => "analysis",
        "english_vocabulary" => "vocabulary",
        _ => return Ok(0),
    };
    let transaction = destination
        .transaction()
        .map_err(|error| error.to_string())?;
    for value in &rows {
        english_repo::put(&transaction, key, value)?;
    }
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(rows.len())
}

/// D1 `english_user_vocabulary` 为真实列，映射为 DTO 后写入。
fn import_d1_vocabulary(source: &Connection, destination: &mut Connection) -> Result<usize, String> {
    if !table_exists(source, "english_user_vocabulary") {
        return Ok(0);
    }
    let mut statement = source
        .prepare(
            "SELECT id, word, normalized_word, lemma, dictionary_word_id, phonetic,
                    selected_meanings_json, part_of_speech, source_article_id,
                    source_article_title, source_sentence, notes, mastery_level, review_stage,
                    review_count, correct_count, incorrect_count, encounter_count,
                    last_reviewed_at, next_review_at, status, frequency_rank, tags_json,
                    created_at, updated_at
             FROM english_user_vocabulary",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            let selected_meanings: Option<String> = row.get(6)?;
            let tags: Option<String> = row.get(22)?;
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "userId": "local-user",
                "word": row.get::<_, String>(1)?,
                "normalizedWord": row.get::<_, String>(2)?,
                "lemma": row.get::<_, String>(3)?,
                "dictionaryWordId": row.get::<_, Option<i64>>(4)?,
                "phonetic": row.get::<_, String>(5)?,
                "selectedMeanings": selected_meanings
                    .as_deref()
                    .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
                    .unwrap_or_else(|| serde_json::json!([])),
                "partOfSpeech": row.get::<_, String>(7)?,
                "sourceArticleId": row.get::<_, Option<String>>(8)?,
                "sourceArticleTitle": row.get::<_, Option<String>>(9)?,
                "sourceSentence": row.get::<_, Option<String>>(10)?,
                "notes": row.get::<_, String>(11)?,
                "masteryLevel": row.get::<_, i64>(12)?,
                "reviewStage": row.get::<_, i64>(13)?,
                "reviewCount": row.get::<_, i64>(14)?,
                "correctCount": row.get::<_, i64>(15)?,
                "incorrectCount": row.get::<_, i64>(16)?,
                "encounterCount": row.get::<_, i64>(17)?,
                "lastReviewedAt": row.get::<_, Option<String>>(18)?,
                "nextReviewAt": row.get::<_, Option<String>>(19)?,
                "status": row.get::<_, String>(20)?,
                "frequencyRank": row.get::<_, Option<i64>>(21)?,
                "tags": tags
                    .as_deref()
                    .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
                    .unwrap_or_else(|| serde_json::json!([])),
                "createdAt": row.get::<_, String>(23)?,
                "updatedAt": row.get::<_, String>(24)?
            }))
        })
        .map_err(|error| error.to_string())?;
    let items = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let transaction = destination
        .transaction()
        .map_err(|error| error.to_string())?;
    for item in &items {
        english_repo::put(&transaction, "vocabulary", item)?;
    }
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(items.len())
}
