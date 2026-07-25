import json

from xunji_service.app.parser import normalize_workout, parse_embedded_json
from xunji_service.app.errors import XunjiError
from xunji_service.app.fetcher import _decode_page
from xunji_service.app.security import validate_share_url
from xunji_service.app.qr_decoder import decode_xunji_qr
import pytest
import cv2
import numpy as np


def test_allows_only_xunji_share_urls():
    assert validate_share_url("https://api.xunjiapp.cn/app_share/abc").endswith("/abc")
    assert validate_share_url(
        "https://api.xunjiapp.cn/app_share?spid=share-token&localid=1784887281937"
    ).endswith("localid=1784887281937")
    with pytest.raises(XunjiError):
        validate_share_url("https://example.com/app_share/abc")


def test_normalizes_embedded_training_json():
    html = """<script id="__NEXT_DATA__" type="application/json">
    {"workout":{"title":"胸部训练","date":"2026-07-24","durationMinutes":60,
    "exercises":[{"name":"卧推","sets":[{"weight":80,"reps":8},{"weight":90,"reps":6}]}]}}
    </script>"""
    result = parse_embedded_json(html)
    assert result is not None
    workout, _ = result
    assert workout.title == "胸部训练"
    assert workout.volumeKg == 1180
    assert workout.exercises[0].sets[1].weightKg == 90


def test_rejects_non_workout_shapes():
    assert normalize_workout({"article": {"title": "not training"}}) is None


def test_defaults_xunji_html_without_charset_to_utf8():
    assert _decode_page("臀腿 · 深蹲".encode(), "text/html", "ISO-8859-1") == "臀腿 · 深蹲"


def test_parses_legacy_window_train_payload():
    movement = json.dumps(
        [
            {
                "key": "squat",
                "label": "深蹲",
                "type": "腿",
                "sets": [
                    {"done": True, "reps": "5", "unit": "kg", "weight": "90"},
                    {"done": True, "reps": "5", "unit": "kg", "weight": "90"},
                ],
            },
            {
                "key": "legpress_machine",
                "label": "腿举",
                "sets": [{"done": False, "reps": "12", "unit": "kg", "weight": "155"}],
            },
        ],
        ensure_ascii=False,
        separators=(",", ":"),
    )
    escaped_movement = movement.replace("\\", "\\\\").replace('"', '\\"').replace("'", "\\'")
    html = f"""<!doctype html><html><body><script>
    window.Train={{
      movement:JSON.parse('{escaped_movement}'),
      title:"臀腿",
      datestr:"2026-07-24"
    }};
    window.user={{avatar:"https://example.com/avatar.jpg",nickname:"Alfred"}};
    </script><div id="root"></div></body></html>"""

    result = parse_embedded_json(html)

    assert result is not None
    workout, raw = result
    assert workout.title == "臀腿"
    assert workout.date == "2026-07-24"
    assert workout.volumeKg == 2760
    assert [exercise.name for exercise in workout.exercises] == ["深蹲", "腿举"]
    assert workout.exercises[0].sets[1].weightKg == 90
    assert raw["movement"][1]["sets"][0]["done"] is False


def test_decodes_rotated_qr_from_compressed_screenshot_bottom():
    url = "https://api.xunjiapp.cn/app_share/test-workout"
    qr = cv2.QRCodeEncoder_create().encode(url)
    qr = cv2.resize(qr, (210, 210), interpolation=cv2.INTER_NEAREST)
    qr = cv2.rotate(qr, cv2.ROTATE_90_CLOCKWISE)
    canvas = np.full((1100, 720), 255, dtype=np.uint8)
    canvas[850:1060, 255:465] = qr
    ok, encoded = cv2.imencode(".jpg", canvas, [cv2.IMWRITE_JPEG_QUALITY, 65])
    assert ok
    assert decode_xunji_qr(encoded.tobytes()) == url
