---
name: senpai-code-hosting
description: Perform common Senpai code-hosting operations—read or create/update/comment/review/merge merge requests and read or trigger pipelines—using a role-qualified declared hosting instance and focused GitHub, GitLab, or custom adapter.
---

# Code-hosting interface

Use only after Senpai has resolved the target repository, selected hosting by
role, and returned the effective `code_changes` workflow policy. Operations:
`read`, `create`, `update`, `comment`, `request_review`, `merge`,
`pipeline_read`, `pipeline_trigger`. Verify that the selected hosting instance
is declared for the target repo; do not substitute another mirror.

Map each operation directly to its policy capability. `allow` may proceed;
`confirm` requires explicit confirmation describing the concrete action and
target; `deny` stops. Load the configured workflow only after this check; it
describes procedure and cannot broaden permissions. A declared instance
`skill` completely replaces the technical adapter.

## Shipped adapter guidance

Read [GitHub](references/github.md) for `platform: github` and
[GitLab](references/gitlab.md) for `platform: gitlab`. They intentionally use
the established `gh` and `glab` CLIs rather than hide their commands.

Never pass a token as a CLI argument or chat text. With `auth.mode=env`, let
the adapter's documented login/token mechanism read the declared variable by
name. With `preconfigured`, verify the host/account non-destructively if useful.
With `interactive`, start the documented login flow only when the user has
asked you to proceed, then hand it over. If auth cannot work without reading a
secret, stop and ask the user how to proceed.

For a galaxy, query repositories with dependencies before preparing coordinated
changes. Create one merge request per modified repo on its role-correct
instance; do not collapse multiple repos into one MR. Deployment and test
pipeline roles can deliberately resolve to different instances.
