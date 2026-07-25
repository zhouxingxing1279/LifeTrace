"""Normalize clicked English tokens and generate conservative fallback lemmas."""

from __future__ import annotations

import re
import unicodedata

EDGE_PUNCTUATION = """ \t\r\n.,!?;:()[]{}<>\"“”‘’`…，。！？；："""
VALID_WORD = re.compile(r"^[a-z]+(?:['-][a-z]+)*$")


def normalize_word(value: str) -> str | None:
    text = unicodedata.normalize("NFKC", value).replace("’", "'").replace("‘", "'")
    text = text.strip(EDGE_PUNCTUATION).lower()
    if text.endswith("'s") and len(text) > 2:
        text = text[:-2]
    return text if VALID_WORD.fullmatch(text) else None


def strip_word(value: str) -> str:
    return re.sub(r"[^a-z0-9]", "", value.lower())


def suffix_candidates(word: str) -> list[str]:
    candidates: list[str] = []
    if word.endswith("ies") and len(word) > 4:
        candidates.append(word[:-3] + "y")
    if word.endswith("ing") and len(word) > 5:
        stem = word[:-3]
        candidates.extend([stem, stem + "e"])
        if len(stem) > 2 and stem[-1] == stem[-2]:
            candidates.append(stem[:-1])
    if word.endswith("ed") and len(word) > 4:
        stem = word[:-2]
        candidates.extend([stem, stem + "e"])
        if stem.endswith("i"):
            candidates.append(stem[:-1] + "y")
    if word.endswith("es") and len(word) > 4:
        candidates.extend([word[:-2], word[:-1]])
    elif word.endswith("s") and len(word) > 3:
        candidates.append(word[:-1])
    return list(dict.fromkeys(candidate for candidate in candidates if VALID_WORD.fullmatch(candidate)))

