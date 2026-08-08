use chrono::Utc;
use rusqlite::Connection;
use serde::Deserialize;

use crate::{
    database::{
        profile,
        repositories::execution_memo::{self as repository, MemoFilter, MemoRecord, MemoWrite},
    },
    execution::{self, ExecutionError, ExecutionErrorKind, ExecutionResult, TaskInput},
    execution_calendar::{self, CalendarEventInput, CalendarEventRecord, CalendarTimingInput},
    execution_waiting::{self, WaitingItemInput},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoInput {
    pub content: String,
    pub context: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MemoQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    pub pinned: Option<bool>,
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinMemoInput { pub pinned: bool }

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoToTaskInput {
    pub project_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub estimated_minutes: Option<i64>,
    pub due_at: Option<String>,
    pub scheduled_start_at: Option<String>,
    pub scheduled_end_at: Option<String>,
    pub timezone: Option<String>,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoToCalendarInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub timing: CalendarTimingInput,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoToWaitingInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub waiting_for: String,
    pub expected_at: Option<String>,
    pub follow_up_at: Option<String>,
}

fn error(kind: ExecutionErrorKind, message: impl Into<String>) -> ExecutionError {
    ExecutionError { kind, message: message.into() }
}
fn validation(message: impl Into<String>) -> ExecutionError { error(ExecutionErrorKind::Validation, message) }
fn not_found(message: impl Into<String>) -> ExecutionError { error(ExecutionErrorKind::NotFound, message) }
fn conflict(message: impl Into<String>) -> ExecutionError { error(ExecutionErrorKind::Conflict, message) }
fn storage(message: impl Into<String>) -> ExecutionError { error(ExecutionErrorKind::Storage, message) }

fn active_user(connection: &Connection) -> ExecutionResult<String> {
    profile::active_profile_id(connection).map_err(storage)
}

fn clean_required(value: &str, label: &str, max: usize) -> ExecutionResult<String> {
    let value=value.trim();
    if value.is_empty(){ return Err(validation(format!("{label}不能为空"))); }
    if value.chars().count()>max { return Err(validation(format!("{label}不能超过 {max} 个字符"))); }
    Ok(value.to_owned())
}

fn clean_optional(value: Option<String>, label: &str, max: usize) -> ExecutionResult<Option<String>> {
    let Some(value)=value else{return Ok(None)};
    let value=value.trim();
    if value.is_empty(){return Ok(None)}
    if value.chars().count()>max{return Err(validation(format!("{label}不能超过 {max} 个字符")))}
    Ok(Some(value.to_owned()))
}

fn normalize_tags(tags: Vec<String>) -> ExecutionResult<Vec<String>> {
    let mut output=Vec::new();
    let mut normalized=std::collections::HashSet::new();
    for tag in tags {
        let tag=clean_required(&tag,"标签",64)?;
        let key=tag.to_lowercase();
        if normalized.insert(key){ output.push(tag); }
    }
    if output.len()>20{return Err(validation("单个备忘录最多 20 个标签"))}
    Ok(output)
}

fn write_from_input(user_id:String,id:Option<String>,input:MemoInput,current:Option<&MemoRecord>)->ExecutionResult<MemoWrite>{
    let content=clean_required(&input.content,"备忘录内容",20_000)?;
    Ok(MemoWrite{
        id,user_id,plain_text:content.clone(),content,
        is_pinned:current.is_some_and(|memo|memo.is_pinned),
        status:current.map(|memo|memo.status.clone()).unwrap_or_else(||"active".to_owned()),
        archived_at:current.and_then(|memo|memo.archived_at.clone()),
        context:clean_optional(input.context,"上下文",512)?,
        tags:normalize_tags(input.tags)?,
    })
}

fn state_write(memo:&MemoRecord,user_id:String,status:&str,pinned:bool)->MemoWrite{
    MemoWrite{id:Some(memo.id.clone()),user_id,content:memo.content.clone(),plain_text:memo.plain_text.clone(),is_pinned:pinned,
        status:status.to_owned(),archived_at:if status=="archived"{memo.archived_at.clone().or_else(||Some(Utc::now().to_rfc3339()))}else{None},
        context:memo.context.clone(),tags:memo.tags.clone()}
}

fn default_title(memo:&MemoRecord)->String{
    let line=memo.plain_text.lines().find(|line|!line.trim().is_empty()).unwrap_or("备忘录").trim();
    line.chars().take(240).collect()
}

pub fn list_memos(connection:&Connection,query:MemoQuery)->ExecutionResult<Vec<MemoRecord>>{
    if let Some(status)=query.status.as_deref(){if !matches!(status,"active"|"archived"){return Err(validation("status 必须是 active 或 archived"))}}
    let user_id=active_user(connection)?;
    repository::list(connection,&user_id,&MemoFilter{query:clean_optional(query.q,"搜索词",500)?,status:query.status.or_else(||Some("active".to_owned())),pinned:query.pinned,tag:clean_optional(query.tag,"标签",64)?}).map_err(storage)
}

pub fn get_memo(connection:&Connection,id:&str)->ExecutionResult<MemoRecord>{
    let user_id=active_user(connection)?;
    repository::get(connection,&user_id,id).map_err(storage)?.ok_or_else(||not_found("备忘录不存在"))
}

pub fn create_memo(connection:&Connection,input:MemoInput)->ExecutionResult<MemoRecord>{
    let user_id=active_user(connection)?;
    let write=write_from_input(user_id,None,input,None)?;
    let transaction=connection.unchecked_transaction().map_err(|error|storage(error.to_string()))?;
    let memo=repository::save(&transaction,&write).map_err(storage)?;
    transaction.commit().map_err(|error|storage(error.to_string()))?;
    Ok(memo)
}

pub fn update_memo(connection:&Connection,id:&str,input:MemoInput)->ExecutionResult<MemoRecord>{
    let user_id=active_user(connection)?;
    let current=repository::get(connection,&user_id,id).map_err(storage)?.ok_or_else(||not_found("备忘录不存在"))?;
    if current.status!="active"{return Err(conflict("归档备忘录需先恢复后再编辑"))}
    let write=write_from_input(user_id,Some(id.to_owned()),input,Some(&current))?;
    let transaction=connection.unchecked_transaction().map_err(|error|storage(error.to_string()))?;
    let memo=repository::save(&transaction,&write).map_err(storage)?;
    transaction.commit().map_err(|error|storage(error.to_string()))?;
    Ok(memo)
}

pub fn set_pinned(connection:&Connection,id:&str,input:PinMemoInput)->ExecutionResult<MemoRecord>{
    let user_id=active_user(connection)?;
    let current=repository::get(connection,&user_id,id).map_err(storage)?.ok_or_else(||not_found("备忘录不存在"))?;
    repository::save(connection,&state_write(&current,user_id,&current.status,input.pinned)).map_err(storage)
}

pub fn archive_memo(connection:&Connection,id:&str)->ExecutionResult<MemoRecord>{
    let user_id=active_user(connection)?;
    let current=repository::get(connection,&user_id,id).map_err(storage)?.ok_or_else(||not_found("备忘录不存在"))?;
    if current.status=="archived"{return Ok(current)}
    repository::save(connection,&state_write(&current,user_id,"archived",current.is_pinned)).map_err(storage)
}

pub fn restore_memo(connection:&Connection,id:&str)->ExecutionResult<MemoRecord>{
    let user_id=active_user(connection)?;
    let current=repository::get(connection,&user_id,id).map_err(storage)?.ok_or_else(||not_found("备忘录不存在"))?;
    if current.status=="active"{return Ok(current)}
    repository::save(connection,&state_write(&current,user_id,"active",current.is_pinned)).map_err(storage)
}

pub fn delete_memo(connection:&Connection,id:&str)->ExecutionResult<()> {
    let user_id=active_user(connection)?;
    if repository::get(connection,&user_id,id).map_err(storage)?.is_none(){return Err(not_found("备忘录不存在"))}
    if repository::soft_delete(connection,&user_id,id).map_err(storage)?{Ok(())}else{Err(not_found("备忘录不存在"))}
}

fn archive_in_transaction(connection:&Connection,user_id:String,memo:&MemoRecord)->ExecutionResult<()> {
    repository::save(connection,&state_write(memo,user_id,"archived",memo.is_pinned)).map_err(storage)?;
    Ok(())
}

pub fn convert_to_task(connection:&Connection,memo_id:&str,input:MemoToTaskInput)->ExecutionResult<crate::database::repositories::execution::TaskRecord>{
    let user_id=active_user(connection)?;
    let memo=repository::get(connection,&user_id,memo_id).map_err(storage)?.ok_or_else(||not_found("备忘录不存在"))?;
    if let Some(target)=repository::find_conversion_target(connection,&user_id,memo_id,"task").map_err(storage)?{
        return execution::get_task(connection,&target).map_err(|_|conflict("备忘录已转换过，但目标任务不存在"));
    }
    let title=clean_optional(input.title,"任务标题",240)?.unwrap_or_else(||default_title(&memo));
    let task_input=TaskInput{project_id:input.project_id,title,description:clean_optional(input.description,"任务描述",20_000)?.or_else(||Some(memo.content.clone())),
        priority:input.priority,estimated_minutes:input.estimated_minutes,actual_minutes:None,due_at:input.due_at,scheduled_start_at:input.scheduled_start_at,
        scheduled_end_at:input.scheduled_end_at,timezone:input.timezone,context:input.context.or_else(||memo.context.clone())};
    let transaction=connection.unchecked_transaction().map_err(|error|storage(error.to_string()))?;
    let target=execution::create_task(&transaction,task_input)?;
    repository::create_conversion_links(&transaction,&user_id,memo_id,"task",&target.id).map_err(storage)?;
    archive_in_transaction(&transaction,user_id,&memo)?;
    transaction.commit().map_err(|error|storage(error.to_string()))?;
    Ok(target)
}

pub fn convert_to_calendar(connection:&Connection,memo_id:&str,input:MemoToCalendarInput)->ExecutionResult<CalendarEventRecord>{
    let user_id=active_user(connection)?;
    let memo=repository::get(connection,&user_id,memo_id).map_err(storage)?.ok_or_else(||not_found("备忘录不存在"))?;
    if let Some(target)=repository::find_conversion_target(connection,&user_id,memo_id,"calendar_event").map_err(storage)?{
        return execution_calendar::get_event(connection,&target).map_err(|_|conflict("备忘录已转换过，但目标日历事件不存在"));
    }
    let target_input=CalendarEventInput{title:clean_optional(input.title,"事件标题",240)?.unwrap_or_else(||default_title(&memo)),
        description:clean_optional(input.description,"事件描述",20_000)?.or_else(||Some(memo.content.clone())),is_all_day:input.timing.is_all_day,
        start_at:input.timing.start_at,end_at:input.timing.end_at,start_local_date:input.timing.start_local_date,end_local_date:input.timing.end_local_date,
        timezone:input.timing.timezone,source_task_id:None};
    let transaction=connection.unchecked_transaction().map_err(|error|storage(error.to_string()))?;
    let target=execution_calendar::create_event(&transaction,target_input)?;
    repository::create_conversion_links(&transaction,&user_id,memo_id,"calendar_event",&target.id).map_err(storage)?;
    archive_in_transaction(&transaction,user_id,&memo)?;
    transaction.commit().map_err(|error|storage(error.to_string()))?;
    Ok(target)
}

pub fn convert_to_waiting(connection:&Connection,memo_id:&str,input:MemoToWaitingInput)->ExecutionResult<crate::database::repositories::execution_waiting::WaitingItemRecord>{
    let user_id=active_user(connection)?;
    let memo=repository::get(connection,&user_id,memo_id).map_err(storage)?.ok_or_else(||not_found("备忘录不存在"))?;
    if let Some(target)=repository::find_conversion_target(connection,&user_id,memo_id,"waiting_item").map_err(storage)?{
        return execution_waiting::get_waiting_item(connection,&target).map_err(|_|conflict("备忘录已转换过，但目标等待事项不存在"));
    }
    let target_input=WaitingItemInput{title:clean_optional(input.title,"等待事项标题",240)?.unwrap_or_else(||default_title(&memo)),
        description:clean_optional(input.description,"等待事项描述",20_000)?.or_else(||Some(memo.content.clone())),waiting_for:input.waiting_for,
        expected_at:input.expected_at,follow_up_at:input.follow_up_at,source_task_id:None};
    let transaction=connection.unchecked_transaction().map_err(|error|storage(error.to_string()))?;
    let target=execution_waiting::create_waiting_item(&transaction,target_input)?;
    repository::create_conversion_links(&transaction,&user_id,memo_id,"waiting_item",&target.id).map_err(storage)?;
    archive_in_transaction(&transaction,user_id,&memo)?;
    transaction.commit().map_err(|error|storage(error.to_string()))?;
    Ok(target)
}

#[cfg(test)]
mod tests{
    use super::*;
    use crate::database::migration_runner::{run,MigrationContext};
    use crate::database::migrations::all;
    use std::time::{SystemTime,UNIX_EPOCH};
    fn db()->Connection{
        let unique=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir=std::env::temp_dir().join(format!("lifetrace-memo-service-{unique}"));std::fs::create_dir_all(&dir).unwrap();
        let mut connection=Connection::open_in_memory().unwrap();connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(&mut connection,&MigrationContext::new(dir),&all()).unwrap();connection
    }
    fn memo_input()->MemoInput{MemoInput{content:"Call Alice about contract".to_owned(),context:Some("work".to_owned()),tags:vec!["Follow-up".to_owned()]}}
    #[test]fn memo_lifecycle_and_search_work(){let connection=db();let memo=create_memo(&connection,memo_input()).unwrap();
        assert_eq!(list_memos(&connection,MemoQuery{q:Some("follow-up".to_owned()),..Default::default()}).unwrap().len(),1);
        let archived=archive_memo(&connection,&memo.id).unwrap();assert_eq!(archived.status,"archived");
        assert!(list_memos(&connection,MemoQuery::default()).unwrap().is_empty());assert_eq!(restore_memo(&connection,&memo.id).unwrap().status,"active");}
    #[test]fn memo_to_task_is_atomic_idempotent_and_archives_source(){let connection=db();let memo=create_memo(&connection,memo_input()).unwrap();
        let input=MemoToTaskInput{project_id:None,title:None,description:None,priority:Some("high".to_owned()),estimated_minutes:None,due_at:None,
            scheduled_start_at:None,scheduled_end_at:None,timezone:None,context:None};
        let first=convert_to_task(&connection,&memo.id,input.clone()).unwrap();let second=convert_to_task(&connection,&memo.id,input).unwrap();
        assert_eq!(first.id,second.id);assert_eq!(get_memo(&connection,&memo.id).unwrap().status,"archived");
        let links:i64=connection.query_row("SELECT COUNT(*) FROM execution_entity_links WHERE (source_id=?1 OR target_id=?1) AND deleted_at IS NULL",[memo.id],|row|row.get(0)).unwrap();assert_eq!(links,2);}
    #[test]fn memo_can_receive_shared_reminder(){let connection=db();let memo=create_memo(&connection,memo_input()).unwrap();
        let reminder=crate::execution_reminder::create_reminder(&connection,crate::execution_reminder::ReminderInput{subject_type:"memo".to_owned(),subject_id:memo.id,
            trigger_at:"2026-08-10T09:00:00+08:00".to_owned(),timezone:Some("Asia/Shanghai".to_owned())}).unwrap();assert_eq!(reminder.subject_type,"memo");}
}
