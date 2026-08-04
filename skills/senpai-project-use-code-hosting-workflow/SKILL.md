---
name: senpai-project-use-code-hosting-workflow
description: Default read-only SenpAI forge workflow. Use after SenpAI has resolved a forge integration and checked its effective local policy when the project declares no custom workflow.
---

# Default code-hosting workflow

This default workflow is intentionally view-only. Use it only after `senpai resolve operation pull_merge_request.view --repo <id>`, `senpai resolve operation pipeline.view --repo <id>`, or `senpai resolve operation pipeline.job.view_log --repo <id>` selected an integration, then use `senpai-code-hosting` to inspect pull/merge requests, pipelines, or one bounded job log.

Do not create or edit pull/merge requests, comment, request review, merge, or trigger pipelines. The default policy denies every write operation. If the user needs one, explain that the project needs an explicit pull/merge request or pipeline workflow policy and instructions; service-level permissions do not create SenpAI permission.
