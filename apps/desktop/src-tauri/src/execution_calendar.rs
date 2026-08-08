use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::{
    database::{
        profile,
        repositories::{
            execution as task_repository,
            execution_calendar::{
                self as repository, CalendarEventRecord, CalendarEventWrite,
                CalendarOccurrenceRecord, CalendarOccurrenceWrite,
            },
            execution_structure::{self as recurrence_repository, RecurrenceRuleRecord, RecurrenceRuleWrite},
        },
    },
    execution::{self, ExecutionError, ExecutionErrorKind, ExecutionResult},
    execution_structure::RecurrenceRuleInput,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEventInput {
    pub title: String,
    pub description: Option<String>,
    pub is_all_day: bool,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub start_local_date: Option<String>,
    pub end_local_date: Option<String>,
    pub timezone: Option<String>,
    pub source_task_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CalendarQuery {
    pub timed_start: Option<String>,
    pub timed_end: Option<String>,
    pub local_start_date: Option<String>,
    pub local_end_date: Option<String>,
    pub include_cancelled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarTimingInput {
    pub is_all_day: bool,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub start_local_date: Option<String>,
    pub end_local_date: Option<String>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarConflictInput {
    pub timing: CalendarTimingInput,
    pub exclude_event_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CalendarConflict {
    pub event_id: String,
    pub occurrence_id: Option<String>,
    pub title: String,
    pub is_all_day: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleTaskInput {
    pub timing: CalendarTimingInput,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarOccurrenceInput {
    pub occurrence_key: String,
    pub timing: CalendarTimingInput,
    pub title_override: Option<String>,
    pub description_override: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarOccurrenceStatusInput {
    pub status: String,
}

#[derive(Debug, Clone)]
struct ValidatedTiming {
    is_all_day: bool,
    start_at: Option<String>,
    end_at: Option<String>,
    start_local_date: Option<String>,
    end_local_date: Option<String>,
    timezone: Option<String>,
}

fn error(kind: ExecutionErrorKind, message: impl Into<String>) -> ExecutionError {
    ExecutionError {
        kind,
        message: message.into(),
    }
}

fn storage(message: impl Into<String>) -> ExecutionError {
    error(ExecutionErrorKind::Storage, message)
}

fn validation(message: impl Into<String>) -> ExecutionError {
    error(ExecutionErrorKind::Validation, message)
}

fn not_found(message: impl Into<String>) -> ExecutionError {
    error(ExecutionErrorKind::NotFound, message)
}

fn conflict(message: impl Into<String>) -> ExecutionError {
    error(ExecutionErrorKind::Conflict, message)
}

fn active_user(connection: &Connection) -> ExecutionResult<String> {
    profile::active_profile_id(connection).map_err(storage)
}

fn clean_required(value: &str, label: &str, max: usize) -> ExecutionResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(validation(format!("{label}不能为空")));
    }
    if value.chars().count() > max {
        return Err(validation(format!("{label}不能超过 {max} 个字符")));
    }
    Ok(value.to_owned())
}

fn clean_optional(value: Option<String>, label: &str, max: usize) -> ExecutionResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max {
        return Err(validation(format!("{label}不能超过 {max} 个字符")));
    }
    Ok(Some(value.to_owned()))
}

fn parse_timestamp(value: &str, label: &str) -> ExecutionResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| validation(format!("{label}必须是 RFC3339 时间")))
}

fn parse_date(value: &str, label: &str) -> ExecutionResult<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| validation(format!("{label}必须是 YYYY-MM-DD")))
}

fn validate_timing(input: CalendarTimingInput) -> ExecutionResult<ValidatedTiming> {
    let timezone = clean_optional(input.timezone, "时区", 128)?;
    if input.is_all_day {
        if input.start_at.is_some() || input.end_at.is_some() {
            return Err(validation("全天事件不能同时设置 startAt/endAt"));
        }
        let start_text = input
            .start_local_date
            .ok_or_else(|| validation("全天事件必须设置 startLocalDate"))?;
        let end_text = input
            .end_local_date
            .ok_or_else(|| validation("全天事件必须设置 endLocalDate"))?;
        let start = parse_date(&start_text, "startLocalDate")?;
        let end = parse_date(&end_text, "endLocalDate")?;
        if end < start {
            return Err(validation("全天事件结束日期不能早于开始日期"));
        }
        Ok(ValidatedTiming {
            is_all_day: true,
            start_at: None,
            end_at: None,
            start_local_date: Some(start_text),
            end_local_date: Some(end_text),
            timezone,
        })
    } else {
        if input.start_local_date.is_some() || input.end_local_date.is_some() {
            return Err(validation("定时事件不能同时设置本地全天日期"));
        }
        let start_text = input
            .start_at
            .ok_or_else(|| validation("定时事件必须设置 startAt"))?;
        let end_text = input
            .end_at
            .ok_or_else(|| validation("定时事件必须设置 endAt"))?;
        let start = parse_timestamp(&start_text, "startAt")?;
        let end = parse_timestamp(&end_text, "endAt")?;
        if end <= start {
            return Err(validation("定时事件结束时间必须晚于开始时间"));
        }
        Ok(ValidatedTiming {
            is_all_day: false,
            start_at: Some(start_text),
            end_at: Some(end_text),
            start_local_date: None,
            end_local_date: None,
            timezone,
        })
    }
}

fn event_timing(event: &CalendarEventRecord) -> CalendarTimingInput {
    CalendarTimingInput {
        is_all_day: event.is_all_day,
        start_at: event.start_at.clone(),
        end_at: event.end_at.clone(),
        start_local_date: event.start_local_date.clone(),
        end_local_date: event.end_local_date.clone(),
        timezone: event.timezone.clone(),
    }
}

fn occurrence_timing(occurrence: &CalendarOccurrenceRecord) -> CalendarTimingInput {
    CalendarTimingInput {
        is_all_day: occurrence.is_all_day,
        start_at: occurrence.start_at.clone(),
        end_at: occurrence.end_at.clone(),
        start_local_date: occurrence.start_local_date.clone(),
        end_local_date: occurrence.end_local_date.clone(),
        timezone: None,
    }
}

fn timing_overlaps(a: &ValidatedTiming, b: &ValidatedTiming) -> bool {
    if a.is_all_day != b.is_all_day {
        return false;
    }
    if a.is_all_day {
        let a_start = a.start_local_date.as_deref().and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok());
        let a_end = a.end_local_date.as_deref().and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok());
        let b_start = b.start_local_date.as_deref().and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok());
        let b_end = b.end_local_date.as_deref().and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok());
        matches!((a_start, a_end, b_start, b_end), (Some(a_start), Some(a_end), Some(b_start), Some(b_end)) if a_start <= b_end && b_start <= a_end)
    } else {
        let parse = |value: Option<&str>| {
            value.and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.with_timezone(&Utc))
        };
        let a_start = parse(a.start_at.as_deref());
        let a_end = parse(a.end_at.as_deref());
        let b_start = parse(b.start_at.as_deref());
        let b_end = parse(b.end_at.as_deref());
        matches!((a_start, a_end, b_start, b_end), (Some(a_start), Some(a_end), Some(b_start), Some(b_end)) if a_start < b_end && b_start < a_end)
    }
}

fn normalize_event(
    user_id: String,
    id: Option<String>,
    input: CalendarEventInput,
    current: Option<&CalendarEventRecord>,
) -> ExecutionResult<CalendarEventWrite> {
    let timing = validate_timing(CalendarTimingInput {
        is_all_day: input.is_all_day,
        start_at: input.start_at,
        end_at: input.end_at,
        start_local_date: input.start_local_date,
        end_local_date: input.end_local_date,
        timezone: input.timezone,
    })?;
    Ok(CalendarEventWrite {
        id,
        user_id,
        title: clean_required(&input.title, "事件标题", 240)?,
        description: clean_optional(input.description, "事件描述", 20_000)?,
        is_all_day: timing.is_all_day,
        start_at: timing.start_at,
        end_at: timing.end_at,
        start_local_date: timing.start_local_date,
        end_local_date: timing.end_local_date,
        timezone: timing.timezone,
        status: current
            .map(|event| event.status.clone())
            .unwrap_or_else(|| "scheduled".to_owned()),
        recurrence_rule_id: current.and_then(|event| event.recurrence_rule_id.clone()),
        source_task_id: clean_optional(input.source_task_id, "来源任务 ID", 128)?,
    })
}

fn ensure_source_task(
    connection: &Connection,
    user_id: &str,
    task_id: Option<&str>,
) -> ExecutionResult<()> {
    let Some(task_id) = task_id else {
        return Ok(());
    };
    if task_repository::get_task(connection, user_id, task_id)
        .map_err(storage)?
        .is_none()
    {
        return Err(validation("来源任务不存在或不属于当前资料"));
    }
    Ok(())
}

fn matches_query(event: &CalendarEventRecord, query: &CalendarQuery) -> ExecutionResult<bool> {
    if event.is_all_day {
        let Some(query_start_text) = query.local_start_date.as_deref() else {
            return Ok(query.local_end_date.is_none());
        };
        let query_end_text = query.local_end_date.as_deref().unwrap_or(query_start_text);
        let query_start = parse_date(query_start_text, "localStartDate")?;
        let query_end = parse_date(query_end_text, "localEndDate")?;
        if query_end < query_start {
            return Err(validation("localEndDate 不能早于 localStartDate"));
        }
        let event_start = event.start_local_date.as_deref().ok_or_else(|| storage("全天事件缺少 start_local_date"))?;
        let event_end = event.end_local_date.as_deref().ok_or_else(|| storage("全天事件缺少 end_local_date"))?;
        let event_start = parse_date(event_start, "event.startLocalDate")?;
        let event_end = parse_date(event_end, "event.endLocalDate")?;
        Ok(event_start <= query_end && query_start <= event_end)
    } else {
        let Some(query_start_text) = query.timed_start.as_deref() else {
            return Ok(query.timed_end.is_none());
        };
        let query_end_text = query.timed_end.as_deref().unwrap_or(query_start_text);
        let query_start = parse_timestamp(query_start_text, "timedStart")?;
        let query_end = parse_timestamp(query_end_text, "timedEnd")?;
        if query_end < query_start {
            return Err(validation("timedEnd 不能早于 timedStart"));
        }
        let event_start = event.start_at.as_deref().ok_or_else(|| storage("定时事件缺少 start_at"))?;
        let event_end = event.end_at.as_deref().ok_or_else(|| storage("定时事件缺少 end_at"))?;
        let event_start = parse_timestamp(event_start, "event.startAt")?;
        let event_end = parse_timestamp(event_end, "event.endAt")?;
        Ok(event_start < query_end && query_start < event_end)
    }
}

pub fn list_events(connection: &Connection, query: CalendarQuery) -> ExecutionResult<Vec<CalendarEventRecord>> {
    let user_id = active_user(connection)?;
    let events = repository::list_events(
        connection,
        &user_id,
        query.include_cancelled.unwrap_or(false),
    )
    .map_err(storage)?;
    events
        .into_iter()
        .filter_map(|event| match matches_query(&event, &query) {
            Ok(true) => Some(Ok(event)),
            Ok(false) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

pub fn get_event(connection: &Connection, id: &str) -> ExecutionResult<CalendarEventRecord> {
    let user_id = active_user(connection)?;
    repository::get_event(connection, &user_id, id)
        .map_err(storage)?
        .ok_or_else(|| not_found("日历事件不存在"))
}

pub fn create_event(
    connection: &Connection,
    input: CalendarEventInput,
) -> ExecutionResult<CalendarEventRecord> {
    let user_id = active_user(connection)?;
    let write = normalize_event(user_id.clone(), None, input, None)?;
    ensure_source_task(connection, &user_id, write.source_task_id.as_deref())?;
    repository::save_event(connection, &write).map_err(storage)
}

pub fn update_event(
    connection: &Connection,
    id: &str,
    input: CalendarEventInput,
) -> ExecutionResult<CalendarEventRecord> {
    let user_id = active_user(connection)?;
    let current = repository::get_event(connection, &user_id, id)
        .map_err(storage)?
        .ok_or_else(|| not_found("日历事件不存在"))?;
    if current.status == "cancelled" {
        return Err(conflict("已取消的事件不能直接编辑"));
    }
    let write = normalize_event(user_id.clone(), Some(id.to_owned()), input, Some(&current))?;
    ensure_source_task(connection, &user_id, write.source_task_id.as_deref())?;
    repository::save_event(connection, &write).map_err(storage)
}

pub fn move_event(
    connection: &Connection,
    id: &str,
    input: CalendarTimingInput,
) -> ExecutionResult<CalendarEventRecord> {
    let user_id = active_user(connection)?;
    let current = repository::get_event(connection, &user_id, id)
        .map_err(storage)?
        .ok_or_else(|| not_found("日历事件不存在"))?;
    if current.status == "cancelled" {
        return Err(conflict("已取消的事件不能移动"));
    }
    let timing = validate_timing(input)?;
    let write = CalendarEventWrite {
        id: Some(current.id),
        user_id,
        title: current.title,
        description: current.description,
        is_all_day: timing.is_all_day,
        start_at: timing.start_at,
        end_at: timing.end_at,
        start_local_date: timing.start_local_date,
        end_local_date: timing.end_local_date,
        timezone: timing.timezone,
        status: current.status,
        recurrence_rule_id: current.recurrence_rule_id,
        source_task_id: current.source_task_id,
    };
    repository::save_event(connection, &write).map_err(storage)
}

pub fn cancel_event(connection: &Connection, id: &str) -> ExecutionResult<CalendarEventRecord> {
    let user_id = active_user(connection)?;
    let current = repository::get_event(connection, &user_id, id)
        .map_err(storage)?
        .ok_or_else(|| not_found("日历事件不存在"))?;
    if current.status == "cancelled" {
        return Ok(current);
    }
    let write = CalendarEventWrite {
        id: Some(current.id),
        user_id,
        title: current.title,
        description: current.description,
        is_all_day: current.is_all_day,
        start_at: current.start_at,
        end_at: current.end_at,
        start_local_date: current.start_local_date,
        end_local_date: current.end_local_date,
        timezone: current.timezone,
        status: "cancelled".to_owned(),
        recurrence_rule_id: current.recurrence_rule_id,
        source_task_id: current.source_task_id,
    };
    repository::save_event(connection, &write).map_err(storage)
}

pub fn delete_event(connection: &Connection, id: &str) -> ExecutionResult<()> {
    let user_id = active_user(connection)?;
    if repository::get_event(connection, &user_id, id)
        .map_err(storage)?
        .is_none()
    {
        return Err(not_found("日历事件不存在"));
    }
    if repository::soft_delete_event(connection, &user_id, id).map_err(storage)? {
        Ok(())
    } else {
        Err(not_found("日历事件不存在"))
    }
}

pub fn find_conflicts(
    connection: &Connection,
    input: CalendarConflictInput,
) -> ExecutionResult<Vec<CalendarConflict>> {
    let user_id = active_user(connection)?;
    let candidate = validate_timing(input.timing)?;
    let events = repository::list_events(connection, &user_id, false).map_err(storage)?;
    let mut conflicts = Vec::new();
    for event in events {
        if input.exclude_event_id.as_deref() == Some(event.id.as_str()) {
            continue;
        }
        let timing = validate_timing(event_timing(&event))?;
        if timing_overlaps(&candidate, &timing) {
            conflicts.push(CalendarConflict {
                event_id: event.id.clone(),
                occurrence_id: None,
                title: event.title.clone(),
                is_all_day: event.is_all_day,
            });
        }
        for occurrence in repository::list_occurrences(connection, &user_id, &event.id).map_err(storage)? {
            if occurrence.status != "scheduled" {
                continue;
            }
            let occurrence_timing = validate_timing(occurrence_timing(&occurrence))?;
            if timing_overlaps(&candidate, &occurrence_timing) {
                conflicts.push(CalendarConflict {
                    event_id: event.id.clone(),
                    occurrence_id: Some(occurrence.id),
                    title: occurrence.title_override.unwrap_or_else(|| event.title.clone()),
                    is_all_day: occurrence.is_all_day,
                });
            }
        }
    }
    Ok(conflicts)
}

pub fn schedule_task(
    connection: &Connection,
    task_id: &str,
    input: ScheduleTaskInput,
) -> ExecutionResult<CalendarEventRecord> {
    let user_id = active_user(connection)?;
    let task = execution::get_task(connection, task_id)?;
    let timing = validate_timing(input.timing)?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| storage(error.to_string()))?;
    let event = if let Some(existing) =
        repository::find_scheduled_event_by_source_task(&transaction, &user_id, task_id)
            .map_err(storage)?
    {
        let write = CalendarEventWrite {
            id: Some(existing.id),
            user_id: user_id.clone(),
            title: task.title.clone(),
            description: task.description.clone(),
            is_all_day: timing.is_all_day,
            start_at: timing.start_at,
            end_at: timing.end_at,
            start_local_date: timing.start_local_date,
            end_local_date: timing.end_local_date,
            timezone: timing.timezone,
            status: "scheduled".to_owned(),
            recurrence_rule_id: existing.recurrence_rule_id,
            source_task_id: Some(task_id.to_owned()),
        };
        repository::save_event(&transaction, &write).map_err(storage)?
    } else {
        let write = CalendarEventWrite {
            id: None,
            user_id: user_id.clone(),
            title: task.title.clone(),
            description: task.description.clone(),
            is_all_day: timing.is_all_day,
            start_at: timing.start_at,
            end_at: timing.end_at,
            start_local_date: timing.start_local_date,
            end_local_date: timing.end_local_date,
            timezone: timing.timezone,
            status: "scheduled".to_owned(),
            recurrence_rule_id: None,
            source_task_id: Some(task_id.to_owned()),
        };
        repository::save_event(&transaction, &write).map_err(storage)?
    };
    repository::create_task_schedule_link(&transaction, &user_id, task_id, &event.id).map_err(storage)?;
    transaction
        .commit()
        .map_err(|error| storage(error.to_string()))?;
    Ok(event)
}

fn normalize_recurrence(
    user_id: String,
    id: Option<String>,
    mut input: RecurrenceRuleInput,
) -> ExecutionResult<RecurrenceRuleWrite> {
    if !matches!(input.frequency.as_str(), "daily" | "weekly" | "monthly") {
        return Err(validation("重复频率必须是 daily、weekly 或 monthly"));
    }
    let interval_value = input.interval_value.unwrap_or(1);
    if interval_value < 1 {
        return Err(validation("重复间隔必须大于等于 1"));
    }
    input.weekdays.sort_unstable();
    input.weekdays.dedup();
    if input.weekdays.iter().any(|day| !(1..=7).contains(day)) {
        return Err(validation("星期值必须位于 1..=7"));
    }
    match input.frequency.as_str() {
        "daily" if !input.weekdays.is_empty() || input.month_day.is_some() => {
            return Err(validation("daily 规则不能设置 weekdays 或 monthDay"));
        }
        "weekly" if input.weekdays.is_empty() || input.month_day.is_some() => {
            return Err(validation("weekly 规则必须设置 weekdays，且不能设置 monthDay"));
        }
        "monthly" if !input.weekdays.is_empty() || !matches!(input.month_day, Some(1..=31)) => {
            return Err(validation("monthly 规则必须设置 1..=31 的 monthDay，且不能设置 weekdays"));
        }
        _ => {}
    }
    if let Some(max_occurrences) = input.max_occurrences {
        if max_occurrences < 1 {
            return Err(validation("maxOccurrences 必须大于等于 1"));
        }
    }
    if let Some(until_at) = input.until_at.as_deref() {
        if NaiveDate::parse_from_str(until_at, "%Y-%m-%d").is_err()
            && DateTime::parse_from_rfc3339(until_at).is_err()
        {
            return Err(validation("untilAt 必须是 YYYY-MM-DD 或 RFC3339 时间"));
        }
    }
    Ok(RecurrenceRuleWrite {
        id,
        user_id,
        frequency: input.frequency,
        interval_value,
        weekdays: input.weekdays,
        month_day: input.month_day,
        timezone: clean_optional(input.timezone, "重复规则时区", 128)?,
        until_at: input.until_at,
        max_occurrences: input.max_occurrences,
    })
}

pub fn get_event_recurrence(
    connection: &Connection,
    event_id: &str,
) -> ExecutionResult<Option<RecurrenceRuleRecord>> {
    let user_id = active_user(connection)?;
    let event = repository::get_event(connection, &user_id, event_id)
        .map_err(storage)?
        .ok_or_else(|| not_found("日历事件不存在"))?;
    let Some(rule_id) = event.recurrence_rule_id else {
        return Ok(None);
    };
    recurrence_repository::get_recurrence_rule(connection, &user_id, &rule_id).map_err(storage)
}

pub fn set_event_recurrence(
    connection: &Connection,
    event_id: &str,
    input: RecurrenceRuleInput,
) -> ExecutionResult<RecurrenceRuleRecord> {
    let user_id = active_user(connection)?;
    let event = repository::get_event(connection, &user_id, event_id)
        .map_err(storage)?
        .ok_or_else(|| not_found("日历事件不存在"))?;
    if event.status == "cancelled" {
        return Err(conflict("已取消的事件不能设置重复规则"));
    }
    let write = normalize_recurrence(user_id.clone(), event.recurrence_rule_id, input)?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| storage(error.to_string()))?;
    let rule = recurrence_repository::save_recurrence_rule(&transaction, &write).map_err(storage)?;
    repository::set_event_recurrence_rule(&transaction, &user_id, event_id, Some(&rule.id))
        .map_err(storage)?;
    transaction
        .commit()
        .map_err(|error| storage(error.to_string()))?;
    Ok(rule)
}

pub fn clear_event_recurrence(connection: &Connection, event_id: &str) -> ExecutionResult<()> {
    let user_id = active_user(connection)?;
    let event = repository::get_event(connection, &user_id, event_id)
        .map_err(storage)?
        .ok_or_else(|| not_found("日历事件不存在"))?;
    let Some(rule_id) = event.recurrence_rule_id else {
        return Ok(());
    };
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| storage(error.to_string()))?;
    repository::set_event_recurrence_rule(&transaction, &user_id, event_id, None).map_err(storage)?;
    recurrence_repository::soft_delete_recurrence_rule(&transaction, &user_id, &rule_id)
        .map_err(storage)?;
    transaction
        .commit()
        .map_err(|error| storage(error.to_string()))?;
    Ok(())
}

pub fn list_occurrences(
    connection: &Connection,
    event_id: &str,
) -> ExecutionResult<Vec<CalendarOccurrenceRecord>> {
    let user_id = active_user(connection)?;
    get_event(connection, event_id)?;
    repository::list_occurrences(connection, &user_id, event_id).map_err(storage)
}

pub fn materialize_occurrence(
    connection: &Connection,
    event_id: &str,
    input: CalendarOccurrenceInput,
) -> ExecutionResult<CalendarOccurrenceRecord> {
    let user_id = active_user(connection)?;
    let event = repository::get_event(connection, &user_id, event_id)
        .map_err(storage)?
        .ok_or_else(|| not_found("日历事件不存在"))?;
    if event.recurrence_rule_id.is_none() {
        return Err(conflict("只有重复事件才能物化 occurrence"));
    }
    let occurrence_key = clean_required(&input.occurrence_key, "occurrenceKey", 160)?;
    let timing = validate_timing(input.timing)?;
    let write = CalendarOccurrenceWrite {
        event_id: event_id.to_owned(),
        occurrence_key,
        is_all_day: timing.is_all_day,
        start_at: timing.start_at,
        end_at: timing.end_at,
        start_local_date: timing.start_local_date,
        end_local_date: timing.end_local_date,
        status: "scheduled".to_owned(),
        title_override: clean_optional(input.title_override, "本次标题", 240)?,
        description_override: clean_optional(input.description_override, "本次描述", 20_000)?,
    };
    repository::create_occurrence(connection, &user_id, &write).map_err(storage)
}

pub fn update_occurrence(
    connection: &Connection,
    event_id: &str,
    occurrence_id: &str,
    input: CalendarOccurrenceInput,
) -> ExecutionResult<CalendarOccurrenceRecord> {
    let user_id = active_user(connection)?;
    get_event(connection, event_id)?;
    let current = repository::get_occurrence(connection, &user_id, event_id, occurrence_id)
        .map_err(storage)?
        .ok_or_else(|| not_found("日历 occurrence 不存在"))?;
    if current.status == "cancelled" {
        return Err(conflict("已取消的 occurrence 不能直接编辑"));
    }
    let timing = validate_timing(input.timing)?;
    let write = CalendarOccurrenceWrite {
        event_id: event_id.to_owned(),
        occurrence_key: current.occurrence_key,
        is_all_day: timing.is_all_day,
        start_at: timing.start_at,
        end_at: timing.end_at,
        start_local_date: timing.start_local_date,
        end_local_date: timing.end_local_date,
        status: current.status,
        title_override: clean_optional(input.title_override, "本次标题", 240)?,
        description_override: clean_optional(input.description_override, "本次描述", 20_000)?,
    };
    repository::update_occurrence(connection, &user_id, occurrence_id, &write).map_err(storage)
}

pub fn change_occurrence_status(
    connection: &Connection,
    event_id: &str,
    occurrence_id: &str,
    input: CalendarOccurrenceStatusInput,
) -> ExecutionResult<CalendarOccurrenceRecord> {
    if !matches!(input.status.as_str(), "scheduled" | "cancelled" | "skipped") {
        return Err(validation("occurrence 状态必须是 scheduled/cancelled/skipped"));
    }
    let user_id = active_user(connection)?;
    get_event(connection, event_id)?;
    let current = repository::get_occurrence(connection, &user_id, event_id, occurrence_id)
        .map_err(storage)?
        .ok_or_else(|| not_found("日历 occurrence 不存在"))?;
    if current.status == input.status {
        return Ok(current);
    }
    if current.status == "cancelled" && input.status != "scheduled" {
        return Err(conflict("cancelled occurrence 只能恢复为 scheduled"));
    }
    let write = CalendarOccurrenceWrite {
        event_id: event_id.to_owned(),
        occurrence_key: current.occurrence_key,
        is_all_day: current.is_all_day,
        start_at: current.start_at,
        end_at: current.end_at,
        start_local_date: current.start_local_date,
        end_local_date: current.end_local_date,
        status: input.status,
        title_override: current.title_override,
        description_override: current.description_override,
    };
    repository::update_occurrence(connection, &user_id, occurrence_id, &write).map_err(storage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migration_runner::{run, MigrationContext};
    use crate::database::migrations::all;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn database() -> Connection {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("lifetrace-calendar-service-{unique}"));
        std::fs::create_dir_all(&data_dir).unwrap();
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(&mut connection, &MigrationContext::new(data_dir), &all()).unwrap();
        connection
    }

    fn timed(title: &str, start: &str, end: &str) -> CalendarEventInput {
        CalendarEventInput {
            title: title.to_owned(),
            description: None,
            is_all_day: false,
            start_at: Some(start.to_owned()),
            end_at: Some(end.to_owned()),
            start_local_date: None,
            end_local_date: None,
            timezone: Some("Asia/Shanghai".to_owned()),
            source_task_id: None,
        }
    }

    #[test]
    fn conflict_detection_allows_adjacent_timed_events_but_rejects_overlap() {
        let connection = database();
        create_event(
            &connection,
            timed("existing", "2026-08-08T09:00:00+08:00", "2026-08-08T10:00:00+08:00"),
        )
        .unwrap();
        let adjacent = find_conflicts(
            &connection,
            CalendarConflictInput {
                timing: CalendarTimingInput {
                    is_all_day: false,
                    start_at: Some("2026-08-08T10:00:00+08:00".to_owned()),
                    end_at: Some("2026-08-08T11:00:00+08:00".to_owned()),
                    start_local_date: None,
                    end_local_date: None,
                    timezone: Some("Asia/Shanghai".to_owned()),
                },
                exclude_event_id: None,
            },
        )
        .unwrap();
        assert!(adjacent.is_empty());
        let overlap = find_conflicts(
            &connection,
            CalendarConflictInput {
                timing: CalendarTimingInput {
                    is_all_day: false,
                    start_at: Some("2026-08-08T09:30:00+08:00".to_owned()),
                    end_at: Some("2026-08-08T10:30:00+08:00".to_owned()),
                    start_local_date: None,
                    end_local_date: None,
                    timezone: Some("Asia/Shanghai".to_owned()),
                },
                exclude_event_id: None,
            },
        )
        .unwrap();
        assert_eq!(overlap.len(), 1);
    }

    #[test]
    fn all_day_event_keeps_local_date_across_timezone_labels() {
        let connection = database();
        let event = create_event(
            &connection,
            CalendarEventInput {
                title: "travel day".to_owned(),
                description: None,
                is_all_day: true,
                start_at: None,
                end_at: None,
                start_local_date: Some("2026-11-01".to_owned()),
                end_local_date: Some("2026-11-01".to_owned()),
                timezone: Some("America/Los_Angeles".to_owned()),
                source_task_id: None,
            },
        )
        .unwrap();
        assert_eq!(event.start_local_date.as_deref(), Some("2026-11-01"));
        assert!(event.start_at.is_none());
        let moved = move_event(
            &connection,
            &event.id,
            CalendarTimingInput {
                is_all_day: true,
                start_at: None,
                end_at: None,
                start_local_date: Some("2026-11-02".to_owned()),
                end_local_date: Some("2026-11-02".to_owned()),
                timezone: Some("Asia/Shanghai".to_owned()),
            },
        )
        .unwrap();
        assert_eq!(moved.start_local_date.as_deref(), Some("2026-11-02"));
    }

    #[test]
    fn scheduling_task_does_not_change_due_at_and_is_idempotent() {
        let connection = database();
        let task = execution::create_task(
            &connection,
            crate::execution::TaskInput {
                project_id: None,
                title: "deep work".to_owned(),
                description: None,
                priority: None,
                estimated_minutes: None,
                actual_minutes: None,
                due_at: Some("2026-08-10T18:00:00+08:00".to_owned()),
                scheduled_start_at: None,
                scheduled_end_at: None,
                timezone: None,
                context: None,
            },
        )
        .unwrap();
        let input = ScheduleTaskInput {
            timing: CalendarTimingInput {
                is_all_day: false,
                start_at: Some("2026-08-09T09:00:00+08:00".to_owned()),
                end_at: Some("2026-08-09T10:00:00+08:00".to_owned()),
                start_local_date: None,
                end_local_date: None,
                timezone: Some("Asia/Shanghai".to_owned()),
            },
        };
        let first = schedule_task(&connection, &task.id, input.clone()).unwrap();
        let second = schedule_task(&connection, &task.id, input).unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(execution::get_task(&connection, &task.id).unwrap().due_at, task.due_at);
    }

    #[test]
    fn recurrence_change_preserves_materialized_occurrence() {
        let connection = database();
        let event = create_event(
            &connection,
            timed("daily", "2026-08-08T09:00:00+08:00", "2026-08-08T09:30:00+08:00"),
        )
        .unwrap();
        set_event_recurrence(
            &connection,
            &event.id,
            RecurrenceRuleInput {
                frequency: "daily".to_owned(),
                interval_value: Some(1),
                weekdays: vec![],
                month_day: None,
                timezone: Some("Asia/Shanghai".to_owned()),
                until_at: None,
                max_occurrences: None,
            },
        )
        .unwrap();
        let occurrence = materialize_occurrence(
            &connection,
            &event.id,
            CalendarOccurrenceInput {
                occurrence_key: "2026-08-09".to_owned(),
                timing: CalendarTimingInput {
                    is_all_day: false,
                    start_at: Some("2026-08-09T09:00:00+08:00".to_owned()),
                    end_at: Some("2026-08-09T09:30:00+08:00".to_owned()),
                    start_local_date: None,
                    end_local_date: None,
                    timezone: Some("Asia/Shanghai".to_owned()),
                },
                title_override: None,
                description_override: None,
            },
        )
        .unwrap();
        change_occurrence_status(
            &connection,
            &event.id,
            &occurrence.id,
            CalendarOccurrenceStatusInput {
                status: "skipped".to_owned(),
            },
        )
        .unwrap();
        set_event_recurrence(
            &connection,
            &event.id,
            RecurrenceRuleInput {
                frequency: "daily".to_owned(),
                interval_value: Some(2),
                weekdays: vec![],
                month_day: None,
                timezone: Some("Asia/Shanghai".to_owned()),
                until_at: None,
                max_occurrences: None,
            },
        )
        .unwrap();
        let history = list_occurrences(&connection, &event.id).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].status, "skipped");
    }
}
