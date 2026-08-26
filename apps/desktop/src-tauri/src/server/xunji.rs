use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use image::{imageops, DynamicImage, GrayImage};
use regex::Regex;
use reqwest::{redirect::Policy, Client};
use rqrr::PreparedImage;
use rusqlite::Connection;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use url::Url;
use uuid::Uuid;

use crate::database::repositories::{habits, workouts};

use super::AppState;

const MAX_IMAGE_SIZE: usize = 15 * 1024 * 1024;
const MAX_PAGE_SIZE: usize = 6 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkoutSet {
    weight_kg: f64,
    reps: u32,
    set_number: usize,
}

#[derive(Clone, Serialize, Deserialize)]
struct WorkoutExercise {
    name: String,
    sets: Vec<WorkoutSet>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Workout {
    source: &'static str,
    date: String,
    title: String,
    duration_minutes: u32,
    calories_kcal: f64,
    volume_kg: f64,
    exercises: Vec<WorkoutExercise>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAction {
    import_id: String,
    action: String,
    workout: Option<Value>,
}

fn stamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn identifier() -> String {
    Uuid::new_v4().to_string()
}

fn failure(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(json!({ "error": message.into() }))).into_response()
}

fn validate_share_url(value: &str, base: Option<&Url>) -> Result<Url, String> {
    let url = if let Some(base) = base {
        base.join(value)
            .map_err(|_| "训练分享地址无效".to_owned())?
    } else {
        Url::parse(value).map_err(|_| "训练分享地址无效".to_owned())?
    };
    if url.scheme() != "https"
        || url.host_str() != Some("api.xunjiapp.cn")
        || !(url.path() == "/app_share" || url.path().starts_with("/app_share/"))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        return Err("训练分享地址不在允许范围内".to_owned());
    }
    Ok(url)
}

fn qr_variants(image: &DynamicImage) -> Vec<GrayImage> {
    let gray = image.to_luma8();
    let height = gray.height();
    let mut bases = vec![gray.clone()];
    // 训记分享图的二维码通常位于下半部分。保留两个底部裁剪版本，
    // 避免对整张高分辨率截图反复旋转、放大造成数十秒阻塞。
    for ratio in [0.4_f32, 0.6] {
        let top = (height as f32 * ratio) as u32;
        if top < height {
            bases.push(imageops::crop_imm(&gray, 0, top, gray.width(), height - top).to_image());
        }
    }
    bases
        .into_iter()
        .map(|base| {
            let longest = base.width().max(base.height());
            if longest <= 1_600 {
                base
            } else {
                let scale = 1_600_f32 / longest as f32;
                imageops::resize(
                    &base,
                    (base.width() as f32 * scale).max(1.0) as u32,
                    (base.height() as f32 * scale).max(1.0) as u32,
                    imageops::FilterType::Triangle,
                )
            }
        })
        .collect()
}

fn decode_qr(bytes: &[u8]) -> Result<Url, String> {
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_SIZE {
        return Err("图片为空或超过 15MB".to_owned());
    }
    let image = image::load_from_memory(bytes)
        .map_err(|_| "无法读取图片，请上传 JPG、PNG 或截图".to_owned())?;
    for variant in qr_variants(&image) {
        let mut prepared = PreparedImage::prepare(variant);
        for grid in prepared.detect_grids() {
            if let Ok((_meta, content)) = grid.decode() {
                if let Ok(url) = validate_share_url(content.trim(), None) {
                    return Ok(url);
                }
            }
        }
    }
    Err("图片中没有识别到训练分享二维码".to_owned())
}

async fn fetch_page(mut url: Url) -> Result<(Url, String), String> {
    let client = Client::builder()
        .redirect(Policy::none())
        .user_agent("Mozilla/5.0 LifeTrace-Rust/2.0")
        .build()
        .map_err(|value| value.to_string())?;
    for _ in 0..4 {
        let response = client
            .get(url.clone())
            .header("accept", "text/html,application/xhtml+xml,application/json")
            .send()
            .await
            .map_err(|_| "训练分享链接访问失败".to_owned())?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| "训练分享跳转地址无效".to_owned())?;
            url = validate_share_url(location, Some(&url))?;
            continue;
        }
        if !response.status().is_success() {
            return Err("训练分享链接已失效".to_owned());
        }
        if response
            .content_length()
            .is_some_and(|size| size as usize > MAX_PAGE_SIZE)
        {
            return Err("训练分享页面数据过大".to_owned());
        }
        let bytes = response.bytes().await.map_err(|value| value.to_string())?;
        if bytes.len() > MAX_PAGE_SIZE {
            return Err("训练分享页面数据过大".to_owned());
        }
        return Ok((url, String::from_utf8_lossy(&bytes).into_owned()));
    }
    Err("训练分享跳转次数过多".to_owned())
}

fn numeric(value: Option<&Value>) -> f64 {
    match value {
        Some(Value::Number(number)) => number.as_f64().unwrap_or_default(),
        Some(value) => Regex::new(r"-?\d+(?:\.\d+)?")
            .expect("numeric regex")
            .find(&value.to_string())
            .and_then(|value| value.as_str().parse().ok())
            .unwrap_or_default(),
        None => 0.0,
    }
}

fn first<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    let object = value.as_object()?;
    keys.iter().find_map(|key| {
        object
            .iter()
            .find_map(|(candidate, value)| candidate.eq_ignore_ascii_case(key).then_some(value))
    })
}

fn find_list<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Vec<Value>> {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if keys
                    .iter()
                    .any(|candidate| key.eq_ignore_ascii_case(candidate))
                {
                    if let Some(items) = value.as_array() {
                        return Some(items);
                    }
                }
            }
            object.values().find_map(|value| find_list(value, keys))
        }
        Value::Array(items) => items.iter().find_map(|value| find_list(value, keys)),
        _ => None,
    }
}

fn workout_date(value: Option<&Value>) -> String {
    let text = value.map(Value::to_string).unwrap_or_default();
    if let Some(found) = Regex::new(r"(20\d{2})[-/.年](\d{1,2})[-/.月](\d{1,2})")
        .expect("date regex")
        .captures(&text)
    {
        let year = found[1].parse::<u32>().unwrap_or(1970);
        let month = found[2].parse::<u32>().unwrap_or(1);
        let day = found[3].parse::<u32>().unwrap_or(1);
        return format!("{year:04}-{month:02}-{day:02}");
    }
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn normalize_workout(raw: &Value) -> Option<Workout> {
    let raw_exercises = find_list(
        raw,
        &[
            "exercises",
            "exercise_list",
            "trainingitems",
            "actions",
            "movement",
            "movements",
        ],
    )?;
    let mut exercises = Vec::new();
    for (index, item) in raw_exercises.iter().enumerate() {
        let name = first(
            item,
            &[
                "name",
                "label",
                "exerciseName",
                "actionName",
                "title",
                "cnName",
            ],
        )
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("动作 {}", index + 1));
        let Some(raw_sets) = find_list(item, &["sets", "setlist", "groups", "details", "records"])
        else {
            continue;
        };
        let sets: Vec<WorkoutSet> = raw_sets
            .iter()
            .filter(|set| {
                first(set, &["done", "completed", "isDone"])
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
            })
            .enumerate()
            .map(|(set_index, set)| WorkoutSet {
                weight_kg: numeric(first(set, &["weightKg", "weight", "kg", "load"])).max(0.0),
                reps: numeric(first(set, &["reps", "rep", "times", "count", "number"])).max(0.0)
                    as u32,
                set_number: set_index + 1,
            })
            .collect();
        if !sets.is_empty() {
            exercises.push(WorkoutExercise { name, sets });
        }
    }
    if exercises.is_empty() {
        return None;
    }
    let selected = first(raw, &["workout", "training", "data"]).unwrap_or(raw);
    let title = first(selected, &["title", "name", "workoutName", "trainingName"])
        .and_then(Value::as_str)
        .unwrap_or("训练记录")
        .trim()
        .to_owned();
    let mut duration = numeric(first(
        selected,
        &["durationMinutes", "duration", "trainingTime", "time"],
    ));
    if duration > 24.0 * 60.0 {
        duration /= 60.0;
    }
    let calories = numeric(first(
        selected,
        &["caloriesKcal", "calories", "kcal", "heat"],
    ));
    let computed_volume: f64 = exercises
        .iter()
        .flat_map(|exercise| &exercise.sets)
        .map(|set| set.weight_kg * set.reps as f64)
        .sum();
    let explicit_volume = numeric(first(
        selected,
        &["volumeKg", "volume", "totalVolume", "capacity"],
    ));
    Some(Workout {
        source: "xunji",
        date: workout_date(first(
            selected,
            &[
                "date",
                "datestr",
                "occurredAt",
                "startTime",
                "trainingDate",
                "createdAt",
            ],
        )),
        title,
        duration_minutes: duration.max(0.0).round() as u32,
        calories_kcal: calories.max(0.0),
        volume_kg: if explicit_volume > 0.0 {
            explicit_volume
        } else {
            computed_volume
        },
        exercises,
    })
}

fn balanced_json(text: &str, start: usize) -> Option<&str> {
    let bytes = text.as_bytes();
    let opening = *bytes.get(start)?;
    let closing = if opening == b'{' { b'}' } else { b']' };
    let mut depth = 0_i32;
    let mut quoted = false;
    let mut escaped = false;
    for index in start..bytes.len() {
        let byte = bytes[index];
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else if byte == b'"' {
            quoted = true;
        } else if byte == opening {
            depth += 1;
        } else if byte == closing {
            depth -= 1;
            if depth == 0 {
                return text.get(start..=index);
            }
        }
    }
    None
}

fn parse_page(html: &str) -> Option<(Workout, Value, &'static str)> {
    let mut candidates = Vec::new();
    if html.contains("window.Train") {
        let movement_pattern =
            Regex::new(r#"(?s)movement\s*:\s*JSON\.parse\('((?:\\.|[^'\\])*)'\)"#).ok();
        if let Some(raw) = movement_pattern
            .as_ref()
            .and_then(|pattern| pattern.captures(html))
            .and_then(|captures| captures.get(1))
            .map(|value| value.as_str().replace("\\'", "'"))
        {
            let wrapped = format!("\"{raw}\"");
            if let Ok(decoded) = serde_json::from_str::<String>(&wrapped) {
                if let Ok(movements) = serde_json::from_str::<Value>(&decoded) {
                    let property = |name: &str| {
                        Regex::new(&format!(
                            r#"(?s){}\s*:\s*"((?:\\.|[^"\\])*)""#,
                            regex::escape(name)
                        ))
                        .ok()
                        .and_then(|pattern| pattern.captures(html))
                        .and_then(|captures| captures.get(1).map(|value| value.as_str().to_owned()))
                    };
                    candidates.push(json!({
                        "movement": movements,
                        "title": property("title"),
                        "datestr": property("datestr")
                    }));
                }
            }
        }
    }
    if let Ok(value) = serde_json::from_str::<Value>(html) {
        candidates.push(value);
    }
    let document = Html::parse_document(html);
    let selector = Selector::parse("script").expect("script selector");
    for script in document.select(&selector) {
        let body = script.text().collect::<String>();
        let kind = script.value().attr("type").unwrap_or_default();
        if script.value().attr("id") == Some("__NEXT_DATA__")
            || matches!(kind, "application/json" | "application/ld+json")
        {
            if let Ok(value) = serde_json::from_str::<Value>(&body) {
                candidates.push(value);
            }
        }
    }
    for marker in [
        "window.__INITIAL_STATE__",
        "window.INITIAL_STATE",
        "__NEXT_DATA__",
    ] {
        let Some(position) = html.find(marker) else {
            continue;
        };
        let start = [html[position..].find('{'), html[position..].find('[')]
            .into_iter()
            .flatten()
            .map(|offset| offset + position)
            .min();
        if let Some(block) = start.and_then(|start| balanced_json(html, start)) {
            if let Ok(value) = serde_json::from_str::<Value>(block) {
                candidates.push(value);
            }
        }
    }
    for candidate in candidates {
        if let Some(workout) = normalize_workout(&candidate) {
            return Some((workout, candidate, "embedded-json"));
        }
    }
    None
}

fn put_entity(connection: &Connection, table: &str, value: &Value) -> Result<(), String> {
    match table {
        "workout_import_records" => workouts::save_import(connection, value),
        "workout_history" => workouts::save_workout(connection, value),
        "training_notes" => workouts::save_training_note(connection, value),
        "activities" => habits::save_activity(connection, value),
        "activity_logs" => habits::save_activity_log(connection, value),
        _ => Err(format!("未知数据表: {table}")),
    }
}

fn read_entities(connection: &Connection, table: &str) -> Result<Vec<Value>, String> {
    match table {
        "workout_import_records" => workouts::list_imports(connection),
        "activities" => habits::list_activities(connection),
        _ => Err(format!("未知数据表: {table}")),
    }
}

pub async fn parse(State(state): State<AppState>, mut multipart: Multipart) -> Response {
    let mut image = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("image") {
            image = field.bytes().await.ok();
            break;
        }
    }
    let Some(image) = image else {
        return failure(StatusCode::BAD_REQUEST, "请选择训练分享图片");
    };
    let share_url = match tokio::task::spawn_blocking(move || decode_qr(&image)).await {
        Ok(Ok(value)) => value,
        Ok(Err(message)) => return failure(StatusCode::UNPROCESSABLE_ENTITY, message),
        Err(_) => {
            return failure(
                StatusCode::INTERNAL_SERVER_ERROR,
                "训练图片解析任务意外中止，请重新上传",
            )
        }
    };
    let (final_url, html) = match fetch_page(share_url).await {
        Ok(value) => value,
        Err(message) => return failure(StatusCode::BAD_GATEWAY, message),
    };
    let Some((workout, raw_data, parser)) = parse_page(&html) else {
        return failure(
            StatusCode::UNPROCESSABLE_ENTITY,
            "二维码已识别，但分享页面中没有可导入的结构化训练数据",
        );
    };
    let stamp = stamp();
    let record = json!({
        "id": identifier(),
        "userId": "local-user",
        "source": "xunji",
        "shareUrl": final_url.as_str(),
        "rawData": raw_data,
        "workout": workout,
        "status": "pending",
        "createdAt": stamp,
        "updatedAt": stamp
    });
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return failure(StatusCode::INTERNAL_SERVER_ERROR, "SQLite 锁已损坏"),
    };
    if let Err(message) = put_entity(&connection, "workout_import_records", &record) {
        return failure(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    Json(json!({
        "importId": record["id"],
        "shareUrl": final_url.as_str(),
        "parser": parser,
        "workout": workout
    }))
    .into_response()
}

pub async fn list(State(state): State<AppState>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return failure(StatusCode::INTERNAL_SERVER_ERROR, "SQLite 锁已损坏"),
    };
    match read_entities(&connection, "workout_import_records") {
        Ok(items) => Json(json!({ "items": items })).into_response(),
        Err(message) => failure(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

pub async fn update(State(state): State<AppState>, Json(body): Json<ImportAction>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return failure(StatusCode::INTERNAL_SERVER_ERROR, "SQLite 锁已损坏"),
    };
    let mut record = match workouts::get_import(&connection, &body.import_id) {
        Ok(Some(value)) => value,
        Ok(None) => return failure(StatusCode::NOT_FOUND, "导入记录不存在"),
        Err(message) => return failure(StatusCode::INTERNAL_SERVER_ERROR, message),
    };
    if body.action == "cancel" {
        record["status"] = json!("failed");
        record["error"] = json!("用户取消导入");
        record["updatedAt"] = json!(stamp());
        return match put_entity(&connection, "workout_import_records", &record) {
            Ok(()) => Json(record).into_response(),
            Err(message) => failure(StatusCode::INTERNAL_SERVER_ERROR, message),
        };
    }
    if body.action != "confirm" {
        return failure(StatusCode::BAD_REQUEST, "不支持的操作");
    }
    let workout = body.workout.unwrap_or_else(|| record["workout"].clone());
    let exercises = workout
        .get("exercises")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if exercises.is_empty() {
        return failure(StatusCode::BAD_REQUEST, "训练至少需要一个动作");
    }
    let workout_date = workout
        .get("date")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let workout_title = workout
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("训练记录");
    let duration = workout
        .get("durationMinutes")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let source_id = record
        .get("shareUrl")
        .and_then(Value::as_str)
        .and_then(|value| Url::parse(value).ok())
        .and_then(|url| {
            url.query_pairs()
                .find(|(key, _)| key == "localid" || key == "spid")
                .map(|(_, value)| value.into_owned())
        })
        .unwrap_or_else(|| body.import_id.clone());
    let history_id = format!("xunji-{source_id}");
    let occurred_at = format!("{workout_date}T12:00:00+08:00");
    if let Ok(Some(existing)) = workouts::get_workout(&connection, &history_id) {
        record["workout"] = workout;
        record["status"] = json!("success");
        record["workoutRecordId"] = json!(history_id);
        record["updatedAt"] = json!(stamp());
        if let Err(message) = put_entity(&connection, "workout_import_records", &record) {
            return failure(StatusCode::INTERNAL_SERVER_ERROR, message);
        }
        return Json(json!({
            "workoutRecord": existing,
            "importRecord": record,
            "duplicate": true
        }))
        .into_response();
    }
    let history = json!({
        "id": history_id,
        "userId": "local-user",
        "templateId": "",
        "name": workout_title,
        "occurredAt": occurred_at,
        "durationSeconds": duration * 60,
        "exerciseCount": exercises.len(),
        "setCount": exercises.iter().map(|exercise| exercise.get("sets").and_then(Value::as_array).map(Vec::len).unwrap_or_default()).sum::<usize>(),
        "status": "completed",
        "source": "xunji",
        "sourceId": source_id,
        "caloriesKcal": workout.get("caloriesKcal").cloned().unwrap_or_else(|| json!(0)),
        "volumeKg": workout.get("volumeKg").cloned().unwrap_or_else(|| json!(0)),
        "exercises": exercises.iter().map(|exercise| {
            let sets = exercise.get("sets").and_then(Value::as_array).cloned().unwrap_or_default();
            json!({
                "name": exercise.get("name").cloned().unwrap_or_else(|| json!("动作")),
                "plannedSets": sets.len(),
                "completedSets": sets.len(),
                "sets": sets.iter().map(|set| json!({
                    "weight": set.get("weightKg").cloned().unwrap_or_else(|| json!(0)),
                    "reps": set.get("reps").cloned().unwrap_or_else(|| json!(0)),
                    "completed": true
                })).collect::<Vec<_>>()
            })
        }).collect::<Vec<_>>(),
        "createdAt": stamp(),
        "updatedAt": stamp()
    });
    if let Err(message) = put_entity(&connection, "workout_history", &history) {
        return failure(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    let mut fitness_activities = read_entities(&connection, "activities")
        .unwrap_or_default()
        .into_iter()
        .filter(|activity| {
            !activity
                .get("isArchived")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && activity.get("checkinMethod").and_then(Value::as_str) == Some("automatic")
                && activity.get("syncSource").and_then(Value::as_str) == Some("fitness")
        })
        .collect::<Vec<_>>();
    if fitness_activities.is_empty() {
        let activity = json!({
            "id": "system-fitness-training", "userId": "local-user", "name": "健身训练",
            "type": "count", "unit": "次", "normalTarget": 4, "targetPeriod": "weekly",
            "targetDays": [1,3,5], "scheduleType": "weekly", "startDate": workout_date,
            "checkinMethod": "automatic", "syncSource": "fitness", "icon": "fitness",
            "color": "emerald", "description": "由训练记录自动完成打卡。",
            "isArchived": false, "createdAt": stamp(), "updatedAt": stamp()
        });
        if let Err(message) = put_entity(&connection, "activities", &activity) {
            return failure(StatusCode::INTERNAL_SERVER_ERROR, message);
        }
        fitness_activities.push(activity);
    }
    let set_count = history
        .get("setCount")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    for activity in fitness_activities {
        let activity_id = activity
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("system-fitness-training");
        let log = json!({
            "id": format!("workout-log-{history_id}-{activity_id}"),
            "userId": "local-user", "activityId": activity_id, "value": 1,
            "status": "completed",
            "note": format!("完成「{workout_title}」· {duration} 分钟 · {set_count} 组 · 训练同步"),
            "createdAt": occurred_at, "updatedAt": stamp()
        });
        if let Err(message) = put_entity(&connection, "activity_logs", &log) {
            return failure(StatusCode::INTERNAL_SERVER_ERROR, message);
        }
    }
    let exercise_names = exercises
        .iter()
        .filter_map(|exercise| exercise.get("name").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("、");
    let training_note = json!({
        "id": format!("training-note-{history_id}"), "userId": "local-user",
        "title": format!("{workout_date} 训练记录"),
        "content": format!(
            "训练：{workout_title}\n日期：{workout_date}\n时长：{duration} 分钟\n动作：{exercise_names}\n来源：寻迹同步"
        ),
        "workoutRecordId": history_id, "source": "xunji", "noteDate": workout_date,
        "createdAt": occurred_at, "updatedAt": stamp()
    });
    if let Err(message) = put_entity(&connection, "training_notes", &training_note) {
        return failure(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    record["workout"] = workout;
    record["status"] = json!("success");
    record["workoutRecordId"] = json!(history_id);
    record["updatedAt"] = json!(stamp());
    if let Err(message) = put_entity(&connection, "workout_import_records", &record) {
        return failure(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    Json(json!({
        "workoutRecord": history,
        "importRecord": record,
        "duplicate": false
    }))
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::parse_page;

    #[test]
    fn parses_legacy_window_train_payload() {
        let html = r#"<html><script>
          window.Train={
            movement:JSON.parse('[{\"label\":\"Squat\",\"sets\":[{\"done\":true,\"reps\":\"5\",\"weight\":\"90\"}]}]'),
            title:"Leg Day",
            datestr:"2026-07-24"
          };
        </script></html>"#;
        let (workout, _, parser) = parse_page(html).expect("legacy workout should parse");
        assert_eq!(parser, "embedded-json");
        assert_eq!(workout.title, "Leg Day");
        assert_eq!(workout.date, "2026-07-24");
        assert_eq!(workout.exercises[0].name, "Squat");
        assert_eq!(workout.volume_kg, 450.0);
    }

    #[test]
    fn excludes_unfinished_xunji_sets_from_imported_workout() {
        let html = r#"<html><script>
          window.Train={
            movement:JSON.parse('[{\"label\":\"杠铃卧推\",\"sets\":[{\"done\":true,\"reps\":\"12\",\"weight\":\"60\"},{\"done\":true,\"reps\":\"4\",\"weight\":\"60\"},{\"done\":true,\"reps\":\"4\",\"weight\":\"60\"},{\"done\":true,\"reps\":\"4\",\"weight\":\"60\"}]},{\"label\":\"上斜杠铃卧推\",\"sets\":[{\"done\":true,\"reps\":\"12\",\"weight\":\"45\"},{\"done\":true,\"reps\":\"12\",\"weight\":\"45\"},{\"done\":true,\"reps\":\"12\",\"weight\":\"45\"},{\"done\":true,\"reps\":\"12\",\"weight\":\"45\"}]},{\"label\":\"蝴蝶机夹胸（版本2）\",\"sets\":[{\"done\":true,\"reps\":\"12\",\"weight\":\"50\"},{\"done\":true,\"reps\":\"12\",\"weight\":\"50\"},{\"done\":true,\"reps\":\"12\",\"weight\":\"50\"},{\"done\":true,\"reps\":\"12\",\"weight\":\"50\"}]},{\"label\":\"绳索臂屈伸\",\"sets\":[{\"done\":true,\"reps\":\"12\",\"weight\":\"40\"},{\"done\":true,\"reps\":\"12\",\"weight\":\"50\"},{\"done\":true,\"reps\":\"12\",\"weight\":\"50\"},{\"done\":true,\"reps\":\"12\",\"weight\":\"40\"}]},{\"label\":\"绳索过头臂屈伸\",\"sets\":[{\"done\":false,\"reps\":\"12\",\"weight\":\"25\"}]},{\"label\":\"绳索十字夹胸\",\"sets\":[{\"done\":false,\"reps\":\"12\",\"weight\":\"50\"}]},{\"label\":\"哑铃臂屈伸\",\"sets\":[{\"done\":false,\"reps\":\"12\",\"weight\":\"30\"}]}]'),
            title:"胸+三头",
            datestr:"2026-08-25"
          };
        </script></html>"#;
        let (workout, _, _) = parse_page(html).expect("completed workout should parse");
        assert_eq!(workout.exercises.len(), 4);
        assert_eq!(workout.volume_kg, 8_160.0);
        assert_eq!(workout.exercises[3].name, "绳索臂屈伸");
    }
}
