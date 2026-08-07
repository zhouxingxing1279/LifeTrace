use std::path::{Path, PathBuf};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use regex::Regex;
use rusqlite::{Connection, OpenFlags, OptionalExtension, Row};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use super::AppState;

const WORD_COLUMNS: &str = "id,word,normalized_word,strip_word,phonetic,definition,translation,pos,collins,oxford,tag,bnc,frq,exchange,detail,audio";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LookupQuery {
    word: String,
    article_id: Option<String>,
    sentence: Option<String>,
}

#[derive(Clone)]
struct WordRow {
    id: i64,
    normalized_word: String,
    phonetic: String,
    definition: String,
    translation: String,
    pos: String,
    collins: i64,
    oxford: bool,
    tag: String,
    bnc: Option<i64>,
    frq: Option<i64>,
    exchange: String,
    detail: String,
    audio: Option<String>,
}

fn response_error(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(json!({ "error": message.into() }))).into_response()
}

fn from_row(row: &Row<'_>) -> rusqlite::Result<WordRow> {
    Ok(WordRow {
        id: row.get(0)?,
        normalized_word: row.get(2)?,
        phonetic: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
        definition: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
        translation: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
        pos: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
        collins: row.get::<_, Option<i64>>(8)?.unwrap_or_default(),
        oxford: row.get::<_, Option<i64>>(9)?.unwrap_or_default() != 0,
        tag: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
        bnc: row.get(11)?,
        frq: row.get(12)?,
        exchange: row.get::<_, Option<String>>(13)?.unwrap_or_default(),
        detail: row.get::<_, Option<String>>(14)?.unwrap_or_default(),
        audio: row.get(15)?,
    })
}

pub fn resolve_path(resource_dir: &Path) -> PathBuf {
    let bundled = resource_dir.join("dictionary").join("dictionary.db");
    if bundled.is_file() {
        return bundled;
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("xunji_service")
        .join("data")
        .join("dictionary.db")
}

fn normalize(value: &str) -> Option<String> {
    let normalized = value
        .trim_matches(|character: char| {
            character.is_whitespace()
                || matches!(
                    character,
                    '.' | ','
                        | '!'
                        | '?'
                        | ';'
                        | ':'
                        | '('
                        | ')'
                        | '['
                        | ']'
                        | '{'
                        | '}'
                        | '<'
                        | '>'
                        | '"'
                        | '\''
                        | '“'
                        | '”'
                        | '‘'
                        | '’'
                        | '…'
                        | '，'
                        | '。'
                        | '！'
                        | '？'
                        | '；'
                        | '：'
                )
        })
        .to_lowercase();
    let normalized = normalized
        .strip_suffix("'s")
        .unwrap_or(&normalized)
        .to_owned();
    Regex::new(r"^[a-z]+(?:['-][a-z]+)*$")
        .expect("valid word regex")
        .is_match(&normalized)
        .then_some(normalized)
}

fn suffix_candidates(word: &str) -> Vec<String> {
    let mut values = Vec::new();
    if word.ends_with("ies") && word.len() > 4 {
        values.push(format!("{}y", &word[..word.len() - 3]));
    }
    if word.ends_with("ing") && word.len() > 5 {
        let stem = &word[..word.len() - 3];
        values.push(stem.to_owned());
        values.push(format!("{stem}e"));
        let chars: Vec<char> = stem.chars().collect();
        if chars.len() > 2 && chars[chars.len() - 1] == chars[chars.len() - 2] {
            values.push(chars[..chars.len() - 1].iter().collect());
        }
    }
    if word.ends_with("ed") && word.len() > 4 {
        let stem = &word[..word.len() - 2];
        values.push(stem.to_owned());
        values.push(format!("{stem}e"));
        if let Some(prefix) = stem.strip_suffix('i') {
            values.push(format!("{prefix}y"));
        }
    }
    if word.ends_with("es") && word.len() > 4 {
        values.push(word[..word.len() - 2].to_owned());
        values.push(word[..word.len() - 1].to_owned());
    } else if word.ends_with('s') && word.len() > 3 {
        values.push(word[..word.len() - 1].to_owned());
    }
    values.sort();
    values.dedup();
    values
}

fn find_word(connection: &Connection, word: &str) -> Result<(Option<WordRow>, String), String> {
    let lemma = connection
        .query_row(
            "SELECT lemma FROM dictionary_lemmas WHERE word_form=?1 COLLATE NOCASE LIMIT 1",
            [word],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|value| value.to_string())?;
    let stripped: String = word
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect();
    for (field, value) in [
        ("word", word),
        ("normalized_word", word),
        ("strip_word", stripped.as_str()),
    ] {
        let sql = format!(
            "SELECT {WORD_COLUMNS} FROM dictionary_words WHERE {field}=?1 COLLATE NOCASE LIMIT 1"
        );
        if let Some(row) = connection
            .query_row(&sql, [value], from_row)
            .optional()
            .map_err(|value| value.to_string())?
        {
            if let Some(lemma) = lemma.as_deref().filter(|lemma| *lemma != word) {
                let sql = format!(
                    "SELECT {WORD_COLUMNS} FROM dictionary_words WHERE normalized_word=?1 COLLATE NOCASE LIMIT 1"
                );
                if let Some(lemma_row) = connection
                    .query_row(&sql, [lemma], from_row)
                    .optional()
                    .map_err(|value| value.to_string())?
                {
                    return Ok((Some(lemma_row), lemma.to_owned()));
                }
            }
            let resolved = lemma.unwrap_or_else(|| row.normalized_word.clone());
            return Ok((Some(row), resolved));
        }
    }
    let mut candidates = lemma.into_iter().collect::<Vec<_>>();
    candidates.extend(suffix_candidates(word));
    candidates.dedup();
    for candidate in candidates {
        let sql = format!(
            "SELECT {WORD_COLUMNS} FROM dictionary_words WHERE normalized_word=?1 COLLATE NOCASE LIMIT 1"
        );
        if let Some(row) = connection
            .query_row(&sql, [&candidate], from_row)
            .optional()
            .map_err(|value| value.to_string())?
        {
            return Ok((Some(row), candidate));
        }
    }
    Ok((None, word.to_owned()))
}

fn lines(value: &str) -> Vec<String> {
    value
        .replace("\\n", "\n")
        .lines()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

pub async fn lookup(State(state): State<AppState>, Query(query): Query<LookupQuery>) -> Response {
    let Some(normalized) = normalize(&query.word) else {
        return Json(json!({
            "queryWord": query.word,
            "normalizedWord": "",
            "found": false,
            "reason": "INVALID_WORD",
            "articleId": query.article_id,
            "sourceSentence": query.sentence.unwrap_or_default().trim()
        }))
        .into_response();
    };
    if !state.dictionary_path.is_file() {
        return response_error(StatusCode::SERVICE_UNAVAILABLE, "离线词典尚未安装");
    }
    let connection =
        match Connection::open_with_flags(&state.dictionary_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        {
            Ok(value) => value,
            Err(value) => {
                return response_error(StatusCode::INTERNAL_SERVER_ERROR, value.to_string())
            }
        };
    let (row, lemma) = match find_word(&connection, &normalized) {
        Ok(value) => value,
        Err(value) => return response_error(StatusCode::INTERNAL_SERVER_ERROR, value),
    };
    let Some(row) = row else {
        return Json(json!({
            "queryWord": query.word,
            "normalizedWord": normalized,
            "lemma": lemma,
            "found": false,
            "reason": "NOT_FOUND",
            "articleId": query.article_id,
            "sourceSentence": query.sentence.unwrap_or_default().trim()
        }))
        .into_response();
    };
    let translations = lines(&row.translation);
    let definitions = lines(&row.definition);
    let pos = if row.pos.is_empty() {
        translations
            .first()
            .and_then(|value| value.split('.').next())
            .unwrap_or("unknown")
            .to_owned()
    } else {
        row.pos
    };
    let mut exchange = Map::new();
    for item in row.exchange.split('/') {
        if let Some((key, value)) = item.split_once(':') {
            exchange.insert(key.to_owned(), json!(value));
        }
    }
    let detail = serde_json::from_str::<Value>(&row.detail).unwrap_or_else(|_| json!(row.detail));
    Json(json!({
        "queryWord": query.word,
        "normalizedWord": normalized,
        "lemma": lemma,
        "found": true,
        "dictionaryWordId": row.id,
        "phonetic": row.phonetic,
        "partsOfSpeech": [{"type": pos, "translation": translations, "definition": definitions}],
        "collins": row.collins,
        "oxford": row.oxford,
        "tags": row.tag.split_whitespace().collect::<Vec<_>>(),
        "bncRank": row.bnc,
        "frequencyRank": row.frq,
        "exchange": exchange,
        "detail": detail,
        "audio": row.audio,
        "articleId": query.article_id,
        "sourceSentence": query.sentence.unwrap_or_default().trim()
    }))
    .into_response()
}
