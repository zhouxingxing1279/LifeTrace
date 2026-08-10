use rusqlite::Transaction;

use crate::database::migration_runner::{
    Migration, MigrationContext, MigrationError, MigrationReport,
};

/// Obsidian-style note-to-note link projection.
///
/// The table is intentionally derived from note Markdown rather than being a second source of
/// truth. It can therefore be rebuilt after migration, restore, or sync without user-data loss.
pub struct M0013NoteLinks;

impl Migration for M0013NoteLinks {
    fn version(&self) -> i64 {
        13
    }

    fn name(&self) -> &'static str {
        "note-bidirectional-links"
    }

    fn checksum(&self) -> &'static str {
        "m0013-note-bidirectional-links-v1"
    }

    fn up(
        &self,
        transaction: &Transaction,
        _context: &MigrationContext,
    ) -> Result<MigrationReport, MigrationError> {
        transaction
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS note_links (
                  id TEXT PRIMARY KEY,
                  source_note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
                  target_note_id TEXT REFERENCES notes(id) ON DELETE SET NULL,
                  target_title TEXT NOT NULL,
                  alias TEXT,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_note_links_source
                  ON note_links(source_note_id);
                CREATE INDEX IF NOT EXISTS idx_note_links_target
                  ON note_links(target_note_id);
                CREATE INDEX IF NOT EXISTS idx_note_links_unresolved_title
                  ON note_links(target_title) WHERE target_note_id IS NULL;
                CREATE UNIQUE INDEX IF NOT EXISTS uq_note_links_source_title_alias
                  ON note_links(source_note_id, target_title, COALESCE(alias, ''));
                "#,
            )
            .map_err(|error| MigrationError {
                version: 13,
                message: format!("create note link index: {error}"),
            })?;

        let link_count =
            crate::database::note_links::rebuild_all(transaction).map_err(|error| {
                MigrationError {
                    version: 13,
                    message: format!("backfill note link index: {error}"),
                }
            })?;

        let mut report = MigrationReport::default();
        report.migrated = link_count;
        report
            .metrics
            .insert("note_links".to_owned(), link_count as i64);
        Ok(report)
    }
}
