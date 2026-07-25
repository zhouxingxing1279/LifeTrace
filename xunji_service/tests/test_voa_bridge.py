import json
from pathlib import Path

from xunji_service.app.voa_bridge import fetch_voa_articles, resolve_fetch_script


def test_uses_fetcher_stored_inside_project() -> None:
    script = resolve_fetch_script()

    assert script.name == "fetch_voa_articles.py"
    assert script.parent.name == "scripts"
    assert script.is_file()


def test_runs_user_fetcher_for_three_categories(tmp_path: Path) -> None:
    script = tmp_path / "fetch_voa_articles.py"
    script.write_text(
        """
import argparse
import json

parser = argparse.ArgumentParser()
parser.add_argument("--category")
parser.add_argument("--limit")
parser.add_argument("--delay")
parser.add_argument("--timeout")
parser.add_argument("--output")
args = parser.parse_args()
ids = {"science": "1001", "health": "1002", "words": "1003"}
title = "Reuters sample" if args.category == "words" else f"{args.category.title()} sample"
article = {
    "source_url": f"https://learningenglish.voanews.com/a/{ids[args.category]}.html",
    "title": title,
    "author": "",
    "summary": "",
    "content": "word " * 80,
    "word_count": 80,
}
with open(args.output, "w", encoding="utf-8") as stream:
    json.dump({"articles": [article]}, stream)
""",
        encoding="utf-8",
    )

    result = fetch_voa_articles(limit_per_feed=1, script_path=script)

    assert result["engine"] == "python"
    assert len(result["articles"]) == 2
    assert result["skipped"] == 1
    assert result["failed"] == 0
