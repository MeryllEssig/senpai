#!/usr/bin/env bash
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)

fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
assert_contains() { grep -Fq -- "$2" "$1" || fail "expected $2 in $1"; }

context_skill="$root/skills/senpai-use-project-context/SKILL.md"

# This is a delivery contract: branch-scoped ticket requests must resolve
# through SenpAI before any Git-context fallback or auth conclusion.
assert_contains "$context_skill" 'Ticket context from the current branch'
assert_contains "$context_skill" 'senpai resolve operation ticket.view --ticket <candidate> --json'
assert_contains "$context_skill" 'retrieve the ticket through `senpai-project-management`'
assert_contains "$context_skill" 'do not substitute Git history for that context'
assert_contains "$context_skill" 'until manifest resolution, operation resolution, policy checking, and the selected adapter request have actually been attempted'

printf 'skill tests passed\n'
