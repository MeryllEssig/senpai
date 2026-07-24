---
name: senpai-code-hosting
description: Perform common SenpAI forge operations after a v2 integration resolution, using a focused GitHub, GitLab, or custom adapter.
---

# Code-hosting interface

Use only after `senpai resolve operation code.* --repo <id>` selected one forge integration. A forge is a code-development platform such as GitHub or GitLab. Operations: `read`, `create`, `update`, `comment`, `request_review`, `merge`, `pipeline_read`, `pipeline_trigger`. The selected integration must be declared on the repository; do not substitute another mirror.

Use the returned policy decision. `allow` may proceed; `confirm` requires explicit confirmation; `deny` stops. Load the returned workflow only after this check; it cannot broaden policy. The returned adapter completely selects the technical adapter.

## Shipped adapter guidance

Read [GitHub](references/github.md) for `platform: github` and [GitLab](references/gitlab.md) for `platform: gitlab`. They intentionally use the established `gh` and `glab` CLIs rather than hide their commands.

Never pass a token as a CLI argument or chat text. With `auth.mode=env`, let the adapter's documented login/token mechanism read the declared variable by name. With `preconfigured`, verify the host/account non-destructively if useful. With `interactive`, start the documented login flow only when the user has asked you to proceed, then hand it over. If auth cannot work without reading a secret, stop and ask the user how to proceed.

`gh` and `glab` use outbound network. Request any environment-level approval before the command; it is separate from SenpAI policy. Do not trust empty or incomplete CLI output.

For a galaxy, query repositories with dependencies before preparing coordinated changes. Create one merge request per modified repo on its role-correct instance; do not collapse multiple repos into one MR. Deployment and test pipeline roles can deliberately resolve to different instances.
