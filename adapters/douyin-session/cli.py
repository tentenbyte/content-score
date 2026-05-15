#!/usr/bin/env python3
from __future__ import annotations

import argparse
import asyncio
import importlib.util
import json
import os
import re
import sys
from pathlib import Path
from typing import Any

from normalize import build_import_row, normalize_comments, normalize_video, parse_aweme_input


CREATOR_HOME = "https://creator.douyin.com/creator-micro/home"
CREATOR_CONTENT = "https://creator.douyin.com/creator-micro/content/manage"
AUTH_DIR = Path.cwd() / ".auth"
DEBUG_DIR = Path.cwd() / ".content-score" / "douyin-debug"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Douyin Playwright adapter for content-score")
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("doctor")
    subparsers.add_parser("login")

    fetch_parser = subparsers.add_parser("fetch")
    fetch_parser.add_argument("input")
    fetch_parser.add_argument("--prediction-id", required=True)
    fetch_parser.add_argument("--output", required=True)

    args = parser.parse_args(argv)
    try:
        if args.command == "doctor":
            return doctor()
        if args.command == "login":
            return asyncio.run(login())
        if args.command == "fetch":
            return asyncio.run(fetch(args.input, args.prediction_id, Path(args.output)))
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    return 0


def doctor() -> int:
    print(f"python: {sys.version.split()[0]}")
    if sys.version_info < (3, 10):
        print("python_status: unsupported; use Python 3.10 or newer")
        return 1
    print("python_status: ok")

    if importlib.util.find_spec("playwright") is None:
        print("playwright: missing")
        print("install: python3 -m pip install -r adapters/douyin-session/requirements.txt")
        print("browser: python3 -m playwright install chromium")
        return 1
    print("playwright: installed")

    chromium = _find_cached_chromium()
    if chromium:
        print(f"chromium: found {chromium}")
    else:
        print("chromium: not detected")
        print("browser: python3 -m playwright install chromium")
    print(f"auth_dir: {AUTH_DIR}")
    return 0


async def login(timeout_s: int = 300) -> int:
    async_playwright = _load_async_playwright()
    async with async_playwright() as pw:
        AUTH_DIR.mkdir(parents=True, exist_ok=True)
        context = await pw.chromium.launch_persistent_context(
            user_data_dir=str(AUTH_DIR),
            headless=False,
            viewport={"width": 1440, "height": 900},
            args=["--disable-blink-features=AutomationControlled"],
        )
        try:
            page = await context.new_page()
            await page.goto(CREATOR_HOME, wait_until="domcontentloaded", timeout=60_000)
            print(f"login: scan or complete Douyin verification in Chromium; waiting up to {timeout_s}s")
            for second in range(timeout_s):
                cookies = await context.cookies("https://creator.douyin.com")
                if _has_session_cookie(cookies) and "login" not in page.url:
                    print(f"login: session detected after {second}s")
                    return 0
                await asyncio.sleep(1)
            print("login: timed out without detecting a creator.douyin.com session", file=sys.stderr)
            return 1
        finally:
            await context.close()


async def fetch(raw_input: str, prediction_id: str, output: Path) -> int:
    parsed = parse_aweme_input(raw_input)
    async_playwright = _load_async_playwright()
    async with async_playwright() as pw:
        AUTH_DIR.mkdir(parents=True, exist_ok=True)
        context = await pw.chromium.launch_persistent_context(
            user_data_dir=str(AUTH_DIR),
            headless=False,
            viewport={"width": 1440, "height": 900},
            args=["--disable-blink-features=AutomationControlled"],
        )
        try:
            aweme_id = await _resolve_aweme_id(context, parsed)
            print(f"aweme_id: {aweme_id}")
            raw_video, raw_comments = await _fetch_video_and_comments(context, aweme_id)
        finally:
            await context.close()

    video = normalize_video(raw_video)
    comments = normalize_comments(raw_comments)
    notes = f"douyin aweme_id={aweme_id}"
    row = build_import_row(prediction_id, video, comments, notes)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps([row], ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"output: {output}")
    print(f"top_comments: {len(comments)}")
    return 0


async def _resolve_aweme_id(context: Any, parsed: str) -> str:
    if re.fullmatch(r"\d{10,30}", parsed):
        return parsed

    print(f"resolve_short_link: {parsed}")
    page = await context.new_page()
    try:
        await page.goto(parsed, wait_until="domcontentloaded", timeout=60_000)
        await page.wait_for_timeout(2_000)
        final_url = page.url
    finally:
        await page.close()
    return parse_aweme_input(final_url)


async def _fetch_video_and_comments(context: Any, aweme_id: str) -> tuple[dict, list[dict]]:
    captured: list[dict] = []
    comments: list[dict] = []

    async def record_json_response(response: Any) -> None:
        url = response.url
        try:
            if _looks_like_video_response(url):
                data = await response.json()
                captured.append(data)
            elif "/aweme/v1/web/comment/list/" in url or ("comment" in url and "creator" in url):
                data = await response.json()
                comments.extend(_extract_comments(data, aweme_id))
        except Exception:
            return

    page = await context.new_page()
    page.on("response", record_json_response)
    try:
        print("fetch: opening creator content page")
        await page.goto(CREATOR_CONTENT, wait_until="domcontentloaded", timeout=60_000)
        await page.wait_for_timeout(6_000)
        for _ in range(3):
            await page.evaluate("window.scrollBy(0, 1400)")
            await page.wait_for_timeout(1_500)

        video = _find_video(captured, aweme_id)
        if video is None:
            print("fetch: creator list did not expose metrics; trying public video page")
            await page.goto(f"https://www.douyin.com/video/{aweme_id}", wait_until="domcontentloaded", timeout=60_000)
            await page.wait_for_timeout(6_000)
            for selector in ('[data-e2e="video-comment-more"]', '[data-e2e="feed-comment-icon"]'):
                try:
                    await page.locator(selector).first.click(force=True, timeout=3_000)
                    break
                except Exception:
                    pass
            for _ in range(12):
                await page.evaluate("window.scrollBy(0, 1500)")
                await page.wait_for_timeout(1_500)
            video = _find_video(captured, aweme_id)

        DEBUG_DIR.mkdir(parents=True, exist_ok=True)
        (DEBUG_DIR / "captured-response-count.txt").write_text(str(len(captured)), encoding="utf-8")
        if video is None:
            raise RuntimeError(
                "could not find required Douyin metrics in captured responses; "
                f"debug data written under {DEBUG_DIR}"
            )
        return video, _dedupe_comments(comments)
    finally:
        await page.close()


def _looks_like_video_response(url: str) -> bool:
    needles = (
        "work_list",
        "item/list",
        "aweme/post",
        "aweme/detail",
        "item_detail",
        "aweme_statistic",
        "data_center",
        "statistics",
    )
    return any(needle in url for needle in needles)


def _find_video(payloads: list[dict], aweme_id: str) -> dict | None:
    for payload in payloads:
        for item in _walk_dicts(payload):
            item_id = str(item.get("aweme_id") or item.get("item_id") or item.get("id") or "")
            if item_id == str(aweme_id):
                return item
    return None


def _extract_comments(payload: dict, aweme_id: str) -> list[dict]:
    found = []
    for item in _walk_dicts(payload):
        text = item.get("text") or item.get("content")
        if not text:
            continue
        item_aweme_id = item.get("aweme_id") or item.get("item_id")
        if item_aweme_id and str(item_aweme_id) != str(aweme_id):
            continue
        found.append(item)
    return found


def _walk_dicts(value: Any):
    if isinstance(value, dict):
        yield value
        for child in value.values():
            yield from _walk_dicts(child)
    elif isinstance(value, list):
        for child in value:
            yield from _walk_dicts(child)


def _dedupe_comments(comments: list[dict]) -> list[dict]:
    seen = set()
    deduped = []
    for comment in comments:
        key = comment.get("cid") or comment.get("comment_id") or comment.get("id") or comment.get("text") or comment.get("content")
        if key in seen:
            continue
        seen.add(key)
        deduped.append(comment)
    return deduped


def _has_session_cookie(cookies: list[dict]) -> bool:
    return any(cookie.get("name") in {"sessionid", "sessionid_ss"} for cookie in cookies)


def _load_async_playwright():
    try:
        from playwright.async_api import async_playwright
    except ImportError as exc:
        raise RuntimeError(
            "Playwright is not installed. Run: python3 -m pip install -r adapters/douyin-session/requirements.txt"
        ) from exc
    return async_playwright


def _find_cached_chromium() -> Path | None:
    cache_roots = [
        Path(os.environ.get("PLAYWRIGHT_BROWSERS_PATH", "")).expanduser() if os.environ.get("PLAYWRIGHT_BROWSERS_PATH") else None,
        Path.home() / ".cache" / "ms-playwright",
    ]
    for root in [path for path in cache_roots if path]:
        if not root.exists():
            continue
        for candidate in root.glob("chromium*/chrome-linux/chrome"):
            if candidate.exists():
                return candidate
    return None


if __name__ == "__main__":
    raise SystemExit(main())
