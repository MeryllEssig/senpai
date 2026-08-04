---
name: senpai-code-hosting
description: Perform common SenpAI forge operations after a v2 integration resolution, using a focused GitHub, GitLab, or custom adapter.
---

# Code-hosting interface

Use only after `senpai resolve operation pull_merge_request.* --repo <id>` or `senpai resolve operation pipeline.* --repo <id>` selected one forge integration. A forge is a code-development platform such as GitHub or GitLab. Pull/merge request operations are `pull_merge_request.view`, `pull_merge_request.create`, `pull_merge_request.edit`, `pull_merge_request.comment`, `pull_merge_request.request_review`, and `pull_merge_request.merge`. Pipeline operations are `pipeline.view`, `pipeline.job.view_log`, and `pipeline.trigger`. The selected integration must be declared on the repository; do not substitute another mirror.

Use the returned policy decision. `allow` may proceed; `confirm` requires explicit confirmation; `deny` stops. Load the returned workflow only after this check; it cannot broaden policy. The returned adapter completely selects the technical adapter.

## Shipped adapter guidance

Read [GitHub](references/github.md) for `platform: github` and [GitLab](references/gitlab.md) for `platform: gitlab`. They intentionally use the established `gh` and `glab` CLIs rather than hide their commands.

Never pass a token as a CLI argument or chat text. With `auth.mode=env`, let the adapter's documented login/token mechanism read the declared variable by name. With `preconfigured`, verify the host/account non-destructively if useful. With `interactive`, start the documented login flow only when the user has asked you to proceed, then hand it over. If auth cannot work without reading a secret, stop and ask the user how to proceed.

`gh` and `glab` use outbound network. Request any environment-level approval before the command; it is separate from SenpAI policy. Do not trust empty or incomplete CLI output.

## Failed job logs

If the user gives a GitHub or GitLab job URL, call `senpai pipeline job-log --repo <id> --job <url> --json` after resolving `pipeline.job.view_log`. Otherwise, resolve `pipeline.view`, identify the current Git branch, and inspect at most 20 failed pipelines for that branch. Select the newest matching pipeline. If none exists, ask for a branch, pipeline id, or URL; never fall back to another branch automatically. List the failed jobs in the selected pipeline: open the only failed job, or ask the user to choose when there are several. Then call `senpai pipeline job-log --repo <id> --pipeline <pipeline-id> --job <job-id> --json`. The native command returns only the final bounded log window; do not stream `gh` or `glab` logs directly.

For a galaxy, query repositories with dependencies before preparing coordinated changes. Create one merge request per modified repo on its role-correct instance; do not collapse multiple repos into one MR. Deployment and test pipeline roles can deliberately resolve to different instances.
