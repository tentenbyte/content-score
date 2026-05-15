from __future__ import annotations

import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


ADAPTER_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ADAPTER_DIR))

import cli  # noqa: E402


class CliTests(unittest.TestCase):
    def test_find_cached_chromium_detects_playwright_chrome_for_testing_layout(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            chrome = root / "chromium-1217" / "chrome-linux64" / "chrome"
            chrome.parent.mkdir(parents=True)
            chrome.write_text("", encoding="utf-8")

            with patch.dict(os.environ, {"PLAYWRIGHT_BROWSERS_PATH": str(root)}):
                self.assertEqual(cli._find_cached_chromium(), chrome)


class FetchNavigationTests(unittest.IsolatedAsyncioTestCase):
    async def test_fetch_recovers_from_navigation_during_scroll_before_reporting_missing_metrics(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            page = _FakePage(raise_on_scroll=1)
            context = _FakeContext(page)

            with patch.object(cli, "debug_dir", return_value=Path(temp_dir) / "debug"):
                with self.assertRaisesRegex(RuntimeError, "could not find required Douyin metrics"):
                    await cli._fetch_video_and_comments(context, "7333333333333333333")

            self.assertGreater(page.scrolls, 1)
            self.assertIn(("domcontentloaded", 10_000), page.load_state_waits)


class _FakeContext:
    def __init__(self, page: "_FakePage") -> None:
        self.page = page

    async def new_page(self) -> "_FakePage":
        return self.page


class _FakePage:
    def __init__(self, raise_on_scroll: int) -> None:
        self.raise_on_scroll = raise_on_scroll
        self.scrolls = 0
        self.load_state_waits: list[tuple[str, int]] = []

    def on(self, _event: str, _callback) -> None:
        return None

    async def goto(self, _url: str, **_kwargs) -> None:
        return None

    async def wait_for_timeout(self, _timeout_ms: int) -> None:
        return None

    async def wait_for_load_state(self, state: str, timeout: int) -> None:
        self.load_state_waits.append((state, timeout))

    async def evaluate(self, _script: str) -> None:
        self.scrolls += 1
        if self.scrolls == self.raise_on_scroll:
            raise RuntimeError("Execution context was destroyed, most likely because of a navigation")

    def locator(self, _selector: str) -> "_FakeLocator":
        return _FakeLocator()

    async def close(self) -> None:
        return None


class _FakeLocator:
    @property
    def first(self) -> "_FakeLocator":
        return self

    async def click(self, **_kwargs) -> None:
        return None


if __name__ == "__main__":
    unittest.main()
