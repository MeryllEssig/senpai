---
name: senpai-use-project-context
description: Use a project's SenpAI manifest to answer requests requiring declared project context or bounded operations, including a ticket identified from the current Git branch, forges, repositories, environments, documentation, logs, databases, tests, builds, and other capsules. Invoke manually when automatic skill selection is unavailable.
---

# Use project context

Respond in the user's language. This skill is the session orchestrator; it does not replace project workflow instructions or platform adapters.

1. From the session launch directory, run `senpai resolve --json` once. If it fails, stop: explain that no manifest was resolved and offer setup. Never change directory to find another manifest.
2. Run `senpai summary --json`. Use scoped `get` or `list` commands after that; never dump the manifest.
3. Query ticket work with `senpai resolve operation ticket.<operation> --ticket <id>`, pull/merge request work with `senpai resolve operation pull_merge_request.<operation> --repo <id>`, and pipeline work with `senpai resolve operation pipeline.<operation> --repo <id>`. Read a selected job only with `senpai pipeline job-log --repo <id> --pipeline <pipeline-id> --job <job-id>`. A forge is a code-development platform such as GitHub or GitLab. Use `--integration` only to settle a reported ambiguity; never infer an undeclared target.
4. Run declared bounded operations only through `senpai run`. Read the capsule declaration first when its parameters or scope matter. Treat run output as potentially scrubbed diagnostics; diagnose failures from it rather than running `doctor` to probe credentials.
5. For tickets and forges, use this single resolution result before loading its workflow or adapter. Check its local policy for every call; a workflow cannot broaden it or select another integration.

## Ticket context from the current branch

When the user asks for the ticket of “this/current branch”, treat it as a request to retrieve ticket context before doing the requested work; do not substitute Git history for that context.

1. After resolving the manifest and reading the summary, identify the checked-out branch with `git branch --show-current`. If it is empty (detached HEAD), say that a branch ticket cannot be inferred and ask for a ticket id; do not select a commit message as a replacement.
2. Extract only explicit ticket-looking references from the branch name (for example `ACME-42` or `#42`), preserving their original form. Try each distinct candidate with `senpai resolve operation ticket.view --ticket <candidate> --json`; the manifest’s declared routing patterns and priority decide whether it is a valid target. Never choose an integration or tracker URL directly.
3. If exactly one candidate resolves, inspect its returned policy, load its selected workflow and adapter, then retrieve the ticket through `senpai-project-management`. Use the returned ticket details as the task context before starting a code review, implementation, or other follow-on task.
4. If no candidate resolves, if several distinct candidates resolve, or if the branch has no explicit reference, say precisely which condition occurred and ask for the ticket id. Only then may Git branch/commit context be used as supplementary context, never as an assertion that SenpAI or Redmine was unavailable.
5. Do not claim that ticket details are inaccessible because of authentication until manifest resolution, operation resolution, policy checking, and the selected adapter request have actually been attempted. Report the actual failed boundary (no manifest, no routed candidate, denied policy, missing declared authentication, network failure, or adapter response) without exposing secrets.

## Authorization and authentication

The resolved configuration is the target and authorization boundary. Workflow instructions may narrow a policy but never broaden it. `allow` may proceed, `confirm` needs explicit confirmation for the concrete write, and `deny` must not be attempted. View operations default to allowed; omitted write operations default to denied in the effective policy returned by the CLI.

Never ask for, read, print, or paste a secret value. For an `env` auth mode, pass only the declared variable *name* to an adapter; it reads the value itself. For `interactive`, hand control to the user. If authentication cannot work without the agent receiving a secret value, stop and ask the user how to proceed. Remind users that a variable newly defined in a shell requires an agent restart to be inherited.

## Network-aware execution

Adapters and CLIs targeting declared tracker or hosting URLs use outbound network. Request environment-level network approval before the call when needed; it is separate from workflow policy. Do not trust empty output.

## Scoped command map

- Ticket target: `senpai resolve operation ticket.<operation> --ticket <id>`.
- Pull/merge request target: `senpai resolve operation pull_merge_request.<operation> --repo <repo>`.
- Pipeline target: `senpai resolve operation pipeline.<operation> --repo <repo>`.
- Repository: `senpai get repo --current` or `--path`; add `--with-dependencies` for an ordered multi-repo change.
- Environment, documentation, and capsules: the corresponding `get` or `list` command in `senpai --help` / the CLI contract.
