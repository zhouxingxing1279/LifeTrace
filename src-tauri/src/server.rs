mod assistant;
mod dictionary;
mod english;
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

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
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
        eprintln!(
            "LifeTrace applied {} migration(s)",
            summary.applied.len()
        );
    }
    state::ensure_schema(&connection)?;
    assistant::ensure_schema(&connection)?;
    imports::ensure_schema(&connection)?;
    notes::ensure_schema(&connection)?;
    xunji::ensure_schema(&connection)?;
    translation::ensure_schema(&connection)?;
    english::ensure_schema(&connection)?;
    photo::ensure_schema(&connection)?;
    match migration::migrate_once(&connection, &data_dir) {
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
