# Senpai

Senpai is a project-scoped, declarative context layer for AI coding agents.
Projects describe their ecosystem in a commented `.senpai.jsonc` manifest;
the `senpai` CLI returns only the relevant slice of that context or runs a
declared, bounded capsule without exposing its local private values.

It is designed to work with Codex, Claude Code, Gemini CLI, OpenCode, and any
agent that can invoke a command and follow Markdown instructions.

## What it provides

- Manifest discovery from the session directory, including parent-directory
  manifests and private local overlays.
- Scoped repository, tracker, code-hosting, environment, documentation,
  workflow and capsule queries in compact Markdown or stable JSON.
- Safe capsule execution: argv only, no shell, no stdin or TTY, timeout and
  output limits, and scrubbing of values read from `.senpai/capsules.local.json`.
- Shipped project-context, setup, management, automation-discovery, ticket and
  code-hosting skills. The Redmine adapter uses only Python's standard library.
- A local installer that copies a built binary and the selected `senpai-*`
  skills, records exactly what it owns, and supports safe uninstall.

The manifest format, semantics and CLI protocol are specified in
[the documentation](doc/goal.md). The complete annotated manifest is in
[the reference example](doc/reference-manifest.jsonc).

## Local development

Prerequisites: Rust (stable), Bun, and Bash. Python 3 is needed only for the
Redmine adapter tests and runtime use.

```sh
bun install
bun run verify
```

`bun run verify` is the required local quality gate. It validates the golden
JSONC examples, exercises installer and CLI contract tests, verifies Rust
formatting, runs Clippy with all warnings denied, and runs the Rust tests.

Build and invoke the CLI locally:

```sh
cargo build
./target/debug/senpai summary --json
./target/debug/senpai validate manifest --json
```

The latter commands resolve `.senpai.jsonc` by walking upward from the current
directory. Use the absolute path returned by `senpai resolve` with
`--manifest` for session-anchored agent usage.

## Local installation

Release downloads are deliberately not implemented yet. Install an already
built local binary for selected agent ecosystems instead:

```sh
./installer.sh --binary ./target/release/senpai --agents codex --yes
```

The installer supports `codex`, `claude`, `gemini`, `opencode`, `all`, and
`none`. It overwrites only shipped `senpai-*` skills and records installed
paths, so removal is limited to its own files:

```sh
./installer.sh --uninstall
```

> [!IMPORTANT]
> Never put a secret in `.senpai.jsonc` or `.senpai.local.jsonc`. Private
> capsule values belong only in `.senpai/capsules.local.json`, which `senpai
> init` scaffolds and adds to `.gitignore`.

## Repository layout

- `src/` — the standalone Rust CLI.
- `schema/` — the versioned JSON Schema.
- `skills/` — skills shipped to users.
- `maintainer/` — maintainer-only QA skills; not distributed.
- `doc/` — product decisions, contract, reference manifest and rationale.
- `tests/` — hermetic local CLI and installer tests.
