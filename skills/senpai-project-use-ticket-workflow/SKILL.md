---
name: senpai-project-use-ticket-workflow
description: Default read-only SenpAI ticket workflow. Use after SenpAI has resolved a ticketing integration and checked its effective local policy when the project declares no custom workflow.
---

# Default ticket workflow

This default workflow is intentionally read-only. Use it only after `senpai resolve operation ticket.read --ticket <id>` selected an integration. Read ticket details and permitted metadata through `senpai-project-management`, then report the integration and ticket id clearly.

Do not create, comment, transition, link, or log time. The default policy denies every ticket write. If the user needs one, explain that the project must declare an explicit ticket workflow policy and instructions; do not infer permission from the target service account.
