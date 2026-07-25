#!/usr/bin/env python3
"""Fetch English-learning articles from VOA Learning English.

Examples:
    python fetch_voa_articles.py --category science --limit 5
    python fetch_voa_articles.py --category health --limit 10 --output health.json
    python fetch_voa_articles.py --feed-url "https://.../api/..." --limit 3
    python fetch_voa_articles.py --article-url "https://learningenglish.voanews.com/a/...html"

Dependencies:
    pip install requests beautifulsoup4
"""

from __future__ import annotations

import argparse
import hashlib
import json
import logging
import re
import sys
import time
import xml.etree.ElementTree as ET
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
from pathlib import Path
from typing import Any, Iterable
from urllib.parse import parse_qsl, urlencode, urljoin, urlparse, urlunparse

import requests
from bs4 import BeautifulSoup, Tag
from requests.adapters import HTTPAdapter
from urllib3.util.retry import Retry


BASE_URL = "https://learningenglish.voanews.com"

# These URLs are the official RSS endpoints linked from VOA Learning English's
# RSS page. --feed-url can be used when you want to add another official feed.
FEEDS: dict[str, str] = {
    "science": f"{BASE_URL}/api/zmg_pl-vomx-tpeymtm",
    "health": f"{BASE_URL}/api/zmmpql-vomx-tpey-_q",
    "words": f"{BASE_URL}/api/zmypyl-vomx-tpeyry_",
}

CATEGORY_PAGES: dict[str, str] = {
    "science": f"{BASE_URL}/z/1579",
    "health": f"{BASE_URL}/z/955",
    "words": f"{BASE_URL}/z/987/episodes",
    "grammar": f"{BASE_URL}/z/4456/episodes",
    "education": f"{BASE_URL}/z/959",
}

DEFAULT_HEADERS = {
    "User-Agent": (
        "PersonalEnglishLearningApp/1.0 "
        "(+local educational use; replace-with-your-contact@example.com)"
    ),
    "Accept-Language": "en-US,en;q=0.9",
}

BLOCKED_TEXT_PATTERNS = (
    "embed",
    "the code has been copied to your clipboard",
    "no media source currently available",
    "direct link",
    "pop-out player",
    "share",
    "subscribe",
    "follow us",
    "breaking news",
)

ARTICLE_SELECTORS = (
    '[data-qa="article-body"]',
    ".article-body",
    ".story-body",
    ".wsw",
    ".content-floated-wrap",
    "main article",
    "article",
)


@dataclass(slots=True)
class FeedEntry:
    title: str
    url: str
    guid: str | None = None
    published_at: str | None = None
    summary: str | None = None
    audio_url: str | None = None


@dataclass(slots=True)
class Article:
    source_key: str
    external_id: str
    source_name: str
    source_url: str
    category: str | None
    title: str
    author: str | None
    published_at: str | None
    source_updated_at: str | None
    summary: str | None
    content: str
    word_count: int
    audio_url: str | None
    image_url: str | None
    language: str
    fetched_at: str
    rights_note: str
    license_type: str
    attribution: str


class FetchError(RuntimeError):
    """Raised when a remote resource cannot be fetched or parsed."""


def build_session() -> requests.Session:
    retry = Retry(
        total=3,
        connect=3,
        read=3,
        backoff_factor=0.8,
        status_forcelist=(429, 500, 502, 503, 504),
        allowed_methods=frozenset({"GET"}),
        respect_retry_after_header=True,
    )
    adapter = HTTPAdapter(max_retries=retry)
    session = requests.Session()
    session.headers.update(DEFAULT_HEADERS)
    session.mount("https://", adapter)
    session.mount("http://", adapter)
    return session


def fetch_text(session: requests.Session, url: str, timeout: int = 20) -> str:
    try:
        response = session.get(url, timeout=timeout)
        response.raise_for_status()
    except requests.RequestException as exc:
        raise FetchError(f"请求失败: {url}: {exc}") from exc

    if not response.encoding or response.encoding.lower() == "iso-8859-1":
        response.encoding = response.apparent_encoding or "utf-8"
    return response.text


def normalize_url(value: str) -> str:
    parsed = urlparse(value.strip())
    tracking = {"fbclid", "gclid", "mc_cid", "mc_eid", "ref", "source"}
    query = [
        (key, item)
        for key, item in parse_qsl(parsed.query, keep_blank_values=True)
        if not key.lower().startswith("utm_") and key.lower() not in tracking
    ]
    path = re.sub(r"/+", "/", parsed.path)
    if path != "/":
        path = path.rstrip("/")
    return urlunparse((
        parsed.scheme.lower(), parsed.netloc.lower(), path, "",
        urlencode(sorted(query)), "",
    ))


def strip_html(value: str | None) -> str | None:
    if not value:
        return None
    text = BeautifulSoup(value, "html.parser").get_text(" ", strip=True)
    text = re.sub(r"\s+", " ", text).strip()
    return text or None


def first_text(element: ET.Element, names: Iterable[str]) -> str | None:
    """Find a child by local XML name, ignoring namespaces."""
    for child in element.iter():
        local_name = child.tag.rsplit("}", 1)[-1].lower()
        if local_name in names and child.text and child.text.strip():
            return child.text.strip()
    return None


def parse_rss(xml_text: str, base_url: str = BASE_URL) -> list[FeedEntry]:
    try:
        root = ET.fromstring(xml_text)
    except ET.ParseError as exc:
        raise FetchError(f"RSS XML 解析失败: {exc}") from exc

    entries: list[FeedEntry] = []
    for item in root.iter():
        if item.tag.rsplit("}", 1)[-1].lower() not in {"item", "entry"}:
            continue

        title = first_text(item, {"title"}) or "Untitled"
        guid = first_text(item, {"guid", "id"})
        published_raw = first_text(item, {"pubdate", "published", "updated"})
        summary = strip_html(first_text(item, {"description", "summary", "content"}))

        link: str | None = None
        audio_url: str | None = None
        for child in item.iter():
            local_name = child.tag.rsplit("}", 1)[-1].lower()
            href = child.attrib.get("href") or child.attrib.get("url")
            rel = child.attrib.get("rel", "")
            mime_type = child.attrib.get("type", "")

            if local_name == "link":
                candidate = href or (child.text.strip() if child.text else None)
                if candidate and (not link or rel in {"", "alternate"}):
                    link = candidate
            elif local_name in {"enclosure", "content"} and href:
                if "audio" in mime_type or href.lower().split("?", 1)[0].endswith(".mp3"):
                    audio_url = href

        if not link:
            logging.warning("跳过没有链接的 RSS 条目: %s", title)
            continue

        entries.append(
            FeedEntry(
                title=title.strip(),
                url=urljoin(base_url, link.strip()),
                guid=guid,
                published_at=normalize_date(published_raw),
                summary=summary,
                audio_url=audio_url,
            )
        )

    return entries


def discover_category_entries(
    session: requests.Session,
    category: str,
    limit: int,
    start_page: int = 0,
    timeout: int = 20,
) -> tuple[list[FeedEntry], int | None, int]:
    """Discover article links from VOA category pagination."""
    page_url = CATEGORY_PAGES[category]
    entries: list[FeedEntry] = []
    seen: set[str] = set()
    page = max(0, start_page)
    request_count = 0
    while len(entries) < limit:
        url = page_url if page == 0 else f"{page_url}?p={page}"
        soup = BeautifulSoup(fetch_text(session, url, timeout=timeout), "html.parser")
        request_count += 1
        before = len(entries)
        for anchor in soup.select('a[href*="/a/"]'):
            absolute = normalize_url(urljoin(BASE_URL, str(anchor.get("href") or "")))
            parsed = urlparse(absolute)
            if parsed.hostname != urlparse(BASE_URL).hostname:
                continue
            if "/amp/" in parsed.path or not re.search(r"/a/(?:[^/]+/)?\d+\.html$", parsed.path):
                continue
            if absolute in seen:
                continue
            title = normalize_paragraph(anchor.get_text(" ", strip=True))
            if len(title) < 5:
                continue
            seen.add(absolute)
            entries.append(FeedEntry(title=title, url=absolute))
            if len(entries) >= limit:
                break
        next_path = f"{urlparse(page_url).path}?p={page + 1}"
        has_next = any(str(anchor.get("href") or "") == next_path for anchor in soup.select("a[href]"))
        if len(entries) == before or not has_next:
            return entries, None, request_count
        page += 1
    return entries, page, request_count


def normalize_date(value: str | None) -> str | None:
    if not value:
        return None

    value = value.strip()
    try:
        dt = parsedate_to_datetime(value)
        if dt.tzinfo is None:
            dt = dt.replace(tzinfo=timezone.utc)
        return dt.astimezone(timezone.utc).isoformat()
    except (TypeError, ValueError, OverflowError):
        pass

    try:
        dt = datetime.fromisoformat(value.replace("Z", "+00:00"))
        if dt.tzinfo is None:
            dt = dt.replace(tzinfo=timezone.utc)
        return dt.astimezone(timezone.utc).isoformat()
    except ValueError:
        return value


def iter_json_ld_objects(data: Any) -> Iterable[dict[str, Any]]:
    if isinstance(data, dict):
        yield data
        graph = data.get("@graph")
        if graph is not None:
            yield from iter_json_ld_objects(graph)
    elif isinstance(data, list):
        for item in data:
            yield from iter_json_ld_objects(item)


def parse_json_ld(soup: BeautifulSoup) -> dict[str, Any]:
    best: dict[str, Any] = {}
    desired_types = {"article", "newsarticle", "reportagenewsarticle"}

    for script in soup.select('script[type="application/ld+json"]'):
        raw = script.string or script.get_text(strip=True)
        if not raw:
            continue
        try:
            data = json.loads(raw)
        except json.JSONDecodeError:
            continue

        for obj in iter_json_ld_objects(data):
            obj_type = obj.get("@type", "")
            types = {str(obj_type).lower()} if isinstance(obj_type, str) else {
                str(x).lower() for x in obj_type if isinstance(x, str)
            }
            if types & desired_types:
                return obj
            if not best and any(k in obj for k in ("headline", "articleBody", "datePublished")):
                best = obj
    return best


def meta_content(soup: BeautifulSoup, *selectors: str) -> str | None:
    for selector in selectors:
        node = soup.select_one(selector)
        if isinstance(node, Tag):
            value = node.get("content")
            if isinstance(value, str) and value.strip():
                return value.strip()
    return None


def author_from_json_ld(data: dict[str, Any]) -> str | None:
    author = data.get("author")
    if isinstance(author, str):
        return author.strip() or None
    if isinstance(author, dict):
        value = author.get("name")
        return str(value).strip() if value else None
    if isinstance(author, list):
        names: list[str] = []
        for item in author:
            if isinstance(item, str):
                names.append(item.strip())
            elif isinstance(item, dict) and item.get("name"):
                names.append(str(item["name"]).strip())
        return ", ".join(filter(None, names)) or None
    return None


def normalize_paragraph(text: str) -> str:
    return re.sub(r"\s+", " ", text).strip()


def is_boilerplate(text: str) -> bool:
    lowered = text.casefold().strip(" .:-")
    if not lowered:
        return True
    if any(lowered == pattern or lowered.startswith(pattern + " ") for pattern in BLOCKED_TEXT_PATTERNS):
        return True
    if re.fullmatch(r"(?:0:00\s*)+(?:\d{1,2}:\d{2}(?::\d{2})?\s*)*", text):
        return True
    if re.fullmatch(r"(?:128|64) kbps(?:\s*\|\s*MP3)?", text, re.IGNORECASE):
        return True
    return False


def extract_paragraphs(container: Tag) -> list[str]:
    paragraphs: list[str] = []
    seen: set[str] = set()

    for node in container.find_all(["p", "h2", "h3", "blockquote"], recursive=True):
        # Ignore text inside navigation, sharing, captions, and media controls.
        if node.find_parent(["nav", "aside", "footer"]):
            continue
        classes = " ".join(node.get("class", []))
        if re.search(r"caption|share|social|related|author|date|media", classes, re.I):
            continue

        text = normalize_paragraph(node.get_text(" ", strip=True))
        key = text.casefold()
        if len(text) < 20 or is_boilerplate(text) or key in seen:
            continue
        seen.add(key)
        paragraphs.append(text)

    return paragraphs


def extract_article_content(soup: BeautifulSoup, json_ld: dict[str, Any]) -> str:
    article_body = json_ld.get("articleBody")
    if isinstance(article_body, str) and len(article_body.split()) >= 80:
        body = re.sub(r"\r\n?", "\n", article_body).strip()
        body = re.sub(r"\n{3,}", "\n\n", body)
        return body

    candidates: list[tuple[int, list[str]]] = []
    for selector in ARTICLE_SELECTORS:
        for container in soup.select(selector):
            if not isinstance(container, Tag):
                continue
            paragraphs = extract_paragraphs(container)
            score = sum(len(p.split()) for p in paragraphs)
            if score:
                candidates.append((score, paragraphs))

    if not candidates:
        main = soup.find("main") or soup.body
        if isinstance(main, Tag):
            paragraphs = extract_paragraphs(main)
            candidates.append((sum(len(p.split()) for p in paragraphs), paragraphs))

    if not candidates:
        return ""

    _, best_paragraphs = max(candidates, key=lambda item: item[0])
    return "\n\n".join(best_paragraphs).strip()


def extract_audio_url(soup: BeautifulSoup, json_ld: dict[str, Any], page_url: str) -> str | None:
    candidates: list[str] = []

    for key in ("audio", "associatedMedia"):
        value = json_ld.get(key)
        if isinstance(value, str):
            candidates.append(value)
        elif isinstance(value, dict):
            for subkey in ("contentUrl", "embedUrl", "url"):
                if value.get(subkey):
                    candidates.append(str(value[subkey]))

    meta_audio = meta_content(
        soup,
        'meta[property="og:audio"]',
        'meta[property="og:audio:url"]',
        'meta[name="twitter:player:stream"]',
    )
    if meta_audio:
        candidates.append(meta_audio)

    for node in soup.select("audio[src], audio source[src], a[href]"):
        value = node.get("src") or node.get("href")
        if not isinstance(value, str):
            continue
        clean = value.lower().split("?", 1)[0]
        node_type = str(node.get("type", "")).lower()
        if clean.endswith((".mp3", ".m4a", ".ogg")) or "audio" in node_type:
            candidates.append(value)

    for candidate in candidates:
        if candidate.strip():
            return urljoin(page_url, candidate.strip())
    return None


def stable_id(url: str, guid: str | None = None) -> str:
    numeric_id = re.search(r"/(\d+)\.html(?:$|\?)", url)
    if numeric_id:
        return numeric_id.group(1)
    source = normalize_url(url)
    return hashlib.sha256(source.encode("utf-8")).hexdigest()[:24]


def parse_article_html(
    html: str,
    page_url: str,
    category: str | None = None,
    feed_entry: FeedEntry | None = None,
    source_key: str | None = None,
) -> Article:
    soup = BeautifulSoup(html, "html.parser")
    json_ld = parse_json_ld(soup)

    h1_title = normalize_paragraph(soup.h1.get_text(" ", strip=True)) if soup.h1 else ""
    title = (
        str(json_ld.get("headline", "")).strip()
        or meta_content(soup, 'meta[property="og:title"]', 'meta[name="twitter:title"]')
        or h1_title
    )
    if not title and feed_entry:
        title = feed_entry.title
    if not title:
        raise FetchError(f"无法从页面中提取标题: {page_url}")

    content = extract_article_content(soup, json_ld)
    if len(content.split()) < 50:
        raise FetchError(f"正文过短或提取失败（{len(content.split())} words）: {page_url}")

    author = author_from_json_ld(json_ld) or meta_content(soup, 'meta[name="author"]')
    published_at = normalize_date(str(json_ld.get("datePublished", "")).strip() or None)
    if not published_at:
        published_at = normalize_date(
            meta_content(
                soup,
                'meta[property="article:published_time"]',
                'meta[name="date"]',
            )
        )
    if not published_at and feed_entry:
        published_at = feed_entry.published_at

    summary = (
        str(json_ld.get("description", "")).strip()
        or meta_content(soup, 'meta[name="description"]', 'meta[property="og:description"]')
        or (feed_entry.summary if feed_entry else None)
    )
    summary = normalize_paragraph(summary) if summary else None

    image_value = json_ld.get("image")
    image_url: str | None = None
    if isinstance(image_value, str):
        image_url = urljoin(page_url, image_value)
    elif isinstance(image_value, dict) and image_value.get("url"):
        image_url = urljoin(page_url, str(image_value["url"]))
    elif isinstance(image_value, list) and image_value:
        first = image_value[0]
        if isinstance(first, str):
            image_url = urljoin(page_url, first)
        elif isinstance(first, dict) and first.get("url"):
            image_url = urljoin(page_url, str(first["url"]))
    if not image_url:
        meta_image = meta_content(soup, 'meta[property="og:image"]')
        image_url = urljoin(page_url, meta_image) if meta_image else None

    audio_url = extract_audio_url(soup, json_ld, page_url)
    if not audio_url and feed_entry:
        audio_url = feed_entry.audio_url

    return Article(
        source_key=source_key or f"voa_{category or 'learning'}",
        external_id=stable_id(page_url, feed_entry.guid if feed_entry else None),
        source_name="VOA Learning English",
        source_url=page_url,
        category=category,
        title=title,
        author=author,
        published_at=published_at,
        source_updated_at=normalize_date(str(json_ld.get("dateModified", "")).strip() or None),
        summary=summary,
        content=content,
        word_count=len(re.findall(r"\b[\w'-]+\b", content)),
        audio_url=audio_url,
        image_url=image_url,
        language="en",
        fetched_at=datetime.now(timezone.utc).isoformat(),
        rights_note=(
            "Verify reuse rights before redistribution. VOA-produced material may be reusable, "
            "but third-party text, images, audio, or video can have separate rights."
        ),
        license_type="VOA terms apply",
        attribution=f"VOA Learning English — {page_url}",
    )


def fetch_article(
    session: requests.Session,
    url: str,
    category: str | None = None,
    feed_entry: FeedEntry | None = None,
    source_key: str | None = None,
) -> Article:
    html = fetch_text(session, url)
    return parse_article_html(
        html, normalize_url(url), category=category,
        feed_entry=feed_entry, source_key=source_key,
    )


def write_json(path: Path, articles: list[Article]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "source": "VOA Learning English",
        "count": len(articles),
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "articles": [asdict(article) for article in articles],
    }
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="获取 VOA Learning English 英文文章")
    parser.add_argument(
        "--category",
        choices=sorted(CATEGORY_PAGES),
        default="science",
        help="内置栏目，默认 science",
    )
    parser.add_argument("--feed-url", help="自定义 RSS 地址；提供后会覆盖 --category")
    parser.add_argument("--article-url", help="只抓取一个文章详情页，不读取 RSS")
    parser.add_argument("--limit", type=int, default=5, help="最多抓取多少篇，默认 5")
    parser.add_argument("--source-key", help="Stable content source key")
    parser.add_argument("--mode", choices=("latest", "history", "detail"), default="latest")
    parser.add_argument("--cursor", type=int, default=0, help="History pagination cursor")
    parser.add_argument("--overlap-days", type=int, default=14, help="Incremental overlap window")
    parser.add_argument("--output", default="voa_articles.json", help="输出 JSON 文件")
    parser.add_argument("--delay", type=float, default=1.0, help="详情页请求间隔秒数，默认 1")
    parser.add_argument("--timeout", type=int, default=20, help="单次请求超时秒数，默认 20")
    parser.add_argument("--verbose", action="store_true", help="输出详细日志")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    logging.basicConfig(
        level=logging.DEBUG if args.verbose else logging.INFO,
        format="%(levelname)s: %(message)s",
    )

    if args.limit <= 0:
        print("--limit 必须大于 0", file=sys.stderr)
        return 2
    if args.delay < 0:
        print("--delay 不能小于 0", file=sys.stderr)
        return 2

    session = build_session()
    articles: list[Article] = []
    failures: list[dict[str, Any]] = []
    entries: list[FeedEntry] = []
    next_cursor: int | None = None
    request_count = 0

    try:
        if args.article_url:
            logging.info("抓取文章: %s", args.article_url)
            article = fetch_article(
                session, args.article_url, category=args.category,
                source_key=args.source_key,
            )
            articles.append(article)
            request_count = 1
        else:
            if args.mode == "latest":
                feed_url = args.feed_url or FEEDS.get(args.category)
                if feed_url:
                    logging.info("读取 RSS: %s", feed_url)
                    entries.extend(parse_rss(fetch_text(session, feed_url, timeout=args.timeout)))
                    request_count += 1
                page_entries, _, page_requests = discover_category_entries(
                    session, args.category, args.limit, timeout=args.timeout,
                )
                entries.extend(page_entries)
                request_count += page_requests
            else:
                entries, next_cursor, request_count = discover_category_entries(
                    session, args.category, args.limit,
                    start_page=args.cursor, timeout=args.timeout,
                )

            unique_entries: dict[str, FeedEntry] = {}
            for entry in entries:
                unique_entries.setdefault(normalize_url(entry.url), entry)
            entries = list(unique_entries.values())
            if args.mode != "latest":
                entries = entries[:args.limit]
            logging.info("共发现 %d 个待处理条目", len(entries))

            seen_urls: set[str] = set()
            for index, entry in enumerate(entries, start=1):
                normalized_url = normalize_url(entry.url)
                if normalized_url in seen_urls:
                    continue
                seen_urls.add(normalized_url)

                try:
                    logging.info("[%d/%d] %s", index, len(entries), entry.title)
                    article = fetch_article(
                        session,
                        entry.url,
                        category=args.category,
                        feed_entry=entry,
                        source_key=args.source_key,
                    )
                    articles.append(article)
                except FetchError as exc:
                    logging.error("跳过文章: %s", exc)
                    failures.append({
                        "sourceUrl": normalized_url,
                        "title": entry.title,
                        "error": str(exc),
                        "retryCount": 3,
                    })

                if index < len(entries) and args.delay:
                    time.sleep(args.delay)

        if not articles and not failures:
            raise FetchError("没有发现或成功获取任何文章")

        output_path = Path(args.output).expanduser().resolve()
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(json.dumps({
            "source": "VOA Learning English",
            "count": len(articles),
            "discovered_count": len(entries) if not args.article_url else 1,
            "next_cursor": str(next_cursor) if next_cursor is not None else None,
            "request_count": request_count,
            "failed": len(failures),
            "errors": failures,
            "generated_at": datetime.now(timezone.utc).isoformat(),
            "articles": [asdict(article) for article in articles],
        }, ensure_ascii=False, indent=2), encoding="utf-8")
        logging.info("已保存 %d 篇文章到 %s", len(articles), output_path)

        for article in articles:
            print(f"- {article.title} ({article.word_count} words)")
        return 0

    except FetchError as exc:
        logging.error("%s", exc)
        return 1
    finally:
        session.close()


if __name__ == "__main__":
    raise SystemExit(main())
