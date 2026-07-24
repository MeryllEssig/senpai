---
name: senpai-project-management
description: Perform common SenpAI ticket operations after a v2 integration resolution, using a focused adapter for Redmine, Jira, Linear, or a declared custom adapter.
---

# Project management interface

Use this skill only after `senpai resolve operation ticket.*` selected one ticketing integration. The common operations are `read`, `create`, `update`, `comment`, `transition`, `link`, and `log_time`; search is part of `read`. Before every operation, use the selected id, URL, scope, and auth metadata. The returned adapter is the complete technical adapter selection.

Enforce the resolution's local policy before every adapter call. Proceed only for `allow`; obtain explicit confirmation for the concrete operation under `confirm`; never attempt `deny`. Then load the configured workflow skill; it may narrow permissions but cannot widen them or select another integration.

## Adapter selection

- `redmine`: use the bundled script below. Read [Redmine reference](references/redmine.md) for endpoint and field details.
- `jira` or `linear`: use an already installed, documented adapter skill/CLI. Pass declared auth metadata by name only and follow its host-selection instructions.
- Any other type: state that no shipped adapter exists and ask the user to install/declare a custom source `skill`; do not improvise undocumented API calls.

For `auth.mode=env`, pass `--api-key-env <declared-name>` to the Redmine script—the script reads it. For `preconfigured`, use `--api-key-env` only if a known variable name is declared; otherwise authentication must be available to the selected adapter without exposing a secret. For `interactive`, Redmine's script has no login flow: stop and ask the user to arrange an API key through a safe mechanism. Never put a token in an argument, command transcript, or chat.

Redmine uses outbound HTTPS. Request any environment-level network approval before the command; it is separate from ticket policy. Empty or malformed output is not a successful result.

## Redmine commands

Use the documented command below for a routed read. Run `python3 <skill-dir>/scripts/redmine.py --help` only when the requested operation or its arguments are not covered by this skill or its Redmine reference. Supply the source URL, declared project identifier where the operation needs it, and the API-key environment-variable name. The script emits bounded JSON and does not print credentials. For example, a routed read is conceptually:

```text
python3 scripts/redmine.py get-issue --url <url> --api-key-env <name> --id <ticket-id>
```

Use server-returned ids for comments, status updates, time entries, and links. Redmine transition is `update-issue --status-id`; linking is `add-relation`. Do not guess status IDs: list issue statuses first. The script does not know SenpAI policy and cannot substitute for it.
