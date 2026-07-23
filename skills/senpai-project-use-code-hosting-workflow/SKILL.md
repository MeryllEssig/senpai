---
name: senpai-project-use-code-hosting-workflow
description: Default read-only SenpAI code-hosting workflow. Use after SenpAI has selected a hosting instance and checked the effective code-change policy when the project declares no custom workflow.
---

# Default code-hosting workflow

This default workflow is intentionally read-only. Use it only after selecting the repository and role-qualified hosting target, then use `senpai-code-hosting` to inspect merge requests or pipelines.

Do not create or update merge requests, comment, request review, merge, or trigger pipelines. The default policy denies every write. If the user needs a write, explain that the project needs an explicit code-change workflow policy and instructions; service-level permissions do not create SenpAI permission.
