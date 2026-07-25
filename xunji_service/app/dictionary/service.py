"""Application-level dictionary lookup with source sentence context."""

from __future__ import annotations

import os
from pathlib import Path
from typing import Any

from .repository import DictionaryRepository

DEFAULT_DATABASE = Path(__file__).resolve().parents[2] / "data" / "dictionary.db"


def database_path() -> Path:
    configured = os.environ.get("LIFETRACE_DICTIONARY_DB")
    return Path(configured).expanduser().resolve() if configured else DEFAULT_DATABASE


repository = DictionaryRepository(database_path())


def lookup_word(word: str, article_id: str | None = None, sentence: str | None = None) -> dict[str, Any]:
    result = repository.lookup(word)
    result["articleId"] = article_id
    result["sourceSentence"] = (sentence or "").strip()
    return result

