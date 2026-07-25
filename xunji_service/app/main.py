"""训记训练数据同步 FastAPI 服务入口。"""

import asyncio
import logging
from pathlib import Path

from fastapi import FastAPI, File, HTTPException, Request, UploadFile
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from pydantic import BaseModel, Field

from .errors import XunjiError
from .dictionary.repository import DictionaryUnavailable
from .dictionary.service import lookup_word
from .fetcher import fetch_share_page
from .models import ParseResponse
from .parser import (
    parse_dom,
    parse_embedded_json,
    parse_with_playwright,
    save_debug,
)
from .qr_decoder import MAX_IMAGE_BYTES, decode_xunji_qr
from .voa_bridge import VoaFetchError, fetch_voa_articles, health_check

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(name)s %(message)s")
logger = logging.getLogger("xunji-import")
DEBUG_ROOT = Path(__file__).resolve().parents[1] / "debug"

app = FastAPI(title="LifeTrace 本地解析与抓取服务", version="1.1.0")
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:3103", "http://127.0.0.1:3103"],
    allow_credentials=False,
    allow_methods=["GET", "POST", "PATCH", "DELETE"],
    allow_headers=["content-type"],
)


@app.exception_handler(XunjiError)
async def handle_xunji_error(_: Request, error: XunjiError) -> JSONResponse:
    logger.warning("%s: %s", error.code, error.message)
    return JSONResponse({"error": error.message, "code": error.code}, status_code=error.status_code)


@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


@app.get("/api/dictionary/lookup")
async def dictionary_lookup(word: str, articleId: str | None = None, sentence: str | None = None) -> dict[str, object]:
    try:
        return await asyncio.to_thread(lookup_word, word, articleId, sentence)
    except DictionaryUnavailable as error:
        logger.warning("Dictionary unavailable: %s", error)
        raise HTTPException(status_code=503, detail=str(error)) from error


class VoaFetchRequest(BaseModel):
    sourceKey: str = Field(default="voa_science", min_length=3, max_length=80)
    category: str = Field(default="science", pattern=r"^(science|health|words|grammar|education)$")
    mode: str = Field(default="latest", pattern=r"^(latest|history|repair|detail)$")
    limit: int = Field(default=30, ge=1, le=500)
    overlapDays: int = Field(default=14, ge=1, le=90)
    cursor: str | None = Field(default=None, max_length=40)
    requestIntervalMs: int = Field(default=1000, ge=200, le=10000)
    articleUrl: str | None = Field(default=None, max_length=2000)


@app.post("/api/voa/articles")
async def fetch_voa(request: VoaFetchRequest) -> dict[str, object]:
    try:
        return await asyncio.to_thread(
            fetch_voa_articles,
            category=request.category,
            source_key=request.sourceKey,
            mode=request.mode,
            limit=request.limit,
            overlap_days=request.overlapDays,
            cursor=request.cursor,
            request_interval_ms=request.requestIntervalMs,
            article_url=request.articleUrl,
        )
    except VoaFetchError as error:
        logger.warning("VOA fetch failed: %s", error)
        status_code = 429 if "429" in str(error) or "too many requests" in str(error).lower() else 502
        raise HTTPException(status_code=status_code, detail=str(error)) from error


class VoaHealthRequest(BaseModel):
    category: str = Field(default="science", pattern=r"^(science|health|words|grammar|education)$")


@app.post("/api/voa/health")
async def voa_health(request: VoaHealthRequest) -> dict[str, object]:
    try:
        return await asyncio.to_thread(health_check, request.category)
    except VoaFetchError as error:
        logger.warning("VOA health check failed: %s", error)
        raise HTTPException(status_code=502, detail=str(error)) from error


@app.post("/api/xunji/parse", response_model=ParseResponse)
async def parse_xunji_share(image: UploadFile = File(...)) -> ParseResponse:
    if image.content_type not in {"image/jpeg", "image/png", "image/webp", "image/bmp"}:
        raise XunjiError("请上传 JPG、PNG 或 WebP 分享图片。", "invalid_image", 400)
    payload = await image.read(MAX_IMAGE_BYTES + 1)
    share_url = await asyncio.to_thread(decode_xunji_qr, payload)
    logger.info("QR decoded: %s", share_url)
    page = await asyncio.to_thread(fetch_share_page, share_url)

    embedded = await asyncio.to_thread(parse_embedded_json, page.text)
    if embedded:
        return ParseResponse(shareUrl=share_url, workout=embedded[0], rawData=embedded[1], parser="embedded_json")

    dom = await asyncio.to_thread(parse_dom, page.text)
    if dom:
        return ParseResponse(shareUrl=share_url, workout=dom[0], rawData=dom[1], parser="dom")

    dynamic = await parse_with_playwright(share_url)
    if dynamic:
        return ParseResponse(shareUrl=share_url, workout=dynamic[0], rawData=dynamic[1], parser="playwright")

    debug_html = dynamic[3] if dynamic and len(dynamic) > 3 and dynamic[3] else page.text
    debug_path = save_debug(DEBUG_ROOT, debug_html, dynamic[2] if dynamic else [], {"url": page.url, "contentType": page.content_type})
    raise XunjiError(
        f"分享网页中没有可导入的结构化训练数据。调试文件已保存：{debug_path}",
        "structured_data_not_found",
        422,
    )
