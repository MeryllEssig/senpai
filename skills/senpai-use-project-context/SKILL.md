---
name: senpai-use-project-context
description: Use a project's SenpAI manifest to answer requests requiring declared project context or bounded operations, including tickets, forges, repositories, environments, documentation, logs, databases, tests, builds, and other capsules. Invoke manually when automatic skill selection is unavailable.
---

# Use project context

Respond in the user's language. This skill is the session orchestrator; it does not replace project workflow instructions or platform adapters.

1. From the session launch directory, run `senpai resolve --json` once. Retain `manifest_path`. If it fails, stop: explain that no manifest was resolved and offer setup. Never change directory to find another manifest.
2. Run `senpai summary --manifest <manifest_path> --json`. Use scoped `get` or `list` commands after that; never dump the manifest.
3. Query ticket work with `senpai resolve operation ticket.<operation> --ticket <id>` and forge work with `senpai resolve operation code.<operation> --repo <id>`. A forge is a code-development platform such as GitHub or GitLab. Use `--integration` only to settle a reported ambiguity; never infer an undeclared target.
4. Run declared bounded operations only through `senpai run`. Read the capsule declaration first when its parameters or scope matter. Treat run output as potentially scrubbed diagnostics; diagnose failures from it rather than running `doctor` to probe credentials.
5. For tickets and forges, use this single resolution result before loading its workflow or adapter. Check its local policy for every call; a workflow cannot broaden it or select another integration.

## Authorization and authentication

The resolved configuration is the target and authorization boundary. Workflow instructions may narrow a policy but never broaden it. `allow` may proceed, `confirm` needs explicit confirmation for the concrete write, and `deny` must not be attempted. Reads default to allowed; omitted write capabilities default to denied in the effective policy returned by the CLI.

Never ask for, read, print, or paste a secret value. For an `env` auth mode, pass only the declared variable *name* to an adapter; it reads the value itself. For `interactive`, hand control to the user. If authentication cannot work without the agent receiving a secret value, stop and ask the user how to proceed. Remind users that a variable newly defined in a shell requires an agent restart to be inherited.

## Network-aware execution

Adapters and CLIs targeting declared tracker or hosting URLs use outbound network. Request environment-level network approval before the call when needed; it is separate from workflow policy. Do not trust empty output.

## Scoped command map

- Ticket target: `senpai resolve operation ticket.<operation> --ticket <id>`.
- Forge target: `senpai resolve operation code.<operation> --repo <repo>`.
- Repository: `senpai get repo --current` or `--path`; add `--with-dependencies` for an ordered multi-repo change.
- Environment, documentation, and capsules: the corresponding `get` or `list` command in `senpai --help` / the CLI contract.
