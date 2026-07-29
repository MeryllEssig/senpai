#!/usr/bin/env python3
"""Regression tests for the bundled Redmine adapter."""
from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


SCRIPT = Path(__file__).parents[1] / "skills/senpai-project-management/scripts/redmine.py"
SPEC = importlib.util.spec_from_file_location("senpai_redmine", SCRIPT)
assert SPEC and SPEC.loader
redmine = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(redmine)


class Response:
    def __init__(self, body: bytes) -> None:
        self.body = body

    def __enter__(self) -> "Response":
        return self

    def __exit__(self, *_: object) -> None:
        return None

    def read(self, limit: int = -1) -> bytes:
        return self.body if limit < 0 else self.body[:limit]


class RedmineAttachmentTests(unittest.TestCase):
    def test_get_issue_downloads_attachments_and_rewrites_references(self) -> None:
        remote = "https://redmine.example/attachments/42/screenshot.png"
        issue = {
            "issue": {
                "description": f"Screenshot: {remote}",
                "journals": [{"notes": f"See {remote}"}],
                "attachments": [{"filename": "screenshot.png", "content_url": remote, "thumbnail_url": remote + "?thumb=1"}],
            }
        }

        def urlopen(request: object, timeout: float) -> Response:
            url = request.full_url  # type: ignore[attr-defined]
            self.assertEqual(request.get_header("X-redmine-api-key"), "secret")  # type: ignore[attr-defined]
            if url.endswith(".json?include=journals,relations,attachments"):
                return Response(json.dumps(issue).encode())
            if url == remote:
                return Response(b"image bytes")
            self.fail(f"unexpected URL: {url}")

        with tempfile.TemporaryDirectory() as directory, patch.object(redmine.urllib.request, "urlopen", side_effect=urlopen):
            client = redmine.Redmine("https://redmine.example", "secret", 1, 1024)
            result = redmine.read_issue_with_attachments("12", client, Path(directory))

            attachment = result["issue"]["attachments"][0]
            local_url = attachment["local_url"]
            self.assertEqual(Path(local_url).read_bytes(), b"image bytes")
            self.assertIn(local_url, result["issue"]["description"])
            self.assertIn(local_url, result["issue"]["journals"][0]["notes"])
            self.assertEqual(attachment["thumbnail_url"], remote + "?thumb=1")


if __name__ == "__main__":
    unittest.main()
