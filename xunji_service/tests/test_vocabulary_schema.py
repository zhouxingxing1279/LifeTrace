import sqlite3
from pathlib import Path


def migrated_database(tmp_path: Path) -> sqlite3.Connection:
    connection = sqlite3.connect(tmp_path / "vocabulary.db")
    connection.execute("PRAGMA foreign_keys=ON")
    connection.execute("CREATE TABLE english_vocabulary(id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL)")
    sql = Path("drizzle/0009_offline_vocabulary.sql").read_text(encoding="utf-8")
    connection.executescript(sql.replace("--> statement-breakpoint", ""))
    return connection


def insert_word(connection: sqlite3.Connection, word_id: str, lemma: str) -> None:
    connection.execute(
        """INSERT INTO english_user_vocabulary
        (id,word,normalized_word,lemma,selected_meanings_json,next_review_at,created_at,updated_at)
        VALUES (?,?,?,?,?,?,?,?)""",
        (word_id, lemma, lemma, lemma, '["释义一","释义二"]', "2026-07-25T00:00:00Z", "2026-07-25T00:00:00Z", "2026-07-25T00:00:00Z"),
    )


def test_selected_meanings_and_status_persist(tmp_path: Path) -> None:
    connection = migrated_database(tmp_path)
    insert_word(connection, "word-1", "maintain")
    connection.execute("UPDATE english_user_vocabulary SET notes='keep it',status='ARCHIVED' WHERE id='word-1'")
    row = connection.execute("SELECT selected_meanings_json,notes,status FROM english_user_vocabulary").fetchone()
    assert row == ('["释义一","释义二"]', "keep it", "ARCHIVED")


def test_lemma_and_occurrence_uniqueness_prevent_duplicates(tmp_path: Path) -> None:
    connection = migrated_database(tmp_path)
    insert_word(connection, "word-1", "maintain")
    try:
        insert_word(connection, "word-2", "maintain")
        raise AssertionError("duplicate lemma should fail")
    except sqlite3.IntegrityError:
        pass
    occurrence = ("o-1", "word-1", "article-1", "Article", "Maintain good health.", "2026-07-25T00:00:00Z")
    connection.execute("INSERT INTO english_vocabulary_occurrences VALUES (?,?,?,?,?,?)", occurrence)
    connection.execute("INSERT OR IGNORE INTO english_vocabulary_occurrences VALUES (?,?,?,?,?,?)", ("o-2", *occurrence[1:]))
    assert connection.execute("SELECT COUNT(*) FROM english_vocabulary_occurrences").fetchone()[0] == 1


def test_delete_cascades_occurrences_and_review_logs(tmp_path: Path) -> None:
    connection = migrated_database(tmp_path)
    insert_word(connection, "word-1", "maintain")
    connection.execute("INSERT INTO english_vocabulary_occurrences VALUES ('o-1','word-1','a','A','sentence','2026-07-25')")
    connection.execute("INSERT INTO english_vocabulary_review_logs VALUES ('r-1','word-1','GOOD',0,1,'2026-07-25','2026-07-26',200)")
    connection.execute("DELETE FROM english_user_vocabulary WHERE id='word-1'")
    assert connection.execute("SELECT COUNT(*) FROM english_vocabulary_occurrences").fetchone()[0] == 0
    assert connection.execute("SELECT COUNT(*) FROM english_vocabulary_review_logs").fetchone()[0] == 0
