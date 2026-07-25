import json
from pathlib import Path

from xunji_service.app.voa_bridge import fetch_voa_articles, resolve_fetch_script


def test_uses_fetcher_stored_inside_project() -> None:
    script = resolve_fetch_script()
    assert script.name == "fetch_voa_articles.py"
    assert script.parent.name == "scripts"
    assert script.is_file()


def make_fake_fetcher(path: Path) -> None:
    path.write_text(
        """
import argparse
import json

parser = argparse.ArgumentParser()
for name in ("category", "limit", "delay", "timeout", "output", "source-key", "mode", "cursor", "overlap-days", "article-url"):
    parser.add_argument("--" + name)
args = parser.parse_args()
article = {
    "source_key": args.source_key,
    "external_id": "1001",
    "source_url": "https://learningenglish.voanews.com/a/sample/1001.html",
    "title": "Science sample",
    "author": "",
    "summary": "",
    "content": "word " * 220,
    "word_count": 220,
    "category": args.category,
    "license_type": "VOA terms apply",
    "attribution": "VOA Learning English",
}
with open(args.output, "w", encoding="utf-8") as stream:
    json.dump({
        "articles": [article],
        "discovered_count": 2,
        "next_cursor": "3",
        "request_count": 2,
        "failed": 1,
        "errors": [{"sourceUrl": "https://learningenglish.voanews.com/a/bad/1002.html", "error": "mock failure"}],
    }, stream)
""",
        encoding="utf-8",
    )


def test_runs_configured_source_and_preserves_partial_failures(tmp_path: Path) -> None:
    script = tmp_path / "fetch_voa_articles.py"
    make_fake_fetcher(script)
    result = fetch_voa_articles(
        category="science", source_key="voa_science", mode="history",
        limit=100, cursor="2", request_interval_ms=200, script_path=script,
    )
    assert result["engine"] == "python"
    assert len(result["articles"]) == 1
    assert result["articles"][0]["source_key"] == "voa_science"
    assert result["failed"] == 1
    assert result["next_cursor"] == "3"
