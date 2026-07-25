#!/usr/bin/env python3
"""Stream ECDICT CSV and optional lemma.en.txt into an indexed SQLite database."""

from __future__ import annotations

import argparse
import csv
import logging
import sqlite3
import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PROJECT_ROOT))
from xunji_service.app.dictionary.normalizer import normalize_word, strip_word  # noqa: E402

SCHEMA = """
CREATE TABLE IF NOT EXISTS dictionary_words (
 id INTEGER PRIMARY KEY AUTOINCREMENT, word TEXT NOT NULL, normalized_word TEXT NOT NULL,
 strip_word TEXT NOT NULL, phonetic TEXT, definition TEXT, translation TEXT, pos TEXT,
 collins INTEGER, oxford INTEGER, tag TEXT, bnc INTEGER, frq INTEGER,
 exchange TEXT, detail TEXT, audio TEXT, UNIQUE(normalized_word)
);
CREATE INDEX IF NOT EXISTS idx_dictionary_word ON dictionary_words(word COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_dictionary_normalized_word ON dictionary_words(normalized_word);
CREATE INDEX IF NOT EXISTS idx_dictionary_strip_word ON dictionary_words(strip_word);
CREATE TABLE IF NOT EXISTS dictionary_lemmas (
 word_form TEXT PRIMARY KEY, lemma TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_dictionary_lemma ON dictionary_lemmas(lemma);
"""
FIELDS = ("word", "phonetic", "definition", "translation", "pos", "collins", "oxford", "tag", "bnc", "frq", "exchange", "detail", "audio")


def integer(value: str | None) -> int | None:
    try:
        return int(value) if value else None
    except ValueError:
        return None


def import_csv(connection: sqlite3.Connection, csv_path: Path, batch_size: int) -> tuple[int, int]:
    inserted = bad = 0
    sql = """INSERT INTO dictionary_words
    (word, normalized_word, strip_word, phonetic, definition, translation, pos, collins, oxford, tag, bnc, frq, exchange, detail, audio)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    ON CONFLICT(normalized_word) DO UPDATE SET phonetic=excluded.phonetic, definition=excluded.definition,
    translation=excluded.translation, pos=excluded.pos, collins=excluded.collins, oxford=excluded.oxford,
    tag=excluded.tag, bnc=excluded.bnc, frq=excluded.frq, exchange=excluded.exchange,
    detail=excluded.detail, audio=excluded.audio"""
    batch: list[tuple[object, ...]] = []
    with csv_path.open("r", encoding="utf-8-sig", newline="") as stream:
        reader = csv.DictReader(stream)
        for line, row in enumerate(reader, 2):
            word = (row.get("word") or "").strip()
            normalized = normalize_word(word)
            if not normalized:
                bad += 1
                logging.warning("跳过异常词条，第 %d 行：%r", line, word)
                continue
            batch.append((word, normalized, strip_word(normalized), row.get("phonetic", ""),
                          row.get("definition", ""), row.get("translation", ""), row.get("pos", ""),
                          integer(row.get("collins")), integer(row.get("oxford")), row.get("tag", ""),
                          integer(row.get("bnc")), integer(row.get("frq")), row.get("exchange", ""),
                          row.get("detail", ""), row.get("audio", "")))
            if len(batch) >= batch_size:
                connection.executemany(sql, batch)
                connection.commit()
                inserted += len(batch)
                batch.clear()
                print(f"\r已处理 {inserted:,} 个词条", end="", flush=True)
    if batch:
        connection.executemany(sql, batch)
        connection.commit()
        inserted += len(batch)
    print()
    return inserted, bad


def import_lemmas(connection: sqlite3.Connection, lemma_path: Path, batch_size: int) -> int:
    count = 0
    batch: list[tuple[str, str]] = []
    with lemma_path.open("r", encoding="utf-8-sig", errors="replace") as stream:
        for raw in stream:
            line = raw.strip()
            if not line or line.startswith(";"):
                continue
            if "->" in line:
                left, right = line.split("->", 1)
                lemma = left.split("/", 1)[0].strip()
                forms = [value.strip() for value in right.split(",") if value.strip()]
            else:
                parts = line.replace("=>", " ").replace("\t", " ").split()
                if len(parts) < 2:
                    continue
                lemma, *forms = parts
            for form in forms:
                batch.append((form.lower(), lemma.lower()))
            if len(batch) >= batch_size:
                connection.executemany("INSERT OR REPLACE INTO dictionary_lemmas(word_form, lemma) VALUES (?, ?)", batch)
                connection.commit()
                count += len(batch)
                batch.clear()
    if batch:
        connection.executemany("INSERT OR REPLACE INTO dictionary_lemmas(word_form, lemma) VALUES (?, ?)", batch)
        connection.commit()
        count += len(batch)
    return count


def main() -> int:
    parser = argparse.ArgumentParser(description="将 ECDICT CSV 流式导入 LifeTrace 离线词典")
    parser.add_argument("csv", type=Path)
    parser.add_argument("--lemma", type=Path)
    parser.add_argument("--output", type=Path, default=PROJECT_ROOT / "xunji_service/data/dictionary.db")
    parser.add_argument("--batch-size", type=int, default=5000)
    parser.add_argument("--rebuild", action="store_true")
    parser.add_argument("--log", type=Path, default=PROJECT_ROOT / "xunji_service/data/dictionary-import.log")
    args = parser.parse_args()
    args.log.parent.mkdir(parents=True, exist_ok=True)
    logging.basicConfig(level=logging.WARNING, format="%(levelname)s: %(message)s", handlers=[logging.FileHandler(args.log, encoding="utf-8")])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    if args.rebuild and args.output.exists():
        args.output.unlink()
    with sqlite3.connect(args.output) as connection:
        connection.executescript(SCHEMA)
        imported, bad = import_csv(connection, args.csv, max(100, args.batch_size))
        lemmas = import_lemmas(connection, args.lemma, args.batch_size) if args.lemma else 0
        total = connection.execute("SELECT COUNT(*) FROM dictionary_words").fetchone()[0]
    print(f"导入完成：处理 {imported:,}，异常 {bad:,}，lemma {lemmas:,}，数据库词条 {total:,}")
    print(f"数据库：{args.output.resolve()}（{args.output.stat().st_size / 1024 / 1024:.1f} MB）")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
