import importlib.util
from pathlib import Path
import sys


FETCHER_PATH = Path(__file__).resolve().parents[2] / "scripts" / "fetch_voa_articles.py"
SPEC = importlib.util.spec_from_file_location("voa_fetcher", FETCHER_PATH)
assert SPEC and SPEC.loader
fetcher = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = fetcher
SPEC.loader.exec_module(fetcher)


class FakeResponse:
    def __init__(self, text: str):
        self.text = text
        self.encoding = "utf-8"
        self.apparent_encoding = "utf-8"

    def raise_for_status(self) -> None:
        return None


class FakeSession:
    def __init__(self, pages: dict[str, str]):
        self.pages = pages
        self.requested: list[str] = []

    def get(self, url: str, timeout: int):
        self.requested.append(url)
        return FakeResponse(self.pages[url])


def test_history_discovery_reads_multiple_pages(monkeypatch) -> None:
    base = fetcher.CATEGORY_PAGES["science"]
    pages = {
        base: '<a href="/a/one/1001.html">First science story</a><a href="/z/1579?p=1">Load more</a>',
        f"{base}?p=1": '<a href="/a/two/1002.html">Second science story</a>',
    }
    session = FakeSession(pages)
    entries, cursor, requests = fetcher.discover_category_entries(session, "science", 5)
    assert [entry.url for entry in entries] == [
        "https://learningenglish.voanews.com/a/one/1001.html",
        "https://learningenglish.voanews.com/a/two/1002.html",
    ]
    assert cursor is None
    assert requests == 2
