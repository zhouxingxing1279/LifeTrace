"""安全获取训记页面，所有跳转都重新执行域名白名单校验。"""

from dataclasses import dataclass
import re

import requests

from .errors import XunjiError
from .security import validate_share_url

MAX_PAGE_BYTES = 6 * 1024 * 1024


@dataclass
class PageResponse:
    url: str
    text: str
    content_type: str


def _decode_page(payload: bytes, content_type: str, fallback_encoding: str | None) -> str:
    charset = re.search(r"charset\s*=\s*[\"']?([^;\"'\s]+)", content_type, re.IGNORECASE)
    encoding = charset.group(1) if charset else "utf-8"
    try:
        return payload.decode(encoding)
    except (LookupError, UnicodeDecodeError):
        return payload.decode(fallback_encoding or "utf-8", errors="replace")


def fetch_share_page(url: str) -> PageResponse:
    current = validate_share_url(url)
    session = requests.Session()
    session.headers.update({
        "User-Agent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 LifeTrace-Xunji-Importer/1.0",
        "Accept": "text/html,application/xhtml+xml,application/json",
    })
    try:
        for _ in range(4):
            response = session.get(current, timeout=(5, 15), allow_redirects=False, stream=True)
            if response.is_redirect or response.is_permanent_redirect:
                location = response.headers.get("location")
                if not location:
                    break
                current = validate_share_url(location, current)
                continue
            if response.status_code >= 400:
                raise XunjiError("训记分享链接失效。", "share_unavailable", 422)
            chunks: list[bytes] = []
            size = 0
            for chunk in response.iter_content(64 * 1024):
                size += len(chunk)
                if size > MAX_PAGE_BYTES:
                    raise XunjiError("训记分享页面数据过大。", "page_too_large", 422)
                chunks.append(chunk)
            payload = b"".join(chunks)
            content_type = response.headers.get("content-type", "")
            return PageResponse(
                url=validate_share_url(response.url),
                text=_decode_page(payload, content_type, response.encoding),
                content_type=content_type,
            )
    except requests.RequestException as exc:
        raise XunjiError("训记分享链接失效。", "share_unavailable", 422) from exc
    raise XunjiError("训记分享链接失效。", "share_unavailable", 422)
