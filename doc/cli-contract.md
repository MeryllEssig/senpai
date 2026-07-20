# AI Manager CLI Contract — v1 foundation

This document fixes the public contract needed for the first vertical slice.
It does not turn AI Manager into an orchestration engine: `get` commands only
read the manifest and never contact a tracker, hosting service, or connector.
Only `run` executes a process, and it executes capsules only.

## Common behavior

- The executable name is `aimanager`.
- A command that reads a manifest accepts `--manifest <absolute-path>`. With
  this option, it must use exactly that file and must not walk the directory
  tree. Without it, it resolves from its current directory by walking upward.
- `--json` emits the stable JSON protocol below. Without it, the command emits
  compact Markdown for an LLM. Both modes write results to stdout and
  diagnostics only to stderr.
- Successful JSON output is:

  ```json
  { "ok": true, "data": {}, "warnings": [] }
  ```

  `data` is an object for one result and an array only when the command's name
  says it lists results. `warnings` is always present and contains objects with
  `code` and `message`.

- Error JSON output is:

  ```json
  { "ok": false, "error": { "code": "…", "message": "…", "details": [] } }
  ```

  Error text and `details` must never include a secret value, a resolved
  capsule command, or the contents of the capsule-values file.

| Exit | Meaning |
|---:|---|
| 0 | success |
| 2 | invalid CLI syntax or incompatible options |
| 3 | manifest, declared id, or declared capability not found |
| 4 | invalid manifest, overlay, schema, or cross-reference |
| 5 | routing ambiguity requiring an explicit selector or user input |
| 6 | missing or incomplete local capsule configuration |
| 7 | capsule process failed, timed out, or exceeded its output limit |

## Resolution and inventory

```text
aimanager resolve [--from <directory>] [--json]
aimanager summary [--manifest <absolute-path>] [--json]
```

`resolve` returns the canonical absolute `manifest_path`, the directory that
contains it, and the project name. `--from` defaults to the process cwd.
`summary` returns only section names, declared ids, roles, capsule ids, and
resolved tool-skill names. It never returns connector coordinates or command
strings.

The usage skill calls `resolve` once from the launch directory, retains
`manifest_path`, then supplies that path to every later command in the session.

## Scoped reads

```text
aimanager get tracker --role <role> [--source <source-id>] [--manifest <path>] [--json]
aimanager get ticket-route --id <ticket-id> [--source <source-id>] [--manifest <path>] [--json]
aimanager get hosting --role <role> --repo <repo-id> [--instance <instance-id>] [--manifest <path>] [--json]
aimanager get repo --id <repo-id> [--with-dependencies] [--manifest <path>] [--json]
aimanager get connectors --repo <repo-id> [--manifest <path>] [--json]
aimanager get logs --env <environment-id> [--manifest <path>] [--json]
aimanager get data-store --id <store-id> [--manifest <path>] [--json]
aimanager get docs [--id <docs-id>] [--manifest <path>] [--json]
aimanager get local-command --id <command-id> [--manifest <path>] [--json]
aimanager get rules [--manifest <path>] [--json]
```

`get ticket-route` returns a source selection, not ticket content. The agent
loads the returned tool skill and contacts the selected tracker itself. An
ambiguous route exits 5 and lists candidate ids only.

`get tracker` and `get hosting` apply the role-selection rules in technical
considerations 1.3. An explicit `--source` or `--instance` is valid only when
that declaration actually holds the requested role. `get hosting` additionally
requires that the selected instance occurs in the selected repo's `hosting`
map.

Every scoped result includes its `id`, its source section, its relevant
`access`, `note`, `repo`, `auth` variable *names*, and tool `skill` when those
facts exist. It never expands an environment-variable value.

## Local setup and diagnostics

```text
aimanager init [--manifest <path>] [--json]
aimanager validate manifest [--manifest <path>] [--json]
aimanager validate local [--manifest <path>] [--json]
aimanager doctor [--manifest <path>] [--json]
```

`init` is idempotent: it creates missing capsule-value stubs and the local
ignore file, but never changes an existing value. `validate manifest` is
machine-independent and is the CI command. `validate local` additionally
checks the locally stored capsule placeholders and `$ENV` reference syntax.
`doctor` is a convenience aggregate for validating the resolved manifest,
overlay, and local capsule configuration. It checks configuration only: it
does not inspect environment-variable availability, installed skills, CLI
sessions, credentials, connectivity, remote authorization, or any external
service.

## Capsule execution

```text
aimanager run <capsule-id> --<supplied-name> <value> … [--manifest <path>] [--json]
```

`run` accepts only ids from the `capsules` section and never executes a
`local_commands` entry.

Every and only the names declared in `supplied` must occur once. The CLI rejects
unknown, missing, or repeated supplied names before it loads local values.
Templates are parsed to argv before substitution; a placeholder may occur only
as a complete argv element or as a documented inline credential fragment such
as `-p{password}`. The validator rejects unmatched braces, undeclared supplied
names, missing local names, empty local values, shell operators, and templates
whose executable or argument count cannot be parsed deterministically.

The child receives no shell. Its combined stdout/stderr is bounded and scrubbed
before return. Every result, successful or not, exposes the literal
`command_template` from the manifest and never the resolved command line. In
Markdown mode, the template is printed before the scrubbed process output. In
JSON mode, successful `data` contains `command_template`, `stdout`, `stderr`,
and the child `exit_code`. When the parent exits 7 because the child failed,
timed out, or exceeded the output limit, `error.details` contains one object
with the same four diagnostic fields when available. `stdout` and `stderr`
always contain scrubbed text only.
