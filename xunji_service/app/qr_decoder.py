"""只识别二维码，不读取或分析图片中的任何文字。"""

from collections.abc import Iterable

import cv2
import numpy as np

from .errors import XunjiError
from .security import validate_share_url

MAX_IMAGE_BYTES = 15 * 1024 * 1024


def _variants(image: np.ndarray) -> Iterable[np.ndarray]:
    """覆盖截图底部二维码、压缩、缩放和旋转等常见分享图形态。"""
    height = image.shape[0]
    bases = [image, image[int(height * 0.4):], image[int(height * 0.6):]]
    for base in bases:
        for turns in range(4):
            rotated = np.rot90(base, turns).copy()
            for scale in (1.0, 1.5, 2.0, 3.0, 0.75):
                resized = cv2.resize(rotated, None, fx=scale, fy=scale, interpolation=cv2.INTER_CUBIC)
                yield resized
                gray = cv2.cvtColor(resized, cv2.COLOR_BGR2GRAY)
                yield gray
                yield cv2.adaptiveThreshold(
                    gray, 255, cv2.ADAPTIVE_THRESH_GAUSSIAN_C, cv2.THRESH_BINARY, 31, 5
                )


def _opencv_values(image: np.ndarray) -> list[str]:
    detector = cv2.QRCodeDetector()
    values: list[str] = []
    try:
        ok, decoded, _, _ = detector.detectAndDecodeMulti(image)
        if ok:
            values.extend(value for value in decoded if value)
    except cv2.error:
        pass
    try:
        value, _, _ = detector.detectAndDecode(image)
        if value:
            values.append(value)
    except cv2.error:
        pass
    return values


def _pyzbar_values(image: np.ndarray) -> list[str]:
    # Windows 缺少 zbar 动态库时继续使用 OpenCV，不让整个服务无法启动。
    try:
        from pyzbar.pyzbar import decode
        return [item.data.decode("utf-8", errors="ignore") for item in decode(image)]
    except (ImportError, OSError):
        return []


def decode_xunji_qr(payload: bytes) -> str:
    if not payload or len(payload) > MAX_IMAGE_BYTES:
        raise XunjiError("图片为空或超过 15MB。", "invalid_image", 400)
    image = cv2.imdecode(np.frombuffer(payload, dtype=np.uint8), cv2.IMREAD_COLOR)
    if image is None:
        raise XunjiError("无法读取图片，请上传 JPG、PNG 或手机截图。", "invalid_image", 400)

    for variant in _variants(image):
        for value in [*_pyzbar_values(variant), *_opencv_values(variant)]:
            try:
                return validate_share_url(value.strip())
            except XunjiError:
                continue
    raise XunjiError("请上传包含训记二维码的分享图片。", "qr_not_found", 422)

