"""VOA source adapter for the local FastAPI service."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any
from urllib.parse import urlparse

import requests


CATEGORIES = ("science", "health", "words", "grammar", "education")
AGENCY_NAMES = ("associated press", "agence france-presse", "reuters", "afp")


class VoaFetchError(RuntimeError):
    """Raised when the bundled fetcher cannot return a valid result."""


def resolve_fetch_script() -> Path:
    configured = os.environ.get("VOA_FETCH_SCRIPT")
    candidates = [
        Path(configured).expanduser() if configured else None,
        Path(__file__).resolve().parents[2] / "scripts" / "fetch_voa_articles.py",
    ]
    for candidate in candidates:
        if candidate and candidate.is_file():
            return candidate.resolve()
    raise VoaFetchError("项目中缺少 scripts/fetch_voa_articles.py")


def _allowed_article(article: dict[str, Any]) -> bool:
    source_url = str(article.get("source_url") or "")
    parsed = urlparse(source_url)
    if parsed.scheme != "https" or parsed.hostname != "learningenglish.voanews.com":
        return False
    rights_text = " ".join(
        str(article.get(field) or "") for field in ("title", "author", "summary")
    ).lower()
    return not any(name in rights_text for name in AGENCY_NAMES)


def fetch_voa_articles(
    *,
    category: str = "science",
    source_key: str | None = None,
    mode: str = "latest",
    limit: int = 30,
    overlap_days: int = 14,
    cursor: str | None = None,
    request_interval_ms: int = 1000,
    article_url: str | None = None,
    script_path: Path | None = None,
) -> dict[str, Any]:
    if category not in CATEGORIES:
        raise VoaFetchError(f"未知 VOA 栏目：{category}")
    if mode not in {"latest", "history", "repair", "detail"}:
        raise VoaFetchError(f"未知抓取模式：{mode}")
    script = script_path or resolve_fetch_script()
    safe_limit = max(1, min(int(limit), 500))
    delay = max(0.2, min(float(request_interval_ms) / 1000, 10))
    with tempfile.TemporaryDirectory(prefix=f"lifetrace-voa-{category}-") as directory:
        output_path = Path(directory) / "result.json"
        command = [
            sys.executable, str(script),
            "--category", category,
            "--source-key", source_key or f"voa_{category}",
            "--mode", "history" if mode == "history" else "latest",
            "--limit", str(safe_limit),
            "--overlap-days", str(max(1, min(int(overlap_days), 90))),
            "--delay", str(delay),
            "--timeout", "20",
            "--output", str(output_path),
        ]
        if cursor and str(cursor).isdigit():
            command.extend(["--cursor", str(cursor)])
        if article_url:
            command.extend(["--article-url", article_url])

        timeout_seconds = min(3600, max(90, safe_limit * (delay + 12)))
        options: dict[str, Any] = {
            "capture_output": True,
            "text": True,
            "encoding": "utf-8",
            "errors": "replace",
            "timeout": timeout_seconds,
            "check": False,
        }
        if os.name == "nt":
            options["creationflags"] = subprocess.CREATE_NO_WINDOW
        try:
            completed = subprocess.run(command, **options)
        except subprocess.TimeoutExpired as error:
            raise VoaFetchError(f"{category} 抓取超过 {timeout_seconds} 秒") from error
        except OSError as error:
            raise VoaFetchError(f"{category} 抓取器无法启动：{error}") from error

        if completed.returncode != 0 or not output_path.is_file():
            lines = (completed.stderr or completed.stdout or "没有生成结果").strip().splitlines()
            raise VoaFetchError(lines[-1] if lines else f"{category} 抓取失败")
        try:
            payload = json.loads(output_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise VoaFetchError(f"{category} 结果读取失败：{error}") from error

    raw_articles = payload.get("articles")
    if not isinstance(raw_articles, list):
        raise VoaFetchError(f"{category} 返回格式无效")
    articles: list[dict[str, Any]] = []
    skipped = 0
    seen: set[str] = set()
    for article in raw_articles:
        if not isinstance(article, dict) or not _allowed_article(article):
            skipped += 1
            continue
        source_url = str(article.get("source_url") or "")
        if source_url in seen:
            skipped += 1
            continue
        seen.add(source_url)
        articles.append({**article, "source_key": source_key or f"voa_{category}"})
    return {
        "source": "voa",
        "engine": "python",
        "articles": articles,
        "skipped": skipped,
        "failed": int(payload.get("failed") or 0),
        "errors": payload.get("errors") if isinstance(payload.get("errors"), list) else [],
        "discovered_count": int(payload.get("discovered_count") or len(raw_articles)),
        "next_cursor": payload.get("next_cursor"),
        "request_count": int(payload.get("request_count") or 0),
    }


def health_check(category: str = "science", script_path: Path | None = None) -> dict[str, Any]:
    result = fetch_voa_articles(
        category=category, source_key=f"voa_{category}",
        mode="latest", limit=1, request_interval_ms=200,
        script_path=script_path,
    )
    audio_ok: bool | None = None
    first = result["articles"][0] if result["articles"] else {}
    audio_url = str(first.get("audio_url") or "")
    if audio_url:
        try:
            response = requests.head(
                audio_url,
                headers={"User-Agent": "LifeTrace/1.0 educational health check"},
                timeout=10,
                allow_redirects=True,
            )
            audio_ok = response.status_code < 400
        except requests.RequestException:
            audio_ok = False
    return {
        "ok": bool(result["articles"]),
        "rate_limited": False,
        "detail": (
            f"发现 {result['discovered_count']} 篇，成功解析 {len(result['articles'])} 篇"
            + (f"，音频样本{'有效' if audio_ok else '失效'}" if audio_ok is not None else "")
        ),
    }
