from __future__ import annotations

import sys
import unittest
from pathlib import Path


ADAPTER_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ADAPTER_DIR))

from normalize import (  # noqa: E402
    build_import_row,
    normalize_comments,
    normalize_video,
    parse_aweme_input,
)


class NormalizeTests(unittest.TestCase):
    def test_parse_aweme_input_accepts_raw_id(self) -> None:
        self.assertEqual(parse_aweme_input("7333333333333333333"), "7333333333333333333")

    def test_parse_aweme_input_accepts_long_url(self) -> None:
        self.assertEqual(
            parse_aweme_input("https://www.douyin.com/video/7333333333333333333?previous_page=app_code_link"),
            "7333333333333333333",
        )
        self.assertEqual(
            parse_aweme_input("https://douyin.com/video/7444444444444444444"),
            "7444444444444444444",
        )

    def test_parse_aweme_input_accepts_short_url_unchanged(self) -> None:
        self.assertEqual(
            parse_aweme_input("https://v.douyin.com/abc123/"),
            "https://v.douyin.com/abc123/",
        )

    def test_normalize_video_maps_required_metrics(self) -> None:
        video = normalize_video(
            {
                "aweme_id": "7333333333333333333",
                "statistics": {
                    "play_count": "1200",
                    "digg_count": 80,
                    "comment_count": 12,
                    "share_count": 4,
                    "collect_count": 9,
                },
            }
        )

        self.assertEqual(
            video,
            {
                "plays": 1200,
                "likes": 80,
                "comments": 12,
                "shares": 4,
                "saves": 9,
            },
        )

    def test_normalize_comments_sorts_by_like_count(self) -> None:
        comments = normalize_comments(
            [
                {"text": "low", "digg_count": 1},
                {"content": "top", "like_count": 30},
                {"text": "middle", "digg_count": 10},
                {"text": "   ", "digg_count": 100},
            ],
            limit=2,
        )

        self.assertEqual(comments, ["top", "middle"])

    def test_normalize_comments_uses_like_count_when_digg_count_missing_or_none(self) -> None:
        comments = normalize_comments(
            [
                {"text": "missing digg", "like_count": 10},
                {"text": "none digg", "digg_count": None, "like_count": 20},
                {"text": "zero digg", "digg_count": 0, "like_count": 30},
            ]
        )

        self.assertEqual(comments, ["none digg", "missing digg", "zero digg"])

    def test_missing_required_metric_raises_clear_error(self) -> None:
        with self.assertRaisesRegex(ValueError, "missing required metric: saves"):
            normalize_video(
                {
                    "statistics": {
                        "play_count": 1200,
                        "digg_count": 80,
                        "comment_count": 12,
                        "share_count": 4,
                    }
                }
            )

    def test_non_numeric_required_metric_raises_clear_error(self) -> None:
        with self.assertRaisesRegex(ValueError, "not numeric"):
            normalize_video(
                {
                    "statistics": {
                        "play_count": "many",
                        "digg_count": 80,
                        "comment_count": 12,
                        "share_count": 4,
                        "collect_count": 9,
                    }
                }
            )

    def test_build_import_row_returns_standard_retro_object(self) -> None:
        row = build_import_row(
            "prediction-1",
            {"plays": 1200, "likes": 80, "comments": 12, "shares": 4, "saves": 9},
            ["top"],
            "douyin aweme_id=7333333333333333333",
        )

        self.assertEqual(
            row,
            {
                "prediction_id": "prediction-1",
                "plays": 1200,
                "likes": 80,
                "comments": 12,
                "shares": 4,
                "saves": 9,
                "top_comments": ["top"],
                "notes": "douyin aweme_id=7333333333333333333",
            },
        )


if __name__ == "__main__":
    unittest.main()
