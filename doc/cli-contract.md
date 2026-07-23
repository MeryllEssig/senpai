# SenpAI CLI Contract — v1 foundation

This document fixes the public contract needed for the first vertical slice.
It does not turn SenpAI into an orchestration engine: `get` commands only
read the manifest and never contact a tracker or hosting service.
Only `run` executes a process, and it executes capsules only.

## Common behavior

- The executable name is `senpai`.
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
senpai resolve [--from <directory>] [--json]
senpai summary [--manifest <absolute-path>] [--json]
```

`resolve` returns the canonical absolute `manifest_path`, the directory that
contains it, and the project name. `--from` defaults to the process cwd.
`summary` returns only section names, declared ids, roles, capsule ids and
types, common interface names, and effective workflow-skill names. It never
returns command strings.

The usage skill calls `resolve` once from the launch directory, retains
`manifest_path`, then supplies that path to every later command in the session.

## Scoped reads

```text
senpai get tracker --role <role> [--source <source-id>] [--manifest <path>] [--json]
senpai get ticket-route --id <ticket-id> [--source <source-id>] [--manifest <path>] [--json]
senpai get hosting --role <role> --repo <repo-id> [--instance <instance-id>] [--manifest <path>] [--json]
senpai get workflow --domain <tickets|code_changes> [--manifest <path>] [--json]
senpai get repo (--id <repo-id> | --path <path> | --current) [--with-dependencies] [--manifest <path>] [--json]
senpai list repos [--manifest <path>] [--json]
senpai get environment --id <environment-id> [--manifest <path>] [--json]
senpai get capsule --id <capsule-id> [--manifest <path>] [--json]
senpai list capsules [--repo <repo-id>] [--env <environment-id>] [--type <type>] [--manifest <path>] [--json]
senpai get docs [--id <docs-id>] [--manifest <path>] [--json]
senpai get rules [--manifest <path>] [--json]
```

`get ticket-route` returns a source selection, not ticket content. The usage
skill passes that selection to `senpai-project-management`, which uses the
source's default or custom adapter. An ambiguous route exits 5 and lists
candidate ids only.

`get tracker` and `get hosting` apply the role-selection rules in technical
considerations 1.3. An explicit `--source` or `--instance` is valid only when
that declaration actually holds the requested role. `get hosting` additionally
requires that the selected instance occurs in the selected repo's `hosting`
map.

`get workflow` returns the requested `domain`, its effective `skill`, and its
fully expanded `policy`. A declared domain always supplies its skill; otherwise
the skill is
`senpai-project-use-ticket-workflow` for `tickets` or
`senpai-project-use-code-hosting-workflow` for `code_changes`. The expanded
policy contains every capability defined for that domain: an omitted `read`
is `allow`, and every other omitted capability is `deny`. This command reads
configuration only; it does not load or inspect the named skill.

`--path` accepts an absolute path or a path relative to the process cwd; the
path does not need to exist. The CLI resolves it lexically to a normalized path
relative to the manifest directory and rejects a path outside that directory.
Manifest paths use `/` separators on every platform; backslashes are invalid.
`--current` uses the process cwd. Repo candidates are declaration paths that
contain the resolved path on a complete path-segment boundary. The longest
candidate wins; an equal-length tie is an ambiguity error (exit 5) that lists
candidate ids.

`get capsule` returns the declaration, including its literal command template
and optional MCP hint, but never local values or a resolved command. The MCP
hint is informational metadata for an external tool skill; it is not an
alternate `senpai run` backend and inherits none of the capsule runner's
timeout, output-limit, or scrubbing guarantees.
`list capsules` returns compact
metadata only: id, label, type, repo, environment, MCP server/tool, and access.
A capsule that names only an environment inherits that environment's repo for filtering.
Filters combine with AND semantics.

Every scoped result includes its `id`, its source section, its relevant
`access`, `note`, `repo`, `environment`, `auth` variable *names*, and custom
adapter `skill` when those facts exist. It never expands an
environment-variable value.

## Local setup and diagnostics

```text
senpai init [--manifest <path>] [--json]
senpai validate manifest [--manifest <path>] [--json]
senpai validate local [--manifest <path>] [--json]
senpai doctor [--manifest <path>] [--json]
```

`init` is idempotent: when at least one capsule has a non-supplied placeholder,
it creates the values file, its ignore file, and the missing stubs, but never
changes an existing value. With no such placeholder it creates neither local
file. `validate manifest` is
machine-independent and is the CI command. `validate local` additionally
checks the locally stored capsule placeholders and `$ENV` reference syntax; an
absent values file is valid when no capsule needs one.
`doctor` is a convenience aggregate for validating the resolved manifest,
overlay, and local capsule configuration. It checks configuration only: it
does not inspect environment-variable availability, installed skills, CLI
sessions, credentials, connectivity, remote authorization, or any external
service.

## Capsule execution

```text
senpai run <capsule-id> [--<supplied-name> <value> …] [--manifest <path>] [--json]
```

`run` accepts only ids from the `capsules` section. A capsule with no supplied
parameters is invoked with its id alone.

Every and only the names declared in `supplied` must occur once; omitted
`supplied` is equivalent to an empty array. The CLI rejects
unknown, missing, or repeated supplied names before it loads local values.
A template may contain zero placeholders. It is parsed to argv before
substitution. Each argv element may contain at most one placeholder, optionally
surrounded by literal prefix or suffix text (for example
`--password={password}`); substitution changes that element's contents but can
never create another argument. Each placeholder name occurs exactly once in a
template. Placeholders are forbidden in the executable
element. Supplied names may not be `help`, `json`, `manifest`, or `version`,
which are reserved CLI options. The validator rejects unmatched braces,
multiple placeholders in one element, undeclared supplied names, missing local
names, empty local values, shell operators, and templates whose executable or
argument count cannot be parsed deterministically.

The child receives no shell, stdin, or TTY. Its cwd is the manifest directory
joined with the capsule's optional normalized `cwd`, defaulting to the manifest
directory. Its combined stdout/stderr is bounded and scrubbed
before return. Every result, successful or not, exposes the literal
`command_template` from the manifest and never the resolved command line. In
Markdown mode, the template is printed before the scrubbed process output. In
JSON mode, successful `data` contains `command_template`, `stdout`, `stderr`,
and the child `exit_code`. When the parent exits 7 because the child failed,
timed out, or exceeded the output limit, `error.details` contains one object
with the same four diagnostic fields when available. `stdout` and `stderr`
always contain scrubbed text only.
