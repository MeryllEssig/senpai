---
name: senpai-use-project-context
description: Use a project's SenpAI manifest to answer requests requiring declared project context or bounded operations, including tickets, code hosting, repositories, environments, documentation, logs, databases, tests, builds, and other capsules. Invoke manually when automatic skill selection is unavailable.
---

# Use project context

Respond in the user's language. This skill is the session orchestrator; it does not replace project workflow instructions or platform adapters.

1. From the session launch directory, run `senpai resolve --json` once. Retain `manifest_path`. If it fails, stop: explain that no manifest was resolved and offer setup. Never change directory to find another manifest.
2. Run `senpai summary --manifest <manifest_path> --json`. Use scoped `get` or `list` commands after that; never dump the manifest.
3. Match relevant declared rules, then query only the needed capability. For a ticket id, try `get ticket-route` first. If it has no match, use `get tracker --role ticket_details` only when that role has exactly one declared source; this is a deterministic role fallback, not a guess. Report the missing routing pattern and propose the narrowest pattern supported by the observed id. Do not edit the manifest during an unrelated read; use `senpai-manage-project-context` when a manifest update is authorized. Zero or multiple role candidates remain a gap or ambiguity and must not be bypassed.
4. Run declared bounded operations only through `senpai run`. Read the capsule declaration first when parameters, scope, or its advisory `access` matter. Treat run output as potentially scrubbed diagnostics; diagnose failures from it rather than running `doctor` to probe credentials.
5. For tickets and code hosting, query the route/target and effective workflow policy first. Check the policy before loading the configured workflow skill. Then use `senpai-project-management` or `senpai-code-hosting`; a declared source/instance `skill` replaces only its technical adapter.

## Authorization and authentication

The resolved configuration is the target and authorization boundary. Workflow instructions may narrow a policy but never broaden it. `allow` may proceed, `confirm` needs explicit confirmation for the concrete write, and `deny` must not be attempted. Reads default to allowed; omitted write capabilities default to denied in the effective policy returned by the CLI.

Never ask for, read, print, or paste a secret value. For an `env` auth mode, pass only the declared variable *name* to an adapter; it reads the value itself. For `interactive`, hand control to the user. If authentication cannot work without the agent receiving a secret value, stop and ask the user how to proceed. Remind users that a variable newly defined in a shell requires an agent restart to be inherited.

## Network-aware execution

Adapters and CLIs targeting declared tracker or hosting URLs use outbound network. Request environment-level network approval before the call when needed; it is separate from workflow policy. Do not trust empty output.

## Scoped command map

- Ticket source: `senpai get ticket-route --id <id>`; role-driven tracker: `senpai get tracker --role <role>`.
- Hosting target: `senpai get hosting --role <role> --repo <repo>`.
- Workflow: `senpai get workflow --domain tickets|code_changes`.
- Repository: `senpai get repo --current` or `--path`; add `--with-dependencies` for an ordered multi-repo change.
- Environment, documentation, rules, and capsules: the corresponding `get` or `list` command in `senpai --help` / the CLI contract.
