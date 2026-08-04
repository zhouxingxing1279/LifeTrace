//! Notes domain DTOs.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::common::EntityMeta;
use crate::domain::enums::NoteType;
use crate::ids::EntityId;
use crate::json_value::JsonValue;

/// `note.folder`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct NoteFolder {
    pub meta: EntityMeta,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub sort_order: i64,
}

/// `note.note`
///
/// The sync payload is the authoritative full snapshot (including content).
/// Tags, relations, attachments and revisions are separate entity types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct Note {
    pub meta: EntityMeta,
    pub title: Option<String>,
    pub note_type: NoteType,
    pub folder_id: Option<EntityId>,
    pub content_json: JsonValue,
    pub content_html: String,
    pub content_text: String,
    pub content_markdown: String,
    pub summary: String,
    pub is_pinned: bool,
    pub is_favorite: bool,
    pub is_archived: bool,
    pub ai_summary: Option<String>,
    pub ai_tags: Option<String>,
    pub embedding_status: Option<String>,
    pub last_ai_processed_at: Option<crate::time::UtcTimestamp>,
}

/// `note.tag`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct NoteTag {
    pub meta: EntityMeta,
    pub name: String,
    pub color: String,
}

/// `note.tag_relation`
///
/// The entity id is the stable composite `<noteId>:<tagId>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct NoteTagRelation {
    pub meta: EntityMeta,
    pub note_id: EntityId,
    pub tag_id: EntityId,
}

/// `note.relation`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct NoteRelation {
    pub meta: EntityMeta,
    pub note_id: EntityId,
    pub entity_type: crate::registry::EntityType,
    pub entity_id: EntityId,
    pub relation_type: String,
}

/// `note.revision`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct NoteRevision {
    pub meta: EntityMeta,
    pub note_id: EntityId,
    /// Revision counter within the note (distinct from `meta.localVersion`).
    pub revision_version: u64,
    pub title: Option<String>,
    pub content_json: JsonValue,
    pub content_html: String,
    pub content_markdown: String,
}
