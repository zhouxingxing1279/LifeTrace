"""Indexed, cached access to the generated ECDICT SQLite database."""

from __future__ import annotations

import json
import sqlite3
from functools import lru_cache
from pathlib import Path
from typing import Any

from .normalizer import normalize_word, strip_word, suffix_candidates

WORD_COLUMNS = """id, word, normalized_word, strip_word, phonetic, definition,
translation, pos, collins, oxford, tag, bnc, frq, exchange, detail, audio"""


class DictionaryUnavailable(RuntimeError):
    pass


class DictionaryRepository:
    def __init__(self, database_path: Path):
        self.database_path = database_path

    def _connect(self) -> sqlite3.Connection:
        if not self.database_path.is_file():
            raise DictionaryUnavailable("离线词典尚未初始化，请先导入 ECDICT 数据。")
        connection = sqlite3.connect(f"file:{self.database_path.as_posix()}?mode=ro", uri=True)
        connection.row_factory = sqlite3.Row
        return connection

    @lru_cache(maxsize=300)
    def lookup(self, query: str) -> dict[str, Any]:
        normalized = normalize_word(query)
        if not normalized:
            return {"queryWord": query, "normalizedWord": "", "found": False, "reason": "INVALID_WORD"}
        with self._connect() as connection:
            row, lemma = self._find(connection, normalized)
        if not row:
            return {"queryWord": query, "normalizedWord": normalized, "lemma": lemma or normalized, "found": False, "reason": "NOT_FOUND"}
        return self._serialize(query, normalized, lemma or row["normalized_word"], row)

    def _find(self, connection: sqlite3.Connection, word: str) -> tuple[sqlite3.Row | None, str | None]:
        stripped = strip_word(word)
        lemma_row = connection.execute(
            "SELECT lemma FROM dictionary_lemmas WHERE word_form = ? COLLATE NOCASE LIMIT 1", (word,)
        ).fetchone()
        lemma = lemma_row["lemma"] if lemma_row else None
        for field, value in (("word", word), ("normalized_word", word), ("strip_word", stripped)):
            row = connection.execute(
                f"SELECT {WORD_COLUMNS} FROM dictionary_words WHERE {field} = ? COLLATE NOCASE LIMIT 1",
                (value,),
            ).fetchone()
            if row:
                if lemma and lemma != word:
                    lemma_definition = connection.execute(
                        f"SELECT {WORD_COLUMNS} FROM dictionary_words WHERE normalized_word = ? COLLATE NOCASE LIMIT 1",
                        (lemma,),
                    ).fetchone()
                    if lemma_definition:
                        return lemma_definition, lemma
                return row, lemma or row["normalized_word"]
        candidates = [lemma] if lemma else []
        candidates.extend(suffix_candidates(word))
        for candidate in dict.fromkeys(filter(None, candidates)):
            row = connection.execute(
                f"SELECT {WORD_COLUMNS} FROM dictionary_words WHERE normalized_word = ? COLLATE NOCASE LIMIT 1",
                (candidate,),
            ).fetchone()
            if row:
                return row, candidate
        return None, lemma

    @staticmethod
    def _lines(value: str | None) -> list[str]:
        return [line.strip() for line in (value or "").replace("\\n", "\n").splitlines() if line.strip()]

    def _serialize(self, query: str, normalized: str, lemma: str, row: sqlite3.Row) -> dict[str, Any]:
        translations = self._lines(row["translation"])
        definitions = self._lines(row["definition"])
        pos = row["pos"] or ""
        if not pos and translations:
            prefix = translations[0].split(".", 1)[0].lower()
            pos = {"v": "verb", "n": "noun", "adj": "adjective", "adv": "adverb"}.get(prefix, prefix)
        exchange: dict[str, str] = {}
        for item in (row["exchange"] or "").split("/"):
            if ":" in item:
                key, value = item.split(":", 1)
                exchange[key] = value
        return {
            "queryWord": query,
            "normalizedWord": normalized,
            "lemma": lemma,
            "found": True,
            "dictionaryWordId": row["id"],
            "phonetic": row["phonetic"] or "",
            "partsOfSpeech": [{"type": pos or "unknown", "translation": translations, "definition": definitions}],
            "collins": row["collins"] or 0,
            "oxford": bool(row["oxford"]),
            "tags": (row["tag"] or "").split(),
            "bncRank": row["bnc"],
            "frequencyRank": row["frq"],
            "exchange": exchange,
            "detail": json.loads(row["detail"]) if (row["detail"] or "").startswith("{") else row["detail"],
            "audio": row["audio"] or None,
        }
