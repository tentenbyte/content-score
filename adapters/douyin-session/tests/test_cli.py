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


if __name__ == "__main__":
    unittest.main()
