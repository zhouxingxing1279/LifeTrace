//! LifeTrace contract exporter.
//!
//! Generates (from the Rust contract types, the authoritative source):
//! - `contracts/json-schema/*.schema.json`
//! - `contracts/openapi/lifetrace-sync-v1.json`
//! - `contracts/typescript/lifetrace-contracts.generated.ts`
//!
//! Output is deterministic: repeated runs produce no meaningful diff.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use lifetrace_contracts::domain::*;
use lifetrace_contracts::sync::v1::*;
use lifetrace_contracts::*;
use schemars::schema_for;
use serde_json::{Map, Value, json};

const GENERATED_NOTICE: &str = "GENERATED FILE - DO NOT EDIT MANUALLY";

fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // tools/contract-exporter -> repo root
    manifest.parent().unwrap().parent().unwrap().to_path_buf()
}

/// All public types that should be exported to JSON Schema and TypeScript.
macro_rules! public_types {
    () => {
        [
            // value objects
            UserId::type_name(),
            DeviceId::type_name(),
            EntityId::type_name(),
            ChangeId::type_name(),
            RequestId::type_name(),
            ConflictId::type_name(),
            AtomicGroupId::type_name(),
            SnapshotId::type_name(),
            Cursor::type_name(),
            ServerVersion::type_name(),
            LocalDate::type_name(),
            CurrencyCode::type_name(),
            MoneyAmount::type_name(),
            EntityMeta::type_name(),
            JsonValue::type_name(),
            // errors
            ErrorCode::type_name(),
            FieldError::type_name(),
            ApiErrorV1::type_name(),
            // registry
            EntityType::type_name(),
            EntityRef::type_name(),
            EntityOwnership::type_name(),
            SyncMode::type_name(),
            ConflictMode::type_name(),
            // domain enums
            TransactionType::type_name(),
            TransactionStatus::type_name(),
            AccountType::type_name(),
            ActivityType::type_name(),
            ActivityScheduleType::type_name(),
            ActivityCheckinMethod::type_name(),
            ActivitySyncSource::type_name(),
            ActivityLogStatus::type_name(),
            NoteType::type_name(),
            EnglishLevel::type_name(),
            EnglishCategory::type_name(),
            EnglishProcessingStatus::type_name(),
            EnglishFetchStatus::type_name(),
            EnglishCompletionStatus::type_name(),
            EnglishReadingStatus::type_name(),
            VocabularyStatus::type_name(),
            VocabularyReviewResult::type_name(),
            HighlightColor::type_name(),
            WorkoutSource::type_name(),
            WorkoutStatus::type_name(),
            ImportStatus::type_name(),
            FileStorageState::type_name(),
            // identity
            User::type_name(),
            Device::type_name(),
            // finance
            FinanceAccount::type_name(),
            TransactionCategory::type_name(),
            Transaction::type_name(),
            TransactionEvidence::type_name(),
            // habits / reviews
            Activity::type_name(),
            ActivityLog::type_name(),
            DailyReview::type_name(),
            // notes
            NoteFolder::type_name(),
            Note::type_name(),
            NoteTag::type_name(),
            NoteTagRelation::type_name(),
            NoteRelation::type_name(),
            NoteRevision::type_name(),
            // english
            EnglishArticle::type_name(),
            ArticleVocabularyItem::type_name(),
            EnglishLearningRecord::type_name(),
            EnglishHighlight::type_name(),
            EnglishNote::type_name(),
            EnglishVocabulary::type_name(),
            VocabularyOccurrence::type_name(),
            VocabularyReviewState::type_name(),
            // workouts
            WorkoutImport::type_name(),
            Workout::type_name(),
            WorkoutExercise::type_name(),
            WorkoutSet::type_name(),
            TrainingNote::type_name(),
            // files / links / preferences
            FileMetadata::type_name(),
            EntityLink::type_name(),
            UserPreference::type_name(),
            // sync v1
            AppId::type_name(),
            ClientPlatform::type_name(),
            SyncClientInfo::type_name(),
            ChangeOperation::type_name(),
            SyncChangeV1::type_name(),
            TombstoneV1::type_name(),
            ConflictReason::type_name(),
            ConflictV1::type_name(),
            PushRequestV1::type_name(),
            PushChangeResultV1::type_name(),
            PushResponseV1::type_name(),
            PullRequestV1::type_name(),
            ServerChangeV1::type_name(),
            PullResponseV1::type_name(),
            SnapshotRequestV1::type_name(),
            EntitySnapshotV1::type_name(),
            SnapshotResponseV1::type_name(),
            MinimumClientVersion::type_name(),
            CapabilitiesResponseV1::type_name(),
        ]
    };
}

trait TypeName {
    fn type_name() -> String;
}

impl<T: ts_rs::TS> TypeName for T {
    fn type_name() -> String {
        <T as ts_rs::TS>::name()
    }
}

fn write_stable_json(path: &Path, value: &Value) -> std::io::Result<()> {
    let mut text = serde_json::to_string_pretty(value).unwrap();
    text.push('\n');
    fs::write(path, text)
}

fn export_json_schemas(contracts_dir: &Path) -> std::io::Result<()> {
    let directory = contracts_dir.join("json-schema");
    fs::create_dir_all(&directory)?;
    let mut names: Vec<String> = public_types!().into_iter().collect();
    names.sort_unstable();
    names.dedup();
    for name in names {
        let schema = schema_for_type(&name);
        let mut value = serde_json::to_value(schema).unwrap();
        if let Value::Object(object) = &mut value {
            object.insert(
                "$comment".to_owned(),
                Value::String(format!("{GENERATED_NOTICE}. Source: crates/lifetrace-contracts.")),
            );
        }
        write_stable_json(&directory.join(format!("{name}.schema.json")), &value)?;
    }
    Ok(())
}

fn schema_for_type(name: &str) -> schemars::Schema {
    macro_rules! match_type {
        ($name:expr, $($ty:ty),* $(,)?) => {
            match $name {
                $(stringify!($ty) => schema_for!($ty),)*
                _ => panic!("unknown exported type: {name}"),
            }
        };
    }
    match_type!(
        name,
        UserId, DeviceId, EntityId, ChangeId, RequestId, ConflictId, AtomicGroupId, SnapshotId,
        Cursor, ServerVersion, LocalDate, CurrencyCode, MoneyAmount, EntityMeta, JsonValue,
        ErrorCode, FieldError, ApiErrorV1, EntityType, EntityRef, EntityOwnership, SyncMode,
        ConflictMode, TransactionType, TransactionStatus, AccountType, ActivityType,
        ActivityScheduleType, ActivityCheckinMethod, ActivitySyncSource, ActivityLogStatus,
        NoteType, EnglishLevel, EnglishCategory, EnglishProcessingStatus, EnglishFetchStatus,
        EnglishCompletionStatus, EnglishReadingStatus, VocabularyStatus, VocabularyReviewResult,
        HighlightColor, WorkoutSource, WorkoutStatus, ImportStatus, FileStorageState, User, Device,
        FinanceAccount, TransactionCategory, Transaction, TransactionEvidence, Activity,
        ActivityLog, DailyReview, NoteFolder, Note, NoteTag, NoteTagRelation, NoteRelation,
        NoteRevision, EnglishArticle, ArticleVocabularyItem, EnglishLearningRecord, EnglishHighlight,
        EnglishNote, EnglishVocabulary, VocabularyOccurrence, VocabularyReviewState, WorkoutImport,
        Workout, WorkoutExercise, WorkoutSet, TrainingNote, FileMetadata, EntityLink, UserPreference,
        AppId, ClientPlatform, SyncClientInfo, ChangeOperation, SyncChangeV1, TombstoneV1,
        ConflictReason, ConflictV1, PushRequestV1, PushChangeResultV1, PushResponseV1, PullRequestV1,
        ServerChangeV1, PullResponseV1, SnapshotRequestV1, EntitySnapshotV1, SnapshotResponseV1,
        MinimumClientVersion, CapabilitiesResponseV1,
    )
}

fn export_typescript(contracts_dir: &Path) -> std::io::Result<()> {
    let directory = contracts_dir.join("typescript");
    fs::create_dir_all(&directory)?;
    let mut declarations = BTreeMap::<String, String>::new();
    for name in public_types!() {
        declarations.insert(name.clone(), ts_decl_for(&name));
    }
    let mut header = String::from(
        "// GENERATED FILE - DO NOT EDIT MANUALLY\n\
         // Generated by tools/contract-exporter (npm run contracts:generate).\n\
         // Rust types in crates/lifetrace-contracts are the authoritative source.\n\n",
    );
    for (_, declaration) in declarations {
        header.push_str(&with_export_prefix(&declaration));
        header.push_str("\n\n");
    }
    fs::write(directory.join("lifetrace-contracts.generated.ts"), header)
}

/// Insert `export` before the first `type`/`interface` declaration line
/// (doc comments may precede it).
fn with_export_prefix(declaration: &str) -> String {
    let mut lines: Vec<String> = declaration.lines().map(str::to_owned).collect();
    if let Some(index) = lines.iter().position(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("type ") || trimmed.starts_with("interface ")
    }) {
        lines[index] = format!("export {}", lines[index]);
    }
    lines.join("\n")
}

fn ts_decl_for(name: &str) -> String {
    macro_rules! match_type {
        ($name:expr, $($ty:ty),* $(,)?) => {
            match $name {
                $(stringify!($ty) => <$ty as ts_rs::TS>::decl(),)*
                _ => panic!("unknown exported type: {name}"),
            }
        };
    }
    match_type!(
        name,
        UserId, DeviceId, EntityId, ChangeId, RequestId, ConflictId, AtomicGroupId, SnapshotId,
        Cursor, ServerVersion, LocalDate, CurrencyCode, MoneyAmount, EntityMeta, JsonValue,
        ErrorCode, FieldError, ApiErrorV1, EntityType, EntityRef, EntityOwnership, SyncMode,
        ConflictMode, TransactionType, TransactionStatus, AccountType, ActivityType,
        ActivityScheduleType, ActivityCheckinMethod, ActivitySyncSource, ActivityLogStatus,
        NoteType, EnglishLevel, EnglishCategory, EnglishProcessingStatus, EnglishFetchStatus,
        EnglishCompletionStatus, EnglishReadingStatus, VocabularyStatus, VocabularyReviewResult,
        HighlightColor, WorkoutSource, WorkoutStatus, ImportStatus, FileStorageState, User, Device,
        FinanceAccount, TransactionCategory, Transaction, TransactionEvidence, Activity,
        ActivityLog, DailyReview, NoteFolder, Note, NoteTag, NoteTagRelation, NoteRelation,
        NoteRevision, EnglishArticle, ArticleVocabularyItem, EnglishLearningRecord, EnglishHighlight,
        EnglishNote, EnglishVocabulary, VocabularyOccurrence, VocabularyReviewState, WorkoutImport,
        Workout, WorkoutExercise, WorkoutSet, TrainingNote, FileMetadata, EntityLink, UserPreference,
        AppId, ClientPlatform, SyncClientInfo, ChangeOperation, SyncChangeV1, TombstoneV1,
        ConflictReason, ConflictV1, PushRequestV1, PushChangeResultV1, PushResponseV1, PullRequestV1,
        ServerChangeV1, PullResponseV1, SnapshotRequestV1, EntitySnapshotV1, SnapshotResponseV1,
        MinimumClientVersion, CapabilitiesResponseV1,
    )
}

fn export_openapi(contracts_dir: &Path) -> std::io::Result<()> {
    let directory = contracts_dir.join("openapi");
    fs::create_dir_all(&directory)?;

    // Collect component schemas for every public type from one generator so
    // $refs resolve inside components.schemas.
    let mut definitions: BTreeMap<String, Value> = BTreeMap::new();
    for name in public_types!() {
        let schema = schema_for_type(&name);
        let value = serde_json::to_value(&schema).unwrap();
        if let Value::Object(object) = &value {
            // The root object contains the type's own schema (plus meta keys).
            let mut own_schema = object.clone();
            own_schema.remove("$schema");
            own_schema.remove("$defs");
            own_schema.remove("$comment");
            own_schema.remove("title");
            definitions.insert(name.clone(), Value::Object(own_schema));
            // Dependencies referenced from other schemas.
            if let Some(Value::Object(defs)) = object.get("$defs") {
                for (key, definition) in defs {
                    definitions.insert(key.clone(), definition.clone());
                }
            }
        }
    }
    // The type itself is referenced by name via its definition entry.
    let mut components_schemas: Map<String, Value> = definitions.into_iter().collect();
    for (_, schema) in components_schemas.iter_mut() {
        rewrite_schema_refs(schema);
    }
    let mut components = Map::new();
    components.insert("schemas".to_owned(), Value::Object(components_schemas));

    let document = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "LifeTrace Sync Protocol v1",
            "version": "1.0.0",
            "description": format!(
                "{GENERATED_NOTICE}.\n\
                 Public sync contract v1 for LifeTrace. Rust types in \
                 crates/lifetrace-contracts are the authoritative source.\n\n\
                 Versioning: protocolVersion 1, schemaVersion 1. Unknown JSON fields are ignored; \
                 unknown enum values are preserved as strings. Cursor/serverVersion are opaque \
                 strings on the wire. Money is integer cents (amountCents). Timestamps are RFC3339 UTC; \
                 natural days are YYYY-MM-DD.\n\n\
                 Push idempotency key: userId + changeId. Same changeId + same payload replays the first \
                 result; same changeId + different payload returns LIFETRACE_CHANGE_ID_REUSE.\n\n\
                 Conflicts are explicit (no automatic last-write-wins): when baseServerVersion != current \
                 serverVersion the server returns conflict with the current entity or tombstone. Resolution \
                 is client-side: keep_server, keep_local (new changeId), manual_merge.\n\n\
                 Deletes produce tombstones. Re-creating a deleted entity requires a NEW entity id; restoring \
                 a soft delete must be based on the tombstone's latest serverVersion.\n\n\
                 Snapshot pages share one consistent view; after completion set the client cursor to \
                 snapshotCursor and continue with Pull."
            )
        },
        "paths": {
            "/api/v1/sync/capabilities": {
                "get": {
                    "operationId": "syncCapabilities",
                    "summary": "Return protocol and server capabilities",
                    "description": "Clients should call this first to validate versions, batch limits and supported entity types.",
                    "responses": {
                        "200": {
                            "description": "Capabilities",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/CapabilitiesResponseV1" }
                                }
                            }
                        },
                        "400": error_response(),
                        "401": error_response(),
                        "426": error_response(),
                        "500": error_response()
                    }
                }
            },
            "/api/v1/sync/push": {
                "post": {
                    "operationId": "syncPush",
                    "summary": "Push a batch of changes",
                    "description": format!(
                        "Each change returns accepted, duplicate, conflict or rejected. Business conflicts do not \
                         fail the whole request. Changes sharing an atomicGroupId must be in this request and succeed \
                         or fail together. Batch limits are declared by capabilities."
                    ),
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/PushRequestV1" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Per-change results",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/PushResponseV1" }
                                }
                            }
                        },
                        "400": error_response(),
                        "401": error_response(),
                        "403": error_response(),
                        "413": error_response(),
                        "426": error_response(),
                        "429": error_response(),
                        "500": error_response()
                    }
                }
            },
            "/api/v1/sync/pull": {
                "post": {
                    "operationId": "syncPull",
                    "summary": "Pull changes after a cursor",
                    "description": format!(
                        "Responses are ordered strictly by server cursor. Clients must apply them in order and never \
                         re-sort by updatedAt; only persist nextCursor after the whole batch succeeded. An expired cursor \
                         returns LIFETRACE_CURSOR_EXPIRED and the client must run a snapshot first."
                    ),
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/PullRequestV1" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Ordered changes",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/PullResponseV1" }
                                }
                            }
                        },
                        "400": error_response(),
                        "401": error_response(),
                        "403": error_response(),
                        "410": error_response(),
                        "426": error_response(),
                        "429": error_response(),
                        "500": error_response()
                    }
                }
            },
            "/api/v1/sync/snapshot": {
                "post": {
                    "operationId": "syncSnapshot",
                    "summary": "Fetch a consistent snapshot of current entities",
                    "description": format!(
                        "All pages of one snapshot correspond to one consistent view. After the final page set the \
                         client cursor to snapshotCursor, then continue with Pull to receive concurrent changes without gaps."
                    ),
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SnapshotRequestV1" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Snapshot page",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/SnapshotResponseV1" }
                                }
                            }
                        },
                        "400": error_response(),
                        "401": error_response(),
                        "403": error_response(),
                        "426": error_response(),
                        "429": error_response(),
                        "500": error_response()
                    }
                }
            }
        },
        "components": components
    });

    write_stable_json(&directory.join("lifetrace-sync-v1.json"), &document)
}

fn error_response() -> Value {
    json!({
        "description": "Uniform API error body with a stable ErrorCode",
        "content": {
            "application/json": {
                "schema": { "$ref": "#/components/schemas/ApiErrorV1" }
            }
        }
    })
}

/// Rewrite `#/$defs/Name` references to `#/components/schemas/Name` so the
/// OpenAPI document resolves against components.schemas.
fn rewrite_schema_refs(value: &mut Value) {
    match value {
        Value::Object(object) => {
            if let Some(reference) = object.get_mut("$ref") {
                if let Some(name) = reference.as_str().and_then(|text| text.strip_prefix("#/$defs/")) {
                    *reference = Value::String(format!("#/components/schemas/{name}"));
                }
            }
            for (_, child) in object.iter_mut() {
                rewrite_schema_refs(child);
            }
        }
        Value::Array(items) => {
            for item in items {
                rewrite_schema_refs(item);
            }
        }
        _ => {}
    }
}

fn main() -> std::io::Result<()> {
    let root = repo_root();
    let contracts_dir = root.join("contracts");
    fs::create_dir_all(&contracts_dir)?;
    export_json_schemas(&contracts_dir)?;
    export_typescript(&contracts_dir)?;
    export_openapi(&contracts_dir)?;
    println!("contracts exported to {}", contracts_dir.display());
    Ok(())
}
