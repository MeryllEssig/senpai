---
name: senpai-project-use-ticket-workflow
description: Default read-only Senpai ticket workflow. Use after Senpai has selected a ticket source and checked the effective ticket policy when the project declares no custom ticket workflow.
---

# Default ticket workflow

This default workflow is intentionally read-only. Use it only after
`senpai get workflow --domain tickets` and source routing. Read ticket details,
comments, and permitted metadata through `senpai-project-management`, then
report source and ticket id clearly.

Do not create, comment, transition, link, or log time. The default policy
denies every ticket write. If the user needs one, explain that the project must
declare an explicit ticket workflow policy and instructions; do not infer
permission from the target service account.
