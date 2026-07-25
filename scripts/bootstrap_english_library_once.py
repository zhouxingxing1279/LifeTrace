#!/usr/bin/env python3
"""One-off local bootstrap for the English article library.

Run this only after the local API and Xunji service are available. The script
starts the configured five-source backfill, waits for the persisted task to
finish, and verifies that the library reached its configured target.
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode
from urllib.request import Request, urlopen


TERMINAL_STATUSES = {"COMPLETED", "PARTIAL_SUCCESS", "FAILED", "CANCELLED"}


def request_json(url: str, method: str = "GET", payload: dict | None = None) -> dict:
    body = json.dumps(payload).encode("utf-8") if payload is not None else None
    request = Request(
        url,
        data=body,
        method=method,
        headers={"Content-Type": "application/json", "User-Agent": "LifeTrace-English-Bootstrap/1.0"},
    )
    try:
        with urlopen(request, timeout=30) as response:
            return json.load(response)
    except HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"{method} {url} returned HTTP {error.code}: {detail}") from error
    except URLError as error:
        raise RuntimeError(f"Cannot reach {url}: {error.reason}") from error


def main() -> int:
    parser = argparse.ArgumentParser(description="Build the one-time English article inventory.")
    parser.add_argument("--base-url", default="http://127.0.0.1:3103")
    parser.add_argument("--poll-seconds", type=float, default=3)
    parser.add_argument("--minimum-total", type=int, default=500)
    args = parser.parse_args()
    base_url = args.base_url.rstrip("/")

    result = request_json(
        f"{base_url}/api/english/sync/backfill",
        method="POST",
        payload={"force": True},
    )
    task_id = result.get("taskId")
    if not task_id:
        raise RuntimeError(result.get("reason") or "The backfill API did not return a task ID.")

    print(f"Started one-off English library bootstrap: {task_id}", flush=True)
    last_line = None
    while True:
        query = urlencode({"taskId": task_id})
        task = request_json(f"{base_url}/api/english/sync/status?{query}")["task"]
        line = (
            f"{task['status']} {round(float(task['progress']) * 100)}% | "
            f"processed={int(task['successCount']) + int(task['failedCount'])}/"
            f"{int(task['totalCount'])} inserted={int(task['insertedCount'])} "
            f"updated={int(task['updatedCount'])} skipped={int(task['skippedCount'])} "
            f"failed={int(task['failedCount'])}"
        )
        if line != last_line:
            print(line, flush=True)
            last_line = line
        if task["status"] in TERMINAL_STATUSES:
            break
        time.sleep(max(args.poll_seconds, 0.5))

    stats = request_json(f"{base_url}/api/english/articles/stats")
    total = int(stats["total"])
    target = int(stats["initialization"]["targetArticleCount"])
    print(
        f"Library total={total}, configured target={target}, "
        f"initialization={stats['initialization']['status']}",
        flush=True,
    )
    if task["status"] not in {"COMPLETED", "PARTIAL_SUCCESS"}:
        raise RuntimeError(task.get("lastError") or f"Bootstrap ended with {task['status']}.")
    if stats["initialization"]["status"] != "completed":
        raise RuntimeError("Initialization state was not persisted as completed.")
    if total < args.minimum_total:
        raise RuntimeError(f"Only {total} articles are present; expected at least {args.minimum_total}.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (RuntimeError, KeyError, ValueError) as error:
        print(f"ERROR: {error}", file=sys.stderr, flush=True)
        raise SystemExit(1)
