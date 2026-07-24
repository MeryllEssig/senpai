# GitHub adapter

Use the official `gh` CLI. Select the declared host explicitly with
`--hostname <host>` or a per-command `GH_HOST=<host>` environment setting;
do not assume github.com for GitHub Enterprise. The repo path comes from the
selected manifest repository's `integrations` mapping.

Useful common mappings: `gh pr view/list` (read), `gh pr create` (create),
`gh pr edit` (update/reviewers), `gh pr comment` (comment), `gh pr merge`
(merge), `gh run list/view` (pipeline read), and `gh workflow run` (trigger).
Use `gh auth status --hostname <host>` as a read-only check. For env auth,
`gh auth login --with-token` consumes stdin and is only suitable when a safe
local mechanism supplies the token without exposing it to the agent; otherwise
ask the user to authenticate interactively themselves.
