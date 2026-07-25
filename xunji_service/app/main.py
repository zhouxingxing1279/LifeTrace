"""训记训练数据同步 FastAPI 服务入口。"""

import asyncio
import logging
from pathlib import Path

from fastapi import FastAPI, File, HTTPException, Request, UploadFile
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from pydantic import BaseModel, Field

from .errors import XunjiError
from .fetcher import fetch_share_page
from .models import ParseResponse
from .parser import (
    parse_dom,
    parse_embedded_json,
    parse_with_playwright,
    save_debug,
)
from .qr_decoder import MAX_IMAGE_BYTES, decode_xunji_qr
from .voa_bridge import VoaFetchError, fetch_voa_articles

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(name)s %(message)s")
logger = logging.getLogger("xunji-import")
DEBUG_ROOT = Path(__file__).resolve().parents[1] / "debug"

app = FastAPI(title="LifeTrace 本地解析与抓取服务", version="1.1.0")
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:3103", "http://127.0.0.1:3103"],
    allow_credentials=False,
    allow_methods=["GET", "POST"],
    allow_headers=["content-type"],
)


@app.exception_handler(XunjiError)
async def handle_xunji_error(_: Request, error: XunjiError) -> JSONResponse:
    logger.warning("%s: %s", error.code, error.message)
    return JSONResponse({"error": error.message, "code": error.code}, status_code=error.status_code)


@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


class VoaFetchRequest(BaseModel):
    limitPerFeed: int = Field(default=2, ge=1, le=5)


@app.post("/api/voa/articles")
async def fetch_voa(request: VoaFetchRequest) -> dict[str, object]:
    try:
        return await asyncio.to_thread(fetch_voa_articles, request.limitPerFeed)
    except VoaFetchError as error:
        logger.warning("VOA fetch failed: %s", error)
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
