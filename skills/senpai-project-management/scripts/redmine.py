#!/usr/bin/env python3
"""Small, stdlib-only Redmine REST adapter. Credentials never enter argv/output."""
from __future__ import annotations

import argparse
import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request
from typing import Any

DEFAULT_TIMEOUT = 30.0
DEFAULT_MAX_OUTPUT = 1024 * 1024


class AdapterError(Exception):
    pass


def parse_json_object(value: str) -> dict[str, Any]:
    try:
        parsed = json.loads(value)
    except json.JSONDecodeError as exc:
        raise argparse.ArgumentTypeError("must be valid JSON") from exc
    if not isinstance(parsed, dict):
        raise argparse.ArgumentTypeError("must be a JSON object")
    return parsed


def positive_float(value: str) -> float:
    try:
        result = float(value)
    except ValueError as exc:
        raise argparse.ArgumentTypeError("must be a number") from exc
    if result <= 0:
        raise argparse.ArgumentTypeError("must be greater than zero")
    return result


def positive_int(value: str) -> int:
    try:
        result = int(value)
    except ValueError as exc:
        raise argparse.ArgumentTypeError("must be an integer") from exc
    if result <= 0:
        raise argparse.ArgumentTypeError("must be greater than zero")
    return result


def base_url(value: str) -> str:
    parsed = urllib.parse.urlparse(value)
    if parsed.scheme not in {"https", "http"} or not parsed.netloc or parsed.username or parsed.password:
        raise argparse.ArgumentTypeError("must be an absolute http(s) URL")
    return value.rstrip("/")


def secret_from_env(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise AdapterError(f"credential environment variable {name!r} is unset or empty")
    return value


class Redmine:
    def __init__(self, url: str, api_key: str, timeout: float, max_response_bytes: int) -> None:
        self.url, self.api_key, self.timeout, self.max_response_bytes = url, api_key, timeout, max_response_bytes

    def request(self, method: str, path: str, body: dict[str, Any] | None = None) -> Any:
        payload = json.dumps(body).encode("utf-8") if body is not None else None
        request = urllib.request.Request(
            self.url + path, data=payload, method=method,
            headers={"Accept": "application/json", "X-Redmine-API-Key": self.api_key,
                     **({"Content-Type": "application/json"} if payload else {})},
        )
        try:
            with urllib.request.urlopen(request, timeout=self.timeout) as response:
                raw = response.read(self.max_response_bytes + 1)
        except urllib.error.HTTPError as exc:
            # Do not read/report a server body: proxies can reflect sensitive inputs.
            raise AdapterError(f"Redmine HTTP {exc.code} {exc.reason}") from exc
        except urllib.error.URLError as exc:
            raise AdapterError(f"Redmine connection failed: {exc.reason}") from exc
        except TimeoutError as exc:
            raise AdapterError("Redmine request timed out") from exc
        if len(raw) > self.max_response_bytes:
            raise AdapterError(f"Redmine response exceeded {self.max_response_bytes} bytes")
        if not raw:
            return {"ok": True}
        try:
            return json.loads(raw.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise AdapterError("Redmine returned invalid JSON") from exc

    def list_pages(self, path: str, limit: int, all_pages: bool) -> dict[str, Any]:
        offset, items, total = 0, [], None
        while True:
            sep = "&" if "?" in path else "?"
            data = self.request("GET", f"{path}{sep}limit={limit}&offset={offset}")
            page = data.get("issues")
            if not isinstance(page, list):
                raise AdapterError("Redmine list response did not contain issues")
            items.extend(page)
            total = data.get("total_count", len(items))
            if not all_pages or len(items) >= total or not page:
                return {"issues": items, "total_count": total, "returned_count": len(items)}
            offset += len(page)


def query(params: dict[str, Any]) -> str:
    return "?" + urllib.parse.urlencode({k: v for k, v in params.items() if v is not None}) if params else ""


def add_common(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--url", required=True, type=base_url)
    parser.add_argument("--api-key-env", required=True, metavar="NAME")
    parser.add_argument("--timeout", type=positive_float, default=DEFAULT_TIMEOUT)
    parser.add_argument("--max-output-bytes", type=positive_int, default=DEFAULT_MAX_OUTPUT)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="operation", required=True)
    def command(name: str) -> argparse.ArgumentParser:
        child = sub.add_parser(name); add_common(child); return child
    p = command("get-issue"); p.add_argument("--id", required=True)
    p = command("list-issues"); p.add_argument("--project"); p.add_argument("--status-id"); p.add_argument("--assigned-to-id"); p.add_argument("--limit", type=lambda x: min(positive_int(x), 100), default=25); p.add_argument("--all", action="store_true")
    p = command("list-statuses")
    p = command("create-issue"); p.add_argument("--project", required=True); p.add_argument("--tracker-id", required=True, type=positive_int); p.add_argument("--subject", required=True); p.add_argument("--description"); p.add_argument("--priority-id", type=positive_int); p.add_argument("--assigned-to-id", type=positive_int); p.add_argument("--custom-fields-json", type=parse_json_object)
    p = command("update-issue"); p.add_argument("--id", required=True); p.add_argument("--subject"); p.add_argument("--description"); p.add_argument("--status-id", type=positive_int); p.add_argument("--priority-id", type=positive_int); p.add_argument("--assigned-to-id", type=positive_int); p.add_argument("--notes"); p.add_argument("--custom-fields-json", type=parse_json_object)
    p = command("add-comment"); p.add_argument("--id", required=True); p.add_argument("--notes", required=True)
    p = command("log-time"); p.add_argument("--id", required=True); p.add_argument("--hours", required=True, type=positive_float); p.add_argument("--activity-id", required=True, type=positive_int); p.add_argument("--comments")
    p = command("add-relation"); p.add_argument("--id", required=True); p.add_argument("--issue-to-id", required=True); p.add_argument("--relation-type", required=True)
    return parser


def run(args: argparse.Namespace, client: Redmine) -> Any:
    if args.operation == "get-issue":
        return client.request("GET", f"/issues/{urllib.parse.quote(args.id, safe='')}.json?include=journals,relations,attachments")
    if args.operation == "list-issues":
        return client.list_pages("/issues.json" + query({"project_id": args.project, "status_id": args.status_id, "assigned_to_id": args.assigned_to_id}), args.limit, args.all)
    if args.operation == "list-statuses": return client.request("GET", "/issue_statuses.json")
    if args.operation == "create-issue":
        issue = {"project_id": args.project, "tracker_id": args.tracker_id, "subject": args.subject}
        for key in ("description", "priority_id", "assigned_to_id"):
            value = getattr(args, key)
            if value is not None: issue[key] = value
        if args.custom_fields_json is not None: issue["custom_fields"] = args.custom_fields_json
        return client.request("POST", "/issues.json", {"issue": issue})
    if args.operation in {"update-issue", "add-comment"}:
        issue: dict[str, Any] = {"notes": args.notes} if args.operation == "add-comment" else {}
        if args.operation == "update-issue":
            for key in ("subject", "description", "status_id", "priority_id", "assigned_to_id", "notes"):
                value = getattr(args, key)
                if value is not None: issue[key] = value
            if args.custom_fields_json is not None: issue["custom_fields"] = args.custom_fields_json
            if not issue: raise AdapterError("update-issue needs at least one field")
        return client.request("PUT", f"/issues/{urllib.parse.quote(args.id, safe='')}.json", {"issue": issue})
    if args.operation == "log-time":
        entry = {"issue_id": args.id, "hours": args.hours, "activity_id": args.activity_id}
        if args.comments is not None: entry["comments"] = args.comments
        return client.request("POST", "/time_entries.json", {"time_entry": entry})
    if args.operation == "add-relation":
        return client.request("POST", f"/issues/{urllib.parse.quote(args.id, safe='')}/relations.json", {"relation": {"issue_to_id": args.issue_to_id, "relation_type": args.relation_type}})
    raise AdapterError("unsupported operation")


def main() -> int:
    args = build_parser().parse_args()
    try:
        secret = secret_from_env(args.api_key_env)
        result = run(args, Redmine(args.url, secret, args.timeout, args.max_output_bytes))
        rendered = json.dumps(result, ensure_ascii=False, separators=(",", ":"))
        if len(rendered.encode("utf-8")) > args.max_output_bytes:
            raise AdapterError(f"output exceeded {args.max_output_bytes} bytes")
        print(rendered)
        return 0
    except AdapterError as exc:
        # Errors deliberately omit the credential and all HTTP response bodies.
        print(json.dumps({"ok": False, "error": str(exc)}), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
