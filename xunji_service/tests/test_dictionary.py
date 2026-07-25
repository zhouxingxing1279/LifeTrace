import sqlite3
import time
from pathlib import Path

from scripts.import_ecdict import SCHEMA, import_csv, import_lemmas
from xunji_service.app.dictionary.normalizer import normalize_word, strip_word
from xunji_service.app.dictionary.repository import DictionaryRepository


def build_dictionary(tmp_path: Path) -> DictionaryRepository:
    source = Path("xunji_service/data/starter_dictionary.csv")
    lemmas = Path("xunji_service/data/starter_lemmas.txt")
    database = tmp_path / "dictionary.db"
    with sqlite3.connect(database) as connection:
        connection.executescript(SCHEMA)
        import_csv(connection, source, 100)
        import_lemmas(connection, lemmas, 100)
    return DictionaryRepository(database)


def test_normalizes_punctuation_possessive_and_hyphen() -> None:
    assert normalize_word("“Maintaining,”") == "maintaining"
    assert normalize_word("children's") == "children"
    assert normalize_word("Well-known") == "well-known"
    assert normalize_word("123") is None
    assert strip_word("long-time") == "longtime"


def test_csv_import_is_repeatable_without_duplicates(tmp_path: Path) -> None:
    database = tmp_path / "dictionary.db"
    with sqlite3.connect(database) as connection:
        connection.executescript(SCHEMA)
        import_csv(connection, Path("xunji_service/data/starter_dictionary.csv"), 100)
        import_csv(connection, Path("xunji_service/data/starter_dictionary.csv"), 100)
        assert connection.execute("SELECT COUNT(*) FROM dictionary_words").fetchone()[0] == 15


def test_lookup_is_case_insensitive_and_returns_metadata(tmp_path: Path) -> None:
    result = build_dictionary(tmp_path).lookup("MAINTAIN")
    assert result["found"] is True
    assert result["lemma"] == "maintain"
    assert result["phonetic"] == "meɪnˈteɪn"
    assert result["oxford"] is True


def test_lemma_lookup_handles_regular_and_irregular_forms(tmp_path: Path) -> None:
    repository = build_dictionary(tmp_path)
    assert repository.lookup("maintaining")["lemma"] == "maintain"
    assert repository.lookup("went")["lemma"] == "go"
    assert repository.lookup("children")["lemma"] == "child"
    assert repository.lookup("better")["lemma"] == "good"
    assert repository.lookup("taken")["lemma"] == "take"


def test_not_found_is_a_normal_result(tmp_path: Path) -> None:
    result = build_dictionary(tmp_path).lookup("zzzzunknown")
    assert result["found"] is False
    assert result["reason"] == "NOT_FOUND"


def test_indexed_cached_lookup_is_fast(tmp_path: Path) -> None:
    repository = build_dictionary(tmp_path)
    started = time.perf_counter()
    for _ in range(500):
        assert repository.lookup("maintaining")["found"]
    assert time.perf_counter() - started < 1
