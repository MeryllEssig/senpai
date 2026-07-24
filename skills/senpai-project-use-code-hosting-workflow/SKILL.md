---
name: senpai-project-use-code-hosting-workflow
description: Default read-only SenpAI forge workflow. Use after SenpAI has resolved a forge integration and checked its effective local policy when the project declares no custom workflow.
---

# Default code-hosting workflow

This default workflow is intentionally read-only. Use it only after `senpai resolve operation code.read --repo <id>` selected an integration, then use `senpai-code-hosting` to inspect merge requests or pipelines.

Do not create or update merge requests, comment, request review, merge, or trigger pipelines. The default policy denies every write. If the user needs a write, explain that the project needs an explicit code-change workflow policy and instructions; service-level permissions do not create SenpAI permission.
