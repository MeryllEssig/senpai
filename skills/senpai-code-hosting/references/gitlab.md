# GitLab adapter

Use the official `glab` CLI. Target the declared GitLab host explicitly (for
example with `--hostname` where supported or an isolated `glab auth login`
entry for that host) and use the selected repo path from the manifest mapping.
Do not rely on the current Git remote when several synchronized instances exist.

Useful common mappings: `glab mr view/list` (read), `glab mr create` (create),
`glab mr update` (update), `glab mr note` (comment), `glab mr merge` (merge),
and `glab pipeline list/view/run` (pipeline operations; command availability
depends on installed glab version). Check `glab auth status` before mutation
when appropriate. For env auth, run the documented host-scoped login flow only
when the token can be consumed locally without appearing in the transcript;
otherwise request user action. Interactive login must be completed by the user.
