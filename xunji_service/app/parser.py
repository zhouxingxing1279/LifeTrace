"""按嵌入 JSON、动态接口和 DOM 三层策略提取训练数据。"""

from __future__ import annotations

import json
import re
from contextlib import suppress
from datetime import date, datetime
from pathlib import Path
from typing import Any
from urllib.parse import urlparse

from bs4 import BeautifulSoup

from .errors import XunjiError
from .models import Workout, WorkoutExercise, WorkoutSet
from .security import validate_share_url

WORKOUT_KEYS = {"workout", "training", "exercise", "exercises", "sets", "weight", "reps", "volume"}


def _number(value: Any, default: float = 0) -> float:
    if isinstance(value, (int, float)):
        return float(value)
    match = re.search(r"-?\d+(?:\.\d+)?", str(value or ""))
    return float(match.group()) if match else default


def _first(data: dict[str, Any], *keys: str, default: Any = None) -> Any:
    lowered = {str(key).lower(): value for key, value in data.items()}
    for key in keys:
        if key.lower() in lowered and lowered[key.lower()] not in (None, ""):
            return lowered[key.lower()]
    return default


def _date(value: Any) -> str:
    text = str(value or "")
    if text.isdigit() and len(text) >= 10:
        stamp = int(text[:13])
        if len(text) == 10:
            stamp *= 1000
        return datetime.fromtimestamp(stamp / 1000).date().isoformat()
    match = re.search(r"(20\d{2})[-/.年](\d{1,2})[-/.月](\d{1,2})", text)
    if match:
        return f"{int(match.group(1)):04d}-{int(match.group(2)):02d}-{int(match.group(3)):02d}"
    return date.today().isoformat()


def _find_lists(node: Any, keys: tuple[str, ...]) -> list[Any]:
    if isinstance(node, dict):
        for key, value in node.items():
            if key.lower() in keys and isinstance(value, list):
                return value
        for value in node.values():
            found = _find_lists(value, keys)
            if found:
                return found
    elif isinstance(node, list):
        for value in node:
            found = _find_lists(value, keys)
            if found:
                return found
    return []


def _normalize_exercises(node: Any) -> list[WorkoutExercise]:
    raw_exercises = _find_lists(node, ("exercises", "exercise_list", "trainingitems", "actions", "movement", "movements"))
    exercises: list[WorkoutExercise] = []
    for index, item in enumerate(raw_exercises):
        if not isinstance(item, dict):
            continue
        name = str(_first(item, "name", "label", "exerciseName", "actionName", "title", "cnName", default=f"动作 {index + 1}")).strip()
        raw_sets = _find_lists(item, ("sets", "setlist", "groups", "details", "records"))
        sets: list[WorkoutSet] = []
        for set_index, set_item in enumerate(raw_sets):
            if not isinstance(set_item, dict):
                continue
            weight = _number(_first(set_item, "weightKg", "weight", "kg", "load", default=0))
            reps = int(_number(_first(set_item, "reps", "rep", "times", "count", "number", default=0)))
            sets.append(WorkoutSet(weightKg=max(0, weight), reps=max(0, reps), setNumber=set_index + 1))
        if sets:
            exercises.append(WorkoutExercise(name=name, sets=sets))
    return exercises


def normalize_workout(raw: Any) -> Workout | None:
    if not isinstance(raw, (dict, list)):
        return None
    exercises = _normalize_exercises(raw)
    if not exercises:
        return None
    root = raw if isinstance(raw, dict) else {}
    workout_nodes = [root]
    if isinstance(raw, dict):
        workout_nodes.extend(value for key, value in raw.items() if key.lower() in ("workout", "training", "data") and isinstance(value, dict))
    selected = next((node for node in reversed(workout_nodes) if _normalize_exercises(node)), root)
    title = str(_first(selected, "title", "name", "workoutName", "trainingName", default="训记训练")).strip()
    duration = _number(_first(selected, "durationMinutes", "duration", "trainingTime", "time", default=0))
    if duration > 24 * 60:
        duration /= 60
    calories = _number(_first(selected, "caloriesKcal", "calories", "kcal", "heat", default=0))
    computed_volume = sum(item.weightKg * item.reps for exercise in exercises for item in exercise.sets)
    volume = _number(_first(selected, "volumeKg", "volume", "totalVolume", "capacity", default=computed_volume), computed_volume)
    return Workout(
        date=_date(_first(selected, "date", "datestr", "occurredAt", "startTime", "trainingDate", "createdAt")),
        title=title or "训记训练",
        durationMinutes=max(0, round(duration)),
        caloriesKcal=max(0, calories),
        volumeKg=max(0, volume or computed_volume),
        exercises=exercises,
    )


def _balanced_json(text: str, start: int) -> str | None:
    opening = text[start]
    closing = "}" if opening == "{" else "]"
    depth = 0
    quoted = False
    escaped = False
    for index in range(start, len(text)):
        char = text[index]
        if quoted:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                quoted = False
        elif char == '"':
            quoted = True
        elif char == opening:
            depth += 1
        elif char == closing:
            depth -= 1
            if depth == 0:
                return text[start:index + 1]
    return None


def _decode_javascript_string(value: str) -> str:
    """Decode JavaScript string contents without executing page-provided code."""
    try:
        return json.loads(f'"{value}"')
    except json.JSONDecodeError:
        # JSON does not recognize JavaScript's escaped single quote. The
        # source payload is a single-quoted literal, so it is safe to normalize.
        normalized = value.replace("\\'", "'")
        return json.loads(f'"{normalized}"')


def _javascript_double_quoted_property(source: str, name: str) -> str | None:
    match = re.search(
        rf'{re.escape(name)}\s*:\s*"((?:\\.|[^"\\])*)"',
        source,
        re.DOTALL,
    )
    return _decode_javascript_string(match.group(1)) if match else None


def xunji_window_train_candidate(html: str) -> dict[str, Any] | None:
    """Extract Xunji's legacy ``window.Train`` payload from the HTML response."""
    soup = BeautifulSoup(html, "html.parser")
    for script in soup.find_all("script"):
        source = script.string or script.get_text()
        if "window.Train" not in source:
            continue
        movement_match = re.search(
            r"""movement\s*:\s*JSON\.parse\('((?:\\.|[^'\\])*)'\)""",
            source,
            re.DOTALL,
        )
        if not movement_match:
            continue
        try:
            movements = json.loads(_decode_javascript_string(movement_match.group(1)))
        except (json.JSONDecodeError, TypeError):
            continue
        if not isinstance(movements, list):
            continue
        return {
            "train": {
                "movement": movements,
                "title": _javascript_double_quoted_property(source, "title"),
                "datestr": _javascript_double_quoted_property(source, "datestr"),
            },
            "user": {
                "avatar": _javascript_double_quoted_property(source, "avatar"),
                "nickname": _javascript_double_quoted_property(source, "nickname"),
            },
        }
    return None


def embedded_json_candidates(html: str) -> list[Any]:
    soup = BeautifulSoup(html, "html.parser")
    candidates: list[Any] = []
    window_train = xunji_window_train_candidate(html)
    if window_train:
        candidates.append(window_train["train"])
    try:
        candidates.append(json.loads(html))
    except json.JSONDecodeError:
        pass
    for script in soup.find_all("script"):
        body = script.string or script.get_text()
        if script.get("id") == "__NEXT_DATA__" or script.get("type") in ("application/json", "application/ld+json"):
            try:
                candidates.append(json.loads(body))
            except (json.JSONDecodeError, TypeError):
                pass
    for marker in ("window.__INITIAL_STATE__", "window.INITIAL_STATE", "__NEXT_DATA__"):
        position = html.find(marker)
        if position < 0:
            continue
        start_candidates = [value for value in (html.find("{", position), html.find("[", position)) if value >= 0]
        if not start_candidates:
            continue
        block = _balanced_json(html, min(start_candidates))
        if block:
            try:
                candidates.append(json.loads(block))
            except json.JSONDecodeError:
                pass
    return candidates


def parse_embedded_json(html: str) -> tuple[Workout, Any] | None:
    for candidate in embedded_json_candidates(html):
        workout = normalize_workout(candidate)
        if workout:
            return workout, candidate
    return None


def parse_dom(html: str) -> tuple[Workout, Any] | None:
    """最终兜底：只读网页 DOM 文本，不读取分享图片中的文字。"""
    soup = BeautifulSoup(html, "html.parser")
    exercises: list[dict[str, Any]] = []
    selectors = [
        "[class*='exercise']", "[class*='movement']", "[class*='action']",
        "[data-exercise]", ".training-item",
    ]
    seen: set[str] = set()
    for element in soup.select(",".join(selectors)):
        text = " ".join(element.stripped_strings)
        if not text or text in seen:
            continue
        seen.add(text)
        name_node = element.select_one("[class*='name'],h2,h3,h4")
        name = " ".join(name_node.stripped_strings) if name_node else text.split(" ", 1)[0]
        pairs = re.findall(r"(\d+(?:\.\d+)?)\s*(?:kg|KG|千克)\s*[×xX*]\s*(\d+)", text)
        if pairs:
            exercises.append({"name": name, "sets": [{"weight": weight, "reps": reps} for weight, reps in pairs]})
    if not exercises:
        return None
    all_text = " ".join(soup.stripped_strings)
    raw = {
        "title": (soup.title.string.strip() if soup.title and soup.title.string else "训记训练"),
        "date": all_text,
        "duration": re.search(r"(?:时长|训练时间)\s*[:：]?\s*(\d+)", all_text).group(1) if re.search(r"(?:时长|训练时间)\s*[:：]?\s*(\d+)", all_text) else 0,
        "calories": re.search(r"(\d+)\s*(?:kcal|千卡)", all_text, re.I).group(1) if re.search(r"(\d+)\s*(?:kcal|千卡)", all_text, re.I) else 0,
        "volume": re.search(r"(?:容量|总容量)\s*[:：]?\s*(\d+(?:\.\d+)?)", all_text).group(1) if re.search(r"(?:容量|总容量)\s*[:：]?\s*(\d+(?:\.\d+)?)", all_text) else 0,
        "exercises": exercises,
    }
    workout = normalize_workout(raw)
    return (workout, raw) if workout else None


async def parse_with_playwright(url: str) -> tuple[Workout, Any, list[Any], str] | None:
    try:
        from playwright.async_api import async_playwright
    except ImportError:
        return None
    responses: list[Any] = []
    html = ""
    browser = None
    try:
        async with async_playwright() as playwright:
            browser = await playwright.chromium.launch(
                headless=True,
                args=["--disable-dev-shm-usage", "--disable-gpu"],
            )
            page = await browser.new_page(
                user_agent=(
                    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) "
                    "AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 "
                    "Mobile/15E148 Safari/604.1"
                ),
            )
            page.set_default_timeout(12_000)

            async def allow_xunji_only(route: Any) -> None:
                parsed = urlparse(route.request.url)
                if route.request.resource_type in {"image", "media", "font", "stylesheet"}:
                    await route.abort()
                elif parsed.scheme in ("data", "blob") or (parsed.scheme == "https" and parsed.hostname == "api.xunjiapp.cn"):
                    await route.continue_()
                else:
                    await route.abort()

            async def capture(response: Any) -> None:
                content_type = response.headers.get("content-type", "")
                if "application/json" in content_type and response.url.startswith("https://api.xunjiapp.cn/"):
                    try:
                        responses.append(await response.json())
                    except Exception:
                        pass

            await page.route("**/*", allow_xunji_only)
            page.on("response", capture)
            await page.goto(validate_share_url(url), wait_until="domcontentloaded", timeout=15_000)
            await page.wait_for_timeout(2_500)
            html = await page.content()
    except Exception:
        pass
    finally:
        if browser:
            with suppress(Exception):
                await browser.close()
    for candidate in responses:
        workout = normalize_workout(candidate)
        if workout:
            return workout, candidate, responses, html
    embedded = parse_embedded_json(html)
    if embedded:
        return embedded[0], embedded[1], responses, html
    return None


def save_debug(debug_root: Path, html: str, network: list[Any], response: Any) -> str:
    stamp = datetime.now().strftime("%Y%m%d-%H%M%S-%f")
    target = debug_root / stamp
    target.mkdir(parents=True, exist_ok=True)
    (target / "page.html").write_text(html, encoding="utf-8")
    (target / "network.json").write_text(json.dumps(network, ensure_ascii=False, indent=2, default=str), encoding="utf-8")
    (target / "response.json").write_text(json.dumps(response, ensure_ascii=False, indent=2, default=str), encoding="utf-8")
    return str(target)


def ensure_structured(result: tuple[Workout, Any] | None, debug_path: str | None = None) -> tuple[Workout, Any]:
    if result:
        return result
    suffix = f" 调试文件：{debug_path}" if debug_path else ""
    raise XunjiError(f"分享网页中没有可导入的结构化训练数据。{suffix}", "structured_data_not_found", 422)
