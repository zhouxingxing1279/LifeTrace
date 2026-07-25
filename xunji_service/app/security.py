"""分享链接白名单校验，阻止该解析器被用作任意网址代理。"""

from urllib.parse import urljoin, urlparse

from .errors import XunjiError

ALLOWED_HOST = "api.xunjiapp.cn"


def validate_share_url(value: str, base: str | None = None) -> str:
    candidate = urljoin(base, value) if base else value
    parsed = urlparse(candidate)
    if parsed.scheme != "https" or parsed.hostname != ALLOWED_HOST:
        raise XunjiError("二维码不是有效的训记分享链接。", "invalid_share_url", 400)
    if parsed.username or parsed.password or parsed.port not in (None, 443):
        raise XunjiError("训记分享链接包含不允许的地址信息。", "invalid_share_url", 400)
    if parsed.path != "/app_share" and not parsed.path.startswith("/app_share/"):
        raise XunjiError("二维码不是训记训练分享页面。", "invalid_share_url", 400)
    return candidate
