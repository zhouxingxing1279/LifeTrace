"""Run the user-provided VOA fetch script and normalize its JSON output."""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import Any
from urllib.parse import urlparse


CATEGORIES = ("science", "health", "words")
AGENCY_PATTERN = re.compile(
    r"\b(?:Associated Press|Agence France-Presse|Reuters|AFP)\b",
    re.IGNORECASE,
)
PROCESS_TIMEOUT_SECONDS = 45


class VoaFetchError(RuntimeError):
    """Raised when the Python VOA fetcher is missing or cannot return articles."""


def resolve_fetch_script() -> Path:
    configured = os.environ.get("VOA_FETCH_SCRIPT")
    candidates = [
        Path(configured).expanduser() if configured else None,
        Path("D:/Download/fetch_voa_articles.py"),
        Path.home() / "Downloads" / "fetch_voa_articles.py",
        Path(__file__).resolve().parents[2] / "scripts" / "fetch_voa_articles.py",
    ]
    for candidate in candidates:
        if candidate and candidate.is_file():
            return candidate.resolve()
    raise VoaFetchError(
        "未找到 fetch_voa_articles.py，请保留 D:/Download/fetch_voa_articles.py "
        "或通过 VOA_FETCH_SCRIPT 指定位置。"
    )


def _run_category(script_path: Path, category: str, limit: int) -> tuple[list[dict[str, Any]], str | None]:
    with tempfile.TemporaryDirectory(prefix=f"lifetrace-voa-{category}-") as directory:
        output_path = Path(directory) / f"{category}.json"
        command = [
            sys.executable,
            str(script_path),
            "--category",
            category,
            "--limit",
            str(limit),
            "--delay",
            "0",
            "--timeout",
            "8",
            "--output",
            str(output_path),
        ]
        options: dict[str, Any] = {
            "capture_output": True,
            "text": True,
            "encoding": "utf-8",
            "errors": "replace",
            "timeout": PROCESS_TIMEOUT_SECONDS,
            "check": False,
        }
        if os.name == "nt":
            options["creationflags"] = subprocess.CREATE_NO_WINDOW
        try:
            completed = subprocess.run(command, **options)
        except subprocess.TimeoutExpired:
            return [], f"{category} 抓取超过 {PROCESS_TIMEOUT_SECONDS} 秒"
        except OSError as error:
            return [], f"{category} 无法启动：{error}"

        if completed.returncode != 0 or not output_path.is_file():
            detail = (completed.stderr or completed.stdout or "没有生成结果").strip().splitlines()
            return [], f"{category} 抓取失败：{detail[-1] if detail else '未知错误'}"
        try:
            payload = json.loads(output_path.read_text(encoding="utf-8"))
            articles = payload.get("articles")
            if not isinstance(articles, list):
                return [], f"{category} 返回格式无效"
            return [
                {**article, "category": category}
                for article in articles
                if isinstance(article, dict)
            ], None
        except (OSError, json.JSONDecodeError) as error:
            return [], f"{category} 结果读取失败：{error}"


def _is_allowed_article(article: dict[str, Any]) -> bool:
    source_url = str(article.get("source_url") or "")
    parsed = urlparse(source_url)
    if parsed.scheme != "https" or parsed.hostname != "learningenglish.voanews.com":
        return False
    rights_text = " ".join(
        str(article.get(field) or "")
        for field in ("title", "author", "summary")
    )
    return not AGENCY_PATTERN.search(rights_text)


def fetch_voa_articles(limit_per_feed: int = 2, script_path: Path | None = None) -> dict[str, Any]:
    script = script_path or resolve_fetch_script()
    limit = max(1, min(int(limit_per_feed), 5))
    collected: list[dict[str, Any]] = []
    errors: list[str] = []

    # 三个独立脚本进程并行运行；单个栏目失败不会阻塞已经成功的结果。
    with ThreadPoolExecutor(max_workers=len(CATEGORIES)) as executor:
        futures = {
            executor.submit(_run_category, script, category, limit): category
            for category in CATEGORIES
        }
        for future in as_completed(futures):
            articles, error = future.result()
            collected.extend(articles)
            if error:
                errors.append(error)

    unique: dict[str, dict[str, Any]] = {}
    skipped = 0
    for article in collected:
        source_url = str(article.get("source_url") or "")
        if not _is_allowed_article(article) or source_url in unique:
            skipped += 1
            continue
        unique[source_url] = article

    if not unique and errors:
        raise VoaFetchError("；".join(errors))
    return {
        "source": "voa",
        "engine": "python",
        "articles": list(unique.values()),
        "skipped": skipped,
        "failed": len(errors),
        "errors": errors,
    }
