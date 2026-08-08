use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::{
    execution::{ExecutionError, ExecutionErrorKind},
    execution_memo::{
        self, MemoInput, MemoQuery, MemoToCalendarInput, MemoToTaskInput, MemoToWaitingInput,
        PinMemoInput,
    },
};
use super::AppState;

#[derive(Serialize)] struct OkResponse{ok:bool}
#[derive(Serialize)] #[serde(rename_all="camelCase")] struct ErrorResponse{error:String,code:&'static str}

fn execution_error(error:ExecutionError)->Response{
    let(status,code)=match error.kind{
        ExecutionErrorKind::Validation=>(StatusCode::BAD_REQUEST,"EXECUTION_VALIDATION"),
        ExecutionErrorKind::NotFound=>(StatusCode::NOT_FOUND,"EXECUTION_NOT_FOUND"),
        ExecutionErrorKind::Conflict=>(StatusCode::CONFLICT,"EXECUTION_CONFLICT"),
        ExecutionErrorKind::Storage=>(StatusCode::INTERNAL_SERVER_ERROR,"EXECUTION_STORAGE_FAILURE"),
    };
    (status,Json(ErrorResponse{error:error.message,code})).into_response()
}
fn lock_error()->Response{(StatusCode::INTERNAL_SERVER_ERROR,Json(ErrorResponse{error:"SQLite 锁已损坏".to_owned(),code:"EXECUTION_DATABASE_LOCK_FAILURE"})).into_response()}

macro_rules! with_db{($state:expr,$body:expr)=>{{let connection=match $state.database.lock(){Ok(value)=>value,Err(_)=>return lock_error()};match $body(&connection){Ok(value)=>value,Err(error)=>return execution_error(error)}}}}

pub async fn list(State(state):State<AppState>,Query(query):Query<MemoQuery>)->Response{
    let items=with_db!(state,|db|execution_memo::list_memos(db,query));Json(items).into_response()
}
pub async fn get(State(state):State<AppState>,Path(id):Path<String>)->Response{
    let item=with_db!(state,|db|execution_memo::get_memo(db,&id));Json(item).into_response()
}
pub async fn create(State(state):State<AppState>,Json(input):Json<MemoInput>)->Response{
    let item=with_db!(state,|db|execution_memo::create_memo(db,input));(StatusCode::CREATED,Json(item)).into_response()
}
pub async fn update(State(state):State<AppState>,Path(id):Path<String>,Json(input):Json<MemoInput>)->Response{
    let item=with_db!(state,|db|execution_memo::update_memo(db,&id,input));Json(item).into_response()
}
pub async fn pin(State(state):State<AppState>,Path(id):Path<String>,Json(input):Json<PinMemoInput>)->Response{
    let item=with_db!(state,|db|execution_memo::set_pinned(db,&id,input));Json(item).into_response()
}
pub async fn archive(State(state):State<AppState>,Path(id):Path<String>)->Response{
    let item=with_db!(state,|db|execution_memo::archive_memo(db,&id));Json(item).into_response()
}
pub async fn restore(State(state):State<AppState>,Path(id):Path<String>)->Response{
    let item=with_db!(state,|db|execution_memo::restore_memo(db,&id));Json(item).into_response()
}
pub async fn delete(State(state):State<AppState>,Path(id):Path<String>)->Response{
    with_db!(state,|db|execution_memo::delete_memo(db,&id));Json(OkResponse{ok:true}).into_response()
}
pub async fn convert_to_task(State(state):State<AppState>,Path(id):Path<String>,Json(input):Json<MemoToTaskInput>)->Response{
    let item=with_db!(state,|db|execution_memo::convert_to_task(db,&id,input));Json(item).into_response()
}
pub async fn convert_to_calendar(State(state):State<AppState>,Path(id):Path<String>,Json(input):Json<MemoToCalendarInput>)->Response{
    let item=with_db!(state,|db|execution_memo::convert_to_calendar(db,&id,input));Json(item).into_response()
}
pub async fn convert_to_waiting(State(state):State<AppState>,Path(id):Path<String>,Json(input):Json<MemoToWaitingInput>)->Response{
    let item=with_db!(state,|db|execution_memo::convert_to_waiting(db,&id,input));Json(item).into_response()
}
