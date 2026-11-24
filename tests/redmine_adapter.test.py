#!/usr/bin/env python3
"""Regression tests for the bundled Redmine adapter."""
from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import sys
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



class RedmineNotesTests(unittest.TestCase):
    COMMON = ["--url", "https://redmine.example", "--api-key-env", "KEY"]
    NOTE = "private text"

    def run_command(
        self,
        argv: list[str],
        journals: list[dict[str, object]] | None = None,
        get_error: Exception | None = None,
    ) -> tuple[object, list[tuple[str, str, object]]]:
        calls: list[tuple[str, str, object]] = []

        def urlopen(request: object, timeout: float) -> Response:
            method = request.get_method()  # type: ignore[attr-defined]
            url = request.full_url  # type: ignore[attr-defined]
            data = request.data  # type: ignore[attr-defined]
            calls.append((method, url, json.loads(data) if data else None))
            if method == "GET":
                if get_error is not None:
                    raise get_error
                return Response(json.dumps({"issue": {"journals": journals or []}}).encode())
            return Response(b"")

        args = redmine.build_parser().parse_args(argv + self.COMMON)
        client = redmine.Redmine("https://redmine.example", "api-key", 1, 1024)
        with patch.object(redmine.urllib.request, "urlopen", side_effect=urlopen):
            return redmine.run(args, client), calls

    def private_comment(self, journals: list[dict[str, object]], **kwargs: object) -> tuple[object, list[tuple[str, str, object]]]:
        return self.run_command(["add-comment", "--id", "12", "--notes", self.NOTE, "--private"], journals=journals, **kwargs)  # type: ignore[arg-type]

    def test_public_comment_does_not_send_private_notes_nor_verify(self) -> None:
        _, calls = self.run_command(["add-comment", "--id", "12", "--notes", "hello"])
        self.assertEqual([(m, u) for m, u, _ in calls], [("PUT", "https://redmine.example/issues/12.json")])
        self.assertEqual(calls[0][2], {"issue": {"notes": "hello"}})

    def test_private_comment_sends_flag_and_verifies_journal(self) -> None:
        result, calls = self.private_comment(
            [{"id": 1, "notes": "old", "private_notes": False}, {"id": 2, "notes": self.NOTE, "private_notes": True}]
        )
        self.assertEqual(calls[0][2], {"issue": {"notes": self.NOTE, "private_notes": True}})
        self.assertEqual(calls[1][0], "GET")
        self.assertIn("/issues/12.json?include=journals", calls[1][1])
        self.assertEqual(result, {"ok": True, "private_notes": True})

    def test_private_comment_ignores_journals_without_notes(self) -> None:
        result, _ = self.private_comment(
            [{"id": 2, "notes": self.NOTE, "private_notes": True}, {"id": 3, "notes": "", "private_notes": False}]
        )
        self.assertEqual(result, {"ok": True, "private_notes": True})

    def test_private_comment_uses_highest_journal_id_regardless_of_order(self) -> None:
        result, _ = self.private_comment(
            [{"id": 9, "notes": self.NOTE, "private_notes": True}, {"id": 4, "notes": "old", "private_notes": False}]
        )
        self.assertEqual(result, {"ok": True, "private_notes": True})

    def test_private_comment_matches_note_with_windows_line_endings(self) -> None:
        result, _ = self.run_command(
            ["add-comment", "--id", "12", "--notes", "a\nb", "--private"],
            journals=[{"id": 1, "notes": "a\r\nb\r\n", "private_notes": True}],
        )
        self.assertEqual(result, {"ok": True, "private_notes": True})

    def test_private_comment_fails_when_redmine_published_it_publicly(self) -> None:
        with self.assertRaisesRegex(redmine.AdapterError, "not private"):
            self.private_comment([{"id": 1, "notes": self.NOTE, "private_notes": False}])

    def test_private_comment_fails_when_journal_lacks_private_notes_key(self) -> None:
        with self.assertRaisesRegex(redmine.AdapterError, "not private"):
            self.private_comment([{"id": 1, "notes": self.NOTE}])

    def test_private_comment_fails_when_latest_note_is_not_ours(self) -> None:
        with self.assertRaisesRegex(redmine.AdapterError, "could not be verified"):
            self.private_comment([{"id": 1, "notes": "someone else", "private_notes": True}])

    def test_private_comment_fails_when_no_journal_is_found(self) -> None:
        with self.assertRaisesRegex(redmine.AdapterError, "could not be verified"):
            self.private_comment([])

    def test_verification_read_failure_says_the_note_was_already_posted(self) -> None:
        error = redmine.urllib.error.HTTPError("https://redmine.example", 403, "Forbidden", {}, None)  # type: ignore[arg-type]
        with self.assertRaisesRegex(redmine.AdapterError, "posted but its privacy could not be verified: Redmine HTTP 403"):
            self.private_comment([], get_error=error)

    def test_update_issue_private_notes_are_sent_and_verified(self) -> None:
        _, calls = self.run_command(
            ["update-issue", "--id", "12", "--status-id", "3", "--notes", self.NOTE, "--private"],
            journals=[{"id": 1, "notes": self.NOTE, "private_notes": True}],
        )
        self.assertEqual(calls[0][2], {"issue": {"status_id": 3, "notes": self.NOTE, "private_notes": True}})
        self.assertEqual(len(calls), 2)

    def test_update_issue_private_requires_notes(self) -> None:
        with self.assertRaisesRegex(redmine.AdapterError, "--private requires"):
            self.run_command(["update-issue", "--id", "12", "--subject", "x", "--private"])

    def test_private_rejects_empty_notes_without_calling_redmine(self) -> None:
        for argv in (["add-comment", "--id", "12", "--notes", "", "--private"], ["update-issue", "--id", "12", "--notes", "", "--private"]):
            with self.subTest(argv=argv), patch.object(redmine.urllib.request, "urlopen", side_effect=AssertionError("no request expected")):
                args = redmine.build_parser().parse_args(argv + self.COMMON)
                with self.assertRaisesRegex(redmine.AdapterError, "--private requires"):
                    redmine.run(args, redmine.Redmine("https://redmine.example", "api-key", 1, 1024))

    def test_failure_output_never_contains_note_or_api_key(self) -> None:
        argv = ["redmine.py", "add-comment", "--id", "12", "--notes", self.NOTE, "--private", *self.COMMON]
        stderr = io.StringIO()
        journals = [{"id": 1, "notes": self.NOTE, "private_notes": False}]
        with (
            patch.object(sys, "argv", argv),
            patch.dict(os.environ, {"KEY": "api-key"}),
            patch.object(redmine.urllib.request, "urlopen", side_effect=lambda r, timeout: Response(json.dumps({"issue": {"journals": journals}}).encode() if r.get_method() == "GET" else b"")),
            contextlib.redirect_stderr(stderr),
            contextlib.redirect_stdout(io.StringIO()) as stdout,
        ):
            self.assertEqual(redmine.main(), 1)
        for output in (stderr.getvalue(), stdout.getvalue()):
            self.assertNotIn(self.NOTE, output)
            self.assertNotIn("api-key", output)
        self.assertIn("not private", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
