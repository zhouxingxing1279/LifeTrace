mod assistant;
mod dictionary;
mod english;
mod execution;
mod execution_structure;
mod imports;
pub(crate) mod migration;
mod notes;
pub(crate) mod photo;
mod state;
mod translation;
mod xunji;

use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use rusqlite::Connection;
use serde::Serialize;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

use crate::database;

#[derive(Clone)]
pub(crate) struct AppState {
    data_dir: PathBuf,
    dictionary_path: PathBuf,
    database: Arc<Mutex<Connection>>,
    photo_runtime: Arc<photo::Runtime>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Health {
    ok: bool,
    runtime: &'static str,
    data_dir: String,
}

async fn signal_local_sync(
    State(sync_state): State<crate::sync::SyncDesktopState>,
    request: Request,
    next: Next,
) -> Response {
    let mutating = matches!(
        request.method(),
        &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE
    );
    let response = next.run(request).await;
    if mutating && response.status().is_success() {
        sync_state.signal_local_change();
    }
    response
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(Health {
            ok: true,
            runtime: "tauri-rust",
            data_dir: state.data_dir.display().to_string(),
        }),
    )
}

pub async fn serve(
    data_dir: PathBuf,
    resource_dir: PathBuf,
    photo_runtime: Arc<photo::Runtime>,
    sync_state: crate::sync::SyncDesktopState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tokio::fs::create_dir_all(&data_dir).await?;
    let database_path = data_dir.join("lifetrace.db");
    // 启动顺序：打开数据库 → 设置 PRAGMA → 版本化 Migration → 初始化模块 → 启动服务。
    let mut connection = database::connection::open(&database_path)?;
    let migration_context = database::migration_runner::MigrationContext::new(data_dir.clone());
    let summary = {
        let migrations = database::migrations::all();
        database::migration_runner::run(&mut connection, &migration_context, &migrations).map_err(
            |error| {
                Box::<dyn std::error::Error + Send + Sync>::from(format!("数据库迁移失败: {error}"))
            },
        )
    }?;
    if !summary.applied.is_empty() {
        for applied in &summary.applied {
            eprintln!(
                "LifeTrace applied migration v{} ({})：迁移 {} 条，warning {} 条，error {} 条，metrics {:?}",
                applied.version,
                applied.name,
                applied.report.migrated,
                applied.report.warnings,
                applied.report.errors,
                applied.report.metrics
            );
        }
    }
    state::ensure_schema(&connection)?;
    assistant::ensure_schema(&connection)?;
    imports::ensure_schema(&connection)?;
    crate::database::repositories::notes::seed_default_folders(&connection)?;
    translation::ensure_schema(&connection)?;
    english::ensure_schema(&connection)?;
    photo::ensure_schema(&connection)?;
    match crate::database::legacy::d1_import::import_once(&mut connection, &data_dir) {
        Ok(count) if count > 0 => eprintln!("LifeTrace migrated {count} legacy records"),
        Ok(_) => {}
        Err(error) => eprintln!("LifeTrace legacy migration skipped: {error}"),
    }
    let state = AppState {
        data_dir,
        dictionary_path: dictionary::resolve_path(&resource_dir),
        database: Arc::new(Mutex::new(connection)),
        photo_runtime,
    };
    let lan_state = state.clone();
    tokio::spawn(async move {
        if let Err(error) = photo::serve_lan(lan_state).await {
            eprintln!("LifeTrace photo sync service stopped: {error}");
        }
    });
    let compatibility_state = state.clone();
    tokio::spawn(async move {
        if let Err(error) = photo::serve_compatibility(compatibility_state).await {
            eprintln!("LifeTrace photo compatibility service stopped: {error}");
        }
    });
    let media_state = state.clone();
    tokio::spawn(async move {
        if let Err(error) = photo::serve_media(media_state).await {
            eprintln!("LifeTrace photo media service stopped: {error}");
        }
    });
    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/state", get(state::get).post(state::mutate))
        .route(
            "/api/execution/projects",
            get(execution::list_projects).post(execution::create_project),
        )
        .route(
            "/api/execution/projects/{id}",
            get(execution::get_project)
                .put(execution::update_project)
                .delete(execution::delete_project),
        )
        .route(
            "/api/execution/tasks",
            get(execution::list_tasks).post(execution::create_task),
        )
        .route(
            "/api/execution/tasks/{id}",
            get(execution::get_task)
                .put(execution::update_task)
                .delete(execution::delete_task),
        )
        .route(
            "/api/execution/tasks/{id}/status",
            axum::routing::put(execution::change_task_status),
        )
        .route(
            "/api/execution/tasks/{id}/subtasks",
            get(execution_structure::list_subtasks).post(execution_structure::add_subtask),
        )
        .route(
            "/api/execution/tasks/{id}/dependencies",
            get(execution_structure::list_dependencies).post(execution_structure::add_dependency),
        )
        .route(
            "/api/execution/tasks/{id}/dependencies/{prerequisite_id}",
            axum::routing::delete(execution_structure::remove_dependency),
        )
        .route(
            "/api/execution/tasks/{id}/blockers",
            get(execution_structure::list_blockers),
        )
        .route(
            "/api/execution/tasks/{id}/recurrence",
            get(execution_structure::get_recurrence)
                .put(execution_structure::set_recurrence)
                .delete(execution_structure::clear_recurrence),
        )
        .route(
            "/api/execution/tasks/{id}/occurrences",
            get(execution_structure::list_occurrences)
                .post(execution_structure::materialize_occurrence),
        )
        .route(
            "/api/execution/tasks/{id}/occurrences/{occurrence_id}",
            axum::routing::put(execution_structure::update_occurrence),
        )
        .route(
            "/api/execution/tasks/{id}/occurrences/{occurrence_id}/status",
            axum::routing::put(execution_structure::change_occurrence_status),
        )
        .route("/api/assistant/catalog", get(assistant::catalog))
        .route("/api/assistant/chat", axum::routing::post(assistant::chat))
        .route(
            "/api/assistant/conversations",
            get(assistant::conversations_get)
                .post(assistant::conversations_save)
                .delete(assistant::conversations_remove),
        )
        .route(
            "/api/settings/ai",
            get(assistant::settings_get)
                .post(assistant::settings_save)
                .delete(assistant::settings_remove),
        )
        .route(
            "/api/imports",
            get(imports::get)
                .post(imports::create)
                .patch(imports::update)
                .delete(imports::remove),
        )
        .route("/api/notes", get(notes::get).post(notes::mutate))
        .route("/api/english/dictionary/lookup", get(dictionary::lookup))
        .route("/api/xunji/parse", axum::routing::post(xunji::parse))
        .route("/api/xunji/imports", get(xunji::list).post(xunji::update))
        .route(
            "/api/settings/translation",
            get(translation::settings_get)
                .post(translation::settings_save)
                .delete(translation::settings_remove),
        )
        .route(
            "/api/english/translate",
            axum::routing::post(translation::translate),
        )
        .route(
            "/api/english/{*path}",
            axum::routing::any(english::dispatch),
        )
        .route(
            "/api/photo-sync/dashboard",
            get(photo::dashboard_get).post(photo::dashboard_post),
        )
        .layer(middleware::from_fn_with_state(
            sync_state,
            signal_local_sync,
        ))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);
    let address = SocketAddr::from(([127, 0, 0, 1], 3103));
    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
