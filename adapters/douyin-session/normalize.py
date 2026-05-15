from __future__ import annotations

import re
from urllib.parse import urlparse


_RAW_AWEME_RE = re.compile(r"^\d{10,30}$")
_LONG_VIDEO_RE = re.compile(r"/video/(\d{10,30})(?:[/?#]|$)")

_METRIC_ALIASES = {
    "plays": ("plays", "play_count", "play_cnt", "play", "view_count", "video_play_count"),
    "likes": ("likes", "digg_count", "like_count", "digg_cnt", "liked_count"),
    "comments": ("comments", "comment_count", "comment_cnt", "reply_count"),
    "shares": ("shares", "share_count", "share_cnt", "forward_count"),
    "saves": ("saves", "collect_count", "collection_count", "save_count", "favorite_count", "favorited_count"),
}

_METRIC_CONTAINERS = ("statistics", "stats", "statistic", "item_stats", "data")


def parse_aweme_input(raw: str) -> str:
    """Return an aweme id, or a v.douyin.com short URL for later live resolution."""
    value = raw.strip()
    if _RAW_AWEME_RE.fullmatch(value):
        return value

    parsed = urlparse(value)
    host = parsed.netloc.lower()
    if host.startswith("www."):
        host = host[4:]

    if host == "v.douyin.com":
        return value

    if host == "douyin.com":
        match = _LONG_VIDEO_RE.search(parsed.path)
        if match:
            return match.group(1)

    raise ValueError(
        "unsupported Douyin input: expected raw aweme id, douyin.com/video/<id>, "
        "or v.douyin.com short link"
    )


def normalize_video(raw: dict) -> dict:
    result = {}
    for metric, aliases in _METRIC_ALIASES.items():
        value = _find_metric(raw, aliases)
        if value is None:
            raise ValueError(f"missing required metric: {metric}")
        result[metric] = _parse_count(value, metric)
    return result


def normalize_comments(raw_comments: list[dict], limit: int = 20) -> list[str]:
    ranked = []
    for index, comment in enumerate(raw_comments):
        text = comment.get("text") or comment.get("content") or ""
        text = str(text).strip()
        if not text:
            continue
        likes = comment.get("digg_count", comment.get("like_count", 0))
        try:
            like_count = _parse_count(likes, "comment likes")
        except ValueError:
            like_count = 0
        ranked.append((like_count, index, text))

    ranked.sort(key=lambda item: (-item[0], item[1]))
    return [text for _, _, text in ranked[:limit]]


def build_import_row(prediction_id: str, video: dict, comments: list[str], notes: str) -> dict:
    return {
        "prediction_id": prediction_id,
        "plays": video["plays"],
        "likes": video["likes"],
        "comments": video["comments"],
        "shares": video["shares"],
        "saves": video["saves"],
        "top_comments": comments,
        "notes": notes,
    }


def _find_metric(raw: dict, aliases: tuple[str, ...]):
    for source in _metric_sources(raw):
        for alias in aliases:
            if alias in source and source[alias] is not None:
                return source[alias]
    return None


def _metric_sources(raw: dict) -> list[dict]:
    sources = [raw]
    for key in _METRIC_CONTAINERS:
        value = raw.get(key)
        if isinstance(value, dict):
            sources.append(value)
    return sources


def _parse_count(value, metric: str) -> int:
    if isinstance(value, bool):
        raise ValueError(f"required metric is not numeric: {metric}")
    if isinstance(value, int):
        return value
    if isinstance(value, float):
        if not value.is_integer():
            raise ValueError(f"required metric is not an integer: {metric}")
        return int(value)
    if isinstance(value, str):
        normalized = value.strip().replace(",", "")
        if not normalized:
            raise ValueError(f"required metric is not numeric: {metric}")
        multiplier = 1
        suffix = normalized[-1].lower()
        if suffix == "万":
            multiplier = 10_000
            normalized = normalized[:-1]
        elif suffix == "亿":
            multiplier = 100_000_000
            normalized = normalized[:-1]
        elif suffix == "k":
            multiplier = 1_000
            normalized = normalized[:-1]
        elif suffix == "m":
            multiplier = 1_000_000
            normalized = normalized[:-1]
        try:
            parsed = float(normalized)
        except ValueError as exc:
            raise ValueError(f"required metric is not numeric: {metric}") from exc
        return int(parsed * multiplier)
    raise ValueError(f"required metric is not numeric: {metric}")
