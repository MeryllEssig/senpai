<div align="center">

<img src="assets/senpai-logo.png" alt="SenpAI" width="200" />

# SenpAI

*Declarative, project-scoped context for AI coding agents*

[![Build Status](https://img.shields.io/github/actions/workflow/status/MeryllEssig/senpai/ci.yml?style=flat-square&label=Build)](https://github.com/MeryllEssig/senpai/actions)
![Rust](https://img.shields.io/badge/Rust-2024-orange?style=flat-square&logo=rust&logoColor=white)

[Features](#features) | [Installation](#installation) | [Quick start](#quick-start) | [Usage](#usage) | [Documentation](#documentation)

</div>

SenpAI gives an AI agent the context it needs to work in a project without requiring that context to be repeated each session. A commented `.senpai.jsonc` manifest—or a standalone `.senpai.local.jsonc`—describes the project's integrations, repositories, environments, documentation, per-integration workflows, and safe bounded operations. The `senpai` CLI then returns only the relevant slice.

It is designed for Codex, Claude Code, Gemini CLI, OpenCode, and other agents that can run commands and follow Markdown instructions.

```sh
senpai resolve --json                          # Discover the project manifest
senpai summary --json                          # Get the project at a glance
senpai resolve operation code.read --repo app --json
senpai run test --json                         # Run a reviewed, bounded operation
```

## Features

- **Project-scoped context** — A small, reviewable manifest travels with the project instead of relying on a central registry or agent-specific configuration.
- **Focused answers** — Agents query only the repository, environment, documentation, workflow, or operation they need.
- **Integration routing** — Ticket and forge operations resolve through the project's declared integrations, policies, and workflows.
- **Multi-repository awareness** — Describe repositories, dependencies, and logical environments in one place.
- **Bounded operations** — Reviewed capsules run deterministic, non-interactive argv commands without shell access.
- **Private local values** — Keep machine-local configuration and capsule secrets out of the shared manifest.
- **Agent skills included** — Install focused skills for Codex, Claude Code, Gemini CLI, and OpenCode.

> [!IMPORTANT]
> Never put secrets in `.senpai.jsonc` or `.senpai.local.jsonc`. Private capsule values belong in the gitignored `.senpai/capsules.local.json` file, where SenpAI resolves and scrubs them during execution.

## Installation

### Quick install

Once a GitHub release is published, install the latest checksum-checked release for macOS or Linux on arm64 or x86_64:

```sh
curl --fail --location --silent --show-error \
  https://raw.githubusercontent.com/MeryllEssig/senpai/main/installer.sh | bash -s -- --agents codex
```

The installer downloads the matching archive, verifies its SHA-256 from the same release, and installs the binary plus the requested skills. Use `--agents claude,gemini,opencode`, `all`, or `none` to choose destinations.

### From source

```sh
git clone https://github.com/MeryllEssig/senpai.git
cd senpai
cargo build --release
./installer.sh --binary ./target/release/senpai --skills-dir ./skills --agents codex
```

See [local installation](doc/installation.md) for install locations, custom prefixes, release selection, and uninstallation.

## Quick start

Create `.senpai.jsonc` at the root of a project:

```jsonc
{
  "version": 2,
  "project": {
    "name": "my-service",
    "label": "My Service",
    "context": "API used by the customer portal.",
    "stack": ["Rust", "PostgreSQL"]
  },
  "integrations": {
    "origin": {
      "kind": "forge",
      "platform": "gitlab",
      "url": "https://git.example",
      "provides": ["code.read"],
      "handles": ["code.read"]
    }
  },
  "repos": {
    "app": { "path": ".", "integrations": { "origin": "my-group/my-service" } }
  },
  "environments": {
    "local": { "label": "Local development", "repo": "app" }
  },
  "capsules": {
    "test": {
      "label": "Run the test suite",
      "type": "test",
      "repo": "app",
      "environment": "local",
      "program": "cargo",
      "args": ["test"]
    }
  }
}
```

From any directory inside the project, SenpAI discovers the manifest by walking upward, like Git:

```sh
senpai validate manifest --json
senpai summary --json
senpai list capsules --json
senpai run test --json
```

Start with the fully annotated [reference manifest](doc/reference-manifest.jsonc) when you need integrations, multiple repositories, workflows, or richer capsule definitions.

## Usage

| Command | Description |
|:---|:---|
| `senpai init` | Create or update private capsule-value placeholders and add their file to `.gitignore`. |
| `senpai resolve --json` | Discover the active manifest and its project root. |
| `senpai summary --json` | Return a compact project overview. |
| `senpai get repo --current --with-dependencies --json` | Get the current repository and its dependencies. |
| `senpai resolve operation ticket.read --ticket ACME-42 --json` | Resolve the integration and workflow for a ticket operation. |
| `senpai resolve operation code.read --repo app --json` | Resolve the integration and workflow for a forge operation. |
| `senpai list capsules --repo app --env preprod --json` | List operations available in a repository or environment. |
| `senpai run <capsule> --json` | Run a declared capsule with bounded output. |
| `senpai validate manifest --json` | Validate the shared manifest. |
| `senpai validate local --json` | Validate local configuration and private capsule values. |

Without `--json`, the CLI prints concise human-readable output. Use `--json` for its stable machine-readable envelope. The full [CLI contract](doc/cli-contract.md) documents all commands, filters, exit codes, and output guarantees.

## How it works

```text
.senpai.jsonc or .senpai.local.jsonc ──► senpai CLI ──► focused context for the agent
       │                │
       │                └──► declared capsules only (no shell, bounded output)
       └──► optional .senpai.local.jsonc overlay
```

The committed manifest is JSONC, so it can document the project ecosystem in place. A personal `.senpai.local.jsonc` beside it is deep-merged for paths, preferences, or authentication configuration; it can also stand alone as a complete machine-local manifest.

SenpAI does not orchestrate external services while querying a manifest. Its only execution surface is `senpai run`, which runs a declared capsule.

## Capsules

A capsule is a deterministic, non-interactive argv command for finite operations: tests, builds, bounded log reads, exports, or one database query.

```jsonc
"db-preprod": {
  "label": "Run one read-only query",
  "type": "database-query",
  "environment": "preprod",
  "program": "mysql",
  "args": ["--password={password}", "app", "--execute", "{query}"],
  "supplied": ["query"],
  "timeout_seconds": 30,
  "max_output_bytes": 1048576
}
```

`{query}` is supplied at invocation time. `{password}` is resolved inside SenpAI from `.senpai/capsules.local.json` and is never printed in resolved arguments or process output.

> [!NOTE]
> Capsules receive no shell, stdin, or TTY. `program` and `args` are passed directly to the operating system; shell and language interpreters are rejected. This is a guardrail for reviewed manifests, not an OS sandbox.

## Agent skills

| Skill | Purpose |
|:---|:---|
| `senpai-use-project-context` | Resolve a manifest and retrieve only the needed context. |
| `senpai-setup-project-context` | Interview, create, and validate a new manifest. |
| `senpai-manage-project-context` | Safely evolve an existing manifest. |
| `senpai-discover-project-automation` | Propose safe automation opportunities without applying them. |
| `senpai-project-management` | Work with ticketing platforms through focused adapters. |
| `senpai-code-hosting` | Work with code-hosting platforms through focused adapters. |
| `senpai-project-use-ticket-workflow` | Apply the default read-only ticket workflow. |
| `senpai-project-use-code-hosting-workflow` | Apply the default read-only forge workflow. |

Ticket and code-change workflows combine explicit `allow`, `confirm`, and `deny` policies with project-specific instructions. If no workflow is declared, reads are allowed and writes are denied.

## Documentation

- [Goals and non-goals](doc/goal.md)
- [User stories](doc/user-stories.md)
- [Technical considerations](doc/technical-considerations.md)
- [CLI contract](doc/cli-contract.md)
- [Manifest JSON Schema](schema/senpai.schema.json)
- [Reference manifest](doc/reference-manifest.jsonc)
- [Reference local capsule values](doc/reference-capsule.jsonc)

## Development

Prerequisites: stable Rust, Bun, and Bash. Python 3 is required only for the Redmine adapter and its tests.

```sh
bun install
bun run verify
```

`bun run verify` validates the reference JSONC files, runs installer and CLI contract tests, checks Rust formatting, runs Clippy with warnings denied, and runs the Rust test suite.

Useful focused checks:

```sh
bun run test:installer
bun run test:cli
cargo test
```
