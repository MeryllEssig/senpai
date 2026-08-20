#!/usr/bin/env bash
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)

fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
assert_contains() { grep -Fq -- "$2" "$1" || fail "expected $2 in $1"; }

context_skill="$root/skills/senpai-use-project-context/SKILL.md"
gitlab_skill="$root/skills/senpai-code-hosting/references/gitlab.md"

# This is a delivery contract: branch-scoped ticket requests must resolve
# through SenpAI before any Git-context fallback or auth conclusion.
assert_contains "$context_skill" 'Ticket context from the current branch'
assert_contains "$context_skill" 'senpai resolve operation ticket.view --ticket <candidate> --json'
assert_contains "$context_skill" 'retrieve the ticket through `senpai-project-management`'
assert_contains "$context_skill" 'do not substitute Git history for that context'
assert_contains "$context_skill" 'until manifest resolution, operation resolution, policy checking, and the selected adapter request have actually been attempted'

# A merge-request creation must stay in the local Git repository declared by
# SenpAI. Otherwise glab can derive a different source project and leave a
# temporary tracking remote in the caller's repository.
assert_contains "$gitlab_skill" 'run every `git` and `glab` command from that exact directory'
assert_contains "$gitlab_skill" 'Verify that it is the Git worktree root'
assert_contains "$gitlab_skill" '`senpai get repo --current --json` returns the same repository id'
assert_contains "$gitlab_skill" 'do not add, replace, or use a temporary remote to make the command work'
assert_contains "$gitlab_skill" '`glab mr create` derives the MR source project from its current Git repository'
assert_contains "$gitlab_skill" 'pass the resolved `--repo <route.repository>` and declared host explicitly'
assert_contains "$gitlab_skill" 'verify that none was added or changed afterwards'
assert_contains "$gitlab_skill" 'Never pass a different project as its target'

printf 'skill tests passed\n'
